mod common;

use std::time::Duration;

use common::TestSession;

/// Verifies the test harness itself: spawns a process, captures its output, and asserts on it.
#[test]
fn harness_captures_process_output() {
    let session =
        TestSession::spawn_process("sh", &["-c", "echo sambil_test_marker"], 80, 24);
    assert!(
        session.wait_for_text("sambil_test_marker", Duration::from_secs(2)),
        "Expected 'sambil_test_marker' on screen, got:\n{}",
        session.screen().full_text()
    );
}
