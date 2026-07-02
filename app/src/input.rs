use std::io::Write;
use std::time::Duration;

use as_source::AsSource;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::pane_manager::{PaneManager, TabIndex, CloseTabError, OpenTabError, SplitError, WriteError, ResizeError};
use crate::renderer::{Renderer, DrawError};
use crate::scroll::ScrollOffset;
use crate::size::TerminalSize;

#[derive(Debug, AsSource)]
pub enum HandleKeyError {
    #[from]
    CouldNotOpenTab(OpenTabError),
    #[from]
    CouldNotSplitPane(SplitError),
    #[from]
    CouldNotWriteInput(WriteError),
    #[from]
    CouldNotClosePane(CloseTabError),
}

#[derive(Debug, AsSource)]
pub enum EventLoopError {
    CouldNotFlushOutput(std::io::Error),
    CouldNotPollEvents(std::io::Error),
    CouldNotReadEvent(std::io::Error),
    #[from]
    CouldNotDraw(DrawError),
    #[from]
    CouldNotHandleKey(HandleKeyError),
    #[from]
    CouldNotWriteInput(WriteError),
    #[from]
    CouldNotResize(ResizeError),
    #[from]
    CouldNotCloseTab(CloseTabError),
}

enum InputMode {
    Normal,
    AwaitingCommand,
    Naming(String),
    Renaming(String),
    ScrollBack(ScrollOffset),
    Help,
    Quit,
}

pub fn event_loop<W: Write>(
    out: &mut W,
    manager: &mut PaneManager,
    renderer: &mut Renderer,
    leader: (KeyCode, KeyModifiers),
    leader_str: &str,
) -> Result<(), EventLoopError> {
    let mut mode = InputMode::Normal;

    loop {
        let prompt = match &mode {
            InputMode::Naming(buf) => Some(format!("New tab name: {}_", buf)),
            InputMode::Renaming(buf) => Some(format!("Rename tab: {}_", buf)),
            InputMode::ScrollBack(_) => {
                Some("-- SCROLL -- (↑↓/PgUp/PgDn, q/Esc to exit)".to_string())
            }
            _ => None,
        };
        let scroll_offset = match &mode {
            InputMode::ScrollBack(n) => *n,
            _ => ScrollOffset::zero(),
        };
        let show_help = matches!(mode, InputMode::Help);
        renderer.draw(out, manager, prompt.as_deref(), scroll_offset, show_help, leader_str)?;
        out.flush().map_err(EventLoopError::CouldNotFlushOutput)?;

        if !event::poll(Duration::from_millis(16)).map_err(EventLoopError::CouldNotPollEvents)? {
        if let Err(CloseTabError::TriedToCloseFinalTab) = manager.close_exited_tabs() {
            return Ok(());
        }
            manager.reap_pending_close();
            continue;
        }

        match event::read().map_err(EventLoopError::CouldNotReadEvent)? {
            Event::Key(key) => {
                mode = handle_key(key.code, key.modifiers, mode, manager, leader)?;
                if matches!(mode, InputMode::Quit) {
                    return Ok(());
                }
            }
            Event::Paste(text) => {
                let bytes = if manager.active_bracketed_paste() {
                    let mut v = b"\x1b[200~".to_vec();
                    v.extend_from_slice(text.as_bytes());
                    v.extend_from_slice(b"\x1b[201~");
                    v
                } else {
                    text.into_bytes()
                };
                manager.write_active(&bytes)?;
            }
            Event::Resize(cols, rows) => {
                let size = TerminalSize::new_clamped(cols, rows);
                manager.resize(size)?;
                renderer.invalidate(size);
            }
            _ => {}
        }
    }
}

fn handle_key(
    code: KeyCode,
    modifiers: KeyModifiers,
    mode: InputMode,
    manager: &mut PaneManager,
    leader: (KeyCode, KeyModifiers),
) -> Result<InputMode, HandleKeyError> {
    match mode {
        InputMode::AwaitingCommand => match code {
            KeyCode::Char('q') => return Ok(InputMode::Quit),
            KeyCode::Char('x') => {
                if let Err(CloseTabError::TriedToCloseFinalTab) = manager.close_active_pane() {
                    return Ok(InputMode::Quit);
                }
            }
            KeyCode::Char('u') => {
                let _ = manager.undo_close();
            }
            KeyCode::Char('c') => {
                manager.open_tab()?;
                return Ok(InputMode::Normal);
            }
            KeyCode::Char('C') => return Ok(InputMode::Naming(String::new())),
            KeyCode::Char('r') => {
                let current = manager.active_name().to_string();
                return Ok(InputMode::Renaming(current));
            }
            KeyCode::Char('|') => {
                manager.split_horizontal()?;
                return Ok(InputMode::Normal);
            }
            KeyCode::Left => {
                manager.focus_prev_pane();
            }
            KeyCode::Right => {
                manager.focus_next_pane();
            }
            KeyCode::Char('[') => return Ok(InputMode::ScrollBack(ScrollOffset::zero())),
            KeyCode::Char('?') => return Ok(InputMode::Help),
            KeyCode::Char('n') => manager.switch_to_next(),
            KeyCode::Char('p') => manager.switch_to_prev(),
            KeyCode::Char(d @ '1'..='9') => {
                manager.switch_to(TabIndex::from((d as usize) - ('1' as usize)));
            }
            _ => {}
        },

        InputMode::Naming(mut buf) => match code {
            KeyCode::Enter => {
                if buf.is_empty() {
                    manager.open_tab()?;
                } else {
                    manager.open_tab_named(buf)?;
                }
                return Ok(InputMode::Normal);
            }
            KeyCode::Esc => return Ok(InputMode::Normal),
            KeyCode::Backspace => {
                buf.pop();
                return Ok(InputMode::Naming(buf));
            }
            KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                buf.push(c);
                return Ok(InputMode::Naming(buf));
            }
            _ => return Ok(InputMode::Naming(buf)),
        },

        InputMode::Renaming(mut buf) => match code {
            KeyCode::Enter => {
                if buf.is_empty() {
                    manager.revert_active_name();
                } else {
                    manager.rename_active(buf);
                }
                return Ok(InputMode::Normal);
            }
            KeyCode::Esc => return Ok(InputMode::Normal),
            KeyCode::Backspace => {
                buf.pop();
                return Ok(InputMode::Renaming(buf));
            }
            KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                buf.push(c);
                return Ok(InputMode::Renaming(buf));
            }
            _ => return Ok(InputMode::Renaming(buf)),
        },

        InputMode::Help => return Ok(InputMode::Normal),

        InputMode::ScrollBack(offset) => {
            let rows = manager.size.into();
            match code {
                KeyCode::Up => return Ok(InputMode::ScrollBack(offset.scroll_up())),
                KeyCode::Down => return Ok(InputMode::ScrollBack(offset.scroll_down())),
                KeyCode::PageUp => return Ok(InputMode::ScrollBack(offset.page_up(rows))),
                KeyCode::PageDown => return Ok(InputMode::ScrollBack(offset.page_down(rows))),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(InputMode::Normal),
                _ => return Ok(InputMode::ScrollBack(offset)),
            }
        }

        InputMode::Normal => {
            if code == leader.0 && modifiers.contains(leader.1) {
                return Ok(InputMode::AwaitingCommand);
            }
            if let Some(bytes) = key_to_bytes(code, modifiers) {
                manager.write_active(&bytes)?;            }
        }
        InputMode::Quit => {}
    }

    Ok(InputMode::Normal)
}

fn key_to_bytes(code: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    match code {
        KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
            let b = c.to_ascii_uppercase() as u8;
            if b.is_ascii() && (0x40..=0x5F).contains(&b) {
                Some(vec![b & 0x1f])
            } else {
                None
            }
        }
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::Esc => Some(vec![b'\x1b']),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        _ => None,
    }
}
