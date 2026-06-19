use std::io::Write;

use anyhow::Result;
use crossterm::{cursor, execute, style::Print, terminal};

use crate::pane_manager::PaneManager;

pub fn draw<W: Write>(out: &mut W, manager: &PaneManager) -> Result<()> {
    execute!(out, terminal::Clear(terminal::ClearType::All))?;
    draw_border(out, manager.cols, manager.rows)?;
    draw_status_bar(out, manager)?;
    Ok(())
}

fn draw_border<W: Write>(out: &mut W, cols: u16, rows: u16) -> Result<()> {
    let border_col = cols / 2;
    for row in 0..rows.saturating_sub(1) {
        execute!(out, cursor::MoveTo(border_col, row), Print("│"))?;
    }
    Ok(())
}

fn draw_status_bar<W: Write>(out: &mut W, manager: &PaneManager) -> Result<()> {
    let label = format!(" pane {} active ", manager.active + 1);
    execute!(
        out,
        cursor::MoveTo(0, manager.rows.saturating_sub(1)),
        Print(&label),
    )?;
    Ok(())
}
