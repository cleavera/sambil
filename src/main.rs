mod cell;
mod config;
mod input;
mod pane;
mod pane_manager;
mod renderer;
mod scroll;
mod size;

use std::io;
use std::panic;

use crossterm::{cursor, event, execute, terminal};

use size::TerminalSize;

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
    if std::env::var("SAMBIL").is_ok() {
        anyhow::bail!("already inside a sambil session (set by $SAMBIL)");
    }

    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        event::EnableBracketedPaste
    )?;

    let cfg = config::load_or_create();
    let leader = config::parse_leader(&cfg.leader);

    let (cols, rows) = terminal::size()?;
    let size = TerminalSize::new_clamped(cols, rows);
    let mut manager = pane_manager::PaneManager::new(size)?;
    let mut renderer = renderer::Renderer::new(size);

    input::event_loop(&mut stdout, &mut manager, &mut renderer, leader, &cfg.leader)?;

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
