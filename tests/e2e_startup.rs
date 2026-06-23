mod common;

use std::time::Duration;

use common::TestSession;

/// sambil should render a tab bar showing the single starting tab.
#[test]
fn startup_renders_tab_bar() {
    let session = TestSession::spawn_sambil(80, 24);
    assert!(
        session.wait_for_text("[●:", Duration::from_secs(2)),
        "Expected tab bar with active tab '[●:name]'\n---\n{}\n---",
        session.screen().full_text()
    );
    // Only one tab at startup — no [2: yet
    assert!(
        !session.screen().contains("[2:"),
        "Expected only one tab at startup\n---\n{}\n---",
        session.screen().full_text()
    );
}
