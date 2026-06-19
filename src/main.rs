mod input;
mod pane;
mod pane_manager;
mod renderer;

use std::io::{self, Write};
use std::panic;

use crossterm::{cursor, execute, terminal};

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
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let (cols, rows) = terminal::size()?;
    let mut manager = pane_manager::PaneManager::new(cols, rows)?;

    renderer::draw(&mut stdout, &manager)?;
    stdout.flush()?;

    input::event_loop(&mut stdout, &mut manager)?;

    Ok(())
}

fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show);
}
