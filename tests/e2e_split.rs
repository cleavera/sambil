mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B, LEFT_ARROW, RIGHT_ARROW};

/// Ctrl-b | splits the active tab, creating a second pane side by side.
/// The tab bar should show only one tab (same tab, now with two panes).
#[test]
fn split_creates_second_pane() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    let tab_count_before = session.screen().tab_count();

    session.send_keys(&[CTRL_B, b'|']);
    // A new pane spawns a shell; wait for the divider to appear.
    assert!(
        session.wait_for_text("│", Duration::from_secs(3)),
        "divider not rendered after split\n---\n{}\n---",
        session.screen().full_text()
    );

    // Tab count must NOT have increased — it's a pane split, not a new tab.
    assert_eq!(
        session.screen().tab_count(),
        tab_count_before,
        "split should not create a new tab"
    );
}

/// Panes are independent: output in one pane appears on the correct side only.
#[test]
fn split_panes_are_independent() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.send_keys(&[CTRL_B, b'|']);
    assert!(session.wait_for_text("│", Duration::from_secs(3)), "split did not render");

    // Type in the right (active) pane.
    session.send_str("echo RIGHTPANE\n");
    assert!(
        session.wait_for_text("RIGHTPANE", Duration::from_secs(2)),
        "output not visible in right pane"
    );

    // Switch focus to the left pane.
    session.send_keys(&[CTRL_B]);
    session.send_keys(LEFT_ARROW);
    session.send_str("echo LEFTPANE\n");
    assert!(
        session.wait_for_text("LEFTPANE", Duration::from_secs(2)),
        "output not visible in left pane"
    );

    // Both outputs should be visible simultaneously.
    let text = session.screen().full_text();
    assert!(text.contains("RIGHTPANE"), "right pane output disappeared");
    assert!(text.contains("LEFTPANE"), "left pane output missing");
}

/// Ctrl-b Left/Right moves focus between panes.
#[test]
fn focus_moves_between_panes() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    // Split — new right pane becomes active.
    session.send_keys(&[CTRL_B, b'|']);
    assert!(session.wait_for_text("│", Duration::from_secs(3)), "split did not render");

    // Move focus left and type a marker there.
    session.send_keys(&[CTRL_B]);
    session.send_keys(LEFT_ARROW);
    session.send_str("echo LEFT_FOCUSED\n");
    assert!(
        session.wait_for_text("LEFT_FOCUSED", Duration::from_secs(2)),
        "focus did not move to left pane"
    );

    // Move focus right and type a marker there.
    session.send_keys(&[CTRL_B]);
    session.send_keys(RIGHT_ARROW);
    session.send_str("echo RIGHT_FOCUSED\n");
    assert!(
        session.wait_for_text("RIGHT_FOCUSED", Duration::from_secs(2)),
        "focus did not move back to right pane"
    );
}

/// Ctrl-b x on a multi-pane tab closes the active pane, not the whole tab.
#[test]
fn close_pane_leaves_tab_open() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.send_keys(&[CTRL_B, b'|']);
    assert!(session.wait_for_text("│", Duration::from_secs(3)), "split did not render");

    // Close the active (right) pane.
    session.send_keys(&[CTRL_B, b'x']);
    // Divider should disappear as we're back to a single pane.
    assert!(
        session.wait_for_no_text("│", Duration::from_secs(2)),
        "divider still present after closing pane"
    );

    // Sambil should still be running.
    session.assert_running();
}

/// Ctrl-b x on the last pane in a tab closes the whole tab (existing behaviour).
#[test]
fn close_last_pane_closes_tab() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    // Open a second tab so closing the first doesn't quit.
    session.open_tab();
    let tab_count_before = session.screen().tab_count();

    session.send_keys(&[CTRL_B, b'x']);
    assert!(
        session.wait_for_screen(|s| s.tab_count() < tab_count_before, Duration::from_secs(2)),
        "closing last pane did not close the tab"
    );
}
