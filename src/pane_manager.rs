use crate::pane::Pane;

pub struct PaneManager {
    pub panes: Vec<Pane>,
    pub active: usize,
    pub cols: u16,
    pub rows: u16,
}

impl PaneManager {
    pub fn new(cols: u16, rows: u16) -> Self {
        let border_col = cols / 2;
        PaneManager {
            panes: vec![
                Pane::new(0, border_col, rows.saturating_sub(1)),
                Pane::new(border_col + 1, cols, rows.saturating_sub(1)),
            ],
            active: 0,
            cols,
            rows,
        }
    }

    pub fn switch_to_next(&mut self) {
        self.active = (self.active + 1) % self.panes.len();
    }

    pub fn switch_to_prev(&mut self) {
        self.active = self.active.checked_sub(1).unwrap_or(self.panes.len() - 1);
    }
}
