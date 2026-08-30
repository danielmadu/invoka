# invoka

[![Release](https://img.shields.io/github/v/release/danielmadu/invoka)](https://github.com/danielmadu/invoka/releases)
[![License](https://img.shields.io/github/license/danielmadu/invoka)](LICENSE)

A minimal Raycast-style launcher for **Linux and Windows**, written in **Rust** (backend) + **QML** (UI), with a catppuccin-based aesthetic.

A daemon app: a floating frameless window invoked by a global hotkey (or IPC), fuzzy search over installed applications, Enter to launch, Esc to dismiss. Nothing more.

## Features

- Fuzzy search over apps (`.desktop` entries from `XDG_DATA_DIRS`, honoring `NoDisplay`/`NotShowIn` filters)
- Score-ranked results, case-insensitive matching
- Native global hotkey on X11 and Windows
- Wayland: daemon + IPC (`invoka toggle`) + DE/compositor keybind
- Single instance via Unix socket / named pipe (which also serves the IPC: `toggle`, `quit`, `query`)
- TOML-based themes (catppuccin included as the default)
- Icons resolved through the icon theme on Linux
- Optional wlr layer shell support (Hyprland/sway) via the `layer-shell`
  feature, with automatic fallback on other compositors

## Stack

| Layer | Technology |
|---|---|
| Backend | Rust + [cxx-qt](https://github.com/KDAB/cxx-qt) 0.9 (Rust ↔ Qt bridge) |
| UI | QML (Qt 6) |
| Fuzzy search | [nucleo](https://github.com/helix-editor/nucleo) |
| Linux apps | freedesktop-desktop-entry + freedesktop-icons |
| Hotkey | global-hotkey |

## Requirements

- Rust (rustup)
- Qt 6 dev packages (`qt6-base-dev` and `libqt6svg6` on Debian/Ubuntu)

## Install

Install the latest release binary with curl:

```sh
curl -fsSL https://raw.githubusercontent.com/danielmadu/invoka/master/install.sh | sh
```

Installs to `~/.local/bin` (override with `INSTALL_DIR`; pin a version with
`VERSION=v0.1.0`) and starts the daemon (`--no-start` or
`INVOKA_NO_START=1` to skip). Linux x86_64/aarch64 only for now — Windows
ships with M3.

## Build

```sh
cargo build --release
```

## Usage

```sh
invoka            # start the daemon
invoka toggle     # toggle the window (useful for Wayland/GNOME keybinds)
invoka quit       # stop the daemon
```

Inside the launcher: `↵` opens the selected app, `esc` closes the window.

## Global hotkey

On X11 and Windows the daemon registers the hotkey itself (default
`Ctrl+Alt+Space`, configurable in `~/.config/invoka/config.toml`).
On Wayland, register a DE/compositor keybind that runs `invoka toggle` —
per-environment instructions (GNOME, Hyprland, sway, KDE) in
[docs/keybinds.md](docs/keybinds.md).

## Layer shell (optional)

For native layer-surface placement on wlroots compositors (Hyprland, sway):

```sh
cargo build --release --features layer-shell
```

Requires the LayerShellQt dev package; other compositors get the regular
floating window automatically. See [docs/keybinds.md](docs/keybinds.md).

## License

MIT — see [LICENSE](LICENSE).
