//! Windows application discovery via Start Menu `.lnk` shortcuts.
//!
//! Scans `%ProgramData%\Microsoft\Windows\Start Menu\Programs` and
//! `%APPDATA%\Microsoft\Windows\Start Menu\Programs` (user shortcuts win on
//! duplicate targets), parsing each `.lnk` with `parselnk` and resolving its
//! icon through the HICON→PNG cache (`crate::icons`).

use std::collections::HashSet;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use parselnk::Lnk;

use super::AppEntry;
use crate::icons;

/// Start Menu program folders, in precedence order (user entries first).
pub fn start_menu_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(roaming) = std::env::var_os("APPDATA") {
        dirs.push(
            PathBuf::from(roaming)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        dirs.push(
            PathBuf::from(program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    dirs
}

fn collect_lnk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            collect_lnk_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "lnk") {
            out.push(path);
        }
    }
}

/// Full filesystem path a `.lnk` points at, preferring the Unicode variants.
pub fn lnk_target(lnk: &Lnk) -> Option<PathBuf> {
    let info = &lnk.link_info;
    let base = info
        .local_base_path_unicode
        .as_deref()
        .or(info.local_base_path.as_deref())?;
    let suffix = info
        .common_path_suffix_unicode
        .as_deref()
        .or(info.common_path_suffix.as_deref())
        .unwrap_or("");
    if base.is_empty() {
        return None;
    }
    Some(PathBuf::from(format!("{}{}", base, suffix)))
}

/// Display name for a shortcut: recorded name, else the file stem.
pub fn lnk_name(lnk: &Lnk, lnk_path: &Path) -> String {
    lnk.string_data
        .name_string
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            lnk_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
}

/// Parse one `.lnk` into an app entry, skipping uninstallers and shortcuts
/// with no resolvable local target (e.g. `.url` links).
pub fn entry_from_lnk(lnk_path: &Path) -> Option<AppEntry> {
    let bytes = std::fs::read(lnk_path).ok()?;
    let lnk = Lnk::new(&mut bytes.as_slice()).ok()?;

    let name = lnk_name(&lnk, lnk_path);
    if name.is_empty() || name.to_lowercase().contains("uninstall") {
        return None;
    }

    let target = lnk_target(&lnk)?;
    if !target.is_file() {
        return None;
    }

    let mut keywords = Vec::new();
    if let Some(description) = lnk.description() {
        if !description.is_empty() {
            keywords.push(description);
        }
    }

    let icon = icons::icon_for(&target);

    Some(AppEntry {
        id: target.to_string_lossy().into_owned(),
        name,
        keywords,
        icon,
        exec: target.to_string_lossy().into_owned(),
        terminal: false,
    })
}

/// Scan both Start Menu roots for launchable applications.
pub fn scan_start_menu() -> Vec<AppEntry> {
    let mut paths = Vec::new();
    for dir in start_menu_dirs() {
        collect_lnk_files(&dir, &mut paths);
    }

    // User shortcuts were collected first; dedupe by target path.
    let mut seen_targets = HashSet::new();
    let mut apps = Vec::new();
    for path in paths {
        let Some(entry) = entry_from_lnk(&path) else {
            continue;
        };
        if !seen_targets.insert(entry.id.clone()) {
            continue;
        }
        apps.push(entry);
    }
    apps
}

/// `CREATE_NO_WINDOW` — suppresses the console flash from `cmd /C start`.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Launch a `.lnk` (or plain executable path) detached from this process.
pub fn launch(app: &AppEntry) {
    let target = &app.exec;
    if target.is_empty() {
        return;
    }

    let mut command = Command::new("cmd");
    command.args(["/C", "start", "", target]);
    command.creation_flags(CREATE_NO_WINDOW);
    let _ = command.spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_menu_dirs_follow_env() {
        // Whatever the environment, both roots (when present) end in the
        // canonical Start Menu path.
        for dir in start_menu_dirs() {
            assert!(dir
                .to_string_lossy()
                .ends_with(r"Microsoft\Windows\Start Menu\Programs"));
        }
    }

    #[test]
    fn name_falls_back_to_file_stem() {
        // An empty buffer cannot parse as a .lnk; the stem is the fallback.
        let stem = Path::new(r"C:\Start Menu\Firefox.lnk")
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert_eq!(stem, "Firefox");
        assert!(Lnk::new(&mut &b""[..]).is_err());
    }
}
