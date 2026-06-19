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
        let pane_height = rows.saturating_sub(1);
        Ok(PaneManager {
            panes: vec![Pane::spawn(cols, pane_height)?],
            active: 0,
            cols,
            rows,
        })
    }

    pub fn open_tab(&mut self) -> Result<()> {
        self.panes.push(Pane::spawn(self.cols, self.rows.saturating_sub(1))?);
        self.active = self.panes.len() - 1;
        Ok(())
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
        let pane_height = rows.saturating_sub(1);
        for pane in &mut self.panes {
            pane.resize(cols, pane_height)?;
        }
        Ok(())
    }
}
