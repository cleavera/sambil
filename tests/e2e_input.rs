mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// RED: typing a command in the active pane should show output in the left half of the screen.
/// Fails until Phase 2 (PTY spawning) is implemented.
#[test]
fn typing_in_pane_0_shows_output_in_left_half() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(
        session.wait_for_text("│", Duration::from_secs(2)),
        "sambil did not render border — is it running?\n---\n{}\n---",
        session.screen().full_text()
    );

    session.send_str("echo pane0_output\n");

    let appeared = session.wait_for_text("pane0_output", Duration::from_secs(2));
    let screen = session.screen();

    assert!(
        appeared,
        "Expected 'pane0_output' on screen after typing in pane 0\n---\n{}\n---",
        screen.full_text()
    );

    assert!(
        screen.left_half().contains("pane0_output"),
        "Expected output in left pane (pane 0), but it wasn't there\nleft:\n{}\nright:\n{}",
        screen.left_half(),
        screen.right_half()
    );
}

/// RED: after switching to pane 1 and typing, output should appear in the right half.
/// Fails until Phase 5 (pane switching) is implemented.
#[test]
fn typing_in_pane_1_after_switch_shows_output_in_right_half() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(
        session.wait_for_text("│", Duration::from_secs(2)),
        "sambil did not render"
    );

    // Switch to pane 1
    session.send_keys(&[CTRL_B, b'n']);

    session.send_str("echo pane1_output\n");

    let appeared = session.wait_for_text("pane1_output", Duration::from_secs(2));
    let screen = session.screen();

    assert!(
        appeared,
        "Expected 'pane1_output' on screen after switching to pane 1\n---\n{}\n---",
        screen.full_text()
    );

    assert!(
        screen.right_half().contains("pane1_output"),
        "Expected output in right pane (pane 1)\nleft:\n{}\nright:\n{}",
        screen.left_half(),
        screen.right_half()
    );
}
