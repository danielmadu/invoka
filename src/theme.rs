//! Theme tokens parsed from Omarchy quattro-style `colors.toml` files.
//!
//! Field names intentionally mirror the upstream Omarchy theme format so users
//! can copy theme files straight from `basecamp/omarchy/themes/<name>/colors.toml`.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub mode: String,
    pub accent: String,
    pub selection: String,
    pub muted: String,
    pub background: String,
    pub dark_background: String,
    pub darker_background: String,
    pub lighter_background: String,
    pub foreground: String,
    pub dark_foreground: String,
    pub light_foreground: String,
    pub bright_foreground: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin()
    }
}

impl Theme {
    /// The default theme: Catppuccin Mocha, matching Omarchy's shipped default.
    pub fn catppuccin() -> Self {
        Self {
            mode: "dark".into(),
            accent: "#89b4fa".into(),
            selection: "#45475a".into(),
            muted: "#585b70".into(),
            background: "#1e1e2e".into(),
            dark_background: "#161622".into(),
            darker_background: "#101019".into(),
            lighter_background: "#313244".into(),
            foreground: "#cdd6f4".into(),
            dark_foreground: "#6c7086".into(),
            light_foreground: "#bac2de".into(),
            bright_foreground: "#cdd6f4".into(),
        }
    }

    /// Parse a theme file. Falls back to the default theme on any error.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => Self::parse(&contents),
            Err(_) => Self::default(),
        }
    }

    pub fn parse(contents: &str) -> Self {
        let mut theme: Theme = toml::from_str(contents).unwrap_or_default();
        sanitize(&mut theme);
        theme
    }
}

/// Guarantee every token is a usable `#rrggbb` color so QML never receives junk.
fn sanitize(theme: &mut Theme) {
    let fallback = Theme::catppuccin();
    let fix = |value: &mut String, fb: &str| {
        if !is_hex_color(value) {
            *value = fb.to_string();
        }
    };
    fix(&mut theme.accent, &fallback.accent);
    fix(&mut theme.selection, &fallback.selection);
    fix(&mut theme.muted, &fallback.muted);
    fix(&mut theme.background, &fallback.background);
    fix(&mut theme.dark_background, &fallback.dark_background);
    fix(&mut theme.darker_background, &fallback.darker_background);
    fix(&mut theme.lighter_background, &fallback.lighter_background);
    fix(&mut theme.foreground, &fallback.foreground);
    fix(&mut theme.dark_foreground, &fallback.dark_foreground);
    fix(&mut theme.light_foreground, &fallback.light_foreground);
    fix(&mut theme.bright_foreground, &fallback.bright_foreground);
}

fn is_hex_color(value: &str) -> bool {
    value.starts_with('#')
        && value.len() == 7
        && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OMARCHY_CATPPUCCIN: &str = r##"
mode = "dark"

accent = "#89b4fa"
selection = "#45475a"
muted = "#585b70"

background = "#1e1e2e"
dark_background = "#161622"
darker_background = "#101019"
lighter_background = "#313244"

foreground = "#cdd6f4"
dark_foreground = "#6c7086"
light_foreground = "#bac2de"
bright_foreground = "#cdd6f4"
"##;

    #[test]
    fn parses_omarchy_colors_toml() {
        let theme = Theme::parse(OMARCHY_CATPPUCCIN);
        assert_eq!(theme.accent, "#89b4fa");
        assert_eq!(theme.background, "#1e1e2e");
        assert_eq!(theme.foreground, "#cdd6f4");
    }

    #[test]
    fn partial_file_falls_back_per_field() {
        let theme = Theme::parse("accent = \"#ff0000\"");
        assert_eq!(theme.accent, "#ff0000");
        assert_eq!(theme.background, "#1e1e2e"); // default
    }

    #[test]
    fn invalid_values_are_replaced() {
        let theme = Theme::parse("accent = \"not-a-color\"");
        assert_eq!(theme.accent, "#89b4fa");
    }

    #[test]
    fn broken_toml_yields_default_theme() {
        let theme = Theme::parse("this is ][ not toml");
        assert_eq!(theme, Theme::catppuccin());
    }

    #[test]
    fn rejects_bad_hex_shapes() {
        assert!(!is_hex_color("#fff"));
        assert!(!is_hex_color("#gggggg"));
        assert!(!is_hex_color("89b4fa"));
        assert!(is_hex_color("#89b4fa"));
    }
}
