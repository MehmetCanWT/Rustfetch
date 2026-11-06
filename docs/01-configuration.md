# Configuration & Quick Start

RustFetch is configured via a standard TOML file located according to the XDG Base Directory specification. It also features automatic, non-destructive migration from existing Fastfetch configurations.

---

## 📍 Configuration File Locations

RustFetch resolves configuration files in the following strict order of precedence:

1. **CLI Flag Override:** Passed via `-c` or `--config <PATH>`:
   ```bash
   rustfetch --config ~/.config/rustfetch/work.toml
   ```
2. **XDG Standard Path:** `$XDG_CONFIG_HOME/rustfetch/config.toml` (defaults to `~/.config/rustfetch/config.toml`).
3. **Internal Default Fallback:** If no configuration file exists, RustFetch runs using optimized built-in defaults with zero mandatory setup.

### Generating a Default Configuration File

To scaffold a well-commented, complete `config.toml` in your configuration directory:
```bash
rustfetch --generate-config
```

`--dump-config` prints the **active, merged** configuration (preset + config file + CLI overrides) to stdout. To print the commented default template instead, use:
```bash
rustfetch --dump-config          # active configuration
rustfetch --print-default-config # commented default template
```

---

## 🔄 Automatic Fastfetch Migration

If you are migrating from **Fastfetch**, RustFetch can automatically import your module layout, ordering, custom icons, and visual preferences:

```bash
rustfetch --import-fastfetch
```

### How Fastfetch Migration Works:
- Inspects `~/.config/fastfetch/config.jsonc` in a strictly read-only manner.
- Strips JSONC comments (`//` and `/* */`) and trailing commas.
- Maps Fastfetch module keys (`OS`, `Kernel`, `Memory`, `Disk`, `Battery`, etc.) to RustFetch collectors.
- Imports `custom` rows through their `key`/`format` fields as static RustFetch `custom` modules.
- Imports display separator/padding and common logo settings (protocol, width, image/ASCII source). Per-module icons and colors are not guaranteed to carry over.
- Exports the translated configuration to `~/.config/rustfetch/config.toml`. Your original Fastfetch setup remains completely untouched.

---

## 🧱 Configuration Structure (`config.toml`)

A typical configuration file consists of a `[general]` table and an ordered array of `[[modules]]`:

```toml
# Top-level preset override (optional). The preset is the base layer and the
# values below are applied on top of it. Available: default, minimal, clean,
# dots, card, brackets, neofetch, retro, modern, compact.
# preset = "card"

[general]
separator = " : "          # Delimiter between label/icon and telemetry value
padding = 1                # Horizontal margin from terminal left edge (0..=64)
center = false             # Horizontally center output in terminal window
icons = true               # Enable or disable Nerd Font glyphs globally
icon_only = false          # Hide text labels and display icons only
key_type = "both"          # "both", "icon", "title", or "none" (none = value only)
box_padding = 1            # Inner horizontal padding of the border box (0..=16)

# Theme and Gradients
theme = "catppuccin"       # Built-in color theme (see --list-themes)
# gradient = ["#f38ba8", "#cba6f7", "#89b4fa"] # Linear text gradient

[general.border]
enabled = false             # Wrap telemetry lines in a border container
style = "rounded"          # rounded, light, heavy, double, dashed, block, ascii, brackets
color = "magenta"          # Color name ("blue", "cyan", "magenta") or hex ("#bd93f9")
title = "System"           # Optional title embedded in top border line
padding = 1                # Inner horizontal spacing between border and content
# Border glyphs can also be overridden as an 11-character string:
# chars = "╭─╮│╯─╰│├─┤"

[general.border_chars]      # ...or as an explicit table
top_left = "╭"
top = "─"
top_right = "╮"
right = "│"
bottom_right = "╯"
bottom = "─"
bottom_left = "╰"
left = "│"
divider_left = "├"
divider = "─"
divider_right = "┤"

[general.border_image]
# path = "~/.config/rustfetch/background.png"
position = "behind"        # "behind", "background", "left", "right", "top", "bottom"
# width = 30
# height = 18
# offset_x = 0
# offset_y = 0
# z_index = -1
# protocol = "auto"

[general.colors]
enabled = true             # Display terminal color palette preview
symbol = "●"               # Palette symbol: "●", "■", "󰮯", etc.
block = false              # Use colored background blocks instead of glyphs
position = "bottom"        # "bottom", "top", or "both"
rows = 1                   # 1 (8 colors) or 2 (16 colors including bright variants)

[general.logo]
enabled = true             # Display logo/graphic
# ASCII logo: "left", "right", "top", "bottom", "behind".
# Image logo: "left" (default), "top", "behind" — "right"/"bottom" only apply to
# ASCII logos; with an image those requests warn on stderr and fall back to "left".
position = "left"
distro = "auto"            # "auto" detects host OS, or override ("arch", "fedora", "ubuntu", etc.)
protocol = "auto"          # Image protocol: "auto", "kitty", "sixel", "iterm", "halfblock"
image_width_cols = 42      # Width of graphic column in terminal cells (1..=256)
auto_color = true          # Adapt accent colors to image dominant color

[[modules]]
name = "os"

[[modules]]
name = "kernel"

[[modules]]
name = "uptime"

[[modules]]
name = "break"             # Renders as internal horizontal divider inside boxes

[[modules]]
name = "cpu"

[[modules]]
name = "gpu"

[[modules]]
name = "memory"
bar = true
bar_width = 12             # 1..=256
bar_style = "blocks"       # "blocks", "line", "dots", "ascii"

[[modules]]
name = "custom"            # Static row: no command execution
label = "Note"
value = "custom text"
```

Unknown module names, theme names, shading modes, bar styles, and out-of-range
values do not abort the run: they are clamped/ignored and reported by
`rustfetch --doctor` (via the config warning list).

---

## 🛠️ CLI Flag Overrides

CLI flags always take precedence over configuration file settings:

```text
  -c, --config <PATH>          Path to a custom TOML config file (skips XDG discovery)
      --preset <NAME>          Built-in preset [default, minimal, clean, dots, card, brackets, neofetch, retro, modern, compact]
      --theme <NAME>           Color theme (13 built-ins; see --list-themes)
      --color <COLOR>          Override module/key color (name or #hex)
      --center                 Center the output horizontally
  -C, --clear                  Clear the screen and scrollback before drawing
      --separator <SEP>        Delimiter between label/icon and value
      --icon-only              Hide labels and show icons + values only
      --no-logo                Disable the logo completely
  -l, --logo <DISTRO>          Override the distro logo (see --list-logos)
      --logo-position <POS>    ASCII: left/right/top/bottom/behind; image: left/top/behind
  -i, --image <PATH>           Render an image logo (Kitty/Sixel/iTerm2/halfblock)
      --protocol <PROTOCOL>    Image protocol: auto, kitty, sixel, iterm, halfblock
      --width <COLS>           Image / 3D canvas width in terminal columns
      --height <ROWS>          Image / 3D canvas height in terminal rows
      --3d                     Real-time 3D animated logo
      --speed <FLOAT>          3D rotation speed (0.05..=10)
      --size <FLOAT>           3D model scale (0.1..=3.0)
      --depth <FLOAT>          3D relief depth (0.1..=5)
      --shading-mode <MODE>    3D shading: ascii, blocks, sextants, braille, dot, wireframe, solid
      --shading <CHARS>        Custom 3D shading character ramp
      --frames <N>             Stop after N frames
      --fps <FPS>              Target frame rate
      --hold                   Do not exit the animation on keypress
      --exit-on-key            Exit the animation on any keypress
      --infinite               Loop the animation indefinitely
      --ascii <PATH>           Use a custom static ASCII art file
      --ascii-anim <PATH>      Play a multi-frame ASCII animation sequence
      --live                   Continuous live monitor with CPU/RAM sparklines
      --border                 Enable the decorative box container
      --border-style <STYLE>   rounded, light, heavy, double, dashed, block, ascii, brackets
      --border-color <COLOR>   Border color (name or #hex)
      --border-title <TITLE>   Title embedded in the top border line
      --border-image <PATH>    Decorative image border/frame
      --border-image-width <N> Border image width in columns
      --border-image-position <POSITION>  behind/background, left, right, top, bottom, frame
      --json                   Output structured JSON telemetry
      --waybar                 Output Waybar/Polybar single-line JSON
      --format <FORMAT>        Output format (waybar, json)
      --benchmark              Profile microsecond latency per collector
      --dump-config            Print the active, merged configuration
      --print-default-config   Print the commented default template
      --generate-config        Write the default config.toml (keeps a .bak)
      --import-fastfetch       Import an existing Fastfetch configuration
      --install-hook           Install the shell startup/resize hook
      --remove-hook            Remove the shell startup/resize hook
      --init <SHELL>           Print the init snippet (bash, zsh, fish)
      --completions <SHELL>    Print a completion script (bash, zsh, fish, elvish, powershell)
      --list-modules           List all 39 modules
      --list-themes            List all 13 color themes
      --list-presets           List all 10 layout presets
      --list-logos             List all 38 built-in logos
      --clean-cache            Clear ~/.cache/rustfetch
      --doctor                 System, graphics, and config diagnostics
      --no-update-check        Skip the GitHub update check
      --self-update            Update in place from GitHub releases
      --setup, --tui           Interactive configuration wizard
```
