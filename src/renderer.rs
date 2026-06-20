use std::io::Write;

use anyhow::Result;
use crossterm::{
    cursor, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
};

use crate::pane::Pane;
use crate::pane_manager::PaneManager;

#[derive(Clone, PartialEq, Default)]
struct Attrs {
    fg: vt100::Color,
    bg: vt100::Color,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

#[derive(Clone, PartialEq)]
struct Cell {
    content: String,
    attrs: Attrs,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { content: " ".to_string(), attrs: Attrs::default() }
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

    fn set(&mut self, row: u16, col: u16, cell: Cell) {
        if row < self.rows && col < self.cols {
            self.cells[(row * self.cols + col) as usize] = cell;
        }
    }

    fn set_text(&mut self, row: u16, col: u16, content: impl Into<String>) {
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
        scroll_offset: usize,
    ) -> Result<()> {
        let mut next = FrameBuffer::new(manager.rows, manager.cols);

        paint_pane(&mut next, &manager.panes[manager.active], scroll_offset);
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
        let mut current_attrs = Attrs::default();

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

                if new_cell.attrs != current_attrs {
                    apply_attrs(out, &new_cell.attrs)?;
                    current_attrs = new_cell.attrs.clone();
                }

                queue!(out, Print(&new_cell.content))?;
                cursor = Some((row, col + 1));
            }
        }

        // Leave the terminal in a clean default state after each frame.
        if current_attrs != (Attrs::default()) {
            queue!(out, SetAttribute(Attribute::Reset))?;
        }

        Ok(())
    }
}

fn apply_attrs<W: Write>(out: &mut W, attrs: &Attrs) -> Result<()> {
    queue!(out, SetAttribute(Attribute::Reset))?;
    queue!(out, SetForegroundColor(vt100_color_to_crossterm(attrs.fg)))?;
    queue!(out, SetBackgroundColor(vt100_color_to_crossterm(attrs.bg)))?;
    if attrs.bold {
        queue!(out, SetAttribute(Attribute::Bold))?;
    }
    if attrs.italic {
        queue!(out, SetAttribute(Attribute::Italic))?;
    }
    if attrs.underline {
        queue!(out, SetAttribute(Attribute::Underlined))?;
    }
    if attrs.inverse {
        queue!(out, SetAttribute(Attribute::Reverse))?;
    }
    Ok(())
}

fn vt100_color_to_crossterm(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(n) => Color::AnsiValue(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb { r, g, b },
    }
}

fn paint_pane(buf: &mut FrameBuffer, pane: &Pane, scroll_offset: usize) {
    let mut parser = pane.parser.lock().unwrap();
    parser.screen_mut().set_scrollback(scroll_offset);
    {
        let screen = parser.screen();
        for row in 0..pane.height {
            for col in 0..pane.width {
                let cell = match screen.cell(row, col) {
                    Some(c) if c.is_wide_continuation() => Cell::default(),
                    Some(c) => {
                        let s = c.contents();
                        let content =
                            if s.is_empty() { " ".to_string() } else { s.to_string() };
                        let attrs = Attrs {
                            fg: c.fgcolor(),
                            bg: c.bgcolor(),
                            bold: c.bold(),
                            italic: c.italic(),
                            underline: c.underline(),
                            inverse: c.inverse(),
                        };
                        Cell { content, attrs }
                    }
                    None => Cell::default(),
                };
                buf.set(row, col, cell);
            }
        }
    }
    parser.screen_mut().set_scrollback(0);
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
            buf.set_text(row, col, ch.to_string());
            col += 1;
        }
        col += 1;
    }
}

fn paint_prompt(buf: &mut FrameBuffer, manager: &PaneManager, text: &str) {
    let row = manager.rows.saturating_sub(1);
    for (col, ch) in text.chars().enumerate() {
        buf.set_text(row, col as u16, ch.to_string());
    }
}

