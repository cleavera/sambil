use anyhow::Result;

use crate::pane::Pane;

pub struct PaneManager {
    pub panes: Vec<Pane>,
    pub active: usize,
    pub cols: u16,
    pub rows: u16,
}

impl PaneManager {
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        let border_col = cols / 2;
        let pane_height = rows.saturating_sub(1);

        let pane0 = Pane::spawn(0, border_col, pane_height)?;
        let pane1 = Pane::spawn(border_col + 1, cols - border_col - 1, pane_height)?;

        Ok(PaneManager { panes: vec![pane0, pane1], active: 0, cols, rows })
    }

    pub fn write_active(&mut self, data: &[u8]) -> Result<()> {
        self.panes[self.active].write(data)
    }

    pub fn switch_to_next(&mut self) {
        self.active = (self.active + 1) % self.panes.len();
    }

    pub fn switch_to_prev(&mut self) {
        self.active = self.active.checked_sub(1).unwrap_or(self.panes.len() - 1);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.cols = cols;
        self.rows = rows;
        let border_col = cols / 2;
        let pane_height = rows.saturating_sub(1);

        self.panes[0].col_start = 0;
        self.panes[0].resize(border_col, pane_height)?;

        self.panes[1].col_start = border_col + 1;
        self.panes[1].resize(cols - border_col - 1, pane_height)?;

        Ok(())
    }
}
