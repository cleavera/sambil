mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// Typing a command in the active tab should show output full-screen.
#[test]
fn typing_in_active_tab_shows_output() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(
        session.wait_for_text("[*1:", Duration::from_secs(2)),
        "sambil did not render tab bar"
    );

    session.send_str("echo tab1_output\n");

    let appeared = session.wait_for_text("tab1_output", Duration::from_secs(2));
    assert!(
        appeared,
        "Expected 'tab1_output' on screen\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Switching tabs shows a fresh session; switching back restores the original tab's content.
#[test]
fn switching_tabs_shows_correct_content() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[*1:", Duration::from_secs(2)), "sambil did not render");

    session.send_str("echo tab1_marker\n");
    assert!(session.wait_for_text("tab1_marker", Duration::from_secs(2)), "tab 1 output did not appear");

    // Open a new tab with cwd name (instant, no prompt)
    session.send_keys(&[CTRL_B, b'c']);

    assert!(
        session.wait_for_text("[*2:", Duration::from_secs(2)),
        "Tab 2 did not become active\n---\n{}\n---",
        session.screen().full_text()
    );

    // tab1_marker should NOT be visible in tab 2
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(
        !session.screen().contains("tab1_marker"),
        "tab1_marker should not be visible in tab 2\n---\n{}\n---",
        session.screen().full_text()
    );

    // Switch back to tab 1
    session.send_keys(&[CTRL_B, b'p']);
    assert!(
        session.wait_for_text("tab1_marker", Duration::from_secs(2)),
        "tab1_marker should reappear after switching back to tab 1\n---\n{}\n---",
        session.screen().full_text()
    );
}
