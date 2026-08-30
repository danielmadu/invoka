//! Filesystem watcher for live theme reload.
//!
//! Watches the config directory and re-applies `theme.toml` whenever it
//! changes (edits, symlink swaps, Omarchy theme switches), debounced so an
//! editor's burst of writes results in a single reload.

use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};

use crate::theme::Theme;

/// Debounce window: events arriving within this span collapse into one.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Start watching `<config_dir>/theme.toml`; returns false when watching is
/// unavailable (no config dir, no watcher backend).
pub fn start_theme_watcher() -> bool {
    let Some(dir) = crate::config::config_dir() else {
        return false;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let theme_path: PathBuf = dir.join("theme.toml");

    let (tx, rx) = channel();
    let mut watcher = match notify::recommended_watcher(tx) {
        Ok(watcher) => watcher,
        Err(err) => {
            eprintln!("[invoka] theme watcher unavailable: {err}");
            return false;
        }
    };
    if watcher.watch(&dir, RecursiveMode::NonRecursive).is_err() {
        eprintln!("[invoka] failed to watch {}", dir.display());
        return false;
    }
    // The watcher must live for the whole process; dropping it unregisters.
    std::mem::forget(watcher);

    std::thread::spawn(move || {
        eprintln!("[invoka] watching {} for theme changes", theme_path.display());
        let mut last: Option<Theme> = None;
        while let Ok(event) = rx.recv() {
            // Only theme.toml edits matter here.
            let relevant = event
                .as_ref()
                .map(|event| {
                    event
                        .paths
                        .iter()
                        .any(|path| path.file_name().is_some_and(|name| name == "theme.toml"))
                })
                .unwrap_or(false);
            if !relevant {
                continue;
            }

            // Debounce: drain everything arriving within the window.
            while rx.recv_timeout(DEBOUNCE).is_ok() {}

            let theme = Theme::load(&theme_path);
            if last.as_ref() == Some(&theme) {
                continue;
            }
            eprintln!(
                "[invoka] theme reloaded (accent {}, background {})",
                theme.accent, theme.background
            );
            last = Some(theme.clone());
            crate::bridge::apply_theme(theme);
        }
    });

    true
}
