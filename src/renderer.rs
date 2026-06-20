use std::io::Write;

use anyhow::Result;
use crossterm::{cursor, queue, style::Print};

use crate::pane::Pane;
use crate::pane_manager::PaneManager;

#[derive(Clone, PartialEq)]
struct Cell {
    content: String,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { content: " ".to_string() }
    }
}

struct FrameBuffer {
    rows: u16,
    cols: u16,
    cells: Vec<Cell>,
}

impl FrameBuffer {
    fn new(rows: u16, cols: u16) -> Self {
        FrameBuffer { rows, cols, cells: vec![Cell::default(); (rows * cols) as usize] }
    }

    fn set(&mut self, row: u16, col: u16, content: impl Into<String>) {
        if row < self.rows && col < self.cols {
            self.cells[(row * self.cols + col) as usize].content = content.into();
        }
    }

    fn get(&self, row: u16, col: u16) -> &Cell {
        &self.cells[(row * self.cols + col) as usize]
    }
}

pub struct Renderer {
    prev: FrameBuffer,
}

impl Renderer {
    pub fn new(cols: u16, rows: u16) -> Self {
        Renderer { prev: FrameBuffer::new(rows, cols) }
    }

    pub fn invalidate(&mut self, cols: u16, rows: u16) {
        self.prev = FrameBuffer::new(rows, cols);
    }

    pub fn draw<W: Write>(
        &mut self,
        out: &mut W,
        manager: &PaneManager,
        prompt: Option<&str>,
    ) -> Result<()> {
        let mut next = FrameBuffer::new(manager.rows, manager.cols);

        paint_pane(&mut next, &manager.panes[manager.active]);
        match prompt {
            Some(text) => paint_prompt(&mut next, manager, text),
            None => paint_tab_bar(&mut next, manager),
        }

        self.flush_diff(out, &next)?;
        self.prev = next;
        Ok(())
    }

    fn flush_diff<W: Write>(&self, out: &mut W, next: &FrameBuffer) -> Result<()> {
        let mut cursor: Option<(u16, u16)> = None;

        for row in 0..next.rows {
            for col in 0..next.cols {
                let new_cell = next.get(row, col);
                let old_cell = if row < self.prev.rows && col < self.prev.cols {
                    self.prev.get(row, col)
                } else {
                    &Cell::default()
                };

                if new_cell == old_cell {
                    cursor = None;
                    continue;
                }

                if cursor != Some((row, col)) {
                    queue!(out, cursor::MoveTo(col, row))?;
                }
                queue!(out, Print(&new_cell.content))?;
                cursor = Some((row, col + 1));
            }
        }
        Ok(())
    }
}

fn paint_pane(buf: &mut FrameBuffer, pane: &Pane) {
    let parser = pane.parser.lock().unwrap();
    let screen = parser.screen();
    for row in 0..pane.height {
        for col in 0..pane.width {
            let content = match screen.cell(row, col) {
                Some(c) if c.is_wide_continuation() => " ".to_string(),
                Some(c) => {
                    let s = c.contents();
                    if s.is_empty() { " ".to_string() } else { s.to_string() }
                }
                None => " ".to_string(),
            };
            buf.set(row, col, content);
        }
    }
}

fn paint_tab_bar(buf: &mut FrameBuffer, manager: &PaneManager) {
    let row = manager.rows.saturating_sub(1);
    let mut col = 1u16;
    for (i, pane) in manager.panes.iter().enumerate() {
        let label = if i == manager.active {
            format!("[*{}:{}]", i + 1, pane.name)
        } else {
            format!("[{}:{}]", i + 1, pane.name)
        };
        for ch in label.chars() {
            buf.set(row, col, ch.to_string());
            col += 1;
        }
        col += 1;
    }
}

fn paint_prompt(buf: &mut FrameBuffer, manager: &PaneManager, input: &str) {
    let row = manager.rows.saturating_sub(1);
    let prompt = format!("New tab name: {}_", input);
    for (col, ch) in prompt.chars().enumerate() {
        buf.set(row, col as u16, ch.to_string());
    }
}
