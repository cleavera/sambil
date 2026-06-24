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
    prev_show_help: bool,
}

impl Renderer {
    pub fn new(cols: u16, rows: u16) -> Self {
        Renderer { prev: FrameBuffer::new(rows, cols), prev_show_help: false }
    }

    pub fn invalidate(&mut self, cols: u16, rows: u16) {
        self.prev = FrameBuffer::new(rows, cols);
        self.prev_show_help = false;
    }

    pub fn draw<W: Write>(
        &mut self,
        out: &mut W,
        manager: &PaneManager,
        prompt: Option<&str>,
        scroll_offset: usize,
        show_help: bool,
        leader: &str,
    ) -> Result<()> {
        // When toggling in or out of the help overlay, clear the physical
        // terminal and invalidate the diff buffer so every cell is re-emitted.
        if show_help != self.prev_show_help {
            queue!(out, crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
            self.prev = FrameBuffer::new(manager.rows, manager.cols);
            self.prev_show_help = show_help;
        }

        let mut next = FrameBuffer::new(manager.rows, manager.cols);

        if show_help {
            paint_help(&mut next, manager, leader);
        } else {
            paint_active_tab(&mut next, manager, scroll_offset);
        }
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

fn paint_active_tab(buf: &mut FrameBuffer, manager: &PaneManager, scroll_offset: usize) {
    let tab = &manager.tabs[manager.active_tab];
    let n = tab.panes.len();
    let mid_row = (buf.rows + 1) / 2; // vertical midpoint of content area
    let mut col_offset = 0u16;
    for (i, pane) in tab.panes.iter().enumerate() {
        let offset = if i == tab.active_pane { scroll_offset } else { 0 };
        paint_pane(buf, pane, col_offset, offset);
        col_offset += pane.width;
        if i + 1 < n {
            // The indicator points toward the active pane.
            let indicator = if i == tab.active_pane {
                "◀"
            } else if i + 1 == tab.active_pane {
                "▶"
            } else {
                "│"
            };
            for row in 1..buf.rows {
                let (content, fg) = if row == mid_row && indicator != "│" {
                    (indicator, vt100::Color::Idx(15))
                } else {
                    ("│", vt100::Color::Idx(8))
                };
                buf.set(row, col_offset, Cell {
                    content: content.to_string(),
                    attrs: Attrs { fg, ..Attrs::default() },
                });
            }
            col_offset += 1;
        }
    }
}

fn paint_pane(buf: &mut FrameBuffer, pane: &Pane, col_offset: u16, scroll_offset: usize) {
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
                buf.set(row + 1, col + col_offset, cell);
            }
        }
    }
    parser.screen_mut().set_scrollback(0);
}

fn paint_tab_bar(buf: &mut FrameBuffer, manager: &PaneManager) {
    let row = 0;
    let bar_bg = vt100::Color::Default;
    let active_fg = vt100::Color::Idx(15);
    let active_bg = vt100::Color::Idx(8);
    let inactive_fg = vt100::Color::Idx(8);

    // Fill entire row with bar background first.
    for col in 0..manager.cols {
        buf.set(row, col, Cell {
            content: " ".to_string(),
            attrs: Attrs { bg: bar_bg, ..Attrs::default() },
        });
    }

    let mut col = 1u16;
    for (i, tab) in manager.tabs.iter().enumerate() {
        let is_active = i == manager.active_tab;
        let indicator = if is_active { "●".to_string() } else { (i + 1).to_string() };
        let label = format!(" [{}:{}] ", indicator, tab.display_name());
        let attrs = Attrs {
            fg: if is_active { active_fg } else { inactive_fg },
            bg: if is_active { active_bg } else { bar_bg },
            bold: is_active,
            ..Attrs::default()
        };
        for ch in label.chars() {
            if col >= manager.cols { break; }
            buf.set(row, col, Cell { content: ch.to_string(), attrs: attrs.clone() });
            col += 1;
        }
    }

    // Undo hint — right-aligned when a closed tab is pending.
    if manager.has_pending_close() {
        let hint = " ↩ u ";
        let hint_col = manager.cols.saturating_sub(hint.chars().count() as u16);
        let hint_attrs = Attrs { fg: vt100::Color::Idx(11), ..Attrs::default() };
        for (offset, ch) in hint.chars().enumerate() {
            let c = hint_col + offset as u16;
            if c < manager.cols {
                buf.set(row, c, Cell { content: ch.to_string(), attrs: hint_attrs.clone() });
            }
        }
    }
}

fn paint_prompt(buf: &mut FrameBuffer, _manager: &PaneManager, text: &str) {
    let row = 0;
    for (col, ch) in text.chars().enumerate() {
        buf.set_text(row, col as u16, ch.to_string());
    }
}

fn paint_help(buf: &mut FrameBuffer, manager: &PaneManager, leader: &str) {
    // Format "ctrl+b" → "Ctrl-b", "ctrl+space" → "Ctrl-space"
    let display = leader
        .to_lowercase()
        .replacen("ctrl+", "Ctrl-", 1);

    let bindings = [
        ("c",    "New tab (cwd name)"),
        ("C",    "New tab (enter name)"),
        ("|",    "Split horizontal"),
        ("x",    "Close pane (tab if last)"),
        ("u",    "Undo close tab"),
        ("r",    "Rename tab"),
        ("n",    "Next tab"),
        ("p",    "Previous tab"),
        ("←/→",  "Previous/next pane"),
        ("1-9",  "Switch to tab N"),
        ("[",    "Scroll mode"),
        ("q",    "Quit"),
        ("?",    "Show this help"),
    ];

    let lines: Vec<String> = std::iter::once(String::new())
        .chain(std::iter::once("  Sambil Key Bindings".to_string()))
        .chain(std::iter::once(format!("  {}", "─".repeat(39))))
        .chain(bindings.iter().map(|(key, desc)| {
            format!("  {} {}  {}", display, key, desc)
        }))
        .chain(std::iter::once(format!("  {}", "─".repeat(39))))
        .chain(std::iter::once("  Press any key to dismiss".to_string()))
        .collect();

    for (i, line) in lines.iter().enumerate() {
        let row = (i as u16) + 1;
        if row >= manager.rows {
            break;
        }
        for (col, ch) in line.chars().enumerate() {
            buf.set_text(row, col as u16, ch.to_string());
        }
    }
}
