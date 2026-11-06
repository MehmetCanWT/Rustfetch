//! Built-in CLI utilities: discovery (--list-*), diagnostics (--doctor),
//! cache management (--clean-cache), self-updater (--self-update),
//! and interactive configuration wizard (--setup / --tui).

use crate::layout;
use crate::logos;
use crate::text::{sanitize_terminal, write_stdout};
use rustfetch_core::cache::Cache;
use rustfetch_core::config::{default_icon_for_module, Config};
use rustfetch_core::modules::update::{
    check_latest_release, codename_for_version, is_newer_version, CURRENT_VERSION,
};
use rustfetch_core::modules::{module_by_name, module_names};
use rustfetch_core::proc::{run_with_timeout, NETWORK_TIMEOUT};
use rustfetch_core::sha256::sha256_file;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";

/// Maximum accepted release archive size (64 MiB).
const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;

/// Manual update instructions shown whenever a self-update fails.
const RELEASES_URL: &str = "https://github.com/MehmetCanWT/rustfetch/releases/latest";

/// ANSI styling that is only emitted when colours are enabled.
struct Style {
    enabled: bool,
}

impl Style {
    fn new() -> Self {
        Self {
            // Never emit ANSI sequences into a pipe or a file: colour support
            // is opt-in only for an interactive terminal.
            enabled: layout::colors_enabled() && std::io::stdout().is_terminal(),
        }
    }

    fn bold(&self) -> &'static str {
        if self.enabled {
            layout::BOLD
        } else {
            ""
        }
    }

    fn dim(&self) -> &'static str {
        if self.enabled {
            layout::DIM
        } else {
            ""
        }
    }

    fn reset(&self) -> &'static str {
        if self.enabled {
            layout::RESET
        } else {
            ""
        }
    }

    fn green(&self) -> &'static str {
        if self.enabled {
            GREEN
        } else {
            ""
        }
    }

    fn red(&self) -> &'static str {
        if self.enabled {
            RED
        } else {
            ""
        }
    }

    fn cyan(&self) -> &'static str {
        if self.enabled {
            CYAN
        } else {
            ""
        }
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

fn update_status_line(
    check_updates: bool,
    latest: Option<&(String, String)>,
    current: &str,
) -> String {
    if !check_updates {
        return "skipped (--no-update-check)".to_string();
    }
    match latest {
        Some((latest_version, latest_code)) => {
            let latest_code = sanitize_terminal(latest_code);
            if is_newer_version(current, latest_version) {
                format!(
                    "Update available: v{latest_version} \"{latest_code}\" (run rustfetch --self-update)"
                )
            } else {
                let current_code = codename_for_version(current);
                format!("Up to date (v{current} \"{current_code}\")")
            }
        }
        None => "Offline / GitHub unreachable".to_string(),
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

struct TempDirGuard {
    path: PathBuf,
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub fn build_release_tarball_url(version: &str, arch: &str, is_musl: bool) -> String {
    let version = version.trim().trim_start_matches(['v', 'V']);
    let libc_tag = if is_musl && arch == "x86_64" {
        "linux-musl"
    } else {
        "linux-gnu"
    };
    format!(
        "https://github.com/MehmetCanWT/rustfetch/releases/download/v{version}/rustfetch-v{version}-{libc_tag}-{arch}.tar.gz"
    )
}

pub fn build_release_sums_url(version: &str) -> String {
    let version = version.trim().trim_start_matches(['v', 'V']);
    format!("https://github.com/MehmetCanWT/rustfetch/releases/download/v{version}/SHA256SUMS.txt")
}

pub fn release_archive_filename(version: &str, arch: &str, is_musl: bool) -> String {
    let version = version.trim().trim_start_matches(['v', 'V']);
    let libc_tag = if is_musl && arch == "x86_64" {
        "linux-musl"
    } else {
        "linux-gnu"
    };
    format!("rustfetch-v{version}-{libc_tag}-{arch}.tar.gz")
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Finds the SHA-256 entry for a release asset in a SHA256SUMS file.
///
/// Malformed lines are skipped instead of aborting the whole search: an entry
/// that appears after one bad line must still be found (a missing entry makes
/// the updater fail closed).
pub fn find_sha256_in_sums(sums_content: &str, target_filename: &str) -> Option<String> {
    for line in sums_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let (Some(hash), Some(file_entry)) = (parts.next(), parts.next()) else {
            continue;
        };
        let clean_entry = file_entry.trim_start_matches('*').trim_start_matches("./");
        if clean_entry == target_filename && is_sha256_hex(hash) {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn create_update_temp_dir() -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join(format!(
        "rustfetch-update-{}-{}",
        std::process::id(),
        now_nanos()
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        builder
            .create(&directory)
            .map_err(|error| format!("Failed to create temporary update directory: {error}"))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("Failed to create temporary update directory: {error}"))?;
    }
    Ok(directory)
}

fn curl_binary() -> &'static str {
    if Path::new("/usr/bin/curl").is_file() {
        "/usr/bin/curl"
    } else {
        "curl"
    }
}

fn curl_download_args(url: &str, destination: &Path, max_time: Duration) -> Vec<String> {
    let max_time = max_time.max(Duration::from_secs(1));
    let connect_timeout = max_time.min(Duration::from_secs(10));
    vec![
        "--fail".to_string(),
        "--silent".to_string(),
        "--show-error".to_string(),
        "--location".to_string(),
        "--proto".to_string(),
        "=https".to_string(),
        "--connect-timeout".to_string(),
        connect_timeout.as_secs().to_string(),
        "--max-time".to_string(),
        max_time.as_secs().to_string(),
        "--max-filesize".to_string(),
        MAX_ARCHIVE_BYTES.to_string(),
        "--output".to_string(),
        destination.to_string_lossy().into_owned(),
        "--url".to_string(),
        url.to_string(),
    ]
}

fn download(url: &str, destination: &Path, timeout: Duration) -> Result<(), String> {
    let args_owned = curl_download_args(url, destination, timeout);
    let args: Vec<&str> = args_owned.iter().map(String::as_str).collect();
    let process_timeout = timeout + Duration::from_secs(2);
    let output = run_with_timeout(curl_binary(), &args, process_timeout)
        .ok_or_else(|| format!("Could not run {}. Is curl installed?", curl_binary()))?;
    if output.timed_out {
        return Err(format!("Timed out downloading {url}"));
    }
    if !output.status.success() {
        return Err(format!(
            "curl failed for {url}: {}",
            sanitize_terminal(&output.stderr_string()).trim()
        ));
    }
    Ok(())
}

fn archive_download_timeout() -> Duration {
    NETWORK_TIMEOUT.saturating_mul(30)
}

fn parse_version_output(output: &str) -> Option<String> {
    for token in sanitize_terminal(output).split_whitespace() {
        let token = token.trim_matches(|character: char| {
            matches!(character, '"' | '\'' | ',' | '(' | ')' | ';')
        });
        let candidate = token.trim_start_matches(['v', 'V']);
        let (core, _) = match candidate.find(['-', '+']) {
            Some(idx) => (&candidate[..idx], Some(&candidate[idx..])),
            None => (candidate, None),
        };
        let mut parts = core.split('.');
        let (Some(major), Some(minor), Some(patch)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if !major.chars().all(|character| character.is_ascii_digit()) || major.is_empty() {
            continue;
        }
        if !minor.chars().all(|character| character.is_ascii_digit()) || minor.is_empty() {
            continue;
        }
        if !patch.chars().all(|character| character.is_ascii_digit()) || patch.is_empty() {
            continue;
        }
        if parts.next().is_some() {
            continue;
        }
        return Some(candidate.to_string());
    }
    None
}

fn verify_binary_version(path: &Path, expected: &str) -> Result<(), String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| format!("{}: path is not valid UTF-8", path.display()))?;
    let output = run_with_timeout(path_str, &["--version"], NETWORK_TIMEOUT)
        .ok_or_else(|| format!("Could not execute {} to verify its version", path.display()))?;
    if output.timed_out {
        return Err(format!(
            "{} --version timed out after {} ms",
            path.display(),
            NETWORK_TIMEOUT.as_millis()
        ));
    }
    if !output.status.success() {
        return Err(format!(
            "{} --version exited with status {}",
            path.display(),
            output.status
        ));
    }
    let reported = parse_version_output(&output.stdout_string()).ok_or_else(|| {
        format!(
            "Could not parse a version from {} --version",
            path.display()
        )
    })?;
    let expected = expected.trim().trim_start_matches(['v', 'V']);
    if reported != expected {
        return Err(format!(
            "Expected version {expected}, but {} reports {reported}",
            path.display()
        ));
    }
    Ok(())
}

fn install_binary_atomic(source: &Path, destination: &Path) -> std::io::Result<()> {
    let temporary = destination.with_extension("new");
    std::fs::copy(source, &temporary)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o755))?;
    }
    if let Err(error) = std::fs::rename(&temporary, destination) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn find_file_named(directory: &Path, filename: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(directory).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|name| name.to_str()) == Some(filename) {
            return Some(path);
        } else if path.is_dir() {
            if let Some(found) = find_file_named(&path, filename) {
                return Some(found);
            }
        }
    }
    None
}

fn update_target_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            home.join(".local/bin")
        })
}

fn self_update() -> Result<(), String> {
    let current = CURRENT_VERSION;
    let current_codename = codename_for_version(current);
    println!(
        "==> Checking GitHub for RustFetch updates (current: v{current} \"{current_codename}\")..."
    );

    let (latest, latest_codename) = check_latest_release().ok_or_else(|| {
        format!("Could not reach GitHub to check for updates (see {RELEASES_URL})")
    })?;

    if !is_newer_version(current, &latest) {
        println!("✓ RustFetch is already up to date (v{current} \"{current_codename}\").");
        return Ok(());
    }

    println!(
        "==> Updating RustFetch: v{current} \"{current_codename}\" -> v{latest} \"{latest_codename}\""
    );
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => {
            return Err(format!(
                "Unsupported prebuilt architecture '{other}'; download a release manually from {RELEASES_URL}"
            ));
        }
    };
    let is_musl = Path::new("/etc/alpine-release").exists();
    let archive_url = build_release_tarball_url(&latest, arch, is_musl);
    let archive_filename = release_archive_filename(&latest, arch, is_musl);
    let sums_url = build_release_sums_url(&latest);

    let temp_dir = create_update_temp_dir()?;
    let _guard = TempDirGuard {
        path: temp_dir.clone(),
    };

    // 1. Checksum metadata first: without it we refuse to install anything.
    println!("==> Fetching {sums_url}...");
    let sums_path = temp_dir.join("SHA256SUMS.txt");
    download(&sums_url, &sums_path, NETWORK_TIMEOUT)
        .map_err(|error| format!("{error}. Refusing to install without release checksums."))?;
    let sums_text = std::fs::read_to_string(&sums_path)
        .map_err(|error| format!("Failed to read SHA256SUMS.txt: {error}"))?;
    let expected_hash = find_sha256_in_sums(&sums_text, &archive_filename).ok_or_else(|| {
        format!(
            "{archive_filename} is not listed in SHA256SUMS.txt; refusing to install an unverified binary"
        )
    })?;

    // 2. Download the archive and verify its digest.
    let archive_path = temp_dir.join(&archive_filename);
    println!("==> Downloading {archive_url}...");
    download(&archive_url, &archive_path, archive_download_timeout())?;
    println!("==> Verifying SHA-256 release integrity...");
    let computed_hash = sha256_file(&archive_path)
        .map_err(|error| format!("Failed to compute SHA-256 of the downloaded archive: {error}"))?;
    if !computed_hash.eq_ignore_ascii_case(&expected_hash) {
        return Err(format!(
            "SHA-256 mismatch for {archive_filename}: expected {expected_hash}, got {computed_hash}; update aborted"
        ));
    }
    println!("✓ SHA-256 verified: {computed_hash}");

    // 3. Extract and locate the binary.
    let tar_args: Vec<String> = vec![
        "-xzf".to_string(),
        archive_path.to_string_lossy().into_owned(),
        "-C".to_string(),
        temp_dir.to_string_lossy().into_owned(),
    ];
    let tar_arg_refs: Vec<&str> = tar_args.iter().map(String::as_str).collect();
    let extracted = run_with_timeout("tar", &tar_arg_refs, archive_download_timeout())
        .ok_or_else(|| "Could not run tar to extract the release archive".to_string())?;
    if extracted.timed_out || !extracted.status.success() {
        return Err(format!(
            "Failed to extract the downloaded archive: {}",
            sanitize_terminal(&extracted.stderr_string()).trim()
        ));
    }
    let source_binary = find_file_named(&temp_dir, "rustfetch")
        .ok_or_else(|| "The downloaded archive does not contain a rustfetch binary".to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&source_binary, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("Failed to make the downloaded binary executable: {error}"))?;
    }
    verify_binary_version(&source_binary, &latest)?;

    // 4. Install atomically and verify the outcome.
    let target_dir = update_target_dir();
    let destination = target_dir.join("rustfetch");
    install_binary_atomic(&source_binary, &destination).map_err(|error| {
        format!(
            "Failed to install {}: {error}. If it is owned by your package manager, update it through the package manager instead",
            destination.display()
        )
    })?;
    verify_binary_version(&destination, &latest)
        .map_err(|error| format!("Installed binary failed version verification: {error}"))?;

    let alias = target_dir.join("rfetch");
    if let Err(error) = install_binary_atomic(&source_binary, &alias) {
        eprintln!(
            "warning: updated {} but could not update the rfetch alias {}: {error}",
            destination.display(),
            alias.display()
        );
    }

    let _ = Cache::clear_all();
    println!(
        "✓ Successfully updated to RustFetch v{latest} \"{latest_codename}\" in {}!",
        target_dir.display()
    );
    Ok(())
}

pub fn run_self_update() {
    if let Err(error) = self_update() {
        eprintln!("✗ Update failed: {error}");
        eprintln!(
            "Manual update: download the latest release from {RELEASES_URL} and replace the binary, or run the install script from a trusted checkout."
        );
        std::process::exit(1);
    }
}

fn theme_choices() -> Vec<String> {
    let names = rustfetch_render::theme_names();
    let mut choices = Vec::with_capacity(names.len() + 1);
    choices.push("none".to_string());
    choices.extend(names.iter().map(|name| (*name).to_string()));
    choices
}

fn backup_config(path: &Path) -> Result<Option<PathBuf>, String> {
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Failed to inspect {}: {error}", path.display())),
        Ok(metadata) if metadata.len() == 0 => Ok(None),
        Ok(_) => {
            let mut backup_name = path.as_os_str().to_os_string();
            backup_name.push(".bak");
            let backup = PathBuf::from(backup_name);
            std::fs::copy(path, &backup)
                .map_err(|error| format!("Failed to back up {}: {error}", path.display()))?;
            Ok(Some(backup))
        }
    }
}

fn save_config(config: &Config, path: &Path) -> Result<Option<PathBuf>, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    let backup = backup_config(path)?;
    let toml = config
        .to_toml_string()
        .map_err(|error| format!("Failed to serialize the configuration: {error}"))?;
    crate::shell_hook::write_atomic(path, &toml)?;
    Ok(backup)
}

pub fn run_setup_tui(config: &Config, config_path: &Path) -> Result<(), String> {
    use crossterm::{
        cursor::{Hide, MoveTo, Show},
        event::{read, Event, KeyCode, KeyModifiers},
        execute, queue,
        terminal::{
            disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
            LeaveAlternateScreen,
        },
    };
    use std::io::IsTerminal;

    let mut stdout = std::io::stdout();
    if !stdout.is_terminal() || !std::io::stdin().is_terminal() {
        return Err(
            "Interactive setup (--setup / --tui) requires an interactive terminal (TTY)"
                .to_string(),
        );
    }

    struct SetupGuard;
    impl Drop for SetupGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), Show, LeaveAlternateScreen);
        }
    }

    let mut config = config.clone();
    let themes = theme_choices();
    let border_styles = [
        "rounded", "single", "double", "thick", "dashed", "block", "ascii", "brackets",
    ];
    let logos_list = [
        "auto",
        "cat",
        "bengal",
        "turkish_van",
        "turkish_angora",
        "arch",
        "fedora",
        "bazzite",
        "nobara",
        "cachyos",
        "nixos",
        "ubuntu",
        "debian",
    ];

    if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
        return Err(format!("Failed to enter the alternate screen: {error}"));
    }
    if let Err(error) = enable_raw_mode() {
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        return Err(format!("Failed to enable raw mode: {error}"));
    }
    let _guard = SetupGuard;

    let mut selected_row = 0_usize;
    let mut status_msg = String::from(
        "Use Up/Down to select, Left/Right or Enter to toggle, [S] to Save, [Q] to Quit. Configuration comments are NOT preserved on save.",
    );

    loop {
        let _ = queue!(stdout, Clear(ClearType::All), MoveTo(0, 0));

        let theme_value = config.general.theme.as_deref().unwrap_or("none");
        let border_style_value = config.general.border_style.as_deref().unwrap_or("rounded");
        let mode_3d = config.general.logo.three_d.enabled;

        let items = [
            ("1. Color Theme", theme_value.to_string()),
            ("2. Logo / Mascot", config.general.logo.distro.clone()),
            (
                "3. Display Mode",
                if mode_3d {
                    "3D Animated".to_string()
                } else {
                    "Standard 2D".to_string()
                },
            ),
            (
                "4. Outer Border Box",
                if config.general.border {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
            ("5. Border Style", border_style_value.to_string()),
            (
                "6. Nerd Font Icons",
                if config.general.icons {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
            (
                "7. Center Output",
                if config.general.center {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
            (
                "8. GitHub Update Check",
                if config.general.check_updates {
                    "Enabled"
                } else {
                    "Disabled"
                }
                .to_string(),
            ),
        ];
        let item_count = items.len();

        print!("  \x1b[1;36m╭──────────────────────────────────────────────────────────────╮\x1b[0m\r\n");
        print!("  \x1b[1;36m│\x1b[0m  \x1b[1mRustFetch Interactive Configuration Wizard (--setup)\x1b[0m        \x1b[1;36m│\x1b[0m\r\n");
        print!("  \x1b[1;36m│\x1b[0m  \x1b[2mExisting settings are preserved; a .bak backup is kept.\x1b[0m   \x1b[1;36m│\x1b[0m\r\n");
        print!("  \x1b[1;36m│\x1b[0m  \x1b[2mComments/key order are not preserved; back up manually if needed.\x1b[0m \x1b[1;36m│\x1b[0m\r\n");
        print!("  \x1b[1;36m╰──────────────────────────────────────────────────────────────╯\x1b[0m\r\n\r\n");

        for (index, (label, value)) in items.iter().enumerate() {
            if index == selected_row {
                print!("  \x1b[1;32m❯ {label:<30}\x1b[0m \x1b[1;33m◀ {value} ▶\x1b[0m\r\n");
            } else {
                print!("    {label:<30} \x1b[2m{value}\x1b[0m\r\n");
            }
        }

        let preview_logo = logos::get_logo(if config.general.logo.distro == "auto" {
            "cat"
        } else {
            &config.general.logo.distro
        });
        let preview_lines = preview_logo.colored_lines();
        print!(
            "\r\n  \x1b[1mLive Logo Preview ({}):\x1b[0m\r\n",
            config.general.logo.distro
        );
        for line in preview_lines.iter().take(8) {
            print!("    {line}\r\n");
        }

        print!("\r\n  \x1b[36mℹ\x1b[0m {status_msg}\r\n");
        let _ = stdout.flush();

        let Ok(event) = read() else {
            return Ok(());
        };
        let Event::Key(key) = event else {
            continue;
        };
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return Ok(()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
            KeyCode::Up | KeyCode::Char('k') => {
                selected_row = if selected_row == 0 {
                    item_count - 1
                } else {
                    selected_row - 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected_row = (selected_row + 1) % item_count;
            }
            KeyCode::Right
            | KeyCode::Left
            | KeyCode::Enter
            | KeyCode::Char(' ')
            | KeyCode::Char('l')
            | KeyCode::Char('h') => {
                let forward = !matches!(key.code, KeyCode::Left | KeyCode::Char('h'));
                match selected_row {
                    0 => {
                        let current = config.general.theme.as_deref().unwrap_or("none");
                        let position = themes
                            .iter()
                            .position(|theme| theme == current)
                            .unwrap_or(0);
                        let next = if forward {
                            (position + 1) % themes.len()
                        } else if position == 0 {
                            themes.len() - 1
                        } else {
                            position - 1
                        };
                        config.general.theme = if themes[next] == "none" {
                            None
                        } else {
                            Some(themes[next].clone())
                        };
                    }
                    1 => {
                        let current = config.general.logo.distro.as_str();
                        let position = logos_list
                            .iter()
                            .position(|logo| *logo == current)
                            .unwrap_or(0);
                        let next = if forward {
                            (position + 1) % logos_list.len()
                        } else if position == 0 {
                            logos_list.len() - 1
                        } else {
                            position - 1
                        };
                        config.general.logo.distro = logos_list[next].to_string();
                    }
                    2 => {
                        config.general.logo.three_d.enabled = !config.general.logo.three_d.enabled;
                    }
                    3 => {
                        config.general.border = !config.general.border;
                    }
                    4 => {
                        let current = config.general.border_style.as_deref().unwrap_or("rounded");
                        let position = border_styles
                            .iter()
                            .position(|style| *style == current)
                            .unwrap_or(0);
                        let next = if forward {
                            (position + 1) % border_styles.len()
                        } else if position == 0 {
                            border_styles.len() - 1
                        } else {
                            position - 1
                        };
                        config.general.border_style = Some(border_styles[next].to_string());
                    }
                    5 => {
                        config.general.icons = !config.general.icons;
                    }
                    6 => {
                        config.general.center = !config.general.center;
                    }
                    7 => {
                        config.general.check_updates = !config.general.check_updates;
                    }
                    _ => {}
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                let backup = save_config(&config, config_path)?;
                status_msg = match backup {
                    Some(backup) => format!(
                        "✓ Saved to {} (backup: {}); note: comments are not preserved.",
                        config_path.display(),
                        backup.display()
                    ),
                    None => format!(
                        "✓ Saved to {}; note: comments are not preserved.",
                        config_path.display()
                    ),
                };
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "rustfetch-tools-{tag}-{}-{}",
            std::process::id(),
            now_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn test_build_release_tarball_url() {
        assert_eq!(
            build_release_tarball_url("v0.1.2", "x86_64", false),
            "https://github.com/MehmetCanWT/rustfetch/releases/download/v0.1.2/rustfetch-v0.1.2-linux-gnu-x86_64.tar.gz"
        );
        assert_eq!(
            build_release_tarball_url("0.2.0", "x86_64", true),
            "https://github.com/MehmetCanWT/rustfetch/releases/download/v0.2.0/rustfetch-v0.2.0-linux-musl-x86_64.tar.gz"
        );
        assert_eq!(
            build_release_tarball_url("0.2.0", "aarch64", false),
            "https://github.com/MehmetCanWT/rustfetch/releases/download/v0.2.0/rustfetch-v0.2.0-linux-gnu-aarch64.tar.gz"
        );
        assert_eq!(
            build_release_sums_url("0.2.0"),
            "https://github.com/MehmetCanWT/rustfetch/releases/download/v0.2.0/SHA256SUMS.txt"
        );
        assert_eq!(
            release_archive_filename("0.2.0", "x86_64", false),
            "rustfetch-v0.2.0-linux-gnu-x86_64.tar.gz"
        );
        assert_eq!(
            release_archive_filename("v0.1.3", "x86_64", true),
            "rustfetch-v0.1.3-linux-musl-x86_64.tar.gz"
        );
    }

    #[test]
    fn test_find_sha256_in_sums_skips_malformed_lines() {
        let sample = r#"
# Release Checksums
SIGNATURE-BLOCK-START
deadbeef

cfd385a2096a224b54b600aa7e282f18835bacb521c8d0905924a317da083964  rustfetch-0.1.3-1.fedora.x86_64.rpm
c4aeaa143ce6999ebab584945f67918f9f051f5b4ddb6dba47d4d6c761e8f172  rustfetch-v0.1.3-linux-gnu-x86_64.tar.gz
25b80e063e52165eeaab542bad9917d974d267a66ed0483e794bce5532e30a25 *./rustfetch-v0.1.3-linux-musl-x86_64.tar.gz
"#;
        assert_eq!(
            find_sha256_in_sums(sample, "rustfetch-v0.1.3-linux-gnu-x86_64.tar.gz"),
            Some("c4aeaa143ce6999ebab584945f67918f9f051f5b4ddb6dba47d4d6c761e8f172".to_string())
        );
        assert_eq!(
            find_sha256_in_sums(sample, "rustfetch-v0.1.3-linux-musl-x86_64.tar.gz"),
            Some("25b80e063e52165eeaab542bad9917d974d267a66ed0483e794bce5532e30a25".to_string())
        );
        assert_eq!(find_sha256_in_sums(sample, "nonexistent.tar.gz"), None);
    }

    #[test]
    fn test_find_sha256_in_sums_validates_hex_and_case() {
        let uppercase = format!("{}  asset.tar.gz\n", "A".repeat(64));
        assert_eq!(
            find_sha256_in_sums(&uppercase, "asset.tar.gz"),
            Some("a".repeat(64))
        );
        let non_hex = format!("{}  asset.tar.gz\n", "z".repeat(64));
        assert_eq!(find_sha256_in_sums(&non_hex, "asset.tar.gz"), None);
        let short = "abcdef  asset.tar.gz\n";
        assert_eq!(find_sha256_in_sums(short, "asset.tar.gz"), None);
    }

    #[test]
    fn test_is_sha256_hex() {
        assert!(is_sha256_hex(&"0".repeat(64)));
        assert!(!is_sha256_hex(&"g".repeat(64)));
        assert!(!is_sha256_hex(&"0".repeat(63)));
    }

    #[test]
    fn test_module_and_preset_descriptions_are_valid() {
        for &(name, description) in MODULE_DESCRIPTIONS {
            assert!(!description.is_empty());
            assert!(
                module_names().contains(&name),
                "Module '{name}' in MODULE_DESCRIPTIONS must be a canonical module name"
            );
        }
        for &name in module_names() {
            assert!(
                module_by_name(name).is_some(),
                "Canonical module '{name}' must resolve via module_by_name"
            );
        }
        for &(preset, description) in PRESET_DESCRIPTIONS {
            assert!(!description.is_empty());
            assert!(
                Config::from_preset(preset).is_some(),
                "Preset '{preset}' in PRESET_DESCRIPTIONS must resolve via Config::from_preset"
            );
        }
    }

    #[test]
    fn test_theme_choices_use_the_renderer_registry() {
        let choices = theme_choices();
        assert_eq!(choices.len(), rustfetch_render::theme_names().len() + 1);
        assert_eq!(choices.first().map(String::as_str), Some("none"));
        for name in rustfetch_render::theme_names() {
            assert!(
                rustfetch_render::get_theme(name).is_some(),
                "Theme '{name}' must resolve via get_theme"
            );
            assert!(choices.iter().any(|choice| choice == name));
        }
    }

    #[test]
    fn test_parse_version_output() {
        assert_eq!(
            parse_version_output("0.1.4 \"Siamese\"\n").as_deref(),
            Some("0.1.4")
        );
        assert_eq!(
            parse_version_output("rustfetch 0.2.0\n").as_deref(),
            Some("0.2.0")
        );
        assert_eq!(parse_version_output("v1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(
            parse_version_output("1.2.3-beta.1\n").as_deref(),
            Some("1.2.3-beta.1")
        );
        assert_eq!(parse_version_output("not a version"), None);
        assert_eq!(parse_version_output("1.2"), None);
    }

    #[test]
    fn test_curl_args_are_https_only_and_bounded() {
        let arguments = curl_download_args(
            "https://example.com/file.tar.gz",
            Path::new("/tmp/file.tar.gz"),
            Duration::from_secs(30),
        );
        assert!(arguments.iter().any(|argument| argument == "--proto"));
        assert!(arguments.iter().any(|argument| argument == "=https"));
        assert!(arguments.contains(&MAX_ARCHIVE_BYTES.to_string()));
        assert!(arguments.iter().any(|argument| argument == "--max-time"));
        assert!(arguments
            .iter()
            .any(|argument| argument == "--max-filesize"));
        assert!(arguments.iter().any(|argument| argument == "--url"));
        assert_eq!(
            arguments.last().map(String::as_str),
            Some("https://example.com/file.tar.gz")
        );
    }

    #[test]
    fn test_update_status_line_without_network() {
        assert_eq!(
            update_status_line(false, None, "0.1.4"),
            "skipped (--no-update-check)"
        );
        assert!(update_status_line(
            false,
            Some(&("9.9.9".to_string(), "X".to_string())),
            "0.1.4"
        )
        .contains("skipped"));
        let newer = ("9.9.9".to_string(), "Test".to_string());
        assert!(update_status_line(true, Some(&newer), "0.1.4").contains("Update available"));
        assert!(update_status_line(true, None, "0.1.4").contains("Offline"));
    }

    #[test]
    fn test_install_binary_atomic_replaces_existing_file() {
        let directory = temp_dir("atomic");
        let source = directory.join("src_bin");
        let destination = directory.join("rustfetch");
        std::fs::write(&source, b"new-binary").unwrap();
        std::fs::write(&destination, b"old-binary").unwrap();

        install_binary_atomic(&source, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"new-binary");
        assert!(!destination.with_extension("new").exists());
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn test_save_config_creates_backup_and_valid_config() {
        let directory = temp_dir("save-config");
        let path = directory.join("config.toml");
        std::fs::write(&path, "# user config\n[general]\npadding = 3\n").unwrap();

        let backup = save_config(&Config::default(), &path).unwrap();
        assert!(backup.is_some(), "existing config must be backed up");
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(Config::from_toml(&written).is_ok());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn test_save_config_reports_write_errors() {
        let directory = temp_dir("save-error");
        let blocker = directory.join("blocker");
        std::fs::write(&blocker, b"not a directory").unwrap();
        let path = blocker.join("config.toml");
        let error = save_config(&Config::default(), &path).unwrap_err();
        assert!(error.contains("Failed to create") || error.contains("Failed to back up"));
        let _ = std::fs::remove_dir_all(&directory);
    }
}
