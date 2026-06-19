use std::io::Write;

use anyhow::Result;
use crossterm::{cursor, queue, style::Print, terminal};

use crate::pane::Pane;
use crate::pane_manager::PaneManager;

pub fn draw<W: Write>(out: &mut W, manager: &PaneManager) -> Result<()> {
    queue!(out, terminal::Clear(terminal::ClearType::All))?;
    for pane in &manager.panes {
        draw_pane(out, pane)?;
    }
    draw_border(out, manager.cols, manager.rows)?;
    draw_status_bar(out, manager)?;
    Ok(())
}

fn draw_pane<W: Write>(out: &mut W, pane: &Pane) -> Result<()> {
    let parser = pane.parser.lock().unwrap();
    let screen = parser.screen();

    for row in 0..pane.height {
        queue!(out, cursor::MoveTo(pane.col_start, row))?;
        let mut col = 0u16;
        while col < pane.width {
            match screen.cell(row, col) {
                Some(cell) if cell.is_wide_continuation() => {
                    queue!(out, Print(" "))?;
                }
                Some(cell) => {
                    let contents = cell.contents();
                    queue!(out, Print(if contents.is_empty() { " " } else { contents }))?;
                }
                None => {
                    queue!(out, Print(" "))?;
                }
            }
            col += 1;
        }
    }
    Ok(())
}

fn draw_border<W: Write>(out: &mut W, cols: u16, rows: u16) -> Result<()> {
    let border_col = cols / 2;
    for row in 0..rows.saturating_sub(1) {
        queue!(out, cursor::MoveTo(border_col, row), Print("│"))?;
    }
    Ok(())
}

fn draw_status_bar<W: Write>(out: &mut W, manager: &PaneManager) -> Result<()> {
    let label = format!(" pane {} active ", manager.active + 1);
    queue!(out, cursor::MoveTo(0, manager.rows.saturating_sub(1)), Print(&label))?;
    Ok(())
}
