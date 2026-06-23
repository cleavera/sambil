mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// Ctrl-b c opens a new tab instantly using the cwd as the name — no prompt.
#[test]
fn ctrl_b_c_opens_a_new_tab() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "sambil did not render");
    assert!(!session.screen().contains("[2:"), "unexpected second tab at startup");

    session.send_keys(&[CTRL_B, b'c']);

    // Wait until tab 1 becomes inactive (its number appears), confirming tab 2 opened.
    assert!(
        session.wait_for_text("[1:", Duration::from_secs(2)),
        "New tab did not open or become active\n---\n{}\n---",
        session.screen().full_text()
    );
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

    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "sambil did not render");

    session.send_keys(&[CTRL_B, b'c']);
    assert!(session.wait_for_text("[1:", Duration::from_secs(2)), "tab 2 did not open");

    session.send_keys(&[CTRL_B, b'c']);
    assert!(
        session.wait_for_text("[2:", Duration::from_secs(2)),
        "Tab 3 did not open\n---\n{}\n---",
        session.screen().full_text()
    );

    let screen = session.screen();
    assert!(screen.contains("[1:"), "tab 1 missing from bar");
    assert!(screen.contains("[2:"), "tab 2 missing from bar");
}
