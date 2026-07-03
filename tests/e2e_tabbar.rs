mod common;

use std::time::Duration;

use common::{CTRL_B, TestSession};

/// The active tab's `*` marker should be rendered with a coloured background
/// (AnsiValue 32 = blue) to distinguish it from the rest of the bar.
#[test]
fn active_tab_has_highlighted_background() {
    let session = TestSession::spawn_sambil(80, 24);

    assert!(
        session.wait_for_char_with_bg('●', vt100::Color::Idx(8), Duration::from_secs(2)),
        "active tab '●' should have a grey background\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Inactive tabs should have the bar background colour (AnsiValue 235),
/// distinct from the active tab's blue background.
#[test]
fn inactive_tab_has_bar_background() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    // Open a second tab — tab 1 becomes inactive.
    session.open_tab();

    // Tab 1's `1` digit should now sit on the bar background, not the active colour.
    assert!(
        session.wait_for_char_with_bg('1', vt100::Color::Default, Duration::from_secs(2)),
        "inactive tab 1 should have default background\n---\n{}\n---",
        session.screen().full_text()
    );
}
