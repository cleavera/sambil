use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq)]
pub struct CellContent(String);

#[derive(Debug)]
pub struct InvalidCellContent;

impl TryFrom<&str> for CellContent {
    type Error = InvalidCellContent;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut clusters = s.graphemes(true);
        match clusters.next() {
            None => Err(InvalidCellContent),
            Some(g) if clusters.next().is_none() => Ok(CellContent(g.to_string())),
            _ => Err(InvalidCellContent),
        }
    }
}

impl From<char> for CellContent {
    fn from(c: char) -> Self { CellContent(c.to_string()) }
}

impl Default for CellContent {
    fn default() -> Self { CellContent(" ".to_string()) }
}

impl CellContent {
    pub fn as_str(&self) -> &str { &self.0 }
}
