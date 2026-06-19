pub struct Pane {
    pub col_start: u16,
    pub col_end: u16,
    pub rows: u16,
}

impl Pane {
    pub fn new(col_start: u16, col_end: u16, rows: u16) -> Self {
        Pane { col_start, col_end, rows }
    }
}
