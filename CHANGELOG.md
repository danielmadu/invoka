# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `layer-shell` cargo feature (T2.1): opt-in wlr-layer-shell support via
  LayerShellQt — overlay layer, centered, keyboard on-demand. Build falls
  back to the stub when LayerShellQt is absent; runtime falls back to the
  plain floating window on compositors without layer shell (GNOME)
- Per-environment keybind documentation (T2.2): GNOME gsettings/dconf,
  Hyprland, sway, KDE, autostart — `docs/keybinds.md`

## [0.1.0] - 2026-08-23

### Added

- Floating frameless window with QML UI (catppuccin aesthetic)
- Linux app scanning (`.desktop` entries from `XDG_DATA_DIRS`, honoring
  `NoDisplay`/`NotShowIn` filters)
- Fuzzy search with score-based ranking (nucleo), case-insensitive
- App launching with `Exec` parsing (field codes, quoting) and detached
  execution, including terminal app wrapper
- Daemon with single instance via Unix socket + IPC (`toggle`, `quit`,
  `debug-scan`)
- Configurable global hotkey (default `Ctrl+Alt+Space`) on X11; Wayland via
  DE/compositor keybind calling `invoka toggle`
- App icons resolved through the icon theme (freedesktop)
- TOML-based themes (catppuccin included as the default)
- Tray icon (QSystemTrayIcon/SNI)
- Hide on focus loss

[0.1.0]: https://github.com/danielmadu/invoka/releases/tag/v0.1.0
