//! Linux application discovery via freedesktop `.desktop` entries.

use std::collections::HashSet;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use freedesktop_desktop_entry::DesktopEntry;

use super::AppEntry;

/// XDG data directories in precedence order (`$XDG_DATA_HOME`, `$XDG_DATA_DIRS`,
/// then Flatpak export dirs). Earlier directories win when ids collide.
pub fn xdg_data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        if !data_home.is_empty() {
            dirs.push(PathBuf::from(data_home));
        }
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share"));
    }

    let system_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| {
        "/usr/local/share:/usr/share".to_string()
    });
    for dir in system_dirs.split(':').filter(|d| !d.is_empty()) {
        dirs.push(PathBuf::from(dir));
    }

    // Flatpak exports, best effort.
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(&home).join(".local/share/flatpak/exports/share"));
    }
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share"));

    dirs
}

fn collect_desktop_files(dir: &Path, out: &mut Vec<PathBuf>) {
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
            collect_desktop_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "desktop") {
            out.push(path);
        }
    }
}

/// Current desktop environments from `XDG_CURRENT_DESKTOP` (e.g. "zorin:GNOME").
pub fn current_desktops() -> Vec<String> {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn visible_on_current_desktop(entry: &DesktopEntry, desktops: &[String]) -> bool {
    if let Some(only_show_in) = entry.only_show_in() {
        return desktops.iter().any(|d| only_show_in.contains(&d.as_str()));
    }
    if let Some(not_show_in) = entry.not_show_in() {
        return !desktops.iter().any(|d| not_show_in.contains(&d.as_str()));
    }
    true
}

/// Scan every XDG data directory for launchable applications.
pub fn scan_desktop_entries() -> Vec<AppEntry> {
    let mut paths = Vec::new();
    for dir in xdg_data_dirs() {
        collect_desktop_files(&dir.join("applications"), &mut paths);
    }

    let desktops = current_desktops();
    let mut seen_ids = HashSet::new();
    let mut apps = Vec::new();

    // Precedence order: earlier dirs were collected first, keep the first hit.
    for path in paths {
        let Ok(entry) = DesktopEntry::from_path(path, None::<&[&str]>) else {
            continue;
        };

        let id = entry.appid.clone();
        if !seen_ids.insert(id.clone()) {
            continue;
        }

        if entry.no_display() || entry.hidden() {
            continue;
        }
        if let Some(app_type) = entry.type_() {
            if !app_type.eq_ignore_ascii_case("application") {
                continue;
            }
        }
        let Some(name) = entry.name(&[] as &[&str]).map(|n| n.into_owned()) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let Some(exec) = entry.exec().map(str::to_string) else {
            continue;
        };
        if !visible_on_current_desktop(&entry, &desktops) {
            continue;
        }

        let mut keywords = Vec::new();
        if let Some(generic_name) = entry.generic_name(&[] as &[&str]) {
            keywords.push(generic_name.into_owned());
        }

        let icon = resolve_icon(entry.icon());

        apps.push(AppEntry {
            id,
            name,
            keywords,
            icon,
            exec,
            terminal: entry.terminal(),
        });
    }

    apps
}

fn resolve_icon(icon: Option<&str>) -> Option<PathBuf> {
    let icon = icon?;
    let as_path = Path::new(icon);
    if as_path.is_absolute() {
        return Some(as_path.to_path_buf());
    }
    freedesktop_icons::lookup(icon)
        .with_theme("hicolor")
        .find()
        .or_else(|| freedesktop_icons::lookup(icon).find())
}

/// Convert a freedesktop `Exec` value into an argv vector, stripping field
/// codes (`%f`, `%U`, ...) per the Desktop Entry Specification.
pub fn exec_to_argv(exec: &str) -> Vec<String> {
    let tokens = tokenize_exec(exec);
    let mut argv = Vec::with_capacity(tokens.len());
    let mut chars_iter = tokens.into_iter();
    while let Some(token) = chars_iter.next() {
        let bytes: Vec<char> = token.chars().collect();
        let mut out = String::with_capacity(token.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == '%' && i + 1 < bytes.len() {
                let code = bytes[i + 1];
                if code == '%' {
                    out.push('%');
                    i += 2;
                    continue;
                }
                if "fFuUdDnNickvm".contains(code) {
                    i += 2; // drop field code entirely
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        if !out.is_empty() {
            argv.push(out);
        }
    }
    argv
}

/// Whitespace tokenizer honoring `'single'`, `"double"` quoting and
/// backslash escapes as mandated by the spec's quoting rules.
fn tokenize_exec(exec: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut chars = exec.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            '\'' => {
                for inner in chars.by_ref() {
                    if inner == '\'' {
                        break;
                    }
                    token.push(inner);
                }
            }
            '"' => {
                while let Some(inner) = chars.next() {
                    match inner {
                        '"' => break,
                        '\\' => {
                            if let Some(&next) = chars.peek() {
                                if matches!(next, '"' | '\\' | '$' | '`') {
                                    token.push(next);
                                    chars.next();
                                } else {
                                    token.push('\\');
                                }
                            }
                        }
                        _ => token.push(inner),
                    }
                }
            }
            '\\' => {
                if let Some(next) = chars.next() {
                    token.push(next);
                }
            }
            _ => token.push(c),
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

/// Terminal emulators tried in order when launching `Terminal=true` entries.
const TERMINALS: &[&str] = &[
    "ghostty", "alacritty", "kitty", "foot", "wezterm", "konsole", "gnome-terminal",
    "xfce4-terminal", "xterm",
];

fn terminal_command() -> Option<&'static str> {
    if let Ok(custom) = std::env::var("INVOKA_TERMINAL") {
        if !custom.is_empty() {
            return Some(Box::leak(custom.into_boxed_str()));
        }
    }
    TERMINALS
        .iter()
        .copied()
        .find(|term| which(*term).is_some())
}

fn which(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.is_file())
}

/// Launch a parsed application fully detached from this process.
pub fn launch(app: &AppEntry) {
    let mut argv = exec_to_argv(&app.exec);
    if argv.is_empty() {
        return;
    }

    if app.terminal {
        if let Some(term) = terminal_command() {
            let mut term_argv = vec![term.to_string()];
            // Most terminals need a "--" separator before the command.
            if matches!(term, "gnome-terminal" | "konsole" | "xfce4-terminal") {
                term_argv.push("--".into());
            }
            term_argv.extend(argv);
            argv = term_argv;
        }
    }

    let program = argv.remove(0);
    let mut command = Command::new(program);
    command.args(argv);
    command.stdin(std::process::Stdio::null());
    #[cfg(unix)]
    command.process_group(0);

    let _ = command.spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_plain_exec() {
        assert_eq!(
            exec_to_argv("firefox %u"),
            vec!["firefox".to_string()]
        );
        assert_eq!(
            exec_to_argv("code --new-window %F"),
            vec!["code".to_string(), "--new-window".to_string()]
        );
    }

    #[test]
    fn handles_quoting() {
        assert_eq!(
            exec_to_argv("sh -c 'echo hello world'"),
            vec!["sh", "-c", "echo hello world"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            exec_to_argv("app \"a b\" c"),
            vec!["app", "a b", "c"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn keeps_double_percent_literal() {
        assert_eq!(exec_to_argv("app 100%% sure"), vec!["app", "100%", "sure"]);
        assert_eq!(
            exec_to_argv("app \"100%% sure\""),
            vec!["app", "100% sure"]
        );
    }

    #[test]
    fn drops_all_field_codes() {
        assert_eq!(
            exec_to_argv("player %f %F %u %U %d %D %n %N %i %c %k %v %m"),
            vec!["player"]
        );
    }

    #[test]
    fn percent_inside_word_is_stripped_only_at_codes() {
        // %z is not a valid code, kept verbatim.
        assert_eq!(exec_to_argv("app %zfile"), vec!["app", "%zfile"]);
        // A trailing lone % survives.
        assert_eq!(exec_to_argv("app 50%"), vec!["app", "50%"]);
    }

    #[test]
    fn escapes_in_double_quotes() {
        assert_eq!(
            exec_to_argv("app \"x\\\"y\""),
            vec!["app", "x\"y"]
        );
    }
}
