/// The dimensions of a terminal or pane in character cells.
/// Both `cols` and `rows` are guaranteed to be at least 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    cols: u16,
    rows: u16,
}

impl TerminalSize {
    /// Returns `None` if either dimension is zero.
    pub fn new(cols: u16, rows: u16) -> Option<Self> {
        if cols == 0 || rows == 0 { None } else { Some(TerminalSize { cols, rows }) }
    }

    /// Clamps a zero dimension to 1 rather than failing.
    pub fn new_clamped(cols: u16, rows: u16) -> Self {
        TerminalSize { cols: cols.max(1), rows: rows.max(1) }
    }

    pub fn cols(&self) -> u16 { self.cols }
    pub fn rows(&self) -> u16 { self.rows }
}

impl From<TerminalSize> for Rows {
    fn from(size: TerminalSize) -> Rows { Rows(size.rows) }
}

/// A row count guaranteed to be at least 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rows(u16);

impl Rows {
    pub fn new_clamped(rows: u16) -> Self { Rows(rows.max(1)) }
}

impl From<Rows> for usize {
    fn from(r: Rows) -> usize { r.0 as usize }
}

/// The drawable area available for panes — the terminal minus the tab bar row.
/// Guaranteed to be at least 1×1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentArea {
    cols: u16,
    rows: u16,
}

impl ContentArea {
    pub fn cols(&self) -> u16 { self.cols }
    pub fn rows(&self) -> u16 { self.rows }

    /// The size of a single full-width pane occupying the entire content area.
    pub fn full_size(&self) -> PaneSize {
        PaneSize { cols: self.cols, rows: self.rows }
    }

    /// Divides the content area evenly into `n` panes separated by 1-column dividers.
    /// Returns one `PaneSize` per pane; the last pane absorbs any remainder columns.
    pub fn split_horizontal(&self, n: usize) -> Vec<PaneSize> {
        if n == 0 { return vec![]; }
        let n16 = n as u16;
        let available = self.cols.saturating_sub(n16.saturating_sub(1));
        let base_w = (available / n16).max(1);
        let last_w = available.saturating_sub(base_w * (n16 - 1)).max(1);
        (0..n).map(|i| PaneSize {
            cols: if i == n - 1 { last_w } else { base_w },
            rows: self.rows,
        }).collect()
    }
}

impl From<TerminalSize> for ContentArea {
    fn from(size: TerminalSize) -> ContentArea {
        ContentArea {
            cols: size.cols,
            rows: size.rows.saturating_sub(1).max(1),
        }
    }
}

/// The dimensions of a single pane within the content area.
/// Guaranteed to be at least 1×1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSize {
    cols: u16,
    rows: u16,
}

impl PaneSize {
    pub fn cols(&self) -> u16 { self.cols }
    pub fn rows(&self) -> u16 { self.rows }
}

/// The horizontal pixel offset of a pane from the left edge of the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColOffset(u16);

impl ColOffset {
    pub fn zero() -> Self { ColOffset(0) }

    /// Advance by a pane width plus a 1-column divider.
    pub fn advance_past_pane(self, pane_width: u16) -> Self {
        ColOffset(self.0.saturating_add(pane_width).saturating_add(1))
    }
}

impl From<ColOffset> for u16 {
    fn from(o: ColOffset) -> u16 { o.0 }
}
