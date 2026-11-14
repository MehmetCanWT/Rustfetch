<div align="center">

# RustFetch (`rfetch`)

**Linux system fetch and hardware telemetry tool written in pure Rust.**

[![CI](https://github.com/MehmetCanWT/rustfetch/actions/workflows/ci.yml/badge.svg)](https://github.com/MehmetCanWT/rustfetch/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Linux](https://img.shields.io/badge/Platform-Linux-orange.svg)](https://github.com/MehmetCanWT/rustfetch)

</div>

---

## Overview

**RustFetch** (`rustfetch` / `rfetch`) is a Linux system information and hardware telemetry tool built in Rust Edition 2024 as a modular workspace (`core`, `render`, `cli`).

- **Kernel Telemetry Without Shell Subprocesses**: Core hardware and OS metrics are read directly from `/proc`, `/sys/bus/pci/devices`, `/sys/class/drm` (pure-Rust EDID binary parser), `/sys/class/hwmon`, `/sys/class/power_supply`, and `statx`/`statvfs` syscalls. Optional collectors without a kernel interface (`rpm`, `curl`, `wpctl`, `nmcli`, `bluetoothctl`, `playerctl`) are timeout-bounded and skipped if unavailable.
- **Terminal Graphics and Resize Stability**: Supports Kitty (`U=1` Unicode placeholders on Kitty, Ghostty, and WezTerm so graphics reflow inside grid cells on window resize), DEC Sixel, iTerm2 inline images, Unicode Braille, and 24-bit TrueColor ANSI half-blocks. Side-by-side image heights clamp automatically to match the card height (`12..=24` rows), animated GIFs stream frame-by-frame without buffering cloned canvases, and dominant accents / 16-color palettes are cached to `.color` sidecars.
- **3D ASCII Engine (`--3d`)**: Extrudes built-in distro logos, custom ASCII art, or `.obj` models into a 3D mesh with Blinn-Phong lighting and `ascii`, `blocks`, `sextants`, `braille`, `dot`, `wireframe`, or `solid` shading. Pressing any key drops cleanly back to the static 2D logo without swallowing keystrokes.
- **Live Monitor (`--live`) and TUI Configurator (`--setup`)**: Real-time CPU/memory Unicode sparklines with interactive theme (`[t]`), border (`[b]`), and in-place self-update (`[u]`) controls, plus `--setup` to edit settings with automatic `.bak` backups.
- **Non-Destructive Updates and Shell Resize Sync**: Checks both GitHub Releases and upstream `main` `Cargo.toml` for newer semver versions (6h cache TTL, OSC 8 clickable links, cat-breed codenames) and updates in place via `--self-update` (`--update`) without touching `~/.config/rustfetch/config.toml`. Shell hooks (`--install-hook`) support Bash, Zsh, and Fish `SIGWINCH` redraws.

---

## Showcase

### Example Configurations

<p align="center">
  <img src="images/image_1.png?v=2" alt="Example Configuration 1" width="840" />
</p>

| Example Configuration 2 | Example Configuration 3 |
| :---: | :---: |
| <img src="images/image_2.png?v=2" alt="Example Configuration 2" width="430" /> | <img src="images/image_3.png?v=2" alt="Example Configuration 3" width="430" /> |

### Distro Logos

| Fedora Linux | Arch Linux | Gentoo Linux |
| :---: | :---: | :---: |
| <img src="images/Fedora.png" alt="Fedora Linux" width="290" /> | <img src="images/arch.png" alt="Arch Linux" width="290" /> | <img src="images/gentoo.png" alt="Gentoo Linux" width="290" /> |

---

## Installation

### One-Line Installer (`install.sh`)

Detects your Linux distribution (`arch`, `fedora`, `bazzite`/`rpm-ostree`, `debian`/`ubuntu`, `nixos`, `opensuse`, `alpine`, `void`), builds `rustfetch` and `rfetch`, and installs binaries, man page, shell completions (`bash`, `zsh`, `fish`), and optional shell hooks:

```bash
curl -fsSL https://raw.githubusercontent.com/MehmetCanWT/rustfetch/main/install.sh | bash
```

Optional environment variables:
- `NONINTERACTIVE=1`: skip prompts and use defaults.
- `RUSTFETCH_FORCE_CONFIG=0`: preserve any existing `~/.config/rustfetch/config.toml` (also default in non-interactive updates).
- `RUSTFETCH_INSTALL_DIR=<DIR>`: override target binary directory.
- `RUSTFETCH_VERSION=<TAG>`: build a specific tag/branch (falls back to `main` if the tag is absent).

### Build From Source (`cargo`)

Requires Rust **1.88+** (`edition = "2024"`, `resolver = "3"`):

```bash
git clone https://github.com/MehmetCanWT/rustfetch.git
cd rustfetch
cargo build --workspace --release --locked
install -Dm755 target/release/rustfetch ~/.local/bin/rustfetch
install -Dm755 target/release/rfetch ~/.local/bin/rfetch
```

---

## Quick Start

Both `rustfetch` and `rfetch` share identical flags:

```bash
# Run with ~/.config/rustfetch/config.toml (or defaults if none exists)
rustfetch

# Interactive TUI configuration builder
rustfetch --setup

# System, terminal graphics protocol, config validity, cache, and update diagnostics
rustfetch --doctor
rustfetch --doctor --no-update-check

# Update in place via the official installer (preserves config.toml)
rustfetch --self-update   # alias: --update

# Live monitor with CPU/RAM sparklines ([t] Theme, [b] Border, [u] Update, [q] Quit)
rustfetch --live

# 3D rotating logo (falls back to static 2D on keypress)
rustfetch --3d
rustfetch --3d -l cachyos --shading-mode braille

# Try presets, themes, logos, and image protocols from CLI
rustfetch --preset card --theme catppuccin
rustfetch -l bazzite --border --border-style rounded
rustfetch -i ~/Pictures/logo.png --protocol kitty

# Discovery lists and config inspection
rustfetch --list-modules
rustfetch --list-logos
rustfetch --list-themes
rustfetch --list-presets
rustfetch --dump-config
rustfetch --print-default-config

# Shell startup & resize hook (.bashrc / .zshrc / config.fish)
rustfetch --install-hook
rustfetch --uninstall-hook   # alias: --remove-hook

# Profile per-module latency in microseconds
rustfetch --benchmark
```

---

## Telemetry Modules (39 Collectors)

Run `rustfetch --list-modules` to list all canonical module names. Full options are covered in [docs/08-modules-and-icons.md](docs/08-modules-and-icons.md).

| Module | Label | Data Source |
| :--- | :--- | :--- |
| `os` | OS | `/etc/os-release` with distro-specific Nerd Font glyph |
| `os_age` | OS Age | Root `/` birth timestamp via `statx(STATX_BTIME)` (`mtime` fallback) |
| `host` | Host | `/sys/class/dmi/id/product_name` and `product_version` |
| `board` | Board | `/sys/class/dmi/id/board_vendor` and `board_name` |
| `kernel` | Kernel | `/proc/version` (`/proc/sys/kernel/osrelease` fallback) |
| `uptime` | Uptime | `/proc/uptime` |
| `packages` | Packages | Filesystem counters (`pacman`, `dpkg`, `rpm` sqlite, `flatpak`, `snap`, `nix`, `apk`, `xbps`, `emerge`; 2h cache, `rpm -qa` fallback) |
| `shell` | Shell | `$SHELL` with parent-process fallback |
| `desktop` | Desktop | DE / Wayland / X11 compositor (`KDE`, `GNOME`, `Hyprland`, `Sway`, `Niri`, `MangoWM`, `COSMIC`) |
| `font` | Font | KDE `kdeglobals` or GTK 3/4 `settings.ini` |
| `display` | Display | `/sys/class/drm` pure-Rust EDID parser (header, checksum, timing descriptors, CTA-861) |
| `terminal` | Terminal | Parent-process and env detection (`kitty`, `ghostty`, `foot`, `wezterm`, `alacritty`, etc.) |
| `cpu` | CPU | `/proc/cpuinfo` model name and logical threads |
| `cpu_usage` | CPU Usage | `/proc/stat` utilization percentage |
| `temp` | Temp | `/sys/class/hwmon` and `/sys/class/thermal` |
| `gpu` | GPU | Priority dGPU with iGPU fallback (`type = "dgpu"` / `"igpu"` / `"all"`) |
| `dgpu` | dGPU | Dedicated GPU only |
| `igpu` | iGPU | Integrated GPU only |
| `gpu_driver` | GPU Driver | Bound kernel module in `/sys/bus/pci/devices/*/driver` |
| `memory` | Memory | `/proc/meminfo` used / total (+ optional bar) |
| `swap` | Swap | `/proc/meminfo` swap used / total |
| `disk` | Disk | `statvfs` with Ostree `/sysroot` awareness and per-mount entries |
| `battery` | Battery | `/sys/class/power_supply/BAT*` charge, AC state, cycle count, health % |
| `power_profile` | Power | `/sys/firmware/acpi/platform_profile`, scaling governor, EPP |
| `brightness` | Brightness | `/sys/class/backlight` percentage |
| `sound` | Sound | Sink volume and mute state via `wpctl` / PulseAudio |
| `media` | Media | Active MPRIS track via `playerctl` |
| `bluetooth` | Bluetooth | Connected devices via `bluetoothctl` |
| `wifi` | Wi-Fi | SSID, signal strength, and band via `iw`, `nmcli`, or `iwgetid` |
| `net_io` | Network | RX/TX totals for the default-route interface (+ rate delta within 10s) |
| `local_ip` | Local IP | Primary LAN IPv4 address via `getifaddrs` |
| `locale` | Locale | `$LANG` / `$LC_ALL` |
| `processes` | Processes | Running and total processes from `/proc/loadavg` |
| `init` | Init | PID 1 init system (`systemd`, `openrc`, `runit`, `dinit`, `s6`) |
| `container` | Container | Docker, Podman, Distrobox, Toolbox, LXC, `systemd-nspawn`, Flatpak |
| `update` | Update | Cached (6h TTL) version notifier comparing GitHub Releases and `main` `Cargo.toml` |
| `custom` | Custom | Static `label` / `value` entry |
| `break` / `divider` | Divider | Internal card divider line with optional `label` |

---

## Configuration (`~/.config/rustfetch/config.toml`)

RustFetch only writes `~/.config/rustfetch/config.toml` when no config file exists, and preserves user configs on upgrades. Fastfetch configs can be converted with `rustfetch --import-fastfetch`.

```toml
# Optional base preset: explicit settings below override it.
# preset = "card"

[general]
separator = ":"
padding = 1
center = true
icons = true
icon_only = false
key_type = "both"            # "both", "icon", "title", "none"
box_padding = 1
check_updates = true
# theme = "catppuccin"
# gradient = ["#89b4fa", "#b4befe", "#cba6f7"]

[general.border]
enabled = false
style = "rounded"            # "rounded", "light", "heavy", "double", "dashed", "block", "ascii", "brackets"
color = "cyan"
padding = 1

[general.colors]
enabled = true
symbol = "●"
block = false
image_palette = true
position = "bottom"
rows = 1

[general.logo]
enabled = true
position = "left"            # ASCII: left/right/top/bottom/behind; Image: left/top/behind
distro = "auto"
image_dir = "~/.config/rustfetch/logos"
image_width_cols = 42
random_image = true
auto_color = true
protocol = "auto"            # "auto", "kitty", "sixel", "iterm", "braille", "halfblock"

[general.logo.three_d]
enabled = false
speed = 1.0
rotate_x = true
rotate_y = true
size = 1.0
depth = 1.0
width = 42
height = 24
shading_mode = "ascii"       # ascii, blocks, sextants, braille, dot, wireframe, solid
light = "top-left"

[[modules]]
name = "os"
[[modules]]
name = "kernel"
[[modules]]
name = "uptime"
[[modules]]
name = "packages"
[[modules]]
name = "cpu"
[[modules]]
name = "gpu"
[[modules]]
name = "memory"
bar = true
bar_style = "blocks"
[[modules]]
name = "disk"
path = "/"
[[modules]]
name = "update"
```

---

## Documentation (`docs/`)

| Guide | Topic |
| :--- | :--- |
| [01 — Configuration & Quick Start](docs/01-configuration.md) | `config.toml` schema, CLI flags, Fastfetch import, shell hooks |
| [02 — Colors, Themes & Gradients](docs/02-colors-themes-gradients.md) | 24-bit TrueColor, built-in themes, gradients, image color extraction |
| [03 — Boxes, Borders & Image Frames](docs/03-boxes-and-borders.md) | Box styles, custom border glyphs, titles, dividers, `border_image` overlays |
| [04 — Custom ASCII Art](docs/04-ascii-art.md) | Custom ASCII files, `$1..$6` color tags, compact logo variants |
| [05 — Images & Graphics Protocols](docs/05-images-and-protocols.md) | Kitty placeholders (`U=1`), Sixel, iTerm2, Braille, half-blocks |
| [06 — 3D Terminal Graphics Engine](docs/06-3d-terminal-engine.md) | 3D Z-buffer rasterizer, Blinn-Phong lighting, shading modes |
| [07 — ASCII Animations & GIF Player](docs/07-ascii-animations-gifs.md) | Multi-frame ASCII animations and streaming Kitty GIF playback |
| [08 — Modules & Nerd Font v3 Icons](docs/08-modules-and-icons.md) | Collectors, progress bars, templates, multi-disk mounts |
| [09 — Layout Presets Showcase](docs/09-presets-showcase.md) | Preset gallery (`default`, `card`, `dots`, `clean`, `neofetch`, `brackets`, `retro`, `minimal`, `modern`, `compact`) |

---

## Workspace Structure & Development

```text
rustfetch/
├── core/        # rustfetch-core: config/ (validation, presets, icons, fastfetch), cache, sha256, proc, modules/
├── render/      # rustfetch-render: image/ (Kitty U=1 placeholders, streaming GIF, halfblock), sixel, iterm, braille, three_d
├── cli/         # rustfetch CLI: args, interactive (--live/--3d), layout/ (ansi, card), logos/, tools/ (update, setup), shell_hook
├── docs/        # User guides and rustfetch(1) man page
├── scripts/     # test.sh and test-cli.sh end-to-end smoke checks
└── install.sh   # Official non-destructive one-line installer
```

```bash
# Format, check, lint, and test
cargo fmt --all -- --check
cargo check --workspace --all-targets --message-format=short
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

# Or run the full verification targets via Make
make check
make test-cli
```

---

## License

Licensed under the [MIT License](LICENSE).
