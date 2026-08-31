//! Theme presets bundled with invoka, taken verbatim from the Omarchy
//! quattro theme collection (`basecamp/omarchy`, branch `quattro`).
//!
//! The TOML sources are embedded at compile time so the binary ships
//! self-contained (no runtime asset lookups in AppImage/zip builds). The
//! settings window (M5) offers these as one-click selections; applying one
//! writes it to `<config_dir>/theme.toml`, which the M4 hot-reload watcher
//! picks up instantly.

/// One selectable theme preset.
#[derive(Debug)]
pub struct ThemePreset {
    /// Stable identifier (file stem).
    pub id: &'static str,
    /// Human-readable label for the settings UI.
    pub label: &'static str,
    /// Verbatim Omarchy `colors.toml` contents.
    pub toml: &'static str,
}

macro_rules! preset {
    ($id:literal, $label:literal) => {
        ThemePreset {
            id: $id,
            label: $label,
            toml: include_str!(concat!("../assets/themes/", $id, ".toml")),
        }
    };
}

/// Bundled presets in display order.
pub const PRESETS: &[ThemePreset] = &[
    preset!("catppuccin", "Catppuccin"),
    preset!("catppuccin-latte", "Catppuccin Latte"),
    preset!("tokyo-night", "Tokyo Night"),
    preset!("nord", "Nord"),
    preset!("gruvbox", "Gruvbox"),
    preset!("everforest", "Everforest"),
    preset!("kanagawa", "Kanagawa"),
    preset!("rose-pine", "Rosé Pine"),
    preset!("matte-black", "Matte Black"),
    preset!("hackerman", "Hackerman"),
];

/// Find a preset by id.
pub fn by_id(id: &str) -> Option<&'static ThemePreset> {
    PRESETS.iter().find(|preset| preset.id == id)
}

/// Which preset matches `theme`, or `"custom"` when the user's theme.toml
/// doesn't correspond to any bundled preset (e.g. hand-edited).
pub fn detect_active_id(theme: &crate::theme::Theme) -> &'static str {
    for preset in PRESETS {
        if &crate::theme::Theme::parse(preset.toml) == theme {
            return preset.id;
        }
    }
    "custom"
}

/// Write the preset `id` to `<config_dir>/theme.toml`; the hot-reload
/// watcher applies it instantly. Returns the written preset.
pub fn write_preset(id: &str) -> Option<&'static ThemePreset> {
    let preset = by_id(id)?;
    let mut path = crate::config::config_dir()?;
    if std::fs::create_dir_all(&path).is_err() {
        return None;
    }
    path.push("theme.toml");
    std::fs::write(&path, preset.toml).ok()?;
    Some(preset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn every_preset_parses_with_sane_colors() {
        assert!(!PRESETS.is_empty());
        let fallback = Theme::catppuccin();
        for preset in PRESETS {
            let theme = Theme::parse(preset.toml);
            for (field, value, fb) in [
                ("accent", &theme.accent, &fallback.accent),
                ("background", &theme.background, &fallback.background),
                ("foreground", &theme.foreground, &fallback.foreground),
                ("selection", &theme.selection, &fallback.selection),
                ("muted", &theme.muted, &fallback.muted),
            ] {
                if preset.id == "catppuccin" {
                    continue; // the default preset legitimately equals the fallback
                }
                assert_ne!(
                    value, fb,
                    "preset {} field {field} fell back to default (invalid color?)",
                    preset.id
                );
            }
            assert!(
                is_hex(&theme.accent) && is_hex(&theme.background),
                "preset {}",
                preset.id
            );
        }
    }

    fn is_hex(value: &str) -> bool {
        value.starts_with('#')
            && value.len() == 7
            && value[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn ids_are_unique_and_file_stems_match() {
        let mut ids: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate preset ids");
    }

    #[test]
    fn lookup_by_id_works() {
        assert!(by_id("catppuccin").is_some());
        assert!(by_id("tokyo-night").is_some());
        assert!(by_id("nonexistent").is_none());
    }
}
