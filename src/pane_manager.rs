use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::pane::Pane;

const UNDO_TIMEOUT: Duration = Duration::from_secs(10);

pub struct PaneManager {
    pub panes: Vec<Pane>,
    pub active: usize,
    pub cols: u16,
    pub rows: u16,
    pending_close: Vec<(Pane, Instant)>,
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
            pending_close: vec![],
        })
    }

    /// Closes any tabs whose shell has exited. Returns `true` if no tabs remain.
    pub fn close_exited_tabs(&mut self) -> bool {
        let mut i = 0;
        while i < self.panes.len() {
            if self.panes[i].exited.load(std::sync::atomic::Ordering::Relaxed) {
                if self.panes.len() == 1 {
                    return true;
                }
                self.panes.remove(i);
                if self.active >= self.panes.len() {
                    self.active = self.panes.len() - 1;
                } else if self.active > i {
                    self.active -= 1;
                }
            } else {
                i += 1;
            }
        }
        false
    }

    /// Closes the active tab. Returns `true` if it was the last tab (caller should quit).
    /// Otherwise the pane is held in a pending queue for up to 10 seconds so it can be undone.
    pub fn close_active_tab(&mut self) -> bool {
        if self.panes.len() == 1 {
            return true;
        }
        let pane = self.panes.remove(self.active);
        self.pending_close.push((pane, Instant::now()));
        if self.active >= self.panes.len() {
            self.active = self.panes.len() - 1;
        }
        false
    }

    /// Restores the most recently closed tab. Returns `true` if a tab was restored.
    pub fn undo_close(&mut self) -> bool {
        if let Some((pane, _)) = self.pending_close.pop() {
            self.panes.push(pane);
            self.active = self.panes.len() - 1;
            return true;
        }
        false
    }

    /// Drops any pending-close panes that have exceeded the undo timeout.
    pub fn reap_pending_close(&mut self) {
        self.pending_close.retain(|(_, closed_at)| closed_at.elapsed() < UNDO_TIMEOUT);
    }

    /// Returns `true` if there are tabs waiting in the undo queue.
    pub fn has_pending_close(&self) -> bool {
        !self.pending_close.is_empty()
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

    pub fn active_bracketed_paste(&self) -> bool {
        self.panes[self.active].parser.lock().unwrap().screen().bracketed_paste()
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
