# Telemetry Modules & Nerd Font v3 Icons

RustFetch gathers system telemetry across **39 canonical module names** (the authoritative list is printed by `rustfetch --list-modules` and matches `rustfetch_core::modules::module_names()`). Core kernel telemetry is read directly from `/proc`, `/sys`, and stat syscalls without shell sub-processes; collectors that have no kernel interface (package-manager fallbacks, the GitHub update check, and the optional sound/Wi-Fi/Bluetooth/media modules) may call bounded external tools and are skipped when those tools are missing.

All default icons use **Nerd Font v3 (Material Design Symbols)** glyphs. RustFetch also detects your Linux distribution and assigns an authentic, distribution-specific OS glyph.

---

## 🎨 Nerd Font v3 Icon Set

### Distro-Aware Operating System Glyphs

The `os` module queries `/etc/os-release` and automatically renders your distribution's Nerd Font glyph:

| Distribution | Glyph | Nerd Font Identifier |
|---|:---:|---|
| **Arch Linux / CachyOS** | `󰣇` | `nf-md-arch` |
| **Fedora Linux** | `󰣛` | `nf-md-fedora` |
| **Ubuntu** | `󰕈` | `nf-md-ubuntu` |
| **Debian** | `󰣚` | `nf-md-debian` |
| **NixOS** | `󱄅` | `nf-md-nix` |
| **Gentoo** | `󰣨` | `nf-md-gentoo` |
| **Manjaro** | `󱘊` | `nf-md-manjaro` |
| **Linux Mint** | `󰣭` | `nf-md-linux_mint` |
| **Alpine** | `󰣠` | `nf-md-alpine` |
| **openSUSE** | `󰣡` | `nf-md-opensuse` |
| **Kali Linux** | `󰣳` | `nf-md-kali_linux` |
| **Pop!_OS** | `󰣩` | `nf-md-pop_os` |
| **EndeavourOS** | `󰣫` | `nf-md-endeavouros` |
| **macOS** | `󰀵` | `nf-md-apple` |
| **Windows** | `󰍲` | `nf-md-microsoft_windows` |
| **Generic Linux / BSD** | `󰌽` | `nf-md-linux` |

---

### Default Module Glyphs

Every icon can be overridden per module with `icon = "..."` in `[[modules]]`.

| Module | Default Icon | Description |
|---|:---:|---|
| `os` | `󰌽` | Operating system |
| `os_age` | `󰃭` | OS installation age |
| `host` | `󰌢` | Host / laptop model |
| `board` | `󰘚` | Motherboard |
| `kernel` | `󰒋` | Kernel release |
| `uptime` | `󱑂` | System uptime |
| `packages` | `󰏖` | Package counts |
| `shell` | `󰞷` | User shell |
| `desktop` | `󰨇` | Desktop environment / WM |
| `font` | `󰛖` | UI font |
| `terminal` | `󰆍` | Terminal emulator |
| `cpu` | `󰍛` | Processor |
| `cpu_usage` | `󰓅` | CPU utilization |
| `temp` | `󰔏` | Temperature |
| `gpu` / `dgpu` / `igpu` | `󰢮` | Graphics card |
| `gpu_driver` | `󰢮` | Kernel GPU driver |
| `memory` | `󰘚` | RAM |
| `swap` | `󰓡` | Swap |
| `disk` | `󰋊` | Storage |
| `battery` | `󰂁` | Battery |
| `power_profile` | `󰓅` | Power profile |
| `brightness` | `󰃠` | Backlight |
| `display` | `󰍹` | Monitor |
| `sound` | `󰕾` | Audio sink |
| `media` | `󰝚` | Media player |
| `bluetooth` | `󰂯` | Bluetooth |
| `wifi` | `󰖩` | Wi-Fi |
| `net_io` | `󰛳` | Network traffic |
| `local_ip` | `󰩟` | Local IP |
| `locale` | `󰗊` | Locale |
| `processes` | `󰒋` | Process count |
| `init` | `` | Init system |
| `container` | `󰡨` | Container / VM |
| `update` | `󰚰` | Update notifier |
| `custom` | `󰅐` | Static custom row |
| `break` / `divider` | — | Section divider line |

---

## 📋 Telemetry Module Catalog

### 1. System & Kernel Modules
- **`os`**: Operating system name, edition, and architecture (`Arch Linux x86_64`).
- **`os_age`**: Installation age from the root filesystem birth time (`statx(STATX_BTIME)`, with an mtime fallback).
- **`host`**: Laptop/desktop model from DMI sysfs (`product_name`, `product_version`).
- **`board`**: Motherboard vendor and model from DMI sysfs.
- **`kernel`**: Running kernel release from `/proc/version` or `/proc/sys/kernel/osrelease`.
- **`uptime`**: Uptime from `/proc/uptime`, formatted into days, hours, and minutes.
- **`packages`**: Installed package counts (pacman, dpkg, rpm via sqlite, flatpak, snap, nix, apk, xbps, emerge), cached for 2 hours.
- **`locale`**: `$LANG` / `$LC_ALL` language and encoding.
- **`processes`**: Total and running process counts from `/proc/loadavg` (PID-directory fallback).
- **`init`**: PID 1 init system (`systemd`, `openrc`, `runit`, `dinit`, `s6`, ...).
- **`container`**: Docker / Podman / Distrobox / Toolbox / LXC / systemd-nspawn / Flatpak detection.
- **`update`**: Cached (6 h TTL) GitHub release notifier with cat-breed codename and OSC 8 link.
- **`custom`**: A static `label`/`value` row from the config (no shell commands).
- **`break` / `divider`**: Internal divider line (`├──┤`), optionally with a `label`.

### 2. Hardware & Sensor Modules
- **`cpu`**: Processor model name and logical thread count from `/proc/cpuinfo`.
- **`cpu_usage`**: Aggregate CPU utilization percentage from `/proc/stat`.
- **`temp`**: Hardware thermal sensors from `/sys/class/hwmon` and `/sys/class/thermal`.
- **`gpu`**: Graphics processors detected through sysfs/PCI, with dGPU priority and iGPU fallback (`type = "dgpu"` / `"igpu"` / `"all"`).
- **`dgpu`** / **`igpu`**: Discrete-only / integrated-only GPU views.
- **`gpu_driver`**: Bound kernel module and version from `/sys/bus/pci/devices/*/driver`.
- **`memory`**: RAM usage and total capacity from `/proc/meminfo`.
- **`swap`**: Swap usage and total capacity from `/proc/meminfo`.
- **`disk`**: Filesystem usage via `statvfs`, with Ostree/`/sysroot` awareness and multi-mount support (`path = "/home"`).
- **`battery`**: Charge level, AC status, cycle count, and design-vs-full health percentage from `/sys/class/power_supply`.
- **`power_profile`**: ACPI platform profile, CPU scaling governor, and `energy_performance_preference`.
- **`brightness`**: Backlight brightness percentage from `/sys/class/backlight`.
- **`display`**: Connected monitors, resolution, and refresh rate from `/sys/class/drm`. The EDID parser validates the 128-byte header and checksum, reads the four detailed timing descriptors, and follows CTA-861 extension blocks. When EDID is unavailable it falls back to the connector's first advertised mode.
- **`sound`**: Sink volume and mute state via `wpctl` / PulseAudio when available.
- **`media`**: Currently playing MPRIS track via `playerctl` when available.
- **`bluetooth`**: Connected Bluetooth peripherals via `bluetoothctl` when available.

### 3. Desktop & Environment Modules
- **`shell`**: Login shell name from `$SHELL` (parent-process fallback). The shell version is not detected.
- **`desktop`**: Desktop environment or window manager (`GNOME`, `KDE`, `Hyprland`, `Sway`, `Niri`, `MangoWM`, `COSMIC`, ...), including the display server.
- **`font`**: Active UI font from KDE `kdeglobals` or GTK 3/4 `settings.ini`.
- **`terminal`**: Terminal emulator parsed from the parent process tree and environment (`kitty`, `ghostty`, `foot`, `wezterm`, `alacritty`).

### 4. Network Modules
- **`wifi`**: Connected SSID, signal quality, and band via `iw`, `nmcli`, or `iwgetid` when available.
- **`net_io`**: Cumulative RX/TX totals for the default-route interface from `/sys/class/net`. When the module runs again within 10 seconds, it also reports the measured throughput rate based on the RX/TX delta.
- **`local_ip`**: Primary LAN IPv4 address via `getifaddrs`.

---

## ⚙️ Module Customization Options

Modules are configured under `[[modules]]` in `config.toml`:

```toml
[[modules]]
name = "os"
icon = "󰣇"
label = "OS"
color = "#1793d1"
format = "{icon} {value}"

[[modules]]
name = "memory"
label = "RAM"
bar = true
bar_width = 12             # 1..=256
bar_style = "blocks"       # "blocks", "line", "dots", "ascii"

[[modules]]
# Horizontal divider line (creates '├── Hardware ──┤' inside boxes)
name = "break"
label = "Hardware"

[[modules]]
name = "cpu"
label = "CPU"

[[modules]]
# Static custom row: no command execution
name = "custom"
label = "Note"
value = "hello"
icon = "󰅐"
color = "yellow"
```

Supported per-module keys: `name`, `label`, `icon`, `color`, `format`, `bar`, `bar_width`, `bar_style`, `path` (disk mounts), and the GPU selector (`type` / `gpu_type`). Unknown module names and out-of-range values are clamped/ignored and reported by `rustfetch --doctor`.

---

## 🎯 Key Display Formatting Options

RustFetch provides flexible key display modes:

### 1. `icon_only` Mode
Hides text labels and displays only the icon and value:
```bash
rustfetch --icon-only
```
```text
󰣇  Arch Linux x86_64
󰒋  Linux 6.13.0
󰍛  AMD Ryzen 7 7840U
󰘚  8.4 GiB / 31.2 GiB
```

### 2. `key_type` Option
- `key_type = "both"`: Displays icon and label (`󰍛 CPU: ...`)
- `key_type = "icon"`: Displays icon only (`󰍛 ...`)
- `key_type = "title"`: Displays label only (`CPU: ...`)
- `key_type = "none"`: Displays the value only.

### 3. Custom Separator (`separator`)
```toml
[general]
separator = " • "
```
```text
󰣇 • Arch Linux x86_64
󰒋 • Linux 6.13.0
󰍛 • AMD Ryzen 7 7840U
```
