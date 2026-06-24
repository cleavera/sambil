use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::pane::{path_basename, Pane};

#[cfg(target_os = "macos")]
mod macos_cwd {
    use std::ffi::CStr;
    use std::path::PathBuf;

    use libproc::libproc::proc_pid::{PIDInfo, PidInfoFlavor, pidinfo};

    const MAXPATHLEN: usize = 1024;

    // Mirror of the vinfo_stat C struct (136 bytes on macOS).
    // The layout matches the Darwin kernel headers exactly.
    #[repr(C)]
    struct VinfoStat([u8; 136]);

    #[repr(C)]
    struct VnodeInfo {
        _stat: VinfoStat,
        _type: i32,
        _pad: i32,
        _fsid: [i32; 2],
    }

    #[repr(C)]
    struct VnodeInfoPath {
        _vi: VnodeInfo,
        vip_path: [u8; MAXPATHLEN],
    }

    #[repr(C)]
    pub struct ProcVnodePathInfo {
        pub pvi_cdir: VnodeInfoPath,
        _pvi_rdir: VnodeInfoPath,
    }

    impl PIDInfo for ProcVnodePathInfo {
        fn flavor() -> PidInfoFlavor { PidInfoFlavor::VNodePathInfo }
    }

    pub fn pid_cwd(pid: u32) -> Option<PathBuf> {
        let info = pidinfo::<ProcVnodePathInfo>(pid as i32, 0).ok()?;
        let path = CStr::from_bytes_until_nul(&info.pvi_cdir.vip_path).ok()?;
        path.to_str().ok().map(PathBuf::from)
    }
}

const UNDO_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Tab {
    pub panes: Vec<Pane>,
    pub active_pane: usize,
    pub name: Option<String>,
}

impl Tab {
    pub fn new(pane: Pane) -> Self {
        Tab { panes: vec![pane], active_pane: 0, name: None }
    }

    /// The name shown in the tab bar.
    /// - Explicit name (`Some`) is shown as-is.
    /// - Auto-named (`None`): delegates to the active pane's OSC 2 / cwd.
    pub fn display_name(&self) -> String {
        if let Some(ref name) = self.name {
            return name.clone();
        }
        self.panes[self.active_pane].auto_name()
    }
}

pub struct PaneManager {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub cols: u16,
    pub rows: u16,
    pending_close: Vec<(Tab, Instant)>,
}

impl PaneManager {
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        let pane_height = rows.saturating_sub(1);
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Ok(PaneManager {
            tabs: vec![Tab::new(Pane::spawn(&cwd, cols, pane_height)?)],
            active_tab: 0,
            cols,
            rows,
            pending_close: vec![],
        })
    }

    /// Closes any panes/tabs whose shell has exited. Returns `true` if no tabs remain.
    pub fn close_exited_tabs(&mut self) -> bool {
        let mut ti = 0;
        while ti < self.tabs.len() {
            // Remove non-last exited panes from this tab.
            let mut changed = false;
            {
                let tab = &mut self.tabs[ti];
                let mut pi = 0;
                while pi < tab.panes.len().saturating_sub(1) {
                    if tab.panes[pi].exited.load(Ordering::Relaxed) {
                        tab.panes.remove(pi);
                        changed = true;
                        if tab.active_pane > pi && tab.active_pane > 0 {
                            tab.active_pane -= 1;
                        } else if tab.active_pane >= tab.panes.len() {
                            tab.active_pane = tab.panes.len() - 1;
                        }
                    } else {
                        pi += 1;
                    }
                }
            }
            if changed {
                let _ = self.resize_tab_panes(ti);
            }

            // Check if the tab's last pane has exited.
            let last_exited = self.tabs[ti]
                .panes
                .last()
                .map(|p| p.exited.load(Ordering::Relaxed))
                .unwrap_or(false);

            if last_exited {
                if self.tabs.len() == 1 {
                    return true;
                }
                self.tabs.remove(ti);
                if self.active_tab >= self.tabs.len() {
                    self.active_tab = self.tabs.len() - 1;
                } else if self.active_tab > ti {
                    self.active_tab -= 1;
                }
            } else {
                ti += 1;
            }
        }
        false
    }

    /// Closes the active pane.
    /// If the tab has multiple panes, only the pane is removed (no undo).
    /// If it was the last pane, the whole tab is closed (with undo unless last tab).
    /// Returns `true` if the last tab was closed (caller should quit).
    pub fn close_active_pane(&mut self) -> bool {
        if self.tabs[self.active_tab].panes.len() > 1 {
            let pi = self.tabs[self.active_tab].active_pane;
            self.tabs[self.active_tab].panes.remove(pi);
            let tab = &mut self.tabs[self.active_tab];
            if tab.active_pane >= tab.panes.len() {
                tab.active_pane = tab.panes.len() - 1;
            }
            let _ = self.resize_tab_panes(self.active_tab);
            false
        } else {
            self.close_active_tab()
        }
    }

    /// Closes the entire active tab (all its panes). Returns `true` if it was the last tab.
    fn close_active_tab(&mut self) -> bool {
        if self.tabs.len() == 1 {
            return true;
        }
        let tab = self.tabs.remove(self.active_tab);
        self.pending_close.push((tab, Instant::now()));
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        false
    }

    /// Restores the most recently closed tab. Returns `true` if a tab was restored.
    pub fn undo_close(&mut self) -> bool {
        if let Some((tab, _)) = self.pending_close.pop() {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
            return true;
        }
        false
    }

    /// Drops any pending-close tabs that have exceeded the undo timeout.
    pub fn reap_pending_close(&mut self) {
        self.pending_close.retain(|(_, closed_at)| closed_at.elapsed() < UNDO_TIMEOUT);
    }

    /// Returns `true` if there are tabs waiting in the undo queue.
    pub fn has_pending_close(&self) -> bool {
        !self.pending_close.is_empty()
    }

    /// Opens a new auto-named tab.
    pub fn open_tab(&mut self) -> Result<()> {
        let cwd = self.active_cwd();
        let pane = Pane::spawn(&cwd, self.cols, self.rows.saturating_sub(1))?;
        self.tabs.push(Tab::new(pane));
        self.active_tab = self.tabs.len() - 1;
        Ok(())
    }

    /// Opens a new tab with an explicit user-provided name (immune to OSC 2 overrides).
    pub fn open_tab_named(&mut self, name: String) -> Result<()> {
        self.open_tab()?;
        self.tabs[self.active_tab].name = Some(name);
        Ok(())
    }

    /// Splits the active tab horizontally, adding a new pane to the right.
    pub fn split_horizontal(&mut self) -> Result<()> {
        let cwd = self.active_cwd();
        let height = self.rows.saturating_sub(1);
        let new_n = self.tabs[self.active_tab].panes.len() + 1;
        let available = self.cols.saturating_sub((new_n as u16).saturating_sub(1));
        let base_w = (available / new_n as u16).max(1);
        let last_w = (available - base_w * (new_n as u16 - 1)).max(1);

        for pane in self.tabs[self.active_tab].panes.iter_mut() {
            pane.resize(base_w, height)?;
        }

        let new_pane = Pane::spawn(&cwd, last_w, height)?;
        self.tabs[self.active_tab].panes.push(new_pane);
        self.tabs[self.active_tab].active_pane = self.tabs[self.active_tab].panes.len() - 1;
        Ok(())
    }

    /// Moves focus to the next pane in the active tab (wraps around).
    pub fn focus_next_pane(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        tab.active_pane = (tab.active_pane + 1) % tab.panes.len();
    }

    /// Moves focus to the previous pane in the active tab (wraps around).
    pub fn focus_prev_pane(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        let n = tab.panes.len();
        tab.active_pane = tab.active_pane.checked_sub(1).unwrap_or(n - 1);
    }

    /// Recalculates and applies even widths to all panes in a tab.
    fn resize_tab_panes(&mut self, tab_idx: usize) -> Result<()> {
        let n = self.tabs[tab_idx].panes.len();
        if n == 0 {
            return Ok(());
        }
        let height = self.rows.saturating_sub(1);
        let available = self.cols.saturating_sub((n as u16).saturating_sub(1));
        let base_w = (available / n as u16).max(1);
        let last_w = (available - base_w * (n as u16 - 1)).max(1);
        for (i, pane) in self.tabs[tab_idx].panes.iter_mut().enumerate() {
            let w = if i == n - 1 { last_w } else { base_w };
            pane.resize(w, height)?;
        }
        Ok(())
    }

    pub fn active_pane_col_offset(&self) -> u16 {
        let tab = &self.tabs[self.active_tab];
        let mut offset = 0u16;
        for i in 0..tab.active_pane {
            offset += tab.panes[i].width + 1; // +1 for divider
        }
        offset
    }

    pub fn active_cwd(&self) -> PathBuf {
        let tab = &self.tabs[self.active_tab];
        let pane = &tab.panes[tab.active_pane];
        #[cfg(target_os = "linux")]
        if let Some(pid) = pane.child_pid {
            if let Ok(path) = std::fs::read_link(format!("/proc/{}/cwd", pid)) {
                return path;
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(pid) = pane.child_pid {
            if let Some(path) = macos_cwd::pid_cwd(pid) {
                return path;
            }
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    }

    pub fn active_cwd_name(&self) -> String {
        path_basename(&self.active_cwd())
    }

    pub fn rename_active(&mut self, name: String) {
        self.tabs[self.active_tab].name = Some(name);
    }

    pub fn active_name(&self) -> String {
        self.tabs[self.active_tab].display_name()
    }

    pub fn write_active(&mut self, data: &[u8]) -> Result<()> {
        let ti = self.active_tab;
        let pi = self.tabs[ti].active_pane;
        self.tabs[ti].panes[pi].write(data)
    }

    pub fn active_bracketed_paste(&self) -> bool {
        let tab = &self.tabs[self.active_tab];
        tab.panes[tab.active_pane].parser.lock().unwrap().screen().bracketed_paste()
    }

    pub fn switch_to(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
        }
    }

    pub fn switch_to_next(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }

    pub fn switch_to_prev(&mut self) {
        self.active_tab = self.active_tab.checked_sub(1).unwrap_or(self.tabs.len() - 1);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.cols = cols;
        self.rows = rows;
        for i in 0..self.tabs.len() {
            self.resize_tab_panes(i)?;
        }
        Ok(())
    }
}
