mod common;

use std::time::Duration;

use common::{CTRL_B, TestSession};

/// Ctrl-b c opens instantly with the cwd name — no prompt shown.
#[test]
fn ctrl_b_c_opens_with_cwd_name_instantly() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.open_tab();

    assert!(
        !session.screen().contains("New tab name:"),
        "Ctrl-b c should not show a prompt\n---\n{}\n---",
        session.screen().full_text()
    );
    // Name should be non-empty (cwd basename)
    assert!(
        !session.screen().contains("[●:]"),
        "Tab name should not be empty\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Ctrl-b C (uppercase) shows a naming prompt before opening the tab.
#[test]
fn ctrl_b_shift_c_shows_naming_prompt() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.send_keys(&[CTRL_B, b'C']);

    assert!(
        session.wait_for_text("New tab name:", Duration::from_secs(2)),
        "Expected naming prompt in status bar\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Typing a name and pressing Enter opens the tab with that name in the tab bar.
#[test]
fn named_tab_shows_name_in_tab_bar() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.send_keys(&[CTRL_B, b'C']);
    session.assert_name_prompt();

    session.send_str("myproject\r");

    assert!(
        session.wait_for_text("[●:myproject]", Duration::from_secs(2)),
        "Expected tab bar to show '[●:myproject]'\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Pressing Enter with no input uses the cwd basename as the tab name.
#[test]
fn empty_name_uses_cwd_basename() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.send_keys(&[CTRL_B, b'C']);
    session.assert_name_prompt();

    session.send_str("\r");

    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "Expected new tab with cwd name\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        !session.screen().contains("[●:]"),
        "Tab name should not be empty — expected cwd basename\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Pressing Esc while naming cancels and does not open a new tab.
#[test]
fn esc_cancels_naming() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.send_keys(&[CTRL_B, b'C']);
    session.assert_name_prompt();

    session.send_keys(&[0x1b]);

    std::thread::sleep(Duration::from_millis(150));
    let screen = session.screen();
    assert!(
        !screen.contains("New tab name:"),
        "Prompt should be gone after Esc\n---\n{}\n---",
        screen.full_text()
    );
    assert!(
        !screen.contains("[2:"),
        "No second tab should exist after cancelling\n---\n{}\n---",
        screen.full_text()
    );
}
