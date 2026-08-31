use std::sync::{Mutex, OnceLock};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// An alias to the QString type
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "C++" {
        include!("tray.h");
        fn invoka_app_init();
        fn invoka_app_exec() -> i32;
        fn invoka_tray_init(on_toggle: fn(), on_quit: fn(), on_settings: fn());
    }

    unsafe extern "C++" {
        include!("layershell.h");
        /// Attach the launcher window to the wlr layer shell (feature builds
        /// only); returns 0 when the fallback plain window is in use.
        fn invoka_layershell_setup() -> i32;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, background)]
        #[qproperty(QString, foreground)]
        #[qproperty(QString, accent)]
        #[qproperty(QString, selection)]
        #[qproperty(QString, muted)]
        #[qproperty(bool, visible)]
        #[qproperty(bool, settings_visible)]
        #[qproperty(QString, themes_json)]
        #[qproperty(QString, active_theme_id)]
        #[namespace = "invoka"]
        type Controller = super::ControllerRust;

        /// One-time setup called from QML `Component.onCompleted`.
        ///
        /// Captures the Qt thread handle so background threads (IPC server,
        /// global hotkey listeners) can drive the launcher window later.
        #[qinvokable]
        fn bootstrap(self: Pin<&mut Controller>);

        /// Run the fuzzy search for `query` over installed applications.
        ///
        /// Returns a JSON array of result rows (`name`, `icon`) ordered by
        /// relevance; the matching indices are kept internally so
        /// `activate_index` can launch them.
        #[qinvokable]
        fn search(self: Pin<&mut Controller>, query: QString) -> QString;

        /// Launch the application currently displayed at `index` and hide.
        #[qinvokable]
        #[cxx_name = "activateIndex"]
        fn activate_index(self: Pin<&mut Controller>, index: i32);

        /// Hide the launcher window (Esc / focus loss / activation end).
        #[qinvokable]
        fn hide(self: Pin<&mut Controller>);

        /// Write the theme preset `id` to `<config_dir>/theme.toml`; the
        /// hot-reload watcher applies it instantly. Also refreshes
        /// `active_theme_id`.
        #[qinvokable]
        #[cxx_name = "selectTheme"]
        fn select_theme(self: Pin<&mut Controller>, id: QString);
    }

    // Enables CxxQtThread: background threads can queue closures that mutate
    // this QObject safely on the Qt event loop.
    impl cxx_qt::Threading for Controller {}
}

use core::pin::Pin;

use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::apps::{self, AppEntry};
use crate::theme::Theme;

/// Maximum number of result rows serialized back to QML.
const MAX_RESULTS: usize = 24;

/// Handle to the Qt event loop, captured during `bootstrap`.
static QT_THREAD: Mutex<Option<CxxQtThread<ffi::Controller>>> = Mutex::new(None);

/// Installed applications, scanned once on first use.
static APPS: OnceLock<Vec<AppEntry>> = OnceLock::new();

fn apps() -> &'static [AppEntry] {
    APPS.get_or_init(apps::scan)
}

/// Toggle launcher visibility from any thread; no-op pre-bootstrap.
pub fn toggle_window() {
    queue_on_qt(|controller| {
        let visible = *controller.visible();
        controller.set_visible(!visible);
    });
}

/// Open the settings window from any thread; no-op pre-bootstrap.
pub fn open_settings_window() {
    queue_on_qt(|controller| {
        controller.set_settings_visible(true);
    });
}

/// Queue `f` on the Qt event loop; no-op if the window was never bootstrapped.
pub fn queue_on_qt(f: impl FnOnce(Pin<&mut ffi::Controller>) + Send + 'static) -> bool {
    let guard = QT_THREAD.lock().unwrap();
    match guard.as_ref() {
        Some(thread) => match thread.queue(f) {
            Ok(()) => true,
            Err(err) => {
                eprintln!("[invoka] queue failed: {err}");
                false
            }
        },
        None => {
            eprintln!("[invoka] queue skipped: no qt thread (bootstrap not called?)");
            false
        }
    }
}

/// Apply a (possibly just-reloaded) theme to the live window; no-op
/// pre-bootstrap.
pub fn apply_theme(theme: Theme) {
    queue_on_qt(move |mut controller| {
        // `set_*` consumes the Pin; reborrow for each property.
        controller.as_mut().set_background(QString::from(theme.background.clone()));
        controller.as_mut().set_foreground(QString::from(theme.foreground.clone()));
        controller.as_mut().set_accent(QString::from(theme.accent.clone()));
        controller.as_mut().set_selection(QString::from(theme.selection.clone()));
        controller.as_mut().set_muted(QString::from(theme.muted.clone()));
        controller.as_mut()
            .set_active_theme_id(QString::from(crate::builtin::detect_active_id(&theme)));
    });
}

/// Theme tokens resolved once at startup.
fn load_theme() -> Theme {
    let Some(mut path) = crate::config::config_dir() else {
        return Theme::default();
    };
    path.push("theme.toml");
    Theme::load(&path)
}

/// Serialize the bundled presets for the settings UI: id, label and the
/// three colors the swatch previews need.
fn themes_json() -> String {
    let mut json = String::from("[");
    for (position, preset) in crate::builtin::PRESETS.iter().enumerate() {
        if position > 0 {
            json.push(',');
        }
        let theme = Theme::parse(preset.toml);
        json.push_str("{\"id\":\"");
        json.push_str(&escape_json(preset.id));
        json.push_str("\",\"label\":\"");
        json.push_str(&escape_json(preset.label));
        json.push_str("\",\"accent\":\"");
        json.push_str(&theme.accent);
        json.push_str("\",\"background\":\"");
        json.push_str(&theme.background);
        json.push_str("\",\"foreground\":\"");
        json.push_str(&theme.foreground);
        json.push_str("\"}");
    }
    json.push(']');
    json
}

/// Rust state backing the `Controller` QObject exposed to QML.
pub struct ControllerRust {
    ranked: Vec<usize>,
    background: QString,
    foreground: QString,
    accent: QString,
    selection: QString,
    muted: QString,
    visible: bool,
    settings_visible: bool,
    themes_json: QString,
    active_theme_id: QString,
}

impl Default for ControllerRust {
    fn default() -> Self {
        let theme = load_theme();
        Self {
            ranked: Vec::new(),
            background: QString::from(theme.background.clone()),
            foreground: QString::from(theme.foreground.clone()),
            accent: QString::from(theme.accent.clone()),
            selection: QString::from(theme.selection.clone()),
            muted: QString::from(theme.muted.clone()),
            visible: false,
            settings_visible: false,
            themes_json: QString::from(themes_json()),
            active_theme_id: QString::from(crate::builtin::detect_active_id(&theme)),
        }
    }
}

/// Tray menu callbacks (invoked from C++ on the Qt main thread).
fn tray_toggle() {
    toggle_window();
}

fn tray_quit() {
    std::process::exit(0);
}

fn tray_open_settings() {
    open_settings_window();
}

fn escape_json(value: &str) -> String {    let mut out = String::with_capacity(value.len() + 2);
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Icon path serialized to QML. Linux emits the raw path (QML prepends
/// `file://`); Windows emits a full `file:///C:/...` URL so `C:\` paths load.
fn icon_url(icon: Option<&std::path::PathBuf>) -> String {
    let Some(path) = icon else {
        return String::new();
    };
    #[cfg(windows)]
    {
        format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().into_owned()
    }
}

fn serialize_rows(ranked: &[usize], apps: &[AppEntry]) -> String {
    let mut rows = String::from("[");
    for (position, &app_index) in ranked.iter().take(MAX_RESULTS).enumerate() {
        if position > 0 {
            rows.push(',');
        }
        let app = &apps[app_index];
        rows.push_str("{\"name\":\"");
        rows.push_str(&escape_json(&app.name));
        rows.push_str("\",\"icon\":\"");
        rows.push_str(&escape_json(&icon_url(app.icon.as_ref())));
        rows.push_str("\"}");
    }
    rows.push(']');
    rows
}

impl ffi::Controller {
    pub fn bootstrap(self: Pin<&mut Self>) {
        // Warm the application cache so the first keystroke is instant.
        let _ = apps();

        let thread = self.qt_thread();
        *QT_THREAD.lock().unwrap() = Some(thread);

        ffi::invoka_tray_init(tray_toggle, tray_quit, tray_open_settings);
    }

    pub fn search(mut self: Pin<&mut Self>, query: QString) -> QString {
        let all_apps = apps();
        let ranked = crate::search::rank(all_apps, &query.to_string());
        let json = serialize_rows(&ranked, all_apps);

        self.as_mut().rust_mut().ranked = ranked;
        QString::from(json)
    }

    pub fn activate_index(mut self: Pin<&mut Self>, index: i32) {
        let position = index.max(0) as usize;
        let app_index = match self.as_mut().rust_mut().ranked.get(position) {
            Some(&app_index) => app_index,
            None => return,
        };
        let all_apps = apps();
        if let Some(app) = all_apps.get(app_index) {
            apps::launch(app);
            crate::usage::record(&app.id);
        }
        self.hide();
    }

    pub fn hide(self: Pin<&mut Self>) {
        self.set_visible(false);
    }

    pub fn select_theme(mut self: Pin<&mut Self>, id: QString) {
        let Some(preset) = crate::builtin::write_preset(&id.to_string()) else {
            eprintln!("[invoka] unknown theme preset '{}'", id.to_string());
            return;
        };
        // Apply immediately; the watcher dedupes (same parsed theme) so no
        // double reload happens when it picks the file change up.
        let theme = Theme::parse(preset.toml);
        self.as_mut().set_background(QString::from(theme.background.clone()));
        self.as_mut().set_foreground(QString::from(theme.foreground.clone()));
        self.as_mut().set_accent(QString::from(theme.accent.clone()));
        self.as_mut().set_selection(QString::from(theme.selection.clone()));
        self.as_mut().set_muted(QString::from(theme.muted.clone()));
        self.as_mut().set_active_theme_id(QString::from(preset.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_json_strings() {
        assert_eq!(escape_json("plain"), "plain");
        assert_eq!(escape_json("quo\"te"), "quo\\\"te");
        assert_eq!(escape_json("back\\slash"), "back\\\\slash");
        assert_eq!(escape_json("nl\n"), "nl\\n");
    }

    #[test]
    fn serializes_rows_as_json_array() {
        let entries = vec![AppEntry::new("a", "Alpha"), AppEntry::new("b", "Beta")];
        let json = serialize_rows(&[1, 0], &entries);
        assert_eq!(
            json,
            r#"[{"name":"Beta","icon":""},{"name":"Alpha","icon":""}]"#
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_icons_serialize_as_file_urls() {
        let mut entry = AppEntry::new("a", "Alpha");
        entry.icon = Some(std::path::PathBuf::from(r"C:\icons\app.png"));
        let json = serialize_rows(&[0], &[entry]);
        assert!(json.contains(r"file:///C:/icons/app.png"), "{json}");
    }

    #[test]
    fn themes_json_lists_every_preset_with_colors() {
        let json = themes_json();
        assert!(json.starts_with('[') && json.ends_with(']'));
        assert_eq!(json.matches("\"id\":\"").count(), crate::builtin::PRESETS.len());
        for preset in crate::builtin::PRESETS {
            assert!(json.contains(&format!("\"id\":\"{}\"", preset.id)), "{preset:?} missing");
            assert!(json.contains(&format!("\"label\":\"{}\"", preset.label)), "{preset:?} missing");
        }
        // Swatch colors present (hex triplets).
        assert!(json.contains("\"accent\":\"#") && json.contains("\"background\":\"#"));
        // Invalid JSON fragments would break QML JSON.parse; spot-check separators.
        assert!(!json.contains(",}"));
        assert!(!json.contains("{,"));
    }

    #[test]
    fn detect_active_matches_presets_and_custom() {
        use crate::builtin::detect_active_id;
        assert_eq!(detect_active_id(&Theme::parse(crate::builtin::PRESETS[0].toml)), "catppuccin");
        let mut tweaked = Theme::parse(crate::builtin::PRESETS[1].toml);
        tweaked.accent = "#123456".into();
        assert_eq!(detect_active_id(&tweaked), "custom");
    }

    #[test]
    fn empty_ranking_serializes_to_empty_array() {
        let entries: Vec<AppEntry> = Vec::new();
        assert_eq!(serialize_rows(&[], &entries), "[]");
    }
}
