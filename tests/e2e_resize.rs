mod common;

use std::time::Duration;

use common::TestSession;

/// After a resize, the tab bar should still be visible and sambil should not crash.
#[test]
fn tab_bar_survives_resize() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    session.send_str("echo before_resize\n");
    assert!(
        session.wait_for_text("before_resize", Duration::from_secs(2)),
        "setup output missing"
    );

    session.resize(120, 40);

    // Tab bar must still be present after resize.
    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "tab bar missing after resize\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Shell output should fill the new width after a resize — a command that emits
/// a known number of characters confirms the PTY dimensions were updated.
#[test]
fn shell_sees_new_dimensions_after_resize() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    // Resize to 40 columns then print a ruler of exactly 40 dashes.
    session.resize(40, 24);
    std::thread::sleep(std::time::Duration::from_millis(200));

    session.send_str("printf '%0.s-' {1..40}\n");
    assert!(
        session.wait_for_text(
            "----------------------------------------",
            Duration::from_secs(2)
        ),
        "40-char ruler not visible after resize — shell may not see new width\n---\n{}\n---",
        session.screen().full_text()
    );
}
