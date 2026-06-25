use crate::size::Rows;

const MAX_SCROLLBACK: usize = 1000;

/// How far back from the live view the scrollback viewport is positioned.
/// 0 means showing the live terminal; higher values scroll further into history.
/// Guaranteed to be in the range `0..=MAX_SCROLLBACK`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollOffset(usize);

impl ScrollOffset {
    pub fn zero() -> Self { ScrollOffset(0) }

    /// Scroll back one line (towards history).
    pub fn scroll_up(self) -> Self {
        ScrollOffset(self.0.saturating_add(1).min(MAX_SCROLLBACK))
    }

    /// Scroll forward one line (towards live view).
    pub fn scroll_down(self) -> Self {
        ScrollOffset(self.0.saturating_sub(1))
    }

    /// Scroll back one page (towards history).
    pub fn page_up(self, rows: Rows) -> Self {
        let page = usize::from(rows).saturating_sub(1);
        ScrollOffset(self.0.saturating_add(page).min(MAX_SCROLLBACK))
    }

    /// Scroll forward one page (towards live view).
    pub fn page_down(self, rows: Rows) -> Self {
        let page = usize::from(rows).saturating_sub(1);
        ScrollOffset(self.0.saturating_sub(page))
    }
}

impl From<ScrollOffset> for usize {
    fn from(s: ScrollOffset) -> usize { s.0 }
}
