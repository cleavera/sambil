mod common;

use std::time::Duration;

use common::TestSession;

/// When a child process emits OSC 2 (set window title), the tab name should
/// update to "title/cwd" — but only if the tab hasn't been explicitly named.
#[test]
fn osc2_updates_auto_named_tab() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    // Emit a known OSC 2 title, then sleep briefly so the next prompt
    // (and any PROMPT_COMMAND/precmd title reset) can't fire during the check.
    session.send_str("printf '\\033]2;testapp\\007' && sleep 0.5\n");

    assert!(
        session.wait_for_text("[●:testapp]", Duration::from_secs(2)),
        "tab name should reflect OSC 2 title\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// If the user has explicitly named a tab (Ctrl-b C), OSC 2 sequences should
/// not override the name.
#[test]
fn osc2_does_not_override_explicit_name() {
    let mut session = TestSession::spawn_sambil(80, 24);
    session.assert_running();

    // Open a tab with an explicit name via Ctrl-b C.
    use common::CTRL_B;
    session.send_keys(&[CTRL_B, b'C']);
    session.assert_name_prompt();
    session.send_str("myproject\r");
    assert!(
        session.wait_for_text("[●:myproject]", Duration::from_secs(2)),
        "explicit name did not appear"
    );

    // Now emit a title sequence — name must not change.
    session.send_str("printf '\\033]2;sometool\\007'\n");
    std::thread::sleep(Duration::from_millis(300));

    assert!(
        session.screen().contains("[●:myproject]"),
        "explicit tab name should not be overridden by OSC 2\n---\n{}\n---",
        session.screen().full_text()
    );
}
