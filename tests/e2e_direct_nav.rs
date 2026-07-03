mod common;

use std::time::Duration;

use common::{CTRL_B, TestSession};

/// Ctrl-b 1, Ctrl-b 2 etc navigate directly to a tab by number.
#[test]
fn ctrl_b_number_switches_directly_to_tab() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    // Open two more tabs so we have 1, 2, 3
    session.open_tab();

    session.open_tab();

    // Jump directly back to tab 1
    session.send_keys(&[CTRL_B, b'1']);
    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "Ctrl-b 1 did not navigate to tab 1\n---\n{}\n---",
        session.screen().full_text()
    );

    // Jump directly to tab 3
    session.send_keys(&[CTRL_B, b'3']);
    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "Ctrl-b 3 did not navigate to tab 3\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Pressing Ctrl-b with a number beyond the tab count is a no-op.
#[test]
fn ctrl_b_out_of_range_number_is_a_noop() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    session.send_keys(&[CTRL_B, b'9']);

    std::thread::sleep(Duration::from_millis(100));
    assert!(
        session.screen().contains("[●:"),
        "Active tab should still be 1 after out-of-range navigation\n---\n{}\n---",
        session.screen().full_text()
    );
}
