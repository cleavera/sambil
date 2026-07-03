mod common;

use std::time::Duration;

use common::{CTRL_B, TestSession};

/// Typing `exit` in the only tab should cause sambil to exit cleanly.
#[test]
fn exit_in_last_tab_quits_sambil() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    session.send_str("exit\n");

    assert!(
        session.wait_for_exit(Duration::from_secs(3)),
        "sambil should exit when the last shell exits"
    );
}

/// Typing `exit` in one of multiple tabs should close just that tab and switch
/// to the next remaining one.
#[test]
fn exit_in_tab_closes_it_and_switches() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    session.open_tab();

    // Exit the shell in tab 2
    session.send_str("exit\n");

    assert!(
        session.wait_for_no_text("[2:", Duration::from_secs(3)),
        "tab 2 should disappear after its shell exits\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "tab 1 should become active\n---\n{}\n---",
        session.screen().full_text()
    );
}
