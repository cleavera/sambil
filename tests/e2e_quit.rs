mod common;

use std::time::Duration;

use common::{TestSession, CTRL_B};

/// Ctrl-b q should cause sambil to exit cleanly within a reasonable time.
#[test]
fn ctrl_b_q_exits_cleanly() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(
        session.wait_for_text("[*1:", Duration::from_secs(2)),
        "sambil did not render before quit attempt"
    );

    session.send_keys(&[CTRL_B, b'q']);

    assert!(
        session.wait_for_exit(Duration::from_secs(2)),
        "sambil did not exit after Ctrl-b q"
    );
}
