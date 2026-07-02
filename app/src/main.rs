mod cell;
mod cursor;
mod config;
mod input;
mod pane;
mod pane_manager;
mod renderer;
mod scroll;
mod size;

use std::io;
use std::panic;

use as_source::AsSource;
use crossterm::{cursor::Hide, cursor::Show, event, execute, terminal};

use config::LoadConfigError;
use input::EventLoopError;
use pane_manager::NewError;
use size::TerminalSize;

#[derive(Debug, AsSource)]
enum RunError {
    AlreadyInSession,
    CouldNotEnableRawMode(std::io::Error),
    CouldNotSetupTerminal(std::io::Error),
    CouldNotGetTerminalSize(std::io::Error),
    #[from]
    CouldNotLoadConfig(LoadConfigError),
    #[from]
    CouldNotInitialiseManager(NewError),
    #[from]
    EventLoopFailed(EventLoopError),
}

fn main() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    if let Err(e) = run() {
        restore_terminal();
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    }
    restore_terminal();
}

fn run() -> Result<(), RunError> {
    if std::env::var("SAMBIL").is_ok() {
        return Err(RunError::AlreadyInSession);
    }

    let mut stdout = io::stdout();

    terminal::enable_raw_mode().map_err(RunError::CouldNotEnableRawMode)?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        Hide,
        event::EnableBracketedPaste
    ).map_err(RunError::CouldNotSetupTerminal)?;

    let cfg = config::load_or_create()?;
    let leader = config::parse_leader(&cfg.leader);

    let (cols, rows) = terminal::size().map_err(RunError::CouldNotGetTerminalSize)?;
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
        Show
    );
}
