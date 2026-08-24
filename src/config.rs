//! User configuration loaded from `<config_dir>/invoka/config.toml`.
//!
//! ```toml
//! [hotkey]
//! modifiers = ["ctrl", "alt"]   # alt, ctrl, shift, super
//! code = "space"                # space, a..z, 0..9, f1..f12, enter, esc, ...
//! ```

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub hotkey: HotkeyConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub modifiers: Vec<String>,
    pub code: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        // Alt+Space is the Raycast muscle memory, but GNOME owns it on Linux;
        // Ctrl+Alt+Space is the conflict-free default here.
        Self {
            modifiers: vec!["ctrl".into(), "alt".into()],
            code: "space".into(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: HotkeyConfig::default(),
        }
    }
}

/// Platform config directory for invoka (`$XDG_CONFIG_HOME/invoka` or
/// `%APPDATA%\invoka`).
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .map(|p| p.join("invoka"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME")?;
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| std::path::PathBuf::from(&home).join(".config"));
        Some(xdg.join("invoka"))
    }
}

impl Config {
    /// Load from `config.toml`, falling back to defaults on any error.
    pub fn load() -> Self {
        let Some(mut path) = config_dir() else {
            return Self::default();
        };
        path.push("config.toml");
        match std::fs::read_to_string(path) {
            Ok(contents) => Self::parse(&contents),
            Err(_) => Self::default(),
        }
    }

    pub fn parse(contents: &str) -> Self {
        let mut config: Config = toml::from_str(contents).unwrap_or_default();
        if config.hotkey.code.is_empty() {
            config.hotkey = HotkeyConfig::default();
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_file_missing_fields() {
        let config = Config::parse("");
        assert_eq!(config.hotkey.code, "space");
        assert_eq!(config.hotkey.modifiers, vec!["ctrl", "alt"]);
    }

    #[test]
    fn parses_custom_hotkey() {
        let config = Config::parse("[hotkey]\nmodifiers = [\"super\"]\ncode = \"KeyP\"\n");
        assert_eq!(config.hotkey.code, "KeyP");
        assert_eq!(config.hotkey.modifiers, vec!["super"]);
    }

    #[test]
    fn broken_toml_yields_defaults() {
        let config = Config::parse("[hotkey\nmodifiers = = =");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn empty_code_falls_back_to_default() {
        let config = Config::parse("[hotkey]\ncode = \"\"\n");
        assert_eq!(config.hotkey.code, "space");
    }
}
