# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Theme hot-reload (T4.1): `notify`-based watcher on the config directory;
  edits to `theme.toml` are debounced, deduplicated and applied live to the
  window. The watcher registers before Qt init so early edits are not missed
- Usage-frequency ranking (T4.2): launches persist counters to
  `~/.local/state/invoka/usage.toml` (`%APPDATA%\invoka` on Windows); empty
  query orders most-used first, real queries get a capped frequency boost
- Packaging (T4.3): `packaging/PKGBUILD` (Arch), `packaging/build-appimage.sh`
  (linuxdeploy + Qt plugin), desktop entries, bilingual README (EN + pt-BR)
- Windows port (M3): Start Menu application scan (`.lnk` parsing via
  parselnk, ProgramData + %APPDATA%, uninstallers filtered), launch through
  `cmd /C start` detached
- Windows icon extraction (T3.3): HICON → RGBA → PNG via GDI with a cache at
  `%APPDATA%\invoka\icons`
- Windows single-instance/IPC (T3.2): named pipe `\\.\pipe\invoka-<user>`
  with the same `toggle`/`quit` line protocol
- Windows global hotkey: manager created on a dedicated Win32 message-pump
  thread, events forwarded to the Qt thread
- Windows release CI (T3.4): MSVC build on GitHub Actions, asset
  `invoka-<tag>-x86_64-windows.zip` attached to tag releases
- `layer-shell` cargo feature (T2.1): opt-in wlr-layer-shell support via
  LayerShellQt — overlay layer, centered, keyboard on-demand. Build falls
  back to the stub when LayerShellQt is absent; runtime falls back to the
  plain floating window on compositors without layer shell (GNOME)
- Per-environment keybind documentation (T2.2): GNOME gsettings/dconf,
  Hyprland, sway, KDE, autostart — `docs/keybinds.md`
- `install.sh`: curl one-liner installer for Linux release binaries, with
  daemon auto-start (`--no-start` / `INVOKA_NO_START=1` to skip)

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
