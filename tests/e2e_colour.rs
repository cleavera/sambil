mod common;

use std::time::Duration;

use common::TestSession;

/// The shell inside a tab should see COLORTERM=truecolor so programs know they
/// can emit 24-bit colour sequences.
#[test]
fn colorterm_env_is_set() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "sambil did not render");

    session.send_str("echo $COLORTERM\n");

    assert!(
        session.wait_for_text("truecolor", Duration::from_secs(2)),
        "COLORTERM was not 'truecolor'\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// A truecolour escape sequence emitted by a child process must survive the
/// full round-trip: pane vt100 parser → renderer SGR output → test harness
/// vt100 parser, so the cell ends up with the correct RGB foreground colour.
#[test]
fn truecolor_sequences_pass_through() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[●:", Duration::from_secs(2)), "sambil did not render");

    // Emit a known RGB red foreground colour on the letter 'Z' (uncommon enough
    // that it won't collide with prompt text).
    session.send_str("printf '\\033[38;2;220;50;47mZ\\033[0m'\n");

    assert!(session.wait_for_text("Z", Duration::from_secs(2)), "'Z' did not appear on screen");

    assert!(
        session.wait_for_char_with_fg('Z', vt100::Color::Rgb(220, 50, 47), Duration::from_secs(2)),
        "cell 'Z' did not have expected RGB foreground colour\n---\n{}\n---",
        session.screen().full_text()
    );
}
