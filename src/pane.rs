use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

use crate::cursor::CursorStyle;
use crate::size::PaneSize;

/// vt100 callbacks implementation that captures OSC 2 window title sequences
/// and DECSCUSR cursor shape sequences.
#[derive(Default)]
pub struct TitleCallbacks {
    pub title: Option<String>,
    pub cursor_style: CursorStyle,
}

impl vt100::Callbacks for TitleCallbacks {
    fn set_window_title(&mut self, _screen: &mut vt100::Screen, title: &[u8]) {
        self.title = Some(String::from_utf8_lossy(title).into_owned());
    }

    fn unhandled_csi(
        &mut self,
        _screen: &mut vt100::Screen,
        i1: Option<u8>,
        _i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        // DECSCUSR: CSI Ps SP q — set cursor shape
        if c == 'q' && i1 == Some(b' ') {
            let ps = params.first().and_then(|p| p.first()).copied().unwrap_or(0);
            self.cursor_style = CursorStyle::from_decscusr(ps);
        }
    }
}

pub struct Pane {
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
    pub fn spawn(cwd: &std::path::Path, size: PaneSize) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: size.rows(),
            cols: size.cols(),
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = default_shell();
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
            vt100::Parser::new_with_callbacks(size.rows(), size.cols(), 1000, TitleCallbacks::default()),
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
            width: size.cols(),
            height: size.rows(),
            writer,
            parser,
            child_pid,
            exited,
            _child: child,
            master: pair.master,
            cwd: cwd.to_path_buf(),
        })
    }

    /// Auto-computed name: OSC 2 title if set, otherwise cwd basename.
    /// Used by `Tab::display_name()` when no explicit tab name is set.
    pub fn auto_name(&self) -> String {
        match self.parser.lock().unwrap().callbacks().title.as_deref() {
            Some(title) if !title.is_empty() => title.to_string(),
            _ => path_basename(&self.cwd),
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        Ok(())
    }

    pub fn resize(&mut self, size: PaneSize) -> Result<()> {
        self.width = size.cols();
        self.height = size.rows();
        self.master.resize(portable_pty::PtySize {
            rows: size.rows(),
            cols: size.cols(),
            pixel_width: 0,
            pixel_height: 0,
        })?;
        self.parser.lock().unwrap().screen_mut().set_size(size.rows(), size.cols());
        Ok(())
    }
}

pub fn path_basename(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "shell".to_string())
}

fn default_shell() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}
