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
