mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B, PAGE_UP};

/// Content that has scrolled off the top of the visible screen should be
/// accessible by entering scroll mode (Ctrl-b [) and scrolling up.
#[test]
fn scrolled_off_content_is_accessible() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    // Print 40 lines — far more than the 23 visible rows — ending with a
    // unique marker so we know all output has arrived.
    session.send_str("for i in $(seq 1 40); do echo LINE_$i; done\n");
    assert!(
        session.wait_for_text("LINE_40", Duration::from_secs(3)),
        "output did not finish\n---\n{}\n---",
        session.screen().full_text()
    );

    // The early lines must have scrolled off the visible area.
    assert!(
        !session.screen().contains("LINE_1"),
        "LINE_1 should have scrolled off the visible screen\n---\n{}\n---",
        session.screen().full_text()
    );

    // Enter scroll mode.
    session.send_keys(&[CTRL_B, b'[']);
    assert!(
        session.wait_for_text("SCROLL", Duration::from_secs(1)),
        "scroll mode indicator did not appear"
    );

    // Page up twice to bring early lines into view.
    session.send_keys(PAGE_UP);
    session.send_keys(PAGE_UP);

    assert!(
        session.wait_for_text("LINE_1", Duration::from_secs(2)),
        "LINE_1 should be visible after scrolling up\n---\n{}\n---",
        session.screen().full_text()
    );

    // Exit scroll mode — the live screen should return.
    session.send_str("q");
    assert!(
        session.wait_for_no_text("SCROLL", Duration::from_secs(1)),
        "scroll indicator should disappear after exiting scroll mode"
    );
}

/// Normal keyboard input must not be forwarded to the shell while in scroll mode.
#[test]
fn scroll_mode_does_not_forward_input_to_shell() {
    let mut session = TestSession::spawn_sambil(80, 24);

    session.assert_running();

    session.send_keys(&[CTRL_B, b'[']);
    assert!(session.wait_for_text("SCROLL", Duration::from_secs(1)), "scroll mode did not activate");

    // Type a command — it should NOT be executed.
    session.send_str("echo SHOULD_NOT_APPEAR\n");

    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(
        !session.screen().contains("SHOULD_NOT_APPEAR"),
        "input should not be forwarded to the shell in scroll mode"
    );

    // Exit scroll mode cleanly.
    session.send_str("q");
    assert!(session.wait_for_no_text("SCROLL", Duration::from_secs(1)));
}
