# Layout Presets Showcase

RustFetch includes 10 built-in layout presets inspired by popular Unix ricing styles. A preset changes the whole layout, border structure, spacing, and module selection with a single flag, while remaining fully customizable through `config.toml`.

The authoritative list is printed by `rustfetch --list-presets`.

---

## 🚀 Using Presets

To apply a preset temporarily on the command line:

```bash
# Minimalist dot-separated layout
rustfetch --preset dots

# Rounded card box with section dividers
rustfetch --preset card

# Classic Neofetch style with color blocks
rustfetch --preset neofetch

# Balanced corner bracket card
rustfetch --preset brackets

# High-speed clean layout without containers
rustfetch --preset clean

# Retro terminal style with progress meters
rustfetch --preset retro

# The default balanced view
rustfetch --preset default
```

To make a preset your default, add it as the top-level `preset` key in `~/.config/rustfetch/config.toml`:

```toml
# config.toml
preset = "card"
```

The preset is the **base layer**: explicit `[general]` and `[[modules]]` values in the
same file are applied on top of it (see [Overriding Preset Properties](#️-overriding-preset-properties)).
An unknown preset name is ignored with a warning surfaced by `rustfetch --doctor`.

---

## 🎨 Built-in Presets Catalog

### 1. `default` (Balanced Overview)
The built-in default configuration: Nerd Font icons, distro logo, and the standard module set.

### 2. `card` (Modern Rounded Container)
Encapsulates system telemetry inside an elegant rounded Unicode box (`╭───╮`, `│   │`, `╰───╯`), using section divider lines (`├───┤`) to organize hardware and desktop information.
- **Attributes:** `border = true`, `border_style = "rounded"`, `padding = 2`, `center = true`, color palette at the bottom, and two `break` dividers.
- **Modules:** `os`, `host`, `kernel`, `uptime`, `break`, `shell`, `terminal`, `desktop`, `break`, `cpu`, `gpu`, `memory`, `disk`.
- **Preview:**
  ```text
  ╭───────────────────────────────────────────────╮
  │ mehmetcan@fedora                              │
  │ ────────────────                              │
  │ 󰣛  OS      : Fedora Linux 41 (Workstation)    │
  │ 󰌢  Host    : ThinkPad T14                     │
  │ 󰒋  Kernel  : Linux 6.13.0                     │
  ├───────────────────────────────────────────────┤
  │ 󰞷  Shell   : bash                             │
  │ 󰆍  Terminal: kitty                            │
  ├───────────────────────────────────────────────┤
  │ 󰍛  CPU     : Intel i5-11400H                  │
  │ 󰘚  Memory  : 8.4 GiB / 31.2 GiB               │
  │ 󰋊  Disk    : 120 GiB / 512 GiB                │
  │                                               │
  │ ● ● ● ● ● ● ● ●                               │
  ╰───────────────────────────────────────────────╯
  ```

---

### 3. `dots` (Minimalist Dot Separators)
For users who favor clean typography over border containers. Uses a `•` separator and shows the bottom palette dots.
- **Attributes:** `icon_only = true`, `separator = "•"`, `padding = 1`, colors enabled with `symbol = "●"`.
- **Modules:** `os`, `kernel`, `uptime`, `shell`, `terminal`, `cpu`, `gpu`, `memory`, `disk`.
- **Preview:**
  ```text
  󰣇 • Arch Linux x86_64
  󰒋 • Linux 6.13.0
  󱑂 • 14 hours, 35 mins
  󰞷 • zsh
  󰍛 • AMD Ryzen 9 7950X
  󰘚 • 14.2 GiB / 64.0 GiB

  ● ● ● ● ● ● ● ●
  ```

---

### 4. `clean` (Ultra-Clean Icon-Only)
Zero visual noise: removes labels, containers, and palettes, presenting only the essential glyph and system value.
- **Attributes:** `icon_only = true`, `separator = ""`, `padding = 1`, `colors.enabled = false`.
- **Modules:** `os`, `kernel`, `desktop`, `shell`, `terminal`, `cpu`, `gpu`, `memory`, `uptime`.
- **Preview:**
  ```text
  󰣇  Arch Linux x86_64
  󰒋  Linux 6.13.0
  󰍛  AMD Ryzen 7 7840U
  󰘚  8.4 GiB / 31.2 GiB
  󱑂  5 hours, 20 mins
  ```

---

### 5. `neofetch` (Classic Terminal Aesthetic)
Emulates the iconic Neofetch format with colon separators and a two-row 16-color block palette.
- **Attributes:** `separator = ":"`, `padding = 1`, `colors.enabled = true`, `colors.block = true`, `colors.rows = 2`.
- **Preview:**
  ```text
  mehmetcan@archlinux
  -------------------
  OS: Arch Linux x86_64
  Host: B650 AORUS ELITE AX
  Kernel: 6.13.0-arch1-1
  Uptime: 2 days, 4 hours
  Packages: 1245 (pacman)
  Shell: zsh
  Terminal: kitty
  CPU: AMD Ryzen 7 7800X3D (16)
  GPU: NVIDIA GeForce RTX 4080 SUPER
  Memory: 11420MiB / 31892MiB

  ████ ████ ████ ████ ████ ████ ████ ████
  ████ ████ ████ ████ ████ ████ ████ ████
  ```

---

### 6. `brackets` (Corner Bracket Accents)
Frames telemetry between top and bottom corner brackets (`[` ... `]` with `─`/`│` rules), giving a modern HUD appearance.
- **Attributes:** `border = true`, `border_style = "brackets"`, `padding = 2`, `center = true`.
- **Preview:**
  ```text
  [                                               ]
    󰣇  OS      : Fedora Linux 41
    󰒋  Kernel  : Linux 6.13.0
    󰍛  CPU     : AMD Ryzen 5 7600X
    󰘚  Memory  : 6.1 GiB / 32.0 GiB
  [                                               ]
  ```

---

### 7. `retro` (Retrowave Container & Progress Bars)
Features rounded borders, embedded progress bars for memory, disk, and battery, and a Pac-Man palette glyph (`󰮯`).
- **Attributes:** `border = true`, `border_style = "rounded"`, `colors.symbol = "󰮯"`, and `bar = true` with `bar_width = 10` on `memory`, `disk`, and `battery`.

---

### 8. `minimal` (Server MOTD & Scripting)
A stripped-down, uncolored hardware summary that is ideal for headless servers or automated login banners.
- **Attributes:** `icons = false`, `colors.enabled = false`, `logo.enabled = false`, `separator = " :"`.
- **Modules:** `os`, `kernel`, `uptime`, `shell`, `cpu`, `memory`, `disk`.

---

### 9. `modern` (Progress Banners)
Displays horizontal arrow separators (`OS -> Fedora`) and embedded progress meters for hardware telemetry.
- **Attributes:** `center = true`, `separator = " ->"`, `colors.block = true`, and `bar = true` with `bar_width = 10` on `memory`, `swap`, `disk`, and `battery`.

---

### 10. `compact` (Small Panes)
Keeps only the seven essential rows, using the regular full-width labels (the preset does not abbreviate labels), with colors disabled.
- **Attributes:** `padding = 1`, `colors.enabled = false`.
- **Modules:** `os`, `host`, `kernel`, `uptime`, `shell`, `cpu`, `memory`.

---

## 🛠️ Overriding Preset Properties

You can use any preset as a foundation and selectively override specific properties in your `config.toml`:

```toml
# Base layout
preset = "card"

[general]
# Override card's border with a custom double line and title:
border_style = "double"
border_color = "cyan"
border_title = "Workstation"
theme = "dracula"
```
RustFetch loads the base preset first, then applies your explicit definitions on top.
`[[modules]]` arrays in the config replace the preset's module list entirely.
