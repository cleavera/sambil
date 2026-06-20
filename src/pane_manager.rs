use std::path::PathBuf;

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
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let name = path_basename(&cwd);
        Ok(PaneManager {
            panes: vec![Pane::spawn(name, &cwd, cols, pane_height)?],
            active: 0,
            cols,
            rows,
        })
    }

    /// Closes the active tab. Returns `true` if it was the last tab (caller should quit).
    pub fn close_active_tab(&mut self) -> bool {
        if self.panes.len() == 1 {
            return true;
        }
        self.panes.remove(self.active);
        if self.active >= self.panes.len() {
            self.active = self.panes.len() - 1;
        }
        false
    }

    pub fn open_tab(&mut self, name: String) -> Result<()> {
        let cwd = self.active_cwd();
        self.panes.push(Pane::spawn(name, &cwd, self.cols, self.rows.saturating_sub(1))?);
        self.active = self.panes.len() - 1;
        Ok(())
    }

    pub fn active_cwd(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        if let Some(pid) = self.panes[self.active].child_pid {
            if let Ok(path) = std::fs::read_link(format!("/proc/{}/cwd", pid)) {
                return path;
            }
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    }

    pub fn active_cwd_name(&self) -> String {
        path_basename(&self.active_cwd())
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

pub fn path_basename(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "shell".to_string())
}
