mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// The active tab's `*` marker should be rendered with a coloured background
/// (AnsiValue 32 = blue) to distinguish it from the rest of the bar.
#[test]
fn active_tab_has_highlighted_background() {
    let session = TestSession::spawn_sambil(80, 24);

    assert!(
        session.wait_for_char_with_bg('●', vt100::Color::Idx(32), Duration::from_secs(2)),
        "active tab '●' should have a blue background\n---\n{}\n---",
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
    session.send_keys(&[CTRL_B, b'c']);
    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "tab 2 did not open");

    // Tab 1's `1` digit should now sit on the bar background, not the active colour.
    assert!(
        session.wait_for_char_with_bg('1', vt100::Color::Idx(235), Duration::from_secs(2)),
        "inactive tab 1 should have bar background colour\n---\n{}\n---",
        session.screen().full_text()
    );
}
