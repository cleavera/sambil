mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// Ctrl-b ? should display an overlay showing the key bindings.
#[test]
fn ctrl_b_question_shows_help_overlay() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[*1:", Duration::from_secs(2)), "sambil did not render");

    session.send_keys(&[CTRL_B, b'?']);

    assert!(
        session.wait_for_text("Ctrl-b c", Duration::from_secs(2)),
        "help overlay did not appear\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        session.screen().contains("Ctrl-b q"),
        "help overlay missing quit binding\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        session.screen().contains("any key"),
        "help overlay should say how to dismiss\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Pressing any key while the help overlay is shown should dismiss it and
/// return to the normal tab view.
#[test]
fn any_key_dismisses_help_overlay() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[*1:", Duration::from_secs(2)), "sambil did not render");

    session.send_keys(&[CTRL_B, b'?']);
    assert!(session.wait_for_text("Ctrl-b c", Duration::from_secs(2)), "help did not appear");

    // Press Space to dismiss
    session.send_str(" ");

    assert!(
        session.wait_for_no_text("Ctrl-b c", Duration::from_secs(2)),
        "help overlay should disappear after keypress\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        session.wait_for_text("[*1:", Duration::from_secs(2)),
        "tab bar should be visible after dismissing help"
    );
}
