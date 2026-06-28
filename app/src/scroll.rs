use crate::size::Rows;

const MAX_SCROLLBACK: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollOffset(usize);

impl ScrollOffset {
    pub fn zero() -> Self { ScrollOffset(0) }

    pub fn scroll_up(self) -> Self {
        ScrollOffset(self.0.saturating_add(1).min(MAX_SCROLLBACK))
    }

    pub fn scroll_down(self) -> Self {
        ScrollOffset(self.0.saturating_sub(1))
    }

    pub fn page_up(self, rows: Rows) -> Self {
        let page = usize::from(rows).saturating_sub(1);
        ScrollOffset(self.0.saturating_add(page).min(MAX_SCROLLBACK))
    }

    pub fn page_down(self, rows: Rows) -> Self {
        let page = usize::from(rows).saturating_sub(1);
        ScrollOffset(self.0.saturating_sub(page))
    }
}

impl From<ScrollOffset> for usize {
    fn from(s: ScrollOffset) -> usize { s.0 }
}
