mod common;

use std::time::Duration;

use common::{CTRL_B, TestSession};

/// Closing a tab (when it isn't the last) should be undoable within 10 seconds.
#[test]
fn closing_tab_can_be_undone() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();
    session.open_tab();

    // Close tab 2 (currently active).
    session.send_keys(&[CTRL_B, b'x']);
    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "tab 1 should be active after close"
    );
    // An undo hint should appear in the tab bar.
    assert!(
        session.wait_for_text("↩", Duration::from_secs(2)),
        "undo hint should be visible after close\n---\n{}\n---",
        session.screen().full_text()
    );

    // Undo the close — tab 2 should return as the active tab.
    session.send_keys(&[CTRL_B, b'u']);
    assert!(
        session.wait_for_text("[1:", Duration::from_secs(2)),
        "tab 1 should be inactive after undo (tab 2 restored as active)\n---\n{}\n---",
        session.screen().full_text()
    );
    // Undo hint should disappear once the queue is empty.
    assert!(
        session.wait_for_no_text("↩", Duration::from_secs(2)),
        "undo hint should disappear after undo\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Closing the last tab still exits immediately — no undo.
#[test]
fn last_tab_close_exits_immediately() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.send_keys(&[CTRL_B, b'x']);
    assert!(
        session.wait_for_exit(Duration::from_secs(2)),
        "sambil should exit when the last tab is closed"
    );
}
