mod common;

use std::time::Duration;

use common::TestSession;

/// sambil should render a tab bar showing both tabs on startup.
#[test]
fn startup_renders_tab_bar() {
    let session = TestSession::spawn_sambil(80, 24);
    assert!(
        session.wait_for_text("[*1]", Duration::from_secs(2)),
        "Expected tab bar with active tab '[*1]'\n---\n{}\n---",
        session.screen().full_text()
    );
    assert!(
        session.screen().contains("[2]"),
        "Expected tab bar to show '[2]'\n---\n{}\n---",
        session.screen().full_text()
    );
}
