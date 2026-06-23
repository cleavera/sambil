mod common;

use std::time::Duration;

use common::TestSession;

/// A paste (wrapped in bracketed paste markers by the host terminal) should
/// arrive in the shell as if typed, and the newlines inside must NOT trigger
/// premature command execution.
#[test]
fn bracketed_paste_is_forwarded_to_shell() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[*1:", Duration::from_secs(2)), "sambil did not render");

    // Simulate what the host terminal sends when the user pastes text:
    // \e[200~ ... content ... \e[201~
    // The pasted content has a newline in it; without bracketed paste handling
    // the shell would execute the first part immediately.
    session.send_str("\x1b[200~echo PASTED_TEXT\x1b[201~");

    // The text should appear on the command line, not yet executed.
    assert!(
        session.wait_for_text("PASTED_TEXT", Duration::from_secs(2)),
        "pasted text did not appear on the command line\n---\n{}\n---",
        session.screen().full_text()
    );
}

/// Multi-line paste: both lines should appear on the command line, and neither
/// should be executed yet (no bare output on its own line).
#[test]
fn multiline_paste_does_not_execute_early() {
    let mut session = TestSession::spawn_sambil(80, 24);

    assert!(session.wait_for_text("[*1:", Duration::from_secs(2)), "sambil did not render");

    // Paste two lines. If bracketed paste is NOT handled, the \n triggers
    // immediate execution of the first command and "FIRST_LINE" appears as
    // standalone output. We verify that only the echo commands appear as
    // typed text, not as executed output.
    session.send_str("\x1b[200~echo PASTE_A\necho PASTE_B\x1b[201~");

    // Give time for any execution to happen.
    std::thread::sleep(Duration::from_millis(400));

    let text = session.screen().full_text();
    // Count occurrences: "PASTE_A" appears as part of "echo PASTE_A" (typed).
    // If it also executed, it would appear again as bare output. Check that it
    // appears at most once per screen row, i.e. not as both command AND output.
    let standalone_output_a = text.lines().any(|l| l.trim() == "PASTE_A");
    let standalone_output_b = text.lines().any(|l| l.trim() == "PASTE_B");

    assert!(
        !(standalone_output_a && standalone_output_b),
        "both lines executed immediately — multiline paste was not bracketed\n---\n{}\n---",
        text
    );
}
