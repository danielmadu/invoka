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

## License

MIT — see [LICENSE](LICENSE).
