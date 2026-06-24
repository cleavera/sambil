use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

/// vt100 callbacks implementation that captures OSC 2 window title sequences.
#[derive(Default)]
pub struct TitleCallbacks {
    pub title: Option<String>,
}

impl vt100::Callbacks for TitleCallbacks {
    fn set_window_title(&mut self, _screen: &mut vt100::Screen, title: &[u8]) {
        self.title = Some(String::from_utf8_lossy(title).into_owned());
    }
}

pub struct Pane {
    /// Explicit user-set name. `None` means auto-named: display is derived
    /// from the OSC 2 window title (if any) and the cwd basename.
    pub name: Option<String>,
    pub width: u16,
    pub height: u16,
    writer: Box<dyn Write + Send>,
    pub parser: Arc<Mutex<vt100::Parser<TitleCallbacks>>>,
    pub child_pid: Option<u32>,
    pub exited: Arc<AtomicBool>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    pub cwd: std::path::PathBuf,
}

impl Pane {
    pub fn spawn(cwd: &std::path::Path, width: u16, height: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: height,
            cols: width,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("SAMBIL", "1");
        let child = pair.slave.spawn_command(cmd)?;
        let child_pid = child.process_id();

        let writer = pair.master.take_writer()?;
        let mut reader = pair.master.try_clone_reader()?;

        let parser = Arc::new(Mutex::new(
            vt100::Parser::new_with_callbacks(height, width, 1000, TitleCallbacks::default()),
        ));
        let parser_clone = Arc::clone(&parser);
        let exited = Arc::new(AtomicBool::new(false));
        let exited_clone = Arc::clone(&exited);

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        parser_clone.lock().unwrap().process(&buf[..n]);
                    }
                }
            }
            exited_clone.store(true, Ordering::Relaxed);
        });

        Ok(Pane {
            name: None,
            width,
            height,
            writer,
            parser,
            child_pid,
            exited,
            _child: child,
            master: pair.master,
            cwd: cwd.to_path_buf(),
        })
    }

    /// The display name shown in the tab bar.
    /// - Explicit name (`Some`) is shown as-is.
    /// - Auto-named (`None`): `title/cwd` if an OSC 2 title is set, otherwise just `cwd`.
    pub fn display_name(&self) -> String {
        if let Some(ref name) = self.name {
            return name.clone();
        }
        let cwd = crate::pane_manager::path_basename(&self.cwd);
        match self.parser.lock().unwrap().callbacks().title.as_deref() {
            Some(title) if !title.is_empty() => title.to_string(),
            _ => cwd,
        }
    }

    /// Returns the latest OSC 2 title emitted by the child, if any.
    pub fn window_title(&self) -> Option<String> {
        self.parser.lock().unwrap().callbacks().title.clone()
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        Ok(())
    }

    pub fn resize(&mut self, new_width: u16, new_height: u16) -> Result<()> {
        self.width = new_width;
        self.height = new_height;
        self.master.resize(portable_pty::PtySize {
            rows: new_height,
            cols: new_width,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        self.parser.lock().unwrap().screen_mut().set_size(new_height, new_width);
        Ok(())
    }
}

