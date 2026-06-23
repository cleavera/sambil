mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// Ctrl-b x closes the active tab and switches to the remaining tab.
#[test]
fn ctrl_b_x_closes_active_tab() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    session.send_keys(&[CTRL_B, b'c']);
    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "tab 2 did not open");

    session.send_keys(&[CTRL_B, b'x']);

    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "tab 1 should be active after closing tab 2\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        !session.screen().contains("[2:"),
        "tab 2 should be gone after closing\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Closing a tab that is not the last one in the list keeps the others intact.
#[test]
fn closing_first_tab_switches_to_next() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    session.send_keys(&[CTRL_B, b'c']);
    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "tab 2 did not open");

    session.send_keys(&[CTRL_B, b'c']);
    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "tab 3 did not open");

    // Go back to tab 1 and close it
    session.send_keys(&[CTRL_B, b'1']);
    // Wait until tab 3 is visible as inactive (confirms we're now on tab 1)
    assert!(session.wait_for_text("[3:", Duration::from_secs(2)), "did not switch to tab 1");

    session.send_keys(&[CTRL_B, b'x']);

    // After closing tab 1, the remaining tabs renumber: old tab 2 becomes tab 1
    assert!(
        session.wait_for_no_text("[3:", Duration::from_secs(2)),
        "tab 3 should have renumbered away after close\n---\n{}\n---",
        session.screen().full_text()
    );
    let screen = session.screen();
    assert!(
        screen.contains("[●:"),
        "should have an active tab 1 after close\n---\n{}\n---",
        screen.full_text()
    );
    assert!(
        screen.contains("[2:"),
        "should still have a tab 2\n---\n{}\n---",
        screen.full_text()
    );
}

/// Closing the last tab exits sambil cleanly.
#[test]
fn closing_last_tab_exits() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    session.send_keys(&[CTRL_B, b'x']);

    assert!(
        session.wait_for_exit(Duration::from_secs(2)),
        "sambil should exit when the last tab is closed"
    );
}
