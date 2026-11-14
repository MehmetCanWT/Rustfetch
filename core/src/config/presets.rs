use super::{Config, modules_from};

impl Config {
    pub fn from_preset(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "minimal" => {
                let mut c = Config::default();
                c.general.icons = false;
                c.general.separator = " :".to_string();
                c.general.colors.enabled = false;
                c.general.logo.enabled = false;
                c.modules =
                    modules_from(&["os", "kernel", "uptime", "shell", "cpu", "memory", "disk"]);
                Some(c)
            }
            "clean" => {
                let mut c = Config::default();
                c.general.icon_only = true;
                c.general.separator = "".to_string();
                c.general.padding = 1;
                c.general.colors.enabled = false;
                c.modules = modules_from(&[
                    "os", "kernel", "desktop", "shell", "terminal", "cpu", "gpu", "memory",
                    "uptime",
                ]);
                Some(c)
            }
            "dots" => {
                let mut c = Config::default();
                c.general.icon_only = true;
                c.general.separator = "•".to_string();
                c.general.padding = 1;
                c.general.colors.enabled = true;
                c.general.colors.symbol = "●".to_string();
                c.modules = modules_from(&[
                    "os", "kernel", "uptime", "shell", "terminal", "cpu", "gpu", "memory", "disk",
                ]);
                Some(c)
            }
            "card" | "modern-card" => {
                let mut c = Config::default();
                c.general.border = true;
                c.general.border_style = Some("rounded".to_string());
                c.general.padding = 2;
                c.general.center = true;
                c.general.colors.enabled = true;
                c.general.colors.position = "bottom".to_string();
                c.modules = modules_from(&[
                    "os", "host", "kernel", "uptime", "break", "shell", "terminal", "desktop",
                    "break", "cpu", "gpu", "memory", "disk",
                ]);
                Some(c)
            }
            "brackets" => {
                let mut c = Config::default();
                c.general.border = true;
                c.general.border_style = Some("brackets".to_string());
                c.general.padding = 2;
                c.general.center = true;
                Some(c)
            }
            "neofetch" => {
                let mut c = Config::default();
                c.general.separator = ":".to_string();
                c.general.padding = 1;
                c.general.colors.enabled = true;
                c.general.colors.block = true;
                c.general.colors.rows = 2;
                c.general.colors.position = "bottom".to_string();
                Some(c)
            }
            "retro" => {
                let mut c = Config::default();
                c.general.border = true;
                c.general.border_style = Some("rounded".to_string());
                c.general.colors.symbol = "󰮯".to_string();
                for m in &mut c.modules {
                    if matches!(m.name.as_str(), "memory" | "disk" | "battery") {
                        m.bar = Some(true);
                        m.bar_width = Some(10);
                    }
                }
                Some(c)
            }
            "modern" => {
                let mut c = Config::default();
                c.general.center = true;
                c.general.separator = " ->".to_string();
                c.general.colors.block = true;
                for m in &mut c.modules {
                    if matches!(m.name.as_str(), "memory" | "swap" | "disk" | "battery") {
                        m.bar = Some(true);
                        m.bar_width = Some(10);
                    }
                }
                Some(c)
            }
            "compact" => {
                let mut c = Config::default();
                c.general.padding = 1;
                c.general.colors.enabled = false;
                c.modules =
                    modules_from(&["os", "host", "kernel", "uptime", "shell", "cpu", "memory"]);
                Some(c)
            }
            "default" => Some(Config::default()),
            _ => None,
        }
    }

    pub fn to_default_toml() -> String {
        r##"# ==============================================================================
# RustFetch Configuration (config.toml)
# ==============================================================================
# Blazing-fast, highly customizable system information tool written in Rust.
# Edit this file to customize layout, borders, overlays, colors, and modules.
# ==============================================================================

# ------------------------------------------------------------------------------
# 1. General Display & Layout
# ------------------------------------------------------------------------------
[general]
# Separator between module labels and values (e.g. ":", ": ", "  ", " ")
separator = ":"

# Outer horizontal margin (in spaces) from terminal edges
padding = 1

# Center output horizontally in terminal
center = false

# Enable/disable Nerd Font icons next to module labels
icons = true

# Display only icons and values (hide module label text)
icon_only = false

# Run in live interactive system monitor mode (continuous refresh)
live = false

# Check GitHub repository for updates and display an Update row at the bottom when available
check_updates = true

# Preset theme: "catppuccin", "catppuccin-latte", "catppuccin-macchiato", "catppuccin-frappe", "dracula", "tokyo-night", "gruvbox", "nord", "rose-pine", "cyberpunk", "monokai", "onedark", "solarized"
# theme = "catppuccin"

# Multi-stop color gradient for text and borders (list of hex codes)
# gradient = ["#89b4fa", "#b4befe", "#cba6f7"]

# ------------------------------------------------------------------------------
# 2. Border & Box Frame
# ------------------------------------------------------------------------------
[general.border]
# Enable or disable the outer border box
enabled = false

# Border frame style: "rounded", "single", "double", "thick", "dashed", "block", "ascii", "brackets"
# style = "rounded"

# Border color: color name ("cyan", "red", "green", "blue", "magenta", "yellow") or hex ("#89b4fa")
# color = "cyan"

# Optional title displayed in the top border frame
# title = "RustFetch"

# Padding (in spaces) between border frame and content
padding = 1

# Custom border characters (overrides style if set)
# [general.border.chars]
# top_left = "╭"
# top = "─"
# top_right = "╮"
# right = "│"
# bottom_right = "╯"
# bottom = "─"
# bottom_left = "╰"
# left = "│"
# divider_left = "├"
# divider = "─"
# divider_right = "┤"

# ------------------------------------------------------------------------------
# 3. Border Image & Background Overlays
# ------------------------------------------------------------------------------
[general.border_image]
# Position options: "behind", "background", "left", "right", "top", "bottom"
# Default is "behind" (render image as background backdrop)
position = "behind"

# Overlay an image behind or alongside the fetch layout
# Supported protocols: "auto", "kitty", "sixel", "iterm", "halfblock", "braille"
# path = "~/.config/rustfetch/background.png"
# width = 30
# height = 18
# offset_x = 0
# offset_y = 0
# z_index = -1
# protocol = "auto"

# Multiple layered border images (optional)
# [[general.border_images]]
# path = "~/.config/rustfetch/badge.png"
# position = "right"
# width = 20
# height = 10
# offset_x = 0
# offset_y = 0
# z_index = 1
# protocol = "auto"

# ------------------------------------------------------------------------------
# 4. Color Palette Bar
# ------------------------------------------------------------------------------
[general.colors]
# Display color test dots/blocks at bottom or top of output
enabled = true

# Character used for color blocks (e.g. "●", "■", "󰝤", "")
symbol = "●"

# Use solid colored rectangular blocks instead of symbols
block = false

# Extract color palette automatically from image logo when image mode is used
image_palette = true

# Position of the palette: "bottom", "top", "none"
position = "bottom"

# Number of rows (1 or 2 rows; 2 rows displays standard + bright colors)
rows = 1

# Custom color palette override (hex codes or ANSI color names)
# custom = ["#1e1e2e", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#94e2d5", "#cdd6f4"]

# ------------------------------------------------------------------------------
# 5. Logo & Visual Artwork
# ------------------------------------------------------------------------------
[general.logo]
# Enable or disable logo display
enabled = true

# Logo placement: "left" (default), "right", "top", "bottom", "behind", "hidden"
# position = "left"

# OS distro logo to display ("auto" detects current OS, or e.g. "arch", "ubuntu", "fedora", "nixos")
distro = "auto"

# Custom ASCII logo file path (overrides built-in distro art)
# ascii_path = "~/.config/rustfetch/ascii_logo.txt"

# Custom colors for multi-color ASCII logos (e.g. ["blue", "white"])
# colors = ["blue", "white"]

# Image logo file path (PNG, JPG, WebP)
# image_path = "~/.config/rustfetch/logo.png"

# Directory to randomly pick an image logo from on each launch
# image_dir = "~/.config/rustfetch/logos"

# Width of image logo in terminal columns
image_width_cols = 42

# Randomly select image from image_dir
random_image = false

# Automatically adapt primary theme colors to match dominant image colors
auto_color = true

# Terminal graphics protocol: "auto", "kitty", "sixel", "iterm", "halfblock", "braille"
protocol = "auto"

# ------------------------------------------------------------------------------
# 6. 3D Spinning ASCII Relief Logo
# ------------------------------------------------------------------------------
[general.logo.three_d]
# Enable real-time animated 3D ASCII relief logo
enabled = false

# Rotation speed multiplier (default: 1.0)
speed = 1.0

# Enable rotation around X axis (pitch / wobble)
rotate_x = true

# Enable rotation around Y axis (horizontal yaw spin)
rotate_y = true

# Model size scaling factor
size = 1.0

# 3D depth extrusion factor
depth = 1.0

# Canvas dimensions in characters
width = 42
height = 24

# Shading mode: "ascii", "blocks", "sextants", "braille", "dot", "wireframe", "solid"
shading_mode = "ascii"

# Custom shading characters from darkest to brightest (default: " .:-=+*#%@")
# shading = " .:-=+*#%@"

# Simulated directional light source: "top-left", "top-right", "front", "top"
light = "top-left"

# Stop animation after N frames (default: None = run indefinitely)
# frames = 120

# Loop animation indefinitely
# infinite = true

# Hold terminal open and keep animating until Ctrl+C / 'q' (disables exit-on-key)
# hold = false

# Automatically exit animation on any keypress (perfect for instant shell access)
exit_on_key = true

# Custom outer and inner colors for 3D model (hex or color name)
# outer_color = "#89b4fa"
# inner_color = "#cdd6f4"

# ------------------------------------------------------------------------------
# 7. Multi-frame ASCII Animation
# ------------------------------------------------------------------------------
[general.logo.animation]
# Enable multi-frame ASCII animation
enabled = false

# Path to multi-frame ASCII animation text file (delimited by '---' or form-feed)
# path = "~/.config/rustfetch/anim.txt"

# Frames per second (FPS)
fps = 15.0

# Exit animation on any keypress
exit_on_key = true

# Loop animation indefinitely
infinite = true

# Hold animation open until Ctrl+C / 'q'
# hold = false

# Stop animation after N frames
# frames = 60

# ==============================================================================
# 8. Information Modules
# ==============================================================================
# Configure which modules are shown, their order, custom labels, icons, and formats.
#
# Available modules:
#   os, os_age, host, board, kernel, uptime, packages, shell, desktop, font,
#   terminal, cpu, cpu_usage, temp, gpu, dgpu, igpu, gpu_driver, memory, swap, disk,
#   battery, power_profile, brightness, display, sound, bluetooth, wifi, net_io,
#   local_ip, locale, processes, media, init, container, update,
#   break (blank spacer line), divider (horizontal rule), custom
#
# Optional per-module fields:
#   label     - Custom display label text (or "" to hide label)
#   icon      - Custom Nerd Font icon
#   color     - Module label color (hex or ANSI name)
#   format    - Value template (e.g. "{value}", "{}")
#   bar       - true/false: display percentage progress bar
#   bar_width - Width of progress bar in characters (default: 10)
#   bar_style - Progress bar style: "blocks", "line", "dots", "ascii"
#   path      - Mount point for disk module (e.g. path = "/home")
#   type      - GPU filter for gpu module: "dgpu" (priority dGPU), "igpu" (integrated only), "all" (both)
#   value     - Static text value (for name = "custom")

[[modules]]
name = "os"

[[modules]]
name = "host"

[[modules]]
name = "board"

[[modules]]
name = "kernel"

[[modules]]
name = "uptime"

[[modules]]
name = "packages"

[[modules]]
name = "shell"

[[modules]]
name = "desktop"

[[modules]]
name = "font"

[[modules]]
name = "terminal"

[[modules]]
name = "cpu"

[[modules]]
name = "gpu"

[[modules]]
name = "memory"

[[modules]]
name = "swap"

[[modules]]
name = "disk"

[[modules]]
name = "battery"

[[modules]]
name = "local_ip"

[[modules]]
name = "locale"
"##
        .to_string()
    }
}
