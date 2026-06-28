use std::io::Write;

use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
};
use unicode_width::UnicodeWidthStr;

use crate::cell::CellContent;
use crate::cursor::CursorStyle;
use crate::pane::Pane;
use crate::pane_manager::PaneManager;
use crate::scroll::ScrollOffset;
use crate::size::{ColOffset, TerminalSize};

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
    content: CellContent,
    attrs: Attrs,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { content: CellContent::default(), attrs: Attrs::default() }
    }
}

struct FrameBuffer {
    size: TerminalSize,
    cells: Vec<Cell>,
}

impl FrameBuffer {
    fn new(size: TerminalSize) -> Self {
        FrameBuffer { size, cells: vec![Cell::default(); size.rows() as usize * size.cols() as usize] }
    }

    fn set(&mut self, row: u16, col: u16, cell: Cell) {
        if row < self.size.rows() && col < self.size.cols() {
            self.cells[row as usize * self.size.cols() as usize + col as usize] = cell;
        }
    }

    fn set_text(&mut self, row: u16, col: u16, content: CellContent) {
        if row < self.size.rows() && col < self.size.cols() {
            self.cells[row as usize * self.size.cols() as usize + col as usize].content = content;
        }
    }

    fn get(&self, row: u16, col: u16) -> Option<&Cell> {
        if row < self.size.rows() && col < self.size.cols() {
            Some(&self.cells[row as usize * self.size.cols() as usize + col as usize])
        } else {
            None
        }
    }
}

pub struct Renderer {
    prev: FrameBuffer,
    prev_show_help: bool,
}

impl Renderer {
    pub fn new(size: TerminalSize) -> Self {
        Renderer { prev: FrameBuffer::new(size), prev_show_help: false }
    }

    pub fn invalidate(&mut self, size: TerminalSize) {
        self.prev = FrameBuffer::new(size);
        self.prev_show_help = false;
    }

    pub fn draw<W: Write>(
        &mut self,
        out: &mut W,
        manager: &PaneManager,
        prompt: Option<&str>,
        scroll_offset: ScrollOffset,
        show_help: bool,
        leader: &str,
    ) -> Result<()> {
        if show_help != self.prev_show_help {
            queue!(out, crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
            self.prev = FrameBuffer::new(manager.size);
            self.prev_show_help = show_help;
        }

        let mut next = FrameBuffer::new(manager.size);

        if show_help {
            paint_help(&mut next, manager, leader);
        } else {
            paint_active_tab(&mut next, manager, scroll_offset);
        }
        match prompt {
            Some(text) => paint_prompt(&mut next, manager, text),
            None => paint_tab_bar(&mut next, manager),
        }

        queue!(out, Hide)?;
        self.flush_diff(out, &next)?;
        self.prev = next;

        if show_help || prompt.is_some() {
            queue!(out, Hide)?;
        } else {
            let col_offset = manager.active_pane_col_offset();
            let tab = manager.tabs.active();
            let active_pane = &tab.panes[tab.active_pane];
            let (cur_row, cur_col, hide, ps) = {
                let parser = active_pane.parser.lock().unwrap();
                let screen = parser.screen();
                let (r, c) = screen.cursor_position();
                let hide = screen.hide_cursor();
                let ps = parser.callbacks().cursor_style;
                (r, c, hide, ps)
            };
            if hide {
                queue!(out, Hide)?;
            } else {
                queue!(out, MoveTo(cur_col + u16::from(col_offset), cur_row + 1))?;
                queue!(out, cursor_style_to_crossterm(ps))?;
                queue!(out, Show)?;
            }
        }

        Ok(())
    }

    fn flush_diff<W: Write>(&self, out: &mut W, next: &FrameBuffer) -> Result<()> {
        let mut cursor: Option<(u16, u16)> = None;
        let mut current_attrs = Attrs::default();

        for row in 0..next.size.rows() {
            for col in 0..next.size.cols() {
                let new_cell = match next.get(row, col) {
                    Some(c) => c,
                    None => continue,
                };
                let default_cell = Cell::default();
                let old_cell = self.prev.get(row, col).unwrap_or(&default_cell);

                if new_cell == old_cell {
                    cursor = None;
                    continue;
                }

                if cursor != Some((row, col)) {
                    queue!(out, MoveTo(col, row))?;
                }

                if new_cell.attrs != current_attrs {
                    apply_attrs(out, &new_cell.attrs)?;
                    current_attrs = new_cell.attrs.clone();
                }

                queue!(out, Print(new_cell.content.as_str()))?;
                let char_width = UnicodeWidthStr::width(new_cell.content.as_str()).max(1) as u16;
                cursor = Some((row, col + char_width));
            }
        }

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

fn cursor_style_to_crossterm(style: CursorStyle) -> SetCursorStyle {
    match style {
        CursorStyle::Default => SetCursorStyle::DefaultUserShape,
        CursorStyle::BlinkingBlock => SetCursorStyle::BlinkingBlock,
        CursorStyle::SteadyBlock => SetCursorStyle::SteadyBlock,
        CursorStyle::BlinkingUnderline => SetCursorStyle::BlinkingUnderScore,
        CursorStyle::SteadyUnderline => SetCursorStyle::SteadyUnderScore,
        CursorStyle::BlinkingBar => SetCursorStyle::BlinkingBar,
        CursorStyle::SteadyBar => SetCursorStyle::SteadyBar,
    }
}

fn paint_active_tab(buf: &mut FrameBuffer, manager: &PaneManager, scroll_offset: ScrollOffset) {
    let tab = manager.tabs.active();
    let n = tab.panes.len();
    let mid_row = (buf.size.rows() + 1) / 2;
    let mut col_offset = ColOffset::zero();
    for (i, pane) in tab.panes.iter().enumerate() {
        let offset = if i == tab.active_pane { scroll_offset } else { ScrollOffset::zero() };
        paint_pane(buf, pane, col_offset, offset);
        if i + 1 < n {
            let divider_col = u16::from(col_offset) + pane.width;
            let indicator = if i == tab.active_pane {
                "◀"
            } else if i + 1 == tab.active_pane {
                "▶"
            } else {
                "│"
            };
            for row in 1..buf.size.rows() {
                let (content, fg) = if row == mid_row && indicator != "│" {
                    (CellContent::try_from(indicator).expect("single grapheme cluster"), vt100::Color::Idx(15))
                } else {
                    (CellContent::from('│'), vt100::Color::Idx(8))
                };
                buf.set(row, divider_col, Cell {
                    content,
                    attrs: Attrs { fg, ..Attrs::default() },
                });
            }
            col_offset = col_offset.advance_past_pane(pane.width);
        } else {
            col_offset = ColOffset::zero();
        }
    }
}

fn paint_pane(buf: &mut FrameBuffer, pane: &Pane, col_offset: ColOffset, scroll_offset: ScrollOffset) {
    let mut parser = pane.parser.lock().unwrap();
    parser.screen_mut().set_scrollback(scroll_offset.into());
    {
        let screen = parser.screen();
        for row in 0..pane.height {
            for col in 0..pane.width {
                let cell = match screen.cell(row, col) {
                    Some(c) if c.is_wide_continuation() => Cell::default(),
                    Some(c) => {
                        let s = c.contents();
                        let content = if s.is_empty() {
                            CellContent::default()
                        } else {
                            CellContent::try_from(s).expect("vt100 cell contains single grapheme cluster")
                        };
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
                buf.set(row + 1, col + u16::from(col_offset), cell);
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

    for col in 0..manager.size.cols() {
        buf.set(row, col, Cell {
            content: CellContent::default(),
            attrs: Attrs { bg: bar_bg, ..Attrs::default() },
        });
    }

    let mut col = 1u16;
    for (tab_num, (is_active, tab)) in manager.tabs.iter().enumerate() {
        let indicator = if is_active { "●".to_string() } else { (tab_num + 1).to_string() };
        let label = format!(" [{}:{}] ", indicator, tab.display_name());
        let attrs = Attrs {
            fg: if is_active { active_fg } else { inactive_fg },
            bg: if is_active { active_bg } else { bar_bg },
            bold: is_active,
            ..Attrs::default()
        };
        for ch in label.chars() {
            if col >= manager.size.cols() { break; }
            buf.set(row, col, Cell { content: CellContent::from(ch), attrs: attrs.clone() });
            col += 1;
        }
    }

    if manager.has_pending_close() {
        let hint = " ↩ u ";
        let hint_col = manager.size.cols().saturating_sub(hint.chars().count() as u16);
        let hint_attrs = Attrs { fg: vt100::Color::Idx(11), ..Attrs::default() };
        for (offset, ch) in hint.chars().enumerate() {
            let c = hint_col + offset as u16;
            if c < manager.size.cols() {
                buf.set(row, c, Cell { content: CellContent::from(ch), attrs: hint_attrs.clone() });
            }
        }
    }
}

fn paint_prompt(buf: &mut FrameBuffer, _manager: &PaneManager, text: &str) {
    let row = 0;
    for (col, ch) in text.chars().enumerate() {
        buf.set_text(row, col as u16, CellContent::from(ch));
    }
}

fn paint_help(buf: &mut FrameBuffer, manager: &PaneManager, leader: &str) {
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
        if row >= manager.size.rows() {
            break;
        }
        for (col, ch) in line.chars().enumerate() {
            buf.set_text(row, col as u16, CellContent::from(ch));
        }
    }
}
