use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::pane_manager::{cwd_name, PaneManager};
use crate::renderer::Renderer;

enum InputMode {
    Normal,
    AwaitingCommand,
    Naming(String),
    Renaming(String),
    Quit,
}

pub fn event_loop<W: Write>(
    out: &mut W,
    manager: &mut PaneManager,
    renderer: &mut Renderer,
) -> Result<()> {
    let mut mode = InputMode::Normal;

    loop {
        let prompt = match &mode {
            InputMode::Naming(buf) => Some(format!("New tab name: {}_", buf)),
            InputMode::Renaming(buf) => Some(format!("Rename tab: {}_", buf)),
            _ => None,
        };
        renderer.draw(out, manager, prompt.as_deref())?;
        out.flush()?;

        if !event::poll(Duration::from_millis(16))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) => {
                mode = handle_key(key.code, key.modifiers, mode, manager)?;
                if matches!(mode, InputMode::Quit) {
                    return Ok(());
                }
            }
            Event::Resize(cols, rows) => {
                manager.resize(cols, rows)?;
                renderer.invalidate(cols, rows);
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
) -> Result<InputMode> {
    match mode {
        InputMode::AwaitingCommand => match code {
            KeyCode::Char('q') => return Ok(InputMode::Quit),
            KeyCode::Char('c') => {
                manager.open_tab(cwd_name())?;
                return Ok(InputMode::Normal);
            }
            KeyCode::Char('C') => return Ok(InputMode::Naming(String::new())),
            KeyCode::Char('r') => {
                let current = manager.active_name().to_string();
                return Ok(InputMode::Renaming(current));
            }
            KeyCode::Char('n') => manager.switch_to_next(),
            KeyCode::Char('p') => manager.switch_to_prev(),
            KeyCode::Char(d @ '1'..='9') => {
                manager.switch_to((d as usize) - ('1' as usize));
            }
            _ => {}
        },

        InputMode::Naming(mut buf) => match code {
            KeyCode::Enter => {
                let name = if buf.is_empty() { cwd_name() } else { buf };
                manager.open_tab(name)?;
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
                let name = if buf.is_empty() { cwd_name() } else { buf };
                manager.rename_active(name);
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

        InputMode::Normal => {
            if code == KeyCode::Char('b') && modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(InputMode::AwaitingCommand);
            }
            if let Some(bytes) = key_to_bytes(code, modifiers) {
                manager.write_active(&bytes)?;
            }
        }
        InputMode::Quit => {}
    }

    Ok(InputMode::Normal)
}

fn key_to_bytes(code: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    match code {
        KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
            let b = c.to_ascii_lowercase() as u8;
            if b.is_ascii_alphabetic() { Some(vec![b - b'a' + 1]) } else { None }
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