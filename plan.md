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
| `Ctrl-b n` | Switch to next pane |
| `Ctrl-b p` | Switch to previous pane |
| `Ctrl-b q` | Quit |

---

## MVP Implementation Phases

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
