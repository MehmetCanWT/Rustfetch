use clap::Parser;
use clap_complete::Shell;

fn cli_version_string() -> &'static str {
    static VERSION_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION_STR.get_or_init(|| {
        let ver = rustfetch_core::CURRENT_VERSION;
        let code = rustfetch_core::codename_for_version(ver);
        format!("{ver} \"{code}\"")
    })
}

/// Parses a 1..=512 terminal dimension for --width/--height style flags.
fn parse_dimension(value: &str) -> Result<usize, String> {
    let n: usize = value
        .trim()
        .parse()
        .map_err(|_| format!("'{value}' is not a whole number"))?;
    if !(1..=512).contains(&n) {
        return Err("must be between 1 and 512".to_string());
    }
    Ok(n)
}

/// Parses a signed border-image offset in the range -10000..=10000.
fn parse_offset(value: &str) -> Result<i32, String> {
    let n: i32 = value
        .trim()
        .parse()
        .map_err(|_| format!("'{value}' is not a whole number"))?;
    if !(-10000..=10000).contains(&n) {
        return Err("must be between -10000 and 10000".to_string());
    }
    Ok(n)
}

fn parse_bounded_f32(value: &str, min: f32, max: f32) -> Result<f32, String> {
    let n: f32 = value
        .trim()
        .parse()
        .map_err(|_| format!("'{value}' is not a number"))?;
    if !n.is_finite() {
        return Err("must be a finite number".to_string());
    }
    if n < min || n > max {
        return Err(format!("must be between {min} and {max}"));
    }
    Ok(n)
}

/// Parses the 3D rotation-speed multiplier (0.05..=10.0, finite).
fn parse_speed(value: &str) -> Result<f32, String> {
    parse_bounded_f32(value, 0.05, 10.0)
}

/// Parses the 3D model scale (0.1..=3.0, finite).
fn parse_size(value: &str) -> Result<f32, String> {
    parse_bounded_f32(value, 0.1, 3.0)
}

/// Parses the 3D extrusion depth (0.1..=5.0, finite).
fn parse_depth(value: &str) -> Result<f32, String> {
    parse_bounded_f32(value, 0.1, 5.0)
}

/// Parses the animation/3D frame rate (1..=120, finite).
fn parse_fps(value: &str) -> Result<f64, String> {
    let n: f64 = value
        .trim()
        .parse()
        .map_err(|_| format!("'{value}' is not a number"))?;
    if !n.is_finite() {
        return Err("must be a finite number".to_string());
    }
    if !(1.0..=120.0).contains(&n) {
        return Err("must be between 1 and 120".to_string());
    }
    Ok(n)
}

/// Canonical 3D shading modes accepted on the command line.
pub const SHADING_MODES: &[&str] = &[
    "ascii",
    "blocks",
    "sextants",
    "braille",
    "dot",
    "wireframe",
    "solid",
];

/// Validates and normalises a --shading-mode value; unknown values are rejected.
pub(crate) fn parse_shading_mode(value: &str) -> Result<String, String> {
    let normalised = value.trim().to_ascii_lowercase();
    if normalised.is_empty() {
        return Err("shading mode must not be empty".to_string());
    }
    if SHADING_MODES.contains(&normalised.as_str()) {
        Ok(normalised)
    } else {
        Err(format!(
            "unknown shading mode '{value}' (expected one of: {})",
            SHADING_MODES.join(", ")
        ))
    }
}

/// Validates a custom 3D shading character set: non-empty, no control characters.
pub(crate) fn parse_shading_chars(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("shading character set must not be empty".to_string());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("shading character set must not contain control characters".to_string());
    }
    if trimmed.chars().count() > 64 {
        return Err("shading character set must contain at most 64 characters".to_string());
    }
    Ok(trimmed.to_string())
}

/// Picks the effective custom shading ramp; empty or whitespace-only values
/// (from the CLI or the config file) are rejected so the renderer falls back
/// to its built-in ramp instead of indexing an empty character set.
pub(crate) fn resolve_shading_chars(
    cli_value: Option<&str>,
    config_value: Option<&str>,
) -> Option<String> {
    cli_value
        .map(str::trim)
        .filter(|chars| !chars.is_empty())
        .or_else(|| {
            config_value
                .map(str::trim)
                .filter(|chars| !chars.is_empty())
        })
        .map(str::to_string)
}

#[derive(Parser, Debug)]
#[command(
    name = "rustfetch",
    version = cli_version_string(),
    about = "Blazing-fast system information fetch tool written in Rust"
)]
pub(crate) struct Cli {
    #[arg(long, hide = true)]
    pub(crate) check_updates_bg: bool,

    #[arg(long, help = "Print the built-in default configuration template")]
    pub(crate) print_default_config: bool,

    #[arg(long, help = "Print the active (resolved) configuration as TOML")]
    pub(crate) dump_config: bool,

    #[arg(long, help = "Write a default configuration file and exit")]
    pub(crate) generate_config: bool,

    #[arg(long, help = "Import an existing Fastfetch configuration")]
    pub(crate) import_fastfetch: bool,

    #[arg(
        short = 'c',
        long,
        value_name = "PATH",
        help = "Path to a custom TOML configuration file"
    )]
    pub(crate) config: Option<String>,

    #[arg(
        short = 'l',
        long,
        value_name = "DISTRO",
        help = "Override the ASCII logo (e.g. arch, ubuntu, cat)"
    )]
    pub(crate) logo: Option<String>,

    #[arg(
        short = 'i',
        long,
        value_name = "PATH",
        help = "Render an image logo (kitty/sixel/iTerm/halfblock)"
    )]
    pub(crate) image: Option<String>,

    #[arg(long, value_name = "COLOR", help = "Override the theme/accent color")]
    pub(crate) color: Option<String>,

    #[arg(long, help = "Center the output horizontally")]
    pub(crate) center: bool,

    #[arg(long, help = "Disable the logo and show only information rows")]
    pub(crate) no_logo: bool,

    #[arg(
        short = 'C',
        long,
        help = "Clear terminal screen and scrollback buffer before displaying"
    )]
    pub(crate) clear: bool,

    #[arg(long, help = "Run the continuous live monitor (requires a TTY)")]
    pub(crate) live: bool,

    #[arg(
        long = "3d",
        help = "Run the real-time 3D animated logo (requires a TTY)"
    )]
    pub(crate) three_d: bool,

    #[arg(
        long,
        value_name = "FLOAT",
        value_parser = parse_speed,
        help = "3D rotation speed multiplier (0.05-10.0)"
    )]
    pub(crate) speed: Option<f32>,

    #[arg(long, help = "Rotate the 3D model around the X axis")]
    pub(crate) rotate_x: bool,

    #[arg(long, help = "Rotate the 3D model around the Y axis")]
    pub(crate) rotate_y: bool,

    #[arg(
        long,
        value_name = "FLOAT",
        value_parser = parse_size,
        help = "3D model scale (0.1-3.0)"
    )]
    pub(crate) size: Option<f32>,

    #[arg(
        long,
        value_name = "FLOAT",
        value_parser = parse_depth,
        help = "3D extrusion depth (0.1-5.0)"
    )]
    pub(crate) depth: Option<f32>,

    #[arg(
        long,
        value_name = "MODE",
        value_parser = parse_shading_mode,
        help = "3D shading mode: ascii, blocks, sextants, braille, dot, wireframe, solid"
    )]
    pub(crate) shading_mode: Option<String>,

    #[arg(
        long,
        value_name = "CHARS",
        value_parser = parse_shading_chars,
        help = "Custom 3D shading character ramp (non-empty)"
    )]
    pub(crate) shading: Option<String>,

    #[arg(
        long,
        value_name = "COLS",
        alias = "image-width-cols",
        value_parser = parse_dimension,
        help = "Logo image width in columns (1-512)"
    )]
    pub(crate) width: Option<usize>,

    #[arg(
        long,
        value_name = "ROWS",
        value_parser = parse_dimension,
        help = "Logo image height in rows (1-512)"
    )]
    pub(crate) height: Option<usize>,

    #[arg(long, value_name = "N", help = "Stop after N animation frames")]
    pub(crate) frames: Option<usize>,

    #[arg(long, help = "Loop animation frames indefinitely")]
    pub(crate) infinite: bool,

    #[arg(long, help = "Keep animating and ignore key presses until quit")]
    pub(crate) hold: bool,

    #[arg(long, help = "Exit the animation on the first key press")]
    pub(crate) exit_on_key: bool,

    #[arg(long, help = "Output structured JSON telemetry to stdout")]
    pub(crate) json: bool,

    #[arg(
        long,
        help = "Output system information as Waybar / Polybar single-line JSON"
    )]
    pub(crate) waybar: bool,

    #[arg(
        long,
        value_name = "FORMAT",
        help = "Output format shortcut (json or waybar)"
    )]
    pub(crate) format: Option<String>,

    #[arg(long, value_name = "PATH", help = "Use a custom static ASCII art file")]
    pub(crate) ascii: Option<String>,

    #[arg(
        long,
        alias = "anim",
        value_name = "PATH",
        help = "Play a multi-frame ASCII animation file or directory"
    )]
    pub(crate) ascii_anim: Option<String>,

    #[arg(
        long,
        value_name = "FLOAT",
        value_parser = parse_fps,
        help = "Animation frame rate in frames per second (1-120)"
    )]
    pub(crate) fps: Option<f64>,

    #[arg(
        long,
        value_name = "PRESET",
        help = "Apply a built-in layout preset (default, minimal, card, ...)"
    )]
    pub(crate) preset: Option<String>,

    #[arg(long, help = "Profile per-module detection latency")]
    pub(crate) benchmark: bool,

    #[arg(long, help = "Draw a decorative border box around the output")]
    pub(crate) border: bool,

    #[arg(
        long,
        value_name = "STYLE",
        help = "Border style (rounded, ascii, ...)"
    )]
    pub(crate) border_style: Option<String>,

    #[arg(long, value_name = "COLOR", help = "Border color (name or hex)")]
    pub(crate) border_color: Option<String>,

    #[arg(long, value_name = "TITLE", help = "Title embedded in the top border")]
    pub(crate) border_title: Option<String>,

    #[arg(long, help = "Show icons only, hiding module labels")]
    pub(crate) icon_only: bool,

    #[arg(long, value_name = "SEP", help = "Separator between label and value")]
    pub(crate) separator: Option<String>,

    #[arg(long, value_name = "THEME", help = "Apply a built-in color theme")]
    pub(crate) theme: Option<String>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Render an image as a decorative border/frame"
    )]
    pub(crate) border_image: Option<String>,

    #[arg(
        long,
        value_name = "COLS",
        value_parser = parse_dimension,
        help = "Border image width in columns (1-512)"
    )]
    pub(crate) border_image_width: Option<usize>,

    #[arg(
        long,
        value_name = "ROWS",
        value_parser = parse_dimension,
        help = "Border image height in rows (1-512)"
    )]
    pub(crate) border_image_height: Option<usize>,

    #[arg(
        long,
        value_name = "COLS",
        value_parser = parse_offset,
        allow_hyphen_values = true,
        help = "Border image horizontal offset (-10000 to 10000)"
    )]
    pub(crate) border_image_offset_x: Option<i32>,

    #[arg(
        long,
        value_name = "ROWS",
        value_parser = parse_offset,
        allow_hyphen_values = true,
        help = "Border image vertical offset (-10000 to 10000)"
    )]
    pub(crate) border_image_offset_y: Option<i32>,

    #[arg(
        long,
        value_name = "INDEX",
        help = "Border image stacking z-index (when text_on_top is false)"
    )]
    pub(crate) border_image_z_index: Option<i32>,

    #[arg(
        long,
        value_name = "POSITION",
        help = "Border image position (behind, left, right, top, bottom, frame)"
    )]
    pub(crate) border_image_position: Option<String>,

    #[arg(
        long,
        value_name = "PROTOCOL",
        help = "Border image graphics protocol (auto, kitty, halfblock)"
    )]
    pub(crate) border_image_protocol: Option<String>,

    #[arg(
        long,
        value_name = "BOOL",
        help = "Preserve border image aspect ratio (true/false)"
    )]
    pub(crate) border_image_preserve_aspect_ratio: Option<bool>,

    #[arg(
        long,
        value_name = "BOOL",
        help = "Draw text on top of the border image (true/false)"
    )]
    pub(crate) border_image_text_on_top: Option<bool>,

    #[arg(
        long,
        value_name = "POSITION",
        help = "Logo position (left, right, top, bottom, behind, none)"
    )]
    pub(crate) logo_position: Option<String>,

    #[arg(
        long,
        value_name = "PROTOCOL",
        help = "Logo graphics protocol (auto, kitty, sixel, iterm, braille, halfblock)"
    )]
    pub(crate) protocol: Option<String>,

    #[arg(
        long,
        value_enum,
        value_name = "SHELL",
        help = "Generate shell completion script and exit"
    )]
    pub(crate) completions: Option<Shell>,

    #[arg(
        long,
        value_name = "SHELL",
        help = "Print shell startup integration script and exit"
    )]
    pub(crate) init: Option<String>,

    #[arg(long, help = "Install the shell startup hook into your shell rc file")]
    pub(crate) install_hook: bool,

    #[arg(
        long,
        alias = "remove-hook",
        help = "Remove the RustFetch shell startup hook"
    )]
    pub(crate) uninstall_hook: bool,

    #[arg(
        long,
        help = "Disable checking GitHub repository for RustFetch updates"
    )]
    pub(crate) no_update_check: bool,

    #[arg(long, help = "List all available system information modules")]
    pub(crate) list_modules: bool,

    #[arg(long, help = "List all built-in color themes with previews")]
    pub(crate) list_themes: bool,

    #[arg(long, help = "List all layout presets")]
    pub(crate) list_presets: bool,

    #[arg(long, help = "List all built-in OS and cat breed mascot logos")]
    pub(crate) list_logos: bool,

    #[arg(long, help = "Clear cached module data and exit")]
    pub(crate) clean_cache: bool,

    #[arg(long, help = "Run system, terminal graphics, and config diagnostics")]
    pub(crate) doctor: bool,

    #[arg(
        long,
        visible_alias = "update",
        help = "Update RustFetch via the official installer while preserving config.toml"
    )]
    pub(crate) self_update: bool,

    #[arg(
        long,
        alias = "tui",
        alias = "configure",
        help = "Launch interactive configuration wizard with live preview"
    )]
    pub(crate) setup: bool,

    #[arg(long, hide = true)]
    pub(crate) keep_logo: bool,
}
