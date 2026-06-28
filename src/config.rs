use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG: &str = r#"# Sambil configuration
# This file was generated on first launch — edit to customise.

# The leader key prefix for all sambil commands.
# Examples: "ctrl+b", "ctrl+a", "ctrl+space"
leader = "ctrl+b"
"#;

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_leader")]
    pub leader: String,
}

fn default_leader() -> String {
    "ctrl+b".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config { leader: default_leader() }
    }
}

pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("sambil");
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("sambil")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("sambil")
    } else {
        PathBuf::from(".sambil")
    }
}

pub fn load_or_create() -> Config {
    let dir = config_dir();
    let path = dir.join("config.toml");

    if !path.exists() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&path, DEFAULT_CONFIG);
        return Config::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn parse_leader(leader: &str) -> (KeyCode, KeyModifiers) {
    let lower = leader.trim().to_lowercase();
    if let Some(rest) = lower.strip_prefix("ctrl+") {
        let code = match rest {
            "space" => KeyCode::Char(' '),
            s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
            _ => KeyCode::Char('b'),
        };
        return (code, KeyModifiers::CONTROL);
    }
    (KeyCode::Char('b'), KeyModifiers::CONTROL)
}
