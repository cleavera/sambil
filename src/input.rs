use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::pane_manager::PaneManager;
use crate::renderer::Renderer;

pub fn event_loop<W: Write>(
    out: &mut W,
    manager: &mut PaneManager,
    renderer: &mut Renderer,
) -> Result<()> {
    let mut awaiting_command = false;

    loop {
        renderer.draw(out, manager)?;
        out.flush()?;

        if !event::poll(Duration::from_millis(16))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if awaiting_command => {
                awaiting_command = false;
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('c') => manager.open_tab()?,
                    KeyCode::Char('n') => manager.switch_to_next(),
                    KeyCode::Char('p') => manager.switch_to_prev(),
                    _ => {}
                }
            }
            Event::Key(key) => {
                if key.code == KeyCode::Char('b')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    awaiting_command = true;
                } else if let Some(bytes) = key_to_bytes(key.code, key.modifiers) {
                    manager.write_active(&bytes)?;
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


fn key_to_bytes(code: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    match code {
        KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
            let b = c.to_ascii_lowercase() as u8;
            if b.is_ascii_alphabetic() {
                Some(vec![b - b'a' + 1])
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
