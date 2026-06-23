mod input;
mod pane;
mod pane_manager;
mod renderer;

use std::io;
use std::panic;

use crossterm::{cursor, event, execute, terminal};

fn main() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    if let Err(e) = run() {
        restore_terminal();
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
    restore_terminal();
}

fn run() -> anyhow::Result<()> {
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        event::EnableBracketedPaste
    )?;

    let (cols, rows) = terminal::size()?;
    let mut manager = pane_manager::PaneManager::new(cols, rows)?;
    let mut renderer = renderer::Renderer::new(cols, rows);

    input::event_loop(&mut stdout, &mut manager, &mut renderer)?;

    Ok(())
}

fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        event::DisableBracketedPaste,
        terminal::LeaveAlternateScreen,
        cursor::Show
    );
}
