mod common;

use std::time::Duration;

use common::TestSession;

/// RED: sambil should render two panes separated by a vertical border.
/// Fails until Phase 1 + 3 are implemented.
#[test]
fn startup_renders_two_panes_with_a_border() {
    let session = TestSession::spawn_sambil(80, 24);
    assert!(
        session.wait_for_text("│", Duration::from_secs(2)),
        "Expected vertical border '│' between the two panes\n---\n{}\n---",
        session.screen().full_text()
    );
}
