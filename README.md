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
- TOML-based themes (catppuccin included as the default), hot-reloaded when
  `~/.config/invoka/theme.toml` changes
- Usage-frequency ranking persisted across restarts (most used apps first)
- Icons resolved through the icon theme on Linux; extracted from shortcut
  targets (HICON → PNG, cached) on Windows
- Optional wlr layer shell support (Hyprland/sway) via the `layer-shell`
  feature, with automatic fallback on other compositors
- Windows: Start Menu scan (`.lnk`), `RegisterHotKey`-backed global hotkey,
  single-instance via named pipe

## Stack

| Layer | Technology |
|---|---|
| Backend | Rust + [cxx-qt](https://github.com/KDAB/cxx-qt) 0.9 (Rust ↔ Qt bridge) |
| UI | QML (Qt 6) |
| Fuzzy search | [nucleo](https://github.com/helix-editor/nucleo) |
| Linux apps | freedesktop-desktop-entry + freedesktop-icons |
| Windows apps | [parselnk](https://crates.io/crates/parselnk) (Start Menu `.lnk`) |
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

## Packaging

- **Arch (AUR-style)**: `packaging/PKGBUILD`
- **AppImage**: `./packaging/build-appimage.sh` (uses linuxdeploy + Qt plugin)

## License

MIT — see [LICENSE](LICENSE).

---

# invoka (português)

Um launcher minimalista estilo Raycast para **Linux e Windows**, escrito em
**Rust** (backend) + **QML** (UI), com estética baseada no catppuccin.

Um app daemon: janela flutuante frameless invocada por hotkey global (ou IPC),
busca fuzzy dos aplicativos instalados, Enter abre, Esc fecha. Nada mais.

## Recursos

- Busca fuzzy de apps (`.desktop` de `XDG_DATA_DIRS`, respeitando `NoDisplay`/`NotShowIn`)
- Ranking por pontuação + frequência de uso (persistido entre reinícios)
- Hotkey global nativa no X11 e Windows
- Wayland: daemon + IPC (`invoka toggle`) + keybind do DE/compositor
- Instância única via Unix socket / named pipe (que também serve o IPC: `toggle`, `quit`)
- Temas via TOML (catppuccin por padrão), com **hot-reload** ao editar
  `~/.config/invoka/theme.toml`
- Ícones via tema de ícones no Linux; extraídos dos atalhos (HICON → PNG, com
  cache) no Windows
- Suporte opcional a wlr layer shell (Hyprland/sway) via feature `layer-shell`,
  com fallback automático nos outros compositors
- Windows: scan do Start Menu (`.lnk`), hotkey global e instância única via
  named pipe

## Instalação

```sh
curl -fsSL https://raw.githubusercontent.com/danielmadu/invoka/master/install.sh | sh
```

Instala em `~/.local/bin` e inicia o daemon. Empacotamento: `packaging/PKGBUILD`
(Arch) e `packaging/build-appimage.sh` (AppImage).

## Uso

```sh
invoka            # inicia o daemon
invoka toggle     # alterna a janela (útil para keybinds no Wayland/GNOME)
invoka quit       # encerra o daemon
```

Dentro do launcher: `↵` abre o app selecionado, `esc` fecha a janela.

## Hotkey global

No X11 e Windows o daemon registra a hotkey sozinho (padrão `Ctrl+Alt+Space`,
configurável em `~/.config/invoka/config.toml`). No Wayland, registre um
keybind do DE/compositor que execute `invoka toggle` — instruções por
ambiente (GNOME, Hyprland, sway, KDE) em [docs/keybinds.md](docs/keybinds.md).

## Licença

MIT — veja [LICENSE](LICENSE).
