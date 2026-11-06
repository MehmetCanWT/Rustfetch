<div align="center">

# 🦀 RustFetch (`rfetch`)

**Blazing-fast, memory-safe Linux system fetch & hardware telemetry tool written in pure Rust, with fork-free kernel telemetry.**

[![CI](https://github.com/MehmetCanWT/rustfetch/actions/workflows/ci.yml/badge.svg)](https://github.com/MehmetCanWT/rustfetch/actions/workflows/ci.yml)
[![Release](https://github.com/MehmetCanWT/rustfetch/actions/workflows/release.yml/badge.svg)](https://github.com/MehmetCanWT/rustfetch/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Linux](https://img.shields.io/badge/Platform-Linux-orange.svg)](https://github.com/MehmetCanWT/rustfetch)

</div>

---

## ✨ Why RustFetch?

**RustFetch** (`rustfetch` / `rfetch`) is built from the ground up in Rust to replace slow shell-script fetchers and fragile C configs with a single static-friendly binary that starts in **~4 milliseconds**:

- ⚡ **Fork-Free Kernel Telemetry**: Core telemetry reads hardware topology directly from `/proc`, `/sys/bus/pci/devices`, `/sys/class/drm` (pure-Rust EDID binary parser), `/sys/class/hwmon`, `/sys/class/power_supply`, and `statx`/`statvfs` syscalls. Collectors that have no kernel interface — package-manager fallbacks, the GitHub update check, and the optional sound/Wi-Fi/Bluetooth/media modules — may invoke external tools (`rpm`, `curl`, `wpctl`, `nmcli`, `bluetoothctl`, `playerctl`); those calls are timeout-bounded and skipped when the tool is missing.
- 🖼️ **True Terminal Graphics (`Kitty`, `Sixel`, `Halfblock`)**: Renders crisp PNG/JPG/WebP/GIF artwork with automatic dominant-color & 16-color palette extraction (a `.color` sidecar file is written next to the image), aspect-ratio fitting, and atomic **Kitty Synchronized Output (`?2026h`)** on window resize.
- 🧊 **Real-Time 3D Spinning ASCII Relief (`--3d`)**: Extrudes any built-in distro logo or custom ASCII file into a 3D mesh with Blinn-Phong lighting and sub-cell `ascii`, `blocks`, `sextants`, `braille`, `dot`, `wireframe`, or `solid` rasterization. Press **any key** or run any command and it **instantly falls back to the static 2D ASCII logo** of the same distro without eating your keystrokes.
- 📈 **Interactive Live Monitor (`--live`) & TUI Config Builder (`--setup`)**: Real-time CPU/RAM Unicode sparkline graphs (`▁▂▃▄▅▆▇█`) with hot-swappable themes, plus an interactive terminal UI to switch theme, logo, 3D display mode, border + border style, icons, centering, and update checking with automatic `.bak` config backups.
- 🛡️ **Non-Destructive Config & Shell Resize Sync (`--install-hook`)**: Never overwrites your existing `~/.config/rustfetch/config.toml`. Shell startup integration automatically redraws and re-centers `rustfetch` when you resize a fresh terminal window before running your first command.
- 🐾 **Self-Updating with Cat Breed Codenames (`--self-update`)**: Built-in cached GitHub release checker (6h TTL) with clickable OSC 8 terminal links and atomic in-place binary updates (`v0.1.0 "Turkish Van"`, `v0.1.1 "Turkish Angora"`, `v0.1.2 "Bengal"`, `v0.1.3 "Maine Coon"`, `v0.1.4 "Siamese"`, `v0.1.5 "Ragdoll"`).

---

## 📸 Screenshots & Showcase

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

## 🚀 Installation

### 1. Universal One-Liner (`install.sh`)
Automatically detects your Linux distro (Arch, Fedora, Bazzite / `rpm-ostree`, Debian/Ubuntu, NixOS, openSUSE, Alpine, Void, Homebrew), installs `rustfetch` and `rfetch`, man pages, shell completions (`bash`, `zsh`, `fish`), and optional shell startup hooks:

```bash
curl -fsSL https://raw.githubusercontent.com/MehmetCanWT/rustfetch/main/install.sh | bash
```

### 2. Prebuilt Distro Packages (GitHub Releases)
Download native packages directly from the [**Releases**](https://github.com/MehmetCanWT/rustfetch/releases/latest) page:

| Distro / Platform | Release Artifact | Install Command |
| :--- | :--- | :--- |
| **Arch / CachyOS / EndeavourOS** | `rustfetch-v<VER>-arch-x86_64.pkg.tar.zst` | `sudo pacman -U rustfetch-v<VER>-arch-x86_64.pkg.tar.zst` |
| **Fedora / Bazzite / Nobara / RHEL** | `rustfetch-<VER>-1.fedora.x86_64.rpm` | `sudo dnf install ./rustfetch-<VER>-1.fedora.x86_64.rpm` *(or `rpm-ostree install`)* |
| **Debian / Ubuntu / Pop!_OS / Mint** | `rustfetch_<VER>_debian_amd64.deb` | `sudo apt install ./rustfetch_<VER>_debian_amd64.deb` |
| **NixOS / Nix Flakes** | `rustfetch-v<VER>-nixos-x86_64.tar.gz` / repo `flake.nix` | `nix run github:MehmetCanWT/rustfetch` |
| **Generic Linux** (`x86_64` gnu/musl, `aarch64` gnu) | `rustfetch-v<VER>-linux-{gnu,musl}-x86_64.tar.gz`, `rustfetch-v<VER>-linux-gnu-aarch64.tar.gz` | Extract `rustfetch` & `rfetch` to `~/.local/bin/` |

> `<VER>` is the release version without the leading `v` (for example `0.1.5`). Every release also publishes `SHA256SUMS.txt` next to these artifacts.

### 3. Build from Source (`cargo`)
```bash
git clone https://github.com/MehmetCanWT/rustfetch.git
cd rustfetch
cargo build --workspace --release
install -Dm755 target/release/rustfetch ~/.local/bin/rustfetch
install -Dm755 target/release/rfetch ~/.local/bin/rfetch
```

---

## 🛠️ Quick Start & Everyday Commands

Both `rustfetch` and the shorthand alias `rfetch` accept the same flags:

```bash
# Standard run (uses ~/.config/rustfetch/config.toml)
rustfetch

# Interactive TUI Configuration Builder (theme, logo, 3D mode, border, icons, centering, update check)
rustfetch --setup

# System & Terminal Capability Diagnostics (graphics protocols, grid size, config validity/warnings, cache, update status)
rustfetch --doctor
rustfetch --doctor --no-update-check   # fully offline diagnostics

# Update RustFetch in-place to the latest GitHub Release
rustfetch --self-update

# Live Hardware Monitor with real-time CPU & RAM sparklines ([t] Theme, [b] Border, [q] Quit)
rustfetch --live

# 3D Spinning ASCII Relief Mode (instantly drops to 2D static ASCII on any keypress)
rustfetch --3d
rustfetch --3d -l cachyos --shading-mode braille

# Try presets, themes, and custom logos on the fly
rustfetch --preset card --theme catppuccin
rustfetch -l bazzite --border --border-style rounded
rustfetch -i ~/Pictures/logo.png --protocol kitty

# Discover all 39 modules, 38 logos, 13 themes, and 10 presets
rustfetch --list-modules
rustfetch --list-logos
rustfetch --list-themes
rustfetch --list-presets

# Print the active configuration, or the commented default template
rustfetch --dump-config
rustfetch --print-default-config

# Install or remove shell startup & window-resize hook (.bashrc / .zshrc / config.fish)
rustfetch --install-hook
rustfetch --remove-hook

# Profile microsecond latency of every active collector
rustfetch --benchmark
```

---

## 📊 Telemetry Modules (39 Built-in Collectors)

Run `rustfetch --list-modules` at any time to inspect all 39 canonical module names. Full schema details are in [**docs/08-modules-and-icons.md**](docs/08-modules-and-icons.md).

| Module | Label | Data Source & Highlights |
| :--- | :--- | :--- |
| `os` | **OS** | `/etc/os-release` with distro-specific Nerd Font v3 glyph (`󰣇`, `󰣛`, `󰕈`, `󱄅`, etc.) |
| `os_age` | **OS Age** | Root `/` filesystem birth timestamp via Linux `statx(STATX_BTIME)` (file-mtime fallback) |
| `host` | **Host** | `/sys/class/dmi/id/product_name` & `product_version` |
| `board` | **Board** | `/sys/class/dmi/id/board_vendor` & `board_name` |
| `kernel` | **Kernel** | `/proc/version`, falling back to `/proc/sys/kernel/osrelease` |
| `uptime` | **Uptime** | `/proc/uptime` formatted into days, hours, and minutes |
| `packages` | **Packages** | Filesystem counters for `pacman`, `dpkg`, `rpm` (sqlite), `flatpak`, `snap`, `nix`, `apk`, `xbps`, `emerge`; 2 h TTL cache and an `rpm -qa` fallback |
| `shell` | **Shell** | Active shell name from `$SHELL` with a parent-process fallback (the version is not detected) |
| `desktop` | **Desktop** | Desktop environment & Wayland/X11 window manager (`KDE`, `GNOME`, `Hyprland`, `Sway`, `Niri`, `MangoWM`, `COSMIC`) |
| `font` | **Font** | UI font from KDE `kdeglobals` or GTK 3/4 `settings.ini` |
| `display` | **Display** | `/sys/class/drm` EDID parser: header + checksum, four detailed timing descriptors, CTA-861 extension blocks (resolution + refresh rate) |
| `terminal` | **Terminal** | Parent-process tree & environment inspection (`kitty`, `ghostty`, `foot`, `wezterm`, `alacritty`) |
| `cpu` | **CPU** | `/proc/cpuinfo` model name and logical thread count |
| `cpu_usage` | **CPU Usage** | `/proc/stat` jiffies utilization percentage (shown as a progress bar in `--live`) |
| `temp` | **Temp** | `/sys/class/hwmon` & `/sys/class/thermal` sensor temperature (`°C`) |
| `gpu` | **GPU** | Graphics processor (priority dGPU with iGPU fallback, configurable via `type = "dgpu"` / `"igpu"` / `"all"`) |
| `dgpu` | **dGPU** | Dedicated discrete GPU only (absent when none is detected) |
| `igpu` | **iGPU** | Integrated GPU only (absent when none is detected) |
| `gpu_driver` | **GPU Driver** | Bound kernel module from `/sys/bus/pci/devices/*/driver` (`amdgpu`, `xe`, `i915`, `nvidia`, ...) |
| `memory` | **Memory** | `/proc/meminfo` used / total + optional percentage bar |
| `swap` | **Swap** | `/proc/meminfo` swap used / total |
| `disk` | **Disk** | `statvfs` with Ostree/Bazzite `/sysroot` awareness and multi-mount support (`path = "/home"`) |
| `battery` | **Battery** | `/sys/class/power_supply/BAT*` charge %, AC status, cycle count, and design-vs-full health % |
| `power_profile` | **Power** | `/sys/firmware/acpi/platform_profile`, CPU scaling governor & `energy_performance_preference` |
| `brightness` | **Brightness** | `/sys/class/backlight` percentage |
| `sound` | **Sound** | Sink volume & mute state via `wpctl` / PulseAudio (optional external tool) |
| `media` | **Media** | Active MPRIS track via `playerctl` (optional external tool) |
| `bluetooth` | **Bluetooth** | Connected Bluetooth peripherals via `bluetoothctl` (optional external tool) |
| `wifi` | **Wi-Fi** | Connected SSID, signal strength, and band via `iw`, `nmcli`, or `iwgetid` (optional external tools) |
| `net_io` | **Network** | RX/TX totals for the default-route interface from `/sys/class/net`; a throughput rate is shown when re-run within 10 s |
| `local_ip` | **Local IP** | Primary LAN IPv4 address via `getifaddrs` |
| `locale` | **Locale** | System `$LANG` / `$LC_ALL` encoding |
| `processes` | **Processes** | Total and running process counts from `/proc/loadavg` (PID-directory fallback) |
| `init` | **Init** | PID 1 init system (`systemd`, `openrc`, `runit`, `dinit`, `s6`) |
| `container` | **Container** | Detects Docker, Podman, Distrobox, Toolbox, LXC, systemd-nspawn, Flatpak |
| `update` | **Update** | Cached (6 h TTL) GitHub release notifier with cat-breed codename & OSC 8 hyperlink |
| `custom` | **Custom** | Static `label`/`value` pair from the config (no command execution) |
| `break` / `divider` | *Divider* | Internal section divider line (`├──┤`), optional `label` |

---

## ⚙️ Configuration (`~/.config/rustfetch/config.toml`)

RustFetch creates `~/.config/rustfetch/config.toml` only when none exists and **never overwrites your custom edits**. You can also migrate an existing Fastfetch setup with `rustfetch --import-fastfetch`.

Print the **active** configuration (merged, with your overrides) with `rustfetch --dump-config`; `rustfetch --print-default-config` prints the commented default template instead.

```toml
# Optional base layout preset; explicit values below override it.
# preset = "card"

[general]
separator = ":"
padding = 1
center = true
icons = true
icon_only = false
key_type = "both"            # "both", "icon", "title", "none" (none = value only)
box_padding = 1              # Inner horizontal padding of the border box
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
image_palette = true         # Extract 16-color palette from active image logo
position = "bottom"
rows = 1

[general.logo]
enabled = true
# ASCII logo: "left", "right", "top", "bottom", "behind".
# Image logo: "left" (default), "top", "behind" — asking for "right"/"bottom"
# with an image prints a warning and falls back to "left".
position = "left"
distro = "auto"              # "auto", "bazzite", "cachyos", "arch", "fedora", "nixos", "ubuntu", "gentoo", ...
image_dir = "~/.config/rustfetch/logos"
image_width_cols = 42
random_image = true
auto_color = true            # Extract dominant accent color from image
protocol = "auto"            # "auto", "kitty", "sixel", "iterm", "halfblock"

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
name = "os_age"
[[modules]]
name = "host"
[[modules]]
name = "kernel"
[[modules]]
name = "uptime"
[[modules]]
name = "packages"
[[modules]]
name = "desktop"
[[modules]]
name = "terminal"
[[modules]]
name = "cpu"
[[modules]]
name = "gpu"
[[modules]]
name = "gpu_driver"
[[modules]]
name = "memory"
bar = true
bar_style = "blocks"         # "blocks", "line", "dots", "ascii"
[[modules]]
name = "disk"
path = "/"
[[modules]]
name = "disk"
path = "/home"
[[modules]]
name = "battery"
[[modules]]
name = "power_profile"
[[modules]]
name = "net_io"
[[modules]]
name = "local_ip"
[[modules]]
name = "update"
```

---

## 📚 Detailed Guides (`docs/`)

| Guide | Topic |
| :--- | :--- |
| [**01 — Configuration & Quick Start**](docs/01-configuration.md) | `config.toml` structure, CLI overrides, Fastfetch import, and shell hooks |
| [**02 — Colors, Themes & Gradients**](docs/02-colors-themes-gradients.md) | 24-bit TrueColor, built-in themes, multi-stop gradients, and image color extraction |
| [**03 — Boxes, Borders & Image Frames**](docs/03-boxes-and-borders.md) | Box styles, custom border glyphs, titles, section dividers, and `border_image` overlays |
| [**04 — Custom ASCII Art**](docs/04-ascii-art.md) | Custom ASCII files, Neofetch `$1..$6` color tags, and small logo variants |
| [**05 — Images & Graphics Protocols**](docs/05-images-and-protocols.md) | Kitty, DEC Sixel, and Unicode Halfblock rendering, aspect-ratio fitting, and random galleries |
| [**06 — 3D Terminal Graphics Engine**](docs/06-3d-terminal-engine.md) | 3D Z-buffer rasterizer, Blinn-Phong lighting, shading modes, and instant 2D ASCII fallback |
| [**07 — ASCII Animations & GIF Player**](docs/07-ascii-animations-gifs.md) | Multi-frame ASCII animations (`--ascii-anim`) and animated Kitty GIFs |
| [**08 — Modules & Nerd Font v3 Icons**](docs/08-modules-and-icons.md) | All telemetry collectors, progress bars, custom formatting, and multi-disk mounts |
| [**09 — Layout Presets Showcase**](docs/09-presets-showcase.md) | Visual tour of `default`, `card`, `dots`, `clean`, `neofetch`, `brackets`, `retro`, `minimal`, `modern`, `compact` |

---

## 🧪 Development & Testing

```bash
# Run formatting check, Clippy lints, and the full unit-test suite
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace

# Run end-to-end CLI smoke test suite (isolated temporary XDG dirs)
make test-cli
```

---

## 📄 License

Licensed under the [MIT License](LICENSE).
