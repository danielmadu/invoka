//! Persisted launch-frequency ranking.
//!
//! Every launched app increments a counter keyed by app id; counts are kept
//! in memory and flushed to `<state>/invoka/usage.toml` so ordering survives
//! restarts. Used by the search ranking (frequency boost) and by the
//! empty-query ordering (most used first).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

/// Cap applied to the per-app frequency used for score boosts so a single
/// app's count can never fully dominate the fuzzy score.
pub const MAX_BOOST_STEPS: u32 = 64;

#[derive(Debug, Default, Serialize, Deserialize)]
struct UsageFile {
    /// app id -> launch count
    counts: HashMap<String, u32>,
}

static COUNTS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();

fn counts() -> &'static Mutex<HashMap<String, u32>> {
    COUNTS.get_or_init(|| Mutex::new(load()))
}

/// Snapshot of the persisted launch counts (app id -> count).
pub fn snapshot() -> HashMap<String, u32> {
    counts().lock().unwrap().clone()
}

/// State directory: `$XDG_STATE_HOME/invoka` (or `~/.local/state/invoka`) on
/// Linux, `%APPDATA%\invoka` on Windows.
#[cfg_attr(test, allow(unused))]
pub fn state_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(overridden) = TEST_STATE_DIR.lock().unwrap().clone() {
        return Some(overridden);
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA").map(PathBuf::from).map(|p| p.join("invoka"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME")?;
        let state_home = std::env::var("XDG_STATE_HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&home).join(".local").join("state"));
        Some(state_home.join("invoka"))
    }
}

pub fn state_path() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join("usage.toml"))
}

/// Load counts from disk; missing/broken files yield an empty map.
fn load() -> HashMap<String, u32> {
    let Some(path) = state_path() else {
        return HashMap::new();
    };
    match std::fs::read_to_string(path) {
        Ok(contents) => toml::from_str::<UsageFile>(&contents)
            .map(|file| file.counts)
            .unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// Persist counts atomically (write + rename); failures are silent — usage
/// data is a nicety, never worth crashing or spamming over.
fn store(counts: &HashMap<String, u32>) {
    let Some(path) = state_path() else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let file = UsageFile { counts: counts.clone() };
    let Ok(serialized) = toml::to_string(&file) else {
        return;
    };
    let tmp = path.with_extension("toml.tmp");
    if std::fs::write(&tmp, serialized).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

/// Record one launch of `app_id` and flush to disk.
pub fn record(app_id: &str) {
    if app_id.is_empty() {
        return;
    }
    let mut counts = counts().lock().unwrap();
    *counts.entry(app_id.to_string()).or_insert(0) += 1;
    store(&counts);
}

/// Frequency of `app_id` (0 if never launched).
pub fn frequency(app_id: &str) -> u32 {
    counts().lock().unwrap().get(app_id).copied().unwrap_or(0)
}

/// Capped frequency used for score boosts.
pub fn boost(app_id: &str) -> u32 {
    frequency(app_id).min(MAX_BOOST_STEPS)
}

/// Reset in-memory counts to the file contents (used by tests).
#[cfg(test)]
fn reload() {
    *counts().lock().unwrap() = load();
}

#[cfg(test)]
static TEST_STATE_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The tests share the process-global counts map and state-dir override, so
/// they must not interleave.
#[cfg(test)]
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn use_temp_state_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("invoka-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        *TEST_STATE_DIR.lock().unwrap() = Some(dir.clone());
        reload();
        dir
    }

    fn release_temp_state_dir(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
        *TEST_STATE_DIR.lock().unwrap() = None;
        reload();
    }

    #[test]
    fn record_persists_and_reloads() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = use_temp_state_dir("record");

        record("app-a");
        record("app-a");
        record("app-b");
        assert_eq!(frequency("app-a"), 2);
        assert_eq!(frequency("app-b"), 1);
        assert_eq!(frequency("never"), 0);

        // Simulate a restart: fresh memory, same file.
        *counts().lock().unwrap() = HashMap::new();
        reload();
        assert_eq!(frequency("app-a"), 2);
        assert_eq!(frequency("app-b"), 1);

        release_temp_state_dir(&dir);
    }

    #[test]
    fn boost_is_capped() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = use_temp_state_dir("boost");

        for _ in 0..(MAX_BOOST_STEPS + 50) {
            record("app-c");
        }
        assert_eq!(boost("app-c"), MAX_BOOST_STEPS);

        release_temp_state_dir(&dir);
    }

    #[test]
    fn empty_id_is_ignored() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = use_temp_state_dir("empty");

        record("");
        assert_eq!(frequency(""), 0);

        release_temp_state_dir(&dir);
    }
}
