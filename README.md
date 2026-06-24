# Sambil

A cross-platform, keyboard-driven terminal multiplexer.

---

## Installation

```sh
cargo install sambil
```

Or build from source:

```sh
git clone https://github.com/cleavera/sambil
cd sambil
cargo build --release
```

---

## Features

- **Tabs** — open, close, rename, and switch between multiple terminal sessions
- **Horizontal splits** — divide a tab into side-by-side panes
- **Scrollback** — scroll through terminal history without leaving the keyboard
- **Undo close** — accidentally closed a tab? Press `Ctrl-b u` within 10 seconds
- **Tab names** — auto-named from the working directory and window title (OSC 2), or set manually
- **Terminal resize** — panes reflow automatically when the terminal is resized
- **Cross-platform** — Linux, macOS, and Windows (via `portable-pty`)
- **Theme-aware** — uses ANSI palette colours so it fits your terminal theme

---

## Key Bindings

All bindings use a leader key (`Ctrl-b` by default, configurable).

| Keys | Action |
|------|--------|
| `Ctrl-b c` | New tab (auto-named) |
| `Ctrl-b C` | New tab (enter name) |
| `Ctrl-b \|` | Split pane horizontally |
| `Ctrl-b x` | Close active pane (closes tab when last pane) |
| `Ctrl-b u` | Undo close tab (within 10 seconds) |
| `Ctrl-b r` | Rename active tab |
| `Ctrl-b n` | Next tab |
| `Ctrl-b p` | Previous tab |
| `Ctrl-b 1–9` | Switch to tab N |
| `Ctrl-b ←/→` | Move focus between panes |
| `Ctrl-b [` | Scroll mode (↑↓/PgUp/PgDn, `q` to exit) |
| `Ctrl-b ?` | Show help |
| `Ctrl-b q` | Quit |

---

## Configuration

Sambil writes a config file on first launch at:

- **Linux/macOS**: `~/.config/sambil/config.toml`
- **Windows**: `%APPDATA%\sambil\config.toml`

```toml
leader = "ctrl+b"
```

---

## License

MIT — see [LICENSE](LICENSE).
