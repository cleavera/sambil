mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// Ctrl-b r shows a rename prompt pre-filled with the current tab name.
#[test]
fn ctrl_b_r_shows_rename_prompt_with_current_name() {
    let mut session = TestSession::spawn_sambil(80, 24);
    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "sambil did not render");

    // Open a named tab so we have a predictable name to check
    session.send_keys(&[CTRL_B, b'C']);
    assert!(session.wait_for_text("New tab name:", Duration::from_secs(2)), "open prompt did not appear");
    session.send_str("original\r");
    assert!(session.wait_for_text("[●:original]", Duration::from_secs(2)), "named tab did not open");

    session.send_keys(&[CTRL_B, b'r']);

    assert!(
        session.wait_for_text("Rename tab:", Duration::from_secs(2)),
        "Expected rename prompt in status bar\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        session.screen().contains("original"),
        "Rename prompt should be pre-filled with current name\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Confirming a new name updates the tab bar.
#[test]
fn rename_updates_tab_bar() {
    let mut session = TestSession::spawn_sambil(80, 24);
    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "sambil did not render");

    session.send_keys(&[CTRL_B, b'C']);
    assert!(session.wait_for_text("New tab name:", Duration::from_secs(2)), "open prompt did not appear");
    session.send_str("original\r");
    assert!(session.wait_for_text("[●:original]", Duration::from_secs(2)), "named tab did not open");

    session.send_keys(&[CTRL_B, b'r']);
    assert!(session.wait_for_text("Rename tab:", Duration::from_secs(2)), "rename prompt did not appear");

    // Clear the pre-filled name and type a new one
    for _ in 0.."original".len() {
        session.send_keys(&[0x7f]); // backspace
    }
    session.send_str("renamed\r");

    assert!(
        session.wait_for_text("[●:renamed]", Duration::from_secs(2)),
        "Tab bar should show new name '[●:renamed]'\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        !session.screen().contains("[●:original]"),
        "Old name should no longer appear\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Esc during rename cancels and leaves the name unchanged.
#[test]
fn esc_cancels_rename() {
    let mut session = TestSession::spawn_sambil(80, 24);
    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "sambil did not render");

    session.send_keys(&[CTRL_B, b'C']);
    assert!(session.wait_for_text("New tab name:", Duration::from_secs(2)), "open prompt did not appear");
    session.send_str("keepme\r");
    assert!(session.wait_for_text("[●:keepme]", Duration::from_secs(2)), "named tab did not open");

    session.send_keys(&[CTRL_B, b'r']);
    assert!(session.wait_for_text("Rename tab:", Duration::from_secs(2)), "rename prompt did not appear");

    session.send_keys(&[0x1b]); // Esc

    std::thread::sleep(Duration::from_millis(150));
    let screen = session.screen();
    assert!(
        !screen.contains("Rename tab:"),
        "Prompt should be gone after Esc\n---\n{}\n---",
        screen.full_text()
    );
    assert!(
        screen.contains("[●:keepme]"),
        "Tab name should be unchanged after Esc\n---\n{}\n---",
        screen.full_text()
    );
}
