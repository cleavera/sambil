#![allow(dead_code)]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

pub struct TestSession {
    writer: Box<dyn Write + Send>,
    parser: Arc<Mutex<vt100::Parser>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _master: Box<dyn portable_pty::MasterPty + Send>,
}

impl TestSession {
    pub fn spawn_sambil(cols: u16, rows: u16) -> Self {
        let bin = env!("CARGO_BIN_EXE_sambil");
        Self::spawn_process(bin, &[], cols, rows, &[])
    }

    pub fn spawn_sambil_with_env(cols: u16, rows: u16, env_vars: &[(&str, &str)]) -> Self {
        let bin = env!("CARGO_BIN_EXE_sambil");
        Self::spawn_process(bin, &[], cols, rows, env_vars)
    }

    pub fn spawn_process(bin: &str, args: &[&str], cols: u16, rows: u16, env_vars: &[(&str, &str)]) -> Self {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .expect("failed to open PTY");

        let mut cmd = CommandBuilder::new(bin);
        for arg in args {
            cmd.arg(arg);
        }
        for (k, v) in env_vars {
            cmd.env(k, v);
        }
        let child = pair.slave.spawn_command(cmd).expect("failed to spawn process");

        let writer = pair.master.take_writer().expect("failed to take PTY writer");
        let mut reader =
            pair.master.try_clone_reader().expect("failed to clone PTY reader");

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));
        let parser_clone = Arc::clone(&parser);

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
        });

        TestSession { writer, parser, child, _master: pair.master }
    }

    pub fn send_str(&mut self, s: &str) {
        self.writer.write_all(s.as_bytes()).expect("failed to write to PTY");
    }

    pub fn send_keys(&mut self, keys: &[u8]) {
        self.writer.write_all(keys).expect("failed to write keys to PTY");
    }

    /// Polls the rendered screen until `text` appears or the timeout is reached.
    pub fn wait_for_text(&self, text: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.screen().contains(text) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    pub fn wait_for_no_text(&self, text: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if !self.screen().contains(text) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// Polls until any cell with content `ch` has foreground colour `fg`.
    pub fn wait_for_char_with_fg(&self, ch: char, fg: vt100::Color, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.screen().has_char_with_fg(ch, fg) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// Polls until any cell with content `ch` has background colour `bg`.
    pub fn wait_for_char_with_bg(&self, ch: char, bg: vt100::Color, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.screen().has_char_with_bg(ch, bg) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    pub fn screen(&self) -> Screen {
        let parser = self.parser.lock().unwrap();
        Screen::capture(parser.screen())
    }

    /// Polls until the child process exits or the timeout is reached.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) => {}
                Err(_) => return false,
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }
}

/// A snapshot of the rendered terminal screen.
pub struct Screen {
    rows: u16,
    cols: u16,
    cells: Vec<String>,
    fg_colors: Vec<vt100::Color>,
    bg_colors: Vec<vt100::Color>,
}

impl Screen {
    fn capture(screen: &vt100::Screen) -> Self {
        let (rows, cols) = screen.size();
        let mut cells = Vec::with_capacity((rows * cols) as usize);
        let mut fg_colors = Vec::with_capacity((rows * cols) as usize);
        let mut bg_colors = Vec::with_capacity((rows * cols) as usize);
        for row in 0..rows {
            for col in 0..cols {
                match screen.cell(row, col) {
                    Some(c) => {
                        let s = c.contents();
                        cells.push(if s.is_empty() { " ".to_string() } else { s.to_string() });
                        fg_colors.push(c.fgcolor());
                        bg_colors.push(c.bgcolor());
                    }
                    None => {
                        cells.push(" ".to_string());
                        fg_colors.push(vt100::Color::Default);
                        bg_colors.push(vt100::Color::Default);
                    }
                }
            }
        }
        Screen { rows, cols, cells, fg_colors, bg_colors }
    }

    pub fn contains(&self, text: &str) -> bool {
        self.full_text().contains(text)
    }

    /// Returns true if any cell contains `ch` with exactly `fg` as its foreground colour.
    pub fn has_char_with_fg(&self, ch: char, fg: vt100::Color) -> bool {
        let ch_str = ch.to_string();
        for i in 0..(self.rows * self.cols) as usize {
            if self.cells[i] == ch_str && self.fg_colors[i] == fg {
                return true;
            }
        }
        false
    }

    /// Returns true if any cell containing `ch` has background colour `bg`.
    pub fn has_char_with_bg(&self, ch: char, bg: vt100::Color) -> bool {
        let ch_str = ch.to_string();
        for i in 0..(self.rows * self.cols) as usize {
            if self.cells[i] == ch_str && self.bg_colors[i] == bg {
                return true;
            }
        }
        false
    }

    pub fn left_half(&self) -> String {
        self.region_text(0, 0, self.rows, self.cols / 2)
    }

    pub fn right_half(&self) -> String {
        self.region_text(0, self.cols / 2, self.rows, self.cols)
    }

    pub fn full_text(&self) -> String {
        self.region_text(0, 0, self.rows, self.cols)
    }

    fn region_text(&self, row_start: u16, col_start: u16, row_end: u16, col_end: u16) -> String {
        let mut text = String::new();
        for row in row_start..row_end.min(self.rows) {
            for col in col_start..col_end.min(self.cols) {
                text.push_str(&self.cells[(row * self.cols + col) as usize]);
            }
            text.push('\n');
        }
        text
    }
}

pub const CTRL_B: u8 = 0x02;
pub const CTRL_C: u8 = 0x03;
pub const CTRL_D: u8 = 0x04;
pub const UP_ARROW: &[u8] = b"\x1b[A";
pub const DOWN_ARROW: &[u8] = b"\x1b[B";
pub const PAGE_UP: &[u8] = b"\x1b[5~";
pub const PAGE_DOWN: &[u8] = b"\x1b[6~";
