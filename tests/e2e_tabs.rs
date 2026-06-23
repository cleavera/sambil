mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// Ctrl-b c opens a new tab instantly using the cwd as the name — no prompt.
#[test]
fn ctrl_b_c_opens_a_new_tab() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();
    assert!(!session.screen().contains("[2:"), "unexpected second tab at startup");

    session.open_tab();

    // Wait until tab 1 becomes inactive (its number appears), confirming tab 2 opened.
    assert!(
        session.screen().contains("[1:"),
        "Tab 1 should still appear in the tab bar\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        !session.screen().contains("New tab name:"),
        "Ctrl-b c should not show a name prompt\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Opening multiple tabs keeps all of them in the tab bar.
#[test]
fn can_open_multiple_tabs() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    session.open_tab();

    session.open_tab();

    let screen = session.screen();
    assert!(screen.contains("[1:"), "tab 1 missing from bar");
    assert!(screen.contains("[2:"), "tab 2 missing from bar");
}
