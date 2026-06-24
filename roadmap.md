# Sambil Roadmap

This is a loose high-level view of where Sambil is headed. Nothing here is a firm commitment or timeline.

---

## Done

- Tabs — open, close, rename, undo close
- Horizontal splits with pane focus indicator
- Scrollback
- Terminal resize
- Window title passthrough (OSC 2 → tab name)
- ANSI palette colours (terminal theme compatible)
- Config file with configurable leader key

---

## Possible Future Work

- **Vertical splits** — stacking panes top/bottom (architecture already supports it)
- **Session persistence** — restore tabs and panes after a restart
- **More config options** — status bar content, colours, scrollback size
- **Themes / colour schemes** — inherits from the terminal

---

## Intentionally Out of Scope

- **Mouse support** — the whole point is to avoid the mouse
- **Scripting / plugins** — keep it simple
