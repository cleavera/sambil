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
            panes: vec![Pane::spawn(cwd_name(), cols, pane_height)?],
            active: 0,
            cols,
            rows,
        })
    }

    pub fn open_tab(&mut self, name: String) -> Result<()> {
        self.panes.push(Pane::spawn(name, self.cols, self.rows.saturating_sub(1))?);
        self.active = self.panes.len() - 1;
        Ok(())
    }

    pub fn rename_active(&mut self, name: String) {
        self.panes[self.active].name = name;
    }

    pub fn active_name(&self) -> &str {
        &self.panes[self.active].name
    }

    pub fn write_active(&mut self, data: &[u8]) -> Result<()> {
        self.panes[self.active].write(data)
    }

    pub fn switch_to(&mut self, index: usize) {
        if index < self.panes.len() {
            self.active = index;
        }
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

pub fn cwd_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "shell".to_string())
}
