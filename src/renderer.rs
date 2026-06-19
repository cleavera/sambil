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

    /// Invalidates the previous frame so the next draw does a full repaint.
    /// Call this after a terminal resize.
    pub fn invalidate(&mut self, cols: u16, rows: u16) {
        self.prev = FrameBuffer::new(rows, cols);
    }

    pub fn draw<W: Write>(&mut self, out: &mut W, manager: &PaneManager) -> Result<()> {
        let mut next = FrameBuffer::new(manager.rows, manager.cols);

        for pane in &manager.panes {
            paint_pane(&mut next, pane);
        }
        paint_border(&mut next, manager.cols, manager.rows);
        paint_status_bar(&mut next, manager);

        self.flush_diff(out, &next)?;
        self.prev = next;
        Ok(())
    }

    fn flush_diff<W: Write>(&self, out: &mut W, next: &FrameBuffer) -> Result<()> {
        // Track where the terminal cursor is to avoid redundant MoveTo calls.
        let mut cursor: Option<(u16, u16)> = None;

        for row in 0..next.rows {
            for col in 0..next.cols {
                let new_cell = next.get(row, col);
                // prev may have different dimensions after a resize — treat missing as default.
                let old_cell = if row < self.prev.rows && col < self.prev.cols {
                    self.prev.get(row, col)
                } else {
                    &Cell::default()
                };

                if new_cell == old_cell {
                    cursor = None; // next write will need an explicit MoveTo
                    continue;
                }

                if cursor != Some((row, col)) {
                    queue!(out, cursor::MoveTo(col, row))?;
                }
                queue!(out, Print(&new_cell.content))?;
                // Cursor is now one column to the right.
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
            buf.set(row, pane.col_start + col, content);
        }
    }
}

fn paint_border(buf: &mut FrameBuffer, cols: u16, rows: u16) {
    let border_col = cols / 2;
    for row in 0..rows.saturating_sub(1) {
        buf.set(row, border_col, "│");
    }
}

fn paint_status_bar(buf: &mut FrameBuffer, manager: &PaneManager) {
    let label = format!(" pane {} active ", manager.active + 1);
    let row = manager.rows.saturating_sub(1);
    for (i, ch) in label.chars().enumerate() {
        buf.set(row, i as u16, ch.to_string());
    }
}
