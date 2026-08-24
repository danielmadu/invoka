//! Application discovery: entries, scanning and launching.

pub mod linux;

use std::path::PathBuf;

/// A single launchable application entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntry {
    /// Stable identifier (freedesktop app id on Linux, target path on Windows).
    pub id: String,
    pub name: String,
    /// Extra searchable text (generic name, keywords, comment).
    pub keywords: Vec<String>,
    /// Resolved absolute icon path, if any.
    pub icon: Option<PathBuf>,
    /// Command line to execute (Exec value with field codes stripped later).
    pub exec: String,
    /// Whether the app must run inside a terminal.
    pub terminal: bool,
}

impl AppEntry {
    #[cfg(test)]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            keywords: Vec::new(),
            icon: None,
            exec: String::new(),
            terminal: false,
        }
    }
}

/// Scan every known source for installed applications.
pub fn scan() -> Vec<AppEntry> {
    let mut apps = linux::scan_desktop_entries();
    sort_by_name(&mut apps);
    apps
}

/// Launch an application detached from this process.
pub fn launch(app: &AppEntry) {
    crate::apps::linux::launch(app);
}

fn sort_by_name(apps: &mut [AppEntry]) {
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
}
