//! Built-in CLI utilities: discovery (--list-*), diagnostics (--doctor),
//! cache management (--clean-cache), self-updater (--self-update),
//! and interactive configuration wizard (--setup / --tui).

use crate::layout;
use crate::logos;
use crate::text::{sanitize_terminal, write_stdout};
use rustfetch_core::cache::Cache;
use rustfetch_core::config::{Config, default_icon_for_module};
use rustfetch_core::modules::update::{
    CURRENT_VERSION, check_latest_release, codename_for_version,
};
use rustfetch_core::modules::{module_by_name, module_names};
use std::io::IsTerminal;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
#[cfg(test)]
use std::time::Duration;

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";

mod setup;
#[cfg(test)]
mod tests;
mod update;

pub use setup::run_setup_tui;
#[cfg(test)]
use setup::{save_config, theme_choices};
pub use update::run_self_update;
pub(crate) use update::self_update;
use update::update_status_line;
#[cfg(test)]
use update::{
    INSTALL_SCRIPT_URL, MAX_SCRIPT_BYTES, build_installer_command, curl_download_args, now_nanos,
    validate_installer_script,
};

/// ANSI styling that is only emitted when colours are enabled.
pub(super) struct Style {
    enabled: bool,
}

impl Style {
    pub(super) fn new() -> Self {
        Self {
            // Never emit ANSI sequences into a pipe or a file: colour support
            // is opt-in only for an interactive terminal.
            enabled: layout::colors_enabled() && std::io::stdout().is_terminal(),
        }
    }

    pub(super) fn bold(&self) -> &'static str {
        if self.enabled { layout::BOLD } else { "" }
    }

    pub(super) fn dim(&self) -> &'static str {
        if self.enabled { layout::DIM } else { "" }
    }

    pub(super) fn reset(&self) -> &'static str {
        if self.enabled { layout::RESET } else { "" }
    }

    pub(super) fn green(&self) -> &'static str {
        if self.enabled { GREEN } else { "" }
    }

    pub(super) fn red(&self) -> &'static str {
        if self.enabled { RED } else { "" }
    }

    pub(super) fn cyan(&self) -> &'static str {
        if self.enabled { CYAN } else { "" }
    }
}

pub const MODULE_DESCRIPTIONS: &[(&str, &str)] = &[
    ("os", "Operating system distribution and architecture"),
    ("os_age", "Operating system installation age (statx btime)"),
    ("host", "System hardware model / motherboard product name"),
    ("board", "Motherboard vendor and model"),
    ("kernel", "Linux kernel release version"),
    ("uptime", "System uptime formatted in days/hours/minutes"),
    ("packages", "Installed package counts across all managers"),
    ("shell", "Current user shell and version"),
    (
        "desktop",
        "Desktop environment, window manager, and display server",
    ),
    ("font", "System UI font family and size"),
    ("terminal", "Active terminal emulator"),
    ("cpu", "CPU model, core/thread count, and max frequency"),
    (
        "cpu_usage",
        "Real-time aggregate CPU utilization percentage",
    ),
    ("temp", "CPU / hardware thermal sensor temperature"),
    (
        "gpu",
        "Graphics processor (priority dGPU with iGPU fallback)",
    ),
    (
        "dgpu",
        "Dedicated discrete GPU only (e.g. NVIDIA / AMD dGPU)",
    ),
    (
        "igpu",
        "Integrated GPU only (e.g. Intel UHD / AMD Radeon iGPU)",
    ),
    ("gpu_driver", "Active kernel GPU driver and module version"),
    ("memory", "RAM usage, total capacity, and percentage"),
    ("swap", "Swap memory usage and total capacity"),
    (
        "disk",
        "Filesystem usage (supports custom mount path = \"/home\")",
    ),
    ("battery", "Battery charge percentage and status"),
    (
        "power_profile",
        "ACPI platform power profile and CPU scaling governor",
    ),
    ("brightness", "Display backlight brightness percentage"),
    (
        "display",
        "Connected monitor model, resolution, and refresh rate",
    ),
    ("sound", "Audio sink volume and mute state"),
    ("media", "Currently playing MPRIS media track"),
    (
        "bluetooth",
        "Connected Bluetooth devices and battery levels",
    ),
    ("wifi", "Connected Wi-Fi SSID, signal quality, and band"),
    (
        "net_io",
        "Active network interface cumulative RX / TX traffic",
    ),
    ("local_ip", "Primary LAN IPv4 address and interface"),
    ("locale", "System language and character encoding"),
    ("processes", "Load average and running/total process counts"),
    (
        "init",
        "PID 1 init system (systemd, openrc, runit, dinit, s6)",
    ),
    ("container", "Container or VM runtime environment"),
    (
        "update",
        "GitHub release update notifier with cat breed codename",
    ),
    ("custom", "Custom user-defined static label and value row"),
    (
        "break",
        "Horizontal divider rule (same rendering as divider)",
    ),
    ("divider", "Horizontal section divider rule"),
];

pub const PRESET_DESCRIPTIONS: &[(&str, &str)] = &[
    (
        "default",
        "Balanced full system overview with Nerd Font icons and color palette",
    ),
    (
        "minimal",
        "Ultra-clean text-only layout without logo, icons, or palette",
    ),
    ("clean", "Icon-only key column with zero separator noise"),
    (
        "dots",
        "Bullet-separated icon-only telemetry with dot color row",
    ),
    (
        "card",
        "Centered rounded border card with sectioned dividers",
    ),
    ("brackets", "Centered bracket-framed box layout"),
    (
        "neofetch",
        "Classic Neofetch-inspired layout with 2-row solid color blocks",
    ),
    (
        "retro",
        "Rounded frame with Pac-Man color symbols and progress bars",
    ),
    (
        "modern",
        "Centered layout with arrow separators and inline progress bars",
    ),
    (
        "compact",
        "Essential 7-module summary for small terminal windows",
    ),
];

fn module_description(name: &str) -> &'static str {
    MODULE_DESCRIPTIONS
        .iter()
        .find(|(module, _)| *module == name)
        .map(|(_, description)| *description)
        .unwrap_or("")
}

pub fn run_list_modules() {
    let style = Style::new();
    let names = module_names();
    let mut out = String::with_capacity(4096);
    out.push_str(&format!(
        "{}Available RustFetch Modules ({}):{}\n\n",
        style.bold(),
        names.len(),
        style.reset()
    ));
    for &name in names {
        let _icon = default_icon_for_module(name).unwrap_or(" ");
        let description = module_description(name);
        let preview = if matches!(name, "update" | "media" | "custom" | "break" | "divider") {
            String::new()
        } else {
            module_by_name(name)
                .and_then(|module| module.detect())
                .map(|info| {
                    format!(
                        " {}=> {}{}",
                        style.dim(),
                        sanitize_terminal(&info.value),
                        style.reset()
                    )
                })
                .unwrap_or_default()
        };
        out.push_str(&format!(
            "  {}●{}  {}{name:<15}{} {description}{preview}\n",
            style.cyan(),
            style.reset(),
            style.bold(),
            style.reset()
        ));
    }
    let _ = write_stdout(&out);
}

pub fn run_list_themes() {
    let style = Style::new();
    let names = rustfetch_render::theme_names();
    let mut out = String::with_capacity(4096);
    out.push_str(&format!(
        "{}Available Built-in Color Themes ({}):{}\n\n",
        style.bold(),
        names.len(),
        style.reset()
    ));
    for &name in names {
        let Some(theme) = rustfetch_render::get_theme(name) else {
            continue;
        };
        let (primary_r, primary_g, primary_b) = theme.primary;
        let (secondary_r, secondary_g, secondary_b) = theme.secondary;
        let (border_r, border_g, border_b) = theme.border;
        if style.enabled {
            let primary_swatch =
                format!("\x1b[48;2;{primary_r};{primary_g};{primary_b}m   \x1b[0m");
            let secondary_swatch =
                format!("\x1b[48;2;{secondary_r};{secondary_g};{secondary_b}m   \x1b[0m");
            let border_swatch = format!("\x1b[48;2;{border_r};{border_g};{border_b}m   \x1b[0m");
            out.push_str(&format!(
                "  {}{name:<22}{} {primary_swatch} Primary (#{primary_r:02x}{primary_g:02x}{primary_b:02x})   {secondary_swatch} Secondary (#{secondary_r:02x}{secondary_g:02x}{secondary_b:02x})   {border_swatch} Border (#{border_r:02x}{border_g:02x}{border_b:02x})\n",
                style.bold(),
                style.reset()
            ));
        } else {
            out.push_str(&format!(
                "  {name:<22} Primary (#{primary_r:02x}{primary_g:02x}{primary_b:02x})   Secondary (#{secondary_r:02x}{secondary_g:02x}{secondary_b:02x})   Border (#{border_r:02x}{border_g:02x}{border_b:02x})\n"
            ));
        }
    }
    out.push_str(&format!(
        "\n{}Usage: rustfetch --theme <THEME>  or  set theme = \"catppuccin\" in the config file{}\n",
        style.dim(),
        style.reset()
    ));
    let _ = write_stdout(&out);
}

pub fn run_list_presets() {
    let style = Style::new();
    let mut out = String::with_capacity(2048);
    out.push_str(&format!(
        "{}Available Layout Presets ({}):{}\n\n",
        style.bold(),
        PRESET_DESCRIPTIONS.len(),
        style.reset()
    ));
    for &(name, description) in PRESET_DESCRIPTIONS {
        out.push_str(&format!(
            "  {}●{}  {}{name:<12}{} {description}\n",
            style.green(),
            style.reset(),
            style.bold(),
            style.reset()
        ));
    }
    out.push_str(&format!(
        "\n{}Usage: rustfetch --preset <PRESET>{}\n",
        style.dim(),
        style.reset()
    ));
    let _ = write_stdout(&out);
}

pub fn run_list_logos() {
    let style = Style::new();
    let logos = logos::list_available_logos();
    let mut out = String::with_capacity(2048);
    out.push_str(&format!(
        "{}Available Built-in Logos ({}):{}\n\n",
        style.bold(),
        logos.len(),
        style.reset()
    ));
    for &(key, label) in logos {
        let sample = logos::get_logo(key);
        let swatch = if style.enabled { sample.color } else { "" };
        out.push_str(&format!(
            "  {swatch}██{}  {}{key:<16}{} {label}\n",
            style.reset(),
            style.bold(),
            style.reset()
        ));
    }
    out.push_str(&format!(
        "\n{}Usage: rustfetch -l <LOGO>  or  rustfetch --3d -l <LOGO>{}\n",
        style.dim(),
        style.reset()
    ));
    let _ = write_stdout(&out);
}

pub fn run_clean_cache() {
    let style = Style::new();
    let directory = Cache::cache_dir();
    let existed = directory.is_dir();
    let removed = Cache::clear_all();
    if !existed {
        let _ = write_stdout(&format!(
            "{}Cache directory {} does not exist; nothing to clear.{}\n",
            style.dim(),
            directory.display(),
            style.reset()
        ));
        return;
    }
    let _ = write_stdout(&format!(
        "{}✓{} Cleared {removed} cached entry/entries from {}\n",
        style.green(),
        style.reset(),
        directory.display()
    ));
}

fn sanitize_optional_env(name: &str) -> String {
    sanitize_terminal(&std::env::var(name).unwrap_or_else(|_| "(unset)".to_string()))
}

fn cache_status_line(directory: &Path) -> String {
    match std::fs::metadata(directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            format!("not created yet ({})", directory.display())
        }
        Ok(metadata) if !metadata.is_dir() => {
            format!("not a directory ({})", directory.display())
        }
        Ok(_) => match std::fs::read_dir(directory) {
            Ok(entries) => {
                let mut readable = 0_usize;
                let mut unreadable = 0_usize;
                for entry in entries {
                    match entry {
                        Ok(_) => readable += 1,
                        Err(_) => unreadable += 1,
                    }
                }
                if unreadable == 0 {
                    format!("{readable} active files ({})", directory.display())
                } else {
                    format!(
                        "{readable} entries, {unreadable} unreadable ({})",
                        directory.display()
                    )
                }
            }
            Err(error) => format!(
                "unreadable: {} ({})",
                sanitize_terminal(&error.to_string()),
                directory.display()
            ),
        },
        Err(error) => format!(
            "unreadable: {} ({})",
            sanitize_terminal(&error.to_string()),
            directory.display()
        ),
    }
}
fn config_status_line(style: &Style, config_path: &Path) -> String {
    if !config_path.is_file() {
        return format!("not created yet (will use {})", config_path.display());
    }
    match Config::load_from(config_path) {
        Ok(loaded) => {
            let warning_count = loaded.warnings().len();
            let suffix = if warning_count == 0 {
                String::new()
            } else {
                format!(", {warning_count} warning(s)")
            };
            format!(
                "{}valid{} ({}{suffix})",
                style.green(),
                style.reset(),
                config_path.display()
            )
        }
        Err(error) => format!(
            "{}error{} ({})",
            style.red(),
            style.reset(),
            sanitize_terminal(&error)
        ),
    }
}

pub fn run_doctor(config: &Config, config_path: &Path, check_updates: bool) {
    let style = Style::new();
    let version = CURRENT_VERSION;
    let codename = codename_for_version(version);
    let mut out = String::with_capacity(4096);
    out.push_str(&format!(
        "{}RustFetch System & Terminal Diagnostics{}\n",
        style.bold(),
        style.reset()
    ));
    out.push_str("────────────────────────────────────────────────────────\n");
    out.push_str(&format!(
        "  {}Version:{}         v{version} \"{codename}\"\n",
        style.bold(),
        style.reset()
    ));
    out.push_str(&format!(
        "  {}Architecture:{}    {}\n",
        style.bold(),
        style.reset(),
        std::env::consts::ARCH
    ));

    let (columns, rows) = layout::terminal_size();
    let term_env = sanitize_optional_env("TERM");
    let colorterm_env = sanitize_optional_env("COLORTERM");
    let kitty = rustfetch_render::supports_kitty();
    let sixel = rustfetch_render::supports_sixel();
    let iterm = rustfetch_render::supports_iterm();
    let (cell_width, cell_height) = rustfetch_render::cell_size_or_default();

    out.push_str(&format!(
        "  {}Terminal:{}        TERM={term_env}, COLORTERM={colorterm_env}\n",
        style.bold(),
        style.reset()
    ));
    out.push_str(&format!(
        "  {}Grid Size:{}       {columns} cols × {rows} rows (cell: {cell_width}×{cell_height} px)\n",
        style.bold(),
        style.reset()
    ));

    let yes = || format!("{}yes{}", style.green(), style.reset());
    let no = "no".to_string();
    let truecolor = if colorterm_env.contains("truecolor") || colorterm_env.contains("24bit") {
        format!("{}yes{}", style.green(), style.reset())
    } else {
        "auto".to_string()
    };
    out.push_str(&format!(
        "  {}Graphics:{}        Kitty={}  Sixel={}  iTerm2={}  TrueColor={truecolor}\n",
        style.bold(),
        style.reset(),
        if kitty { yes() } else { no.clone() },
        if sixel { yes() } else { no.clone() },
        if iterm { yes() } else { no.clone() }
    ));

    out.push_str(&format!(
        "  {}Config File:{}     {}\n",
        style.bold(),
        style.reset(),
        config_status_line(&style, config_path)
    ));

    let warnings = config.warnings();
    out.push_str(&format!(
        "  {}Warnings:{}        {}\n",
        style.bold(),
        style.reset(),
        if warnings.is_empty() {
            "none".to_string()
        } else {
            format!("{} warning(s)", warnings.len())
        }
    ));
    for warning in &warnings {
        out.push_str(&format!("    - {}\n", sanitize_terminal(warning)));
    }

    let cache_dir = Cache::cache_dir();
    out.push_str(&format!(
        "  {}Cache Dir:{}       {}\n",
        style.bold(),
        style.reset(),
        cache_status_line(&cache_dir)
    ));

    let latest = if check_updates {
        check_latest_release()
    } else {
        None
    };
    out.push_str(&format!(
        "  {}Update Status:{}   {}\n",
        style.bold(),
        style.reset(),
        update_status_line(check_updates, latest.as_ref(), version)
    ));
    out.push_str("────────────────────────────────────────────────────────\n");
    let _ = write_stdout(&out);
}
