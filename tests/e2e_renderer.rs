mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// After typing in tab 1, switching away and back, the output should still be visible.
#[test]
fn pane_content_persists_across_pane_switches() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[*1:", Duration::from_secs(2)), "sambil did not render");

    session.send_str("echo persist_check\n");
    assert!(
        session.wait_for_text("persist_check", Duration::from_secs(2)),
        "initial output did not appear"
    );

    // Open a second tab so we can actually switch away
    session.send_keys(&[CTRL_B, b'c']);
    assert!(session.wait_for_text("New tab name:", Duration::from_secs(2)), "prompt did not appear");
    session.send_str("\r");
    assert!(session.wait_for_text("[*2:", Duration::from_secs(2)), "tab 2 did not open");

    // Switch back to tab 1 — persist_check should still be there
    session.send_keys(&[CTRL_B, b'p']);
    assert!(
        session.wait_for_text("persist_check", Duration::from_secs(2)),
        "tab 1 content disappeared after switching away and back\n---\n{}\n---",
        session.screen().full_text()
    );
}
