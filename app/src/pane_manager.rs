use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::pane::{Pane, SpawnError as PaneSpawnError, ResizeError as PaneResizeError, WriteError as PaneWriteError};
use crate::size::{ColOffset, ContentArea, TerminalSize};

use as_source::AsSource;

#[cfg(target_os = "macos")]
mod macos_cwd {
    use std::ffi::CStr;
    use std::path::PathBuf;

    use libproc::libproc::proc_pid::{PIDInfo, PidInfoFlavor, pidinfo};

    const MAXPATHLEN: usize = 1024;

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
        fn flavor() -> PidInfoFlavor {
            PidInfoFlavor::VNodePathInfo
        }
    }

    pub fn pid_cwd(pid: u32) -> Option<PathBuf> {
        let info = pidinfo::<ProcVnodePathInfo>(pid as i32, 0).ok()?;
        let path = CStr::from_bytes_until_nul(&info.pvi_cdir.vip_path).ok()?;
        path.to_str().ok().map(PathBuf::from)
    }
}

const UNDO_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabIndex(usize);

impl From<usize> for TabIndex {
    fn from(n: usize) -> Self {
        TabIndex(n)
    }
}

impl From<TabIndex> for usize {
    fn from(t: TabIndex) -> usize {
        t.0
    }
}

pub enum TabActiveState {
    Active,
    Inactive
}

impl From<bool> for TabActiveState {
    fn from(value: bool) -> Self {
        if value {
            return TabActiveState::Active;
        }
        
        TabActiveState::Inactive
    }
}

pub struct TabSet {
    tabs: Vec<Tab>,
    active: usize,
}

impl TabSet {
    pub fn new(first: Tab) -> Self {
        TabSet {
            tabs: vec![first],
            active: 0,
        }
    }

    pub fn get_active(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn get_active_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    pub fn get_active_index(&self) -> TabIndex {
        TabIndex::from(self.active)
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (TabActiveState, &Tab)> {
        let active = self.active;
        self.tabs
            .iter()
            .enumerate()
            .map(move |(i, tab)| (TabActiveState::from(i == active), tab))
    }

    pub fn push_and_activate(&mut self, tab: Tab) {
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    pub fn switch_to(&mut self, index: TabIndex) {
        let i = usize::from(index);

        if i < self.tabs.len() {
            self.active = i;
        }
    }

    pub fn switch_next(&mut self) {
        self.active = (self.active + 1) % self.tabs.len();
    }

    pub fn switch_prev(&mut self) {
        self.active = self
            .active
            .checked_sub(1)
            .unwrap_or(self.tabs.len().saturating_sub(1));
    }

    pub fn remove_active(&mut self) -> Option<Tab> {
        if self.tabs.len() == 1 {
            return None;
        }
        let tab = self.tabs.remove(self.active);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        Some(tab)
    }

    pub fn remove_at(&mut self, idx: usize) -> Tab {
        let tab = self.tabs.remove(idx);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len().saturating_sub(1);
        } else if self.active > idx {
            self.active -= 1;
        }
        tab
    }

    pub fn get(&self, idx: usize) -> Option<&Tab> {
        self.tabs.get(idx)
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Tab> {
        self.tabs.get_mut(idx)
    }
}

pub struct PaneSet {
    panes: Vec<Pane>,
    active: usize,
}

impl PaneSet {
    pub fn new(first: Pane) -> Self {
        PaneSet {
            panes: vec![first],
            active: 0,
        }
    }

    pub fn get_active(&self) -> &Pane {
        &self.panes[self.active]
    }

    pub fn get_active_mut(&mut self) -> &mut Pane {
        &mut self.panes[self.active]
    }

    pub fn len(&self) -> usize {
        self.panes.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Pane> {
        self.panes.iter()
    }

    pub fn is_active(&self, pane: &Pane) -> bool {
        std::ptr::eq(pane, self.get_active())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Pane> {
        self.panes.iter_mut()
    }

    pub fn get_panes_before_active(&self) -> impl Iterator<Item = &Pane> {
        self.panes[..self.active].iter()
    }

    pub fn push_and_activate(&mut self, pane: Pane) {
        self.panes.push(pane);
        self.active = self.panes.len() - 1;
    }

    pub fn remove_active(&mut self) -> Option<Pane> {
        if self.panes.len() == 1 {
            return None;
        }
        let pane = self.panes.remove(self.active);
        if self.active >= self.panes.len() {
            self.active = self.panes.len() - 1;
        }
        Some(pane)
    }

    pub fn focus_next(&mut self) {
        self.active = (self.active + 1) % self.panes.len();
    }

    pub fn focus_prev(&mut self) {
        let n = self.panes.len();
        self.active = self.active.checked_sub(1).unwrap_or(n.saturating_sub(1));
    }

    pub fn remove_exited_non_last(&mut self) {
        let mut pi = 0;
        while pi < self.panes.len().saturating_sub(1) {
            if self.panes[pi].is_exited() {
                self.panes.remove(pi);
                if self.active > pi && self.active > 0 {
                    self.active -= 1;
                } else if self.active >= self.panes.len() {
                    self.active = self.panes.len() - 1;
                }
            } else {
                pi += 1;
            }
        }
    }

    pub fn is_last_pane_exited(&self) -> bool {
        self.panes
            .last()
            .map(|p| p.is_exited())
            .unwrap_or(false)
    }
}

pub struct Tab {
    pub panes: PaneSet,
    pub name: Option<String>,
}

impl Tab {
    pub fn new(pane: Pane) -> Self {
        Tab {
            panes: PaneSet::new(pane),
            name: None,
        }
    }

    pub fn get_display_name(&self) -> String {
        if let Some(ref name) = self.name {
            return name.clone();
        }
        self.panes.get_active().get_auto_name()
    }
}

#[derive(Debug, AsSource)]
pub enum CloseTabError {
    TriedToCloseFinalTab,
}

#[derive(Debug, AsSource)]
pub enum UndoCloseError {
    NoPendingClosures,
}

#[derive(Debug, AsSource)]
pub enum NewError {
    #[from]
    SpawnFailed(PaneSpawnError),
}

#[derive(Debug, AsSource)]
pub enum OpenTabError {
    #[from]
    SpawnFailed(PaneSpawnError),
}

#[derive(Debug, AsSource)]
pub enum ResizeError {
    #[from]
    PaneResizeFailed(PaneResizeError),
}

#[derive(Debug, AsSource)]
pub enum SplitError {
    #[from]
    SpawnFailed(PaneSpawnError),
    #[from]
    ResizeFailed(ResizeError),
}

#[derive(Debug, AsSource)]
pub enum WriteError {
    #[from]
    WriteFailed(PaneWriteError),
}

pub struct PaneManager {
    pub tabs: TabSet,
    pub size: TerminalSize,
    pending_close: Vec<(Tab, Instant)>,
}

impl PaneManager {
    pub fn new(size: TerminalSize) -> Result<Self, NewError> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let pane = Pane::spawn(&cwd, ContentArea::from(size).full_size())?;
        Ok(PaneManager {
            tabs: TabSet::new(Tab::new(pane)),
            size,
            pending_close: vec![],
        })
    }

    pub fn close_exited_tabs(&mut self) -> Result<(), CloseTabError> {
        let mut ti = 0;
        while ti < self.tabs.len() {
            self.tabs.get_mut(ti).unwrap().panes.remove_exited_non_last();
            let _ = self.resize_tab_panes(ti);

            if self.tabs.get(ti).map(|t| t.panes.is_last_pane_exited()).unwrap_or(false) {
                if self.tabs.len() == 1 {
                    return Err(CloseTabError::TriedToCloseFinalTab);
                }
                self.tabs.remove_at(ti);
            } else {
                ti += 1;
            }
        }
        Ok(())
    }

    pub fn close_active_pane(&mut self) -> Result<(), CloseTabError> {
        if self.tabs.get_active_mut().panes.remove_active().is_some() {
            let at = usize::from(self.tabs.get_active_index());
            let _ = self.resize_tab_panes(at);
            Ok(())
        } else {
            self.close_active_tab()
        }
    }

    fn close_active_tab(&mut self) -> Result<(), CloseTabError> {
        if let Some(tab) = self.tabs.remove_active() {
            self.pending_close.push((tab, Instant::now()));
            Ok(())
        } else {
            Err(CloseTabError::TriedToCloseFinalTab)
        }
    }

    pub fn undo_close(&mut self) -> Result<(), UndoCloseError> {
        if let Some((tab, _)) = self.pending_close.pop() {
            self.tabs.push_and_activate(tab);
            return Ok(());
        }
        
        Err(UndoCloseError::NoPendingClosures)
    }

    pub fn reap_pending_close(&mut self) {
        self.pending_close
            .retain(|(_, closed_at)| closed_at.elapsed() < UNDO_TIMEOUT);
    }

    pub fn is_pending_close(&self) -> bool {
        !self.pending_close.is_empty()
    }

    pub fn open_tab(&mut self) -> Result<(), OpenTabError> {
        let cwd = self.get_active_cwd();
        let pane_size = ContentArea::from(self.size).full_size();
        let pane = Pane::spawn(&cwd, pane_size)?;
        self.tabs.push_and_activate(Tab::new(pane));
        Ok(())
    }

    pub fn open_tab_named(&mut self, name: String) -> Result<(), OpenTabError> {
        self.open_tab()?;
        self.tabs.get_active_mut().name = Some(name);
        Ok(())
    }

    pub fn split_horizontal(&mut self) -> Result<(), SplitError> {
        let cwd = self.get_active_cwd();
        let new_pane = Pane::spawn(&cwd, ContentArea::from(self.size).full_size())?;
        self.tabs.get_active_mut().panes.push_and_activate(new_pane);
        let at = usize::from(self.tabs.get_active_index());
        self.resize_tab_panes(at)?;
        Ok(())
    }

    pub fn focus_next_pane(&mut self) {
        self.tabs.get_active_mut().panes.focus_next();
    }

    pub fn focus_prev_pane(&mut self) {
        self.tabs.get_active_mut().panes.focus_prev();
    }

    fn resize_tab_panes(&mut self, tab_idx: usize) -> Result<(), ResizeError> {
        let tab = match self.tabs.get_mut(tab_idx) {
            Some(t) => t,
            None => return Ok(()),
        };
        let n = tab.panes.len();
        if n == 0 {
            return Ok(());
        }
        let sizes = ContentArea::from(self.size).split_horizontal(n);
        for (pane, size) in tab.panes.iter_mut().zip(sizes) {
            pane.resize(size)?;
        }
        Ok(())
    }

    pub fn get_active_pane_col_offset(&self) -> ColOffset {
        let tab = self.tabs.get_active();
        let mut offset = ColOffset::zero();
        for pane in tab.panes.get_panes_before_active() {
            offset = offset.advance_past_pane(pane.width);
        }
        offset
    }

    pub fn get_active_cwd(&self) -> PathBuf {
        let pane = self.tabs.get_active().panes.get_active();
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

    pub fn rename_active(&mut self, name: String) {
        self.tabs.get_active_mut().name = Some(name);
    }

    pub fn revert_active_name(&mut self) {
        self.tabs.get_active_mut().name = None;
    }

    pub fn get_active_name(&self) -> String {
        self.tabs.get_active().get_display_name()
    }

    pub fn write_active(&mut self, data: &[u8]) -> Result<(), WriteError> {
        self.tabs.get_active_mut().panes.get_active_mut().write(data)?;
        Ok(())
    }

    pub fn is_active_pane_bracketed_paste(&self) -> bool {
        self.tabs.get_active().panes.get_active()
            .parser
            .lock()
            .unwrap()
            .screen()
            .bracketed_paste()
    }

    pub fn switch_to(&mut self, index: TabIndex) {
        self.tabs.switch_to(index);
    }

    pub fn switch_to_next(&mut self) {
        self.tabs.switch_next();
    }

    pub fn switch_to_prev(&mut self) {
        self.tabs.switch_prev();
    }

    pub fn resize(&mut self, size: TerminalSize) -> Result<(), ResizeError> {
        self.size = size;
        for i in 0..self.tabs.len() {
            self.resize_tab_panes(i)?;
        }
        Ok(())
    }
}
