# Sambil — Terminal Multiplexer Implementation Plan

## Overview

Sambil is a cross-platform terminal multiplexer written in Rust. The MVP provides two independent
terminal sessions rendered side-by-side in a single terminal window, with keyboard-driven switching
between them.

---

## Technology Stack

| Concern | Crate | Reason |
|---|---|---|
| Terminal I/O | `crossterm` | Cross-platform raw mode, input events, ANSI rendering |
| Pseudo-terminal | `portable-pty` | Cross-platform PTY from the WezTerm project (works on Linux, macOS, Windows) |
| Async runtime | `tokio` | Non-blocking reads from multiple PTY outputs simultaneously |
| E2E test harness | `vt100` | Parses ANSI escape codes into a queryable character grid for assertions |

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   App (main loop)                │
│                                                 │
│  ┌──────────┐   input   ┌──────────────────┐   │
│  │  Input   │ ────────► │  InputRouter     │   │
│  │  Reader  │           │  (active pane)   │   │
│  └──────────┘           └────────┬─────────┘   │
│                                  │ write        │
│  ┌────────────────────────────── ▼ ──────────┐  │
│  │              PaneManager                  │  │
│  │                                           │  │
│  │  ┌─────────────┐   ┌─────────────┐       │  │
│  │  │   Pane 0    │   │   Pane 1    │       │  │
│  │  │  PTY + buf  │   │  PTY + buf  │       │  │
│  │  └─────────────┘   └─────────────┘       │  │
│  └───────────────────────────────────────────┘  │
│                         │ render                │
│  ┌──────────────────────▼──────────────────┐    │
│  │               Renderer                  │    │
│  │   draws borders, pane contents, status  │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### Core Components

**`Pane`**
- Owns a `portable-pty` `PtyPair` and a child shell process
- Holds a scrollback output buffer (ring buffer of lines)
- Tracks cursor position within its viewport

**`PaneManager`**
- Holds `Vec<Pane>` (2 for MVP)
- Knows which pane is active
- Spawns output-reader tasks that write into each pane's buffer

**`InputRouter`**
- Reads raw keyboard events from `crossterm`
- Intercepts multiplexer key bindings (e.g. `Ctrl-b` prefix)
- Forwards all other bytes to the active pane's PTY stdin

**`Renderer`**
- Computes pane viewport dimensions from terminal size
- Draws vertical divider, pane borders, and status bar
- Renders each pane's visible output buffer
- Highlights the active pane's border

---

## Key Bindings (MVP)

| Key | Action |
|---|---|
| `Ctrl-b n` | Switch to next tab |
| `Ctrl-b p` | Switch to previous tab |
| `Ctrl-b 1`–`9` | Switch directly to tab N |
| `Ctrl-b c` | Open new tab (named after cwd) |
| `Ctrl-b C` | Open new tab (prompt for name) |
| `Ctrl-b x` | Close active tab (exits if last) |
| `Ctrl-b r` | Rename active tab |
| `Ctrl-b q` | Quit |

---

## Completed Features

- [x] Phase 0: Test harness (`TestSession`, `Screen`, `wait_for_text`, `wait_for_no_text`)
- [x] Phase 1: Project scaffold, raw mode, panic-safe terminal restore
- [x] Phase 2: PTY shell spawning, async output reader threads, cwd inheritance
- [x] Phase 3: Diff renderer (double-buffer, no flicker), tab bar with names
- [x] Phase 4: Input routing, `key_to_bytes`, arrow/ctrl keys
- [x] Phase 5: Tab switching (n/p/1–9), open (c/C), close (x), rename (r), quit (q)
- [x] Tab naming: cwd on creation, optional prompt, rename flow

---

## Next Phases

### Phase 7 — Colour fidelity
The PTY is not advertising full colour support to child processes, causing tools like `gitui` and
`ls --color` to produce fewer colours than expected. The fix is to set the `TERM` and `COLORTERM`
environment variables on the spawned shell to advertise 24-bit (truecolour) support, and ensure
the renderer emits the full SGR colour sequences from the `vt100` cell attributes rather than
approximating them.

Red/green: test that a child process which emits a known truecolour escape sequence has that colour
faithfully reproduced in the rendered output.

### Phase 8 — Shell exit handling
When the shell inside a tab exits (e.g. the user types `exit`), the pane becomes silently dead —
input is ignored and no new output arrives. The correct behaviour is to automatically close the tab
when its shell exits, the same way `Ctrl-b x` does (exiting sambil if it was the last tab).

Implementation: the output-reader thread detects EOF on the PTY master and sends a signal back to
the main event loop (e.g. via a channel) to trigger tab close.

Red/green: test that typing `exit\n` in a tab causes that tab to disappear from the tab bar (or
sambil to exit if it was the only tab).

### Phase 9 — Scrollback
Add a fixed-size scrollback buffer per pane (e.g. 1000 lines beyond the visible viewport). A new
`ScrollBack` input mode (entered with `Ctrl-b [`), navigated with arrow keys or Page Up/Down,
and exited with `q` or `Escape`. The renderer shows a scrollback indicator in the status bar when
not at the bottom.

Red/green: test that output scrolled off the top of the screen is accessible in scroll mode.

### Phase 10 — Paste & bracketed paste
Negotiate bracketed paste mode with the PTY so that pasted text is wrapped in `\e[200~` / `\e[201~`
escape sequences. This prevents shells and editors from misinterpreting pasted newlines as command
executions. Also handle large pastes without overflowing the PTY write buffer.

Red/green: test that text sent via a simulated paste is received by the shell wrapped in the
bracketed paste markers.

---

## Testing Strategy — Red/Green E2E

All features are driven by end-to-end tests written **before** their implementation (red), then made
to pass (green). There are no unit tests for internal components; the external behaviour is the
contract.

### How it works

Each test spawns `sambil` as a real subprocess inside a PTY, sends input bytes, reads back the
rendered output, and parses it through `vt100` into a character grid. Assertions query that grid
by screen region rather than raw escape sequences.

```
Test
 │
 ├─ spawn sambil in a PTY (via portable-pty)
 │
 ├─ write input bytes ──► sambil stdin
 │
 ├─ read output bytes ◄── sambil stdout
 │
 ├─ feed bytes into vt100::Parser → Screen (character grid)
 │
 └─ assert on Screen regions (left half, right half, status bar)
```

### `TestSession` helper

A shared test helper wraps all of the above:

```rust
let mut session = TestSession::spawn(cols: 80, rows: 24);

session.send_str("echo hello\n");
session.wait_for_text("hello", timeout);   // polls vt100 screen until text appears

session.send_keys(&[ctrl_b(), b'n']);      // switch pane

let screen = session.screen();
assert!(screen.region(left_half).contains("hello"));
assert!(!screen.region(right_half).contains("hello"));
```

`wait_for_text` polls with a short sleep rather than asserting immediately, handling the async
nature of PTY output. Tests fail with a timeout error if the expected text never appears.

### Pitfalls and mitigations

| Pitfall | Mitigation |
|---|---|
| Shell prompt varies per machine | Assert only on command output, not prompt text |
| PTY output is async | All assertions use `wait_for_text(timeout)` not instant reads |
| Windows PTY differences | CI runs Linux/macOS for MVP; Windows support validated manually |
| Flaky timing | Generous but finite timeouts (e.g. 2s); fail fast with clear messages |

### Test file structure

```
tests/
  common/
    mod.rs         — TestSession, Screen helpers, key constants
  e2e_startup.rs   — sambil launches and renders two panes
  e2e_input.rs     — typing in a pane produces output in that pane
  e2e_switching.rs — pane switching routes input to the correct shell
  e2e_quit.rs      — Ctrl-b q exits cleanly and restores the terminal
```

### Red/green workflow per phase

For each implementation phase:
1. Write the e2e test(s) — they must **fail** (red) before any code is written
2. Implement the minimum code to make them pass (green)
3. Refactor if needed, keeping tests green

---

## MVP Implementation Phases

Each phase follows red/green: write the failing test first, then implement.

### Phase 0 — Test harness
- Add `vt100` and `portable-pty` as dev-dependencies
- Implement `TestSession` in `tests/common/mod.rs`
- Implement `Screen` region helpers and `wait_for_text`
- Verify the harness itself can spawn a plain `cat` process and assert on its output

### Phase 1 — Project scaffold & dependencies
- Add `crossterm`, `portable-pty`, and `tokio` to `Cargo.toml`
- Set up module structure: `main.rs`, `pane.rs`, `pane_manager.rs`, `input.rs`, `renderer.rs`
- Enter raw terminal mode on startup, restore on exit (including panic handler)

### Phase 2 — PTY & shell spawning
- Implement `Pane::new(shell, size)` that spawns a PTY pair with `portable-pty`
- Launch the user's default shell (`$SHELL` / `cmd.exe` on Windows) as the child process
- Implement async output reader that appends bytes to the pane's buffer

### Phase 3 — Rendering
- Implement `Renderer::draw()` that:
  - Splits terminal width into two equal pane viewports
  - Draws a vertical border between panes
  - Renders each pane's last N lines of output into its viewport
  - Draws a status bar at the bottom showing active pane indicator

### Phase 4 — Input routing
- Implement raw input loop using `crossterm::event::read()`
- Detect `Ctrl-b` prefix and handle multiplexer commands
- Write all other input bytes directly to the active pane's PTY master

### Phase 5 — Pane switching
- Track active pane index in `PaneManager`
- `Ctrl-b n` / `Ctrl-b p` cycles active pane
- Renderer highlights the active pane border
- Input router directs keystrokes to the new active pane immediately

### Phase 6 — Integration & cleanup
- Wire all components together in `main.rs`
- Handle terminal resize events (`SIGWINCH` / crossterm resize events)
- Graceful shutdown: kill child processes, restore terminal state
- Basic manual smoke testing across Linux, macOS, Windows

---

## File Structure (target)

```
src/
  main.rs          — entry point, wires components, runs event loop
  pane.rs          — Pane struct, PTY lifecycle, output buffer
  pane_manager.rs  — owns Vec<Pane>, active index, spawn/teardown
  input.rs         — InputRouter, key binding detection
  renderer.rs      — terminal drawing, layout calculation

tests/
  common/
    mod.rs         — TestSession, Screen region helpers, key constants
  e2e_startup.rs   — sambil launches and renders two panes
  e2e_input.rs     — typing in a pane produces output in that pane
  e2e_switching.rs — pane switching routes input to the correct shell
  e2e_quit.rs      — Ctrl-b q exits cleanly and restores the terminal
```

---

## Out of Scope for MVP

- More than 2 panes
- Horizontal splits
- Scrollback navigation
- Copy/paste
- Configuration file
- Session save/restore
- Window/tab concepts
