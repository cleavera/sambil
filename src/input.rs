use std::io::Write;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::pane_manager::PaneManager;
use crate::renderer;

pub fn event_loop<W: Write>(out: &mut W, manager: &mut PaneManager) -> Result<()> {
    let mut awaiting_command = false;

    loop {
        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if awaiting_command => {
                awaiting_command = false;
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('n') => {
                        manager.switch_to_next();
                        renderer::draw(out, manager)?;
                        out.flush()?;
                    }
                    KeyCode::Char('p') => {
                        manager.switch_to_prev();
                        renderer::draw(out, manager)?;
                        out.flush()?;
                    }
                    _ => {}
                }
            }
            Event::Key(key) => {
                if key.code == KeyCode::Char('b')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    awaiting_command = true;
                }
                // All other keys: forward to active pane's PTY (Phase 2)
            }
            Event::Resize(cols, rows) => {
                manager.cols = cols;
                manager.rows = rows;
                renderer::draw(out, manager)?;
                out.flush()?;
            }
            _ => {}
        }
    }
}
