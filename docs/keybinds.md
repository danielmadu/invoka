# Atalhos de teclado por ambiente

Como invocar o launcher `invoka` em cada ambiente. O padrão de projeto é:

- **X11 e Windows** — hotkey global nativa registrada pelo próprio daemon
  (crate `global-hotkey`), configurável em `~/.config/invoka/config.toml`.
- **Wayland** — hotkeys globais são impossíveis client-side por design. O
  daemon fica esperando no socket IPC e o keybind é registrado no
  DE/compositor apontando para:

  ```sh
  invoka toggle
  ```

## X11 (i3, bspwm, XFCE, ...) e Windows

Nada a configurar: o daemon registra a hotkey sozinho. Para trocar:

```toml
# ~/.config/invoka/config.toml
[hotkey]
modifiers = ["ctrl", "alt"]   # alt, ctrl, shift, super
code = "space"                # space, a..z, 0..9, f1..f12, enter, esc, ...
```

> GNOME rouba `Alt+Space` (menu da janela); por isso o default é
> `Ctrl+Alt+Space`.

## GNOME (Wayland)

Custom keybinding com `invoka toggle`:

```sh
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/']"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ name 'Invoka'
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ command 'invoka toggle'
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ binding '<Ctrl><Alt>Space'
```

> **Se o atalho não funcionar imediatamente:** em algumas sessões o serviço
> dconf não grava writes do `gsettings` (o valor fica só em memória e some).
> Gravar via `dconf write` direto resolve:
>
> ```sh
> dconf write /org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/binding "'<Ctrl><Alt>Space'"
> dconf write /org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/command "'invoka toggle'"
> ```

Verificação: `dconf dump /org/gnome/settings-daemon/plugins/media-keys/`.

## Hyprland

```ini
# ~/.config/hypr/hyprland.conf
bind = CTRL ALT, space, exec, invoka toggle
```

No Hyprland vale compilar com layer-shell para o launcher virar uma layer
surface nativa (ver abaixo).

## sway / wlroots em geral

```ini
# ~/.config/sway/config
bindsym $mod+space exec invoka toggle
```

## KDE Plasma (Wayland ou X11)

Preferência > Atalhos > Atalhos personalizados (GUI), ou via CLI:

```sh
kwriteconfig5 --file khotkeysrc --...   # KDE 5
```

O caminho mais confiável é pela GUI: *System Settings → Shortcuts → Add
Command*, comando `invoka toggle`, atalho `Ctrl+Alt+Space`.

## Autostart (qualquer ambiente freedesktop)

Copie o desktop entry do daemon:

```sh
cp ~/.local/share/applications/invoka-daemon.desktop ~/.config/autostart/ 2>/dev/null \
  || cat > ~/.config/autostart/invoka-daemon.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Invoka daemon
Exec=invoka
X-GNOME-Autostart-enabled=true
EOF
```

## Layer shell (opcional, wlroots/Hyprland)

Build opt-in que transforma a janela em layer surface (`overlay`, centrada,
keyboard on-demand), eliminando animações de foco e o "pulo" de posição:

```sh
cargo build --release --features layer-shell
```

Requisitos de build:

| Distro | Pacote |
|---|---|
| Arch | `layer-shell-qt` |
| Debian/Ubuntu | `liblayershellqtinterface-dev` |
| Fedora | `layer-shell-qt-devel` |

Em tempo de build o script procura LayerShellQt via pkg-config; sem a lib, o
build cai no fallback normal (stub). Em tempo de execução, se o compositor
não suportar layer shell (ex.: GNOME), o launcher usa a janela flutuante
comum — o fallback é automático.
