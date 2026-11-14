mod args;
mod image_picker;
mod interactive;
mod json_export;
mod layout;
mod logos;
mod shell_hook;
mod text;
mod tools;

pub use image_picker::{
    default_logo_dir, fastfetch_logo_dir, filter_candidate_images, get_image_color_and_palette,
    pick_random_image, tilde_expand, update_recent_history,
};
pub use json_export::extract_percentage;
pub use shell_hook::{
    get_shell_target_file, install_shell_hook, shell_init_script, strip_rustfetch_hook,
    uninstall_shell_hook,
};

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use rustfetch_core::config::{Config, ModuleConfig};
use rustfetch_core::modules::{all_modules, module_by_name, module_from_config};
use rustfetch_core::{Module, default_icon_for_module, detect_all};
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

use layout::{InfoLine, LogoBlock, color_code};
use text::{write_stdout, write_stdout_line};

use args::Cli;
pub use args::SHADING_MODES;
#[cfg(test)]
use args::{parse_shading_chars, parse_shading_mode, resolve_shading_chars};
use interactive::{build_ascii_logo_block, run_3d, run_ascii_animation, run_live};

pub(crate) fn resolve_distro_logo(distro_name: &str, padding: usize) -> logos::Logo {
    let logo = logos::get_logo(distro_name);
    let (term_w, term_h) = layout::terminal_size();
    if term_w < logo.width + layout::LOGO_GAP + 45 + padding * 2 || term_h < logo.lines.len() + 4 {
        let small_logo = logos::get_logo(&format!("{distro_name}_small"));
        if small_logo.width < logo.width || small_logo.lines.len() < logo.lines.len() {
            if term_w < small_logo.width + layout::LOGO_GAP + 30 + padding * 2
                || term_h < small_logo.lines.len() + 3
            {
                return logos::Logo {
                    lines: Vec::new(),
                    color: logo.color,
                    colors: logo.colors,
                    width: 0,
                };
            }
            return small_logo;
        }
    }
    logo
}

fn output_failure() -> ! {
    eprintln!("rustfetch: failed to write output to stdout");
    std::process::exit(1);
}

pub(crate) fn out(text: &str) {
    if !write_stdout(text) {
        output_failure();
    }
}

fn out_line(text: &str) {
    if !write_stdout_line(text) {
        output_failure();
    }
}

pub(crate) fn stdout_is_tty() -> bool {
    std::io::stdout().is_terminal()
}

#[cfg(unix)]
static TERMINATE_REQUESTED: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn handle_terminate(_: libc::c_int) {
    TERMINATE_REQUESTED.store(true, Ordering::SeqCst);
}

/// Installs SIGINT/SIGTERM/SIGHUP handlers that only set an atomic flag.
///
/// Every interactive loop polls [`terminate_requested`] and breaks so that the
/// terminal guards (raw mode / alternate screen / cursor) run their `Drop`.
pub(crate) fn install_termination_handlers() {
    #[cfg(unix)]
    unsafe {
        let handler = handle_terminate as *const () as libc::sighandler_t;
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            libc::signal(signal, handler);
        }
    }
}

pub(crate) fn terminate_requested() -> bool {
    #[cfg(unix)]
    {
        TERMINATE_REQUESTED.load(Ordering::SeqCst)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub(crate) fn reset_termination_flag() {
    #[cfg(unix)]
    TERMINATE_REQUESTED.store(false, Ordering::SeqCst);
}

/// Best-effort terminal restore for panics in interactive modes.
pub(crate) fn install_terminal_panic_hook() {
    use std::sync::Once;
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = crossterm::terminal::disable_raw_mode();
            {
                use crossterm::cursor::Show;
                use crossterm::terminal::LeaveAlternateScreen;
                let mut stdout = std::io::stdout();
                let _ = crossterm::execute!(stdout, Show, LeaveAlternateScreen);
            }
            let _ = write_stdout("\x1b[0m\x1b[?7h");
            previous(info);
        }));
    });
}

/// Resolves the active configuration (--preset > --config > XDG default) and
/// applies every CLI override before any command runs.
fn resolve_config(
    cli: &Cli,
    allow_missing_path: bool,
) -> Result<(Config, std::path::PathBuf), String> {
    let path = cli
        .config
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(Config::config_path);
    let mut config = if let Some(preset_name) = &cli.preset {
        Config::from_preset(preset_name).ok_or_else(|| {
            format!(
                "Unknown preset: {preset_name}. Available presets: default, minimal, clean, dots, card, brackets, neofetch, retro, modern, compact"
            )
        })?
    } else if allow_missing_path {
        if path.exists() {
            Config::load_from(&path).unwrap_or_default()
        } else {
            Config::default()
        }
    } else if cli.config.is_some() || path.exists() {
        Config::load_from(&path)?
    } else {
        Config::load()
    };
    apply_cli_overrides(&mut config, cli);
    if cli.no_update_check {
        config.general.check_updates = false;
    }
    Ok((config, path))
}

/// Applies every command-line override on top of the loaded configuration.
fn apply_cli_overrides(config: &mut Config, cli: &Cli) {
    if let Some(logo) = &cli.logo {
        config.general.logo.distro = logo.clone();
        config.general.logo.image_path = None;
        config.general.logo.random_image = false;
    }
    if let Some(ascii) = &cli.ascii {
        config.general.logo.ascii_path = Some(ascii.clone());
        config.general.logo.image_path = None;
        config.general.logo.random_image = false;
    }
    if let Some(img) = &cli.image {
        config.general.logo.image_path = Some(img.clone());
        config.general.logo.random_image = false;
        config.general.logo.enabled = true;
    }
    if cli.no_logo {
        config.general.logo.enabled = false;
    }
    if cli.clear {
        config.general.clear_screen = Some(true);
    }
    if cli.live {
        config.general.live = true;
    }
    if let Some(w) = cli.width {
        config.general.logo.image_width_cols = w;
        config.general.logo.three_d.width = Some(w);
    }
    if let Some(proto) = &cli.protocol {
        config.general.logo.protocol = proto.clone();
    }
    if let Some(pos) = &cli.logo_position {
        config.general.logo.position = Some(pos.clone());
    }
    if cli.three_d {
        config.general.logo.three_d.enabled = true;
    }
    if let Some(speed) = cli.speed {
        config.general.logo.three_d.speed = speed;
    }
    if cli.rotate_x || cli.rotate_y {
        config.general.logo.three_d.rotate_x = cli.rotate_x;
        config.general.logo.three_d.rotate_y = cli.rotate_y;
    }
    if let Some(size) = cli.size {
        config.general.logo.three_d.size = size;
    }
    if let Some(depth) = cli.depth {
        config.general.logo.three_d.depth = depth;
    }
    if let Some(mode) = &cli.shading_mode {
        config.general.logo.three_d.shading_mode = mode.clone();
    }
    if let Some(shading) = &cli.shading {
        config.general.logo.three_d.shading = Some(shading.clone());
    }
    if let Some(frames) = cli.frames {
        config.general.logo.three_d.frames = Some(frames);
        config.general.logo.animation.frames = Some(frames);
    }
    if cli.infinite {
        config.general.logo.three_d.infinite = Some(true);
        config.general.logo.animation.infinite = true;
    }
    if cli.hold {
        config.general.logo.three_d.hold = Some(true);
        config.general.logo.animation.hold = Some(true);
    }
    if cli.exit_on_key {
        config.general.logo.three_d.exit_on_key = Some(true);
        config.general.logo.animation.exit_on_key = Some(true);
    }
    if let Some(anim) = &cli.ascii_anim {
        config.general.logo.animation.enabled = true;
        config.general.logo.animation.path = Some(anim.clone());
    }
    if let Some(fps) = cli.fps {
        config.general.logo.animation.fps = fps;
    }
    if cli.center {
        config.general.center = true;
    }
    if let Some(color) = &cli.color {
        for mc in &mut config.modules {
            mc.color = Some(color.clone());
        }
    }
    if cli.border {
        config.general.border = true;
    }
    if let Some(bs) = &cli.border_style {
        config.general.border = true;
        config.general.border_style = Some(bs.clone());
    }
    if let Some(bc) = &cli.border_color {
        config.general.border_color = Some(bc.clone());
    }
    if let Some(bt) = &cli.border_title {
        config.general.border_title = Some(bt.clone());
    }
    if cli.icon_only {
        config.general.icon_only = true;
    }
    if let Some(sep) = &cli.separator {
        config.general.separator = sep.clone();
    }
    if let Some(t) = &cli.theme {
        config.general.theme = Some(t.clone());
    }
    if let Some(bi) = &cli.border_image {
        config.general.border_image = Some(bi.clone());
    }
    if let Some(bw) = cli.border_image_width {
        config.general.border_image_width = Some(bw);
    }
    if let Some(bh) = cli.border_image_height {
        config.general.border_image_height = Some(bh);
    }
    if let Some(ox) = cli.border_image_offset_x {
        config.general.border_image_offset_x = Some(ox);
    }
    if let Some(oy) = cli.border_image_offset_y {
        config.general.border_image_offset_y = Some(oy);
    }
    if let Some(zi) = cli.border_image_z_index {
        config.general.border_image_z_index = Some(zi);
    }
    if let Some(position) = &cli.border_image_position {
        config.general.border_image_position = position.clone();
    }
    if let Some(proto) = &cli.border_image_protocol {
        config.general.border_image_protocol = Some(proto.clone());
    }
    if let Some(par) = cli.border_image_preserve_aspect_ratio {
        config.general.border_image_preserve_aspect_ratio = Some(par);
    }
    if let Some(tot) = cli.border_image_text_on_top {
        config.general.border_image_text_on_top = Some(tot);
    }
    if let Some(theme_name) = &config.general.theme
        && let Some(t) = rustfetch_render::get_theme(theme_name)
        && config.general.border_color.is_none()
    {
        config.general.border_color = Some(format!(
            "#{:02x}{:02x}{:02x}",
            t.border.0, t.border.1, t.border.2
        ));
    }
}

/// Builds the module list, then applies the update-module filtering rules.
fn build_modules(config: &Config) -> (Vec<Box<dyn Module>>, Vec<ModuleConfig>) {
    let (mut modules, mut configs): (Vec<Box<dyn Module>>, Vec<ModuleConfig>) =
        if config.modules.is_empty() {
            let all = all_modules();
            let cfgs = all.iter().map(|m| ModuleConfig::new(m.name())).collect();
            (all, cfgs)
        } else {
            config
                .modules
                .iter()
                .filter_map(|mc| module_from_config(mc).map(|m| (m, mc.clone())))
                .unzip()
        };

    if !config.general.check_updates {
        let mut filtered_mods = Vec::with_capacity(modules.len());
        let mut filtered_cfgs = Vec::with_capacity(configs.len());
        for (m, c) in modules.into_iter().zip(configs) {
            if c.name != "update" {
                filtered_mods.push(m);
                filtered_cfgs.push(c);
            }
        }
        modules = filtered_mods;
        configs = filtered_cfgs;
    } else if !configs.iter().any(|mc| mc.name == "update")
        && let Some(m) = module_by_name("update")
    {
        modules.push(m);
        configs.push(ModuleConfig::new("update"));
    }
    (modules, configs)
}

/// Writes the generated configuration file, honouring --config/--preset.
fn generate_config_file(cli: &Cli, config: &Config, path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create configuration directory {}: {e}",
                parent.display()
            )
        })?;
    }
    if let Some(bak) = Config::backup_if_exists(path) {
        out_line(&format!(
            "Backed up existing configuration to {}",
            bak.display()
        ));
    }
    let content = if cli.preset.is_some() {
        config
            .to_toml_string()
            .map_err(|e| format!("Failed to serialize preset configuration: {e}"))?
    } else {
        Config::to_default_toml()
    };
    std::fs::write(path, &content).map_err(|e| format!("Failed to write configuration: {e}"))?;
    out_line(&format!("Configuration generated at {}", path.display()));
    Ok(())
}

/// Imports a Fastfetch configuration, honouring --config as the output path.
fn import_fastfetch_config(path: &std::path::Path) -> Result<(), String> {
    let ff_path = Config::find_fastfetch_config()
        .ok_or_else(|| "No Fastfetch configuration file found to import.".to_string())?;
    let content = std::fs::read_to_string(&ff_path).map_err(|e| {
        format!(
            "Could not read Fastfetch config from {}: {e}",
            ff_path.display()
        )
    })?;
    let imported = Config::import_from_fastfetch(&content).ok_or_else(|| {
        format!(
            "Could not parse Fastfetch config from {}",
            ff_path.display()
        )
    })?;
    let toml_str = imported
        .to_toml_string()
        .map_err(|e| format!("Failed to serialize imported config: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create configuration directory {}: {e}",
                parent.display()
            )
        })?;
    }
    if let Some(bak) = Config::backup_if_exists(path) {
        out_line(&format!(
            "Backed up existing configuration to {}",
            bak.display()
        ));
    }
    std::fs::write(path, &toml_str).map_err(|e| format!("Failed to save imported config: {e}"))?;
    out_line(&format!(
        "Successfully imported Fastfetch config to {}",
        path.display()
    ));
    Ok(())
}

/// Builds one warning line per requested output mode that is shadowed by a
/// higher-precedence mode (benchmark > waybar > json > 3d > ascii-anim > live).
fn mode_conflict_warnings(requested_modes: &[(&str, bool)]) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(selected) = requested_modes.iter().position(|(_, requested)| *requested) {
        for (index, (name, requested)) in requested_modes.iter().enumerate() {
            if *requested && index != selected {
                warnings.push(format!(
                    "warning: {name} is ignored because {} takes precedence",
                    requested_modes[selected].0
                ));
            }
        }
    }
    warnings
}

pub fn run() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    install_termination_handlers();
    let cli = Cli::parse();

    if cli.check_updates_bg {
        rustfetch_core::modules::update::refresh_update_cache();
        return;
    }

    if let Some(shell) = &cli.init {
        match shell_init_script(shell) {
            Some(script) => out(script),
            None => {
                eprintln!("Unknown shell: {shell}. Supported shells: bash, zsh, fish");
                std::process::exit(1);
            }
        }
        return;
    }

    if cli.install_hook {
        install_shell_hook();
        return;
    }

    if cli.uninstall_hook {
        uninstall_shell_hook();
        return;
    }

    if let Some(shell) = cli.completions {
        let bin_name = std::env::args_os()
            .next()
            .and_then(|arg| {
                std::path::Path::new(&arg)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "rustfetch".to_string());
        let mut cmd = Cli::command();
        let mut buffer: Vec<u8> = Vec::new();
        generate(shell, &mut cmd, &bin_name, &mut buffer);
        out(&String::from_utf8_lossy(&buffer));
        return;
    }

    if cli.print_default_config {
        out(&Config::to_default_toml());
        return;
    }

    if cli.list_modules {
        tools::run_list_modules();
        return;
    }

    if cli.list_themes {
        tools::run_list_themes();
        return;
    }

    if cli.list_presets {
        tools::run_list_presets();
        return;
    }

    if cli.list_logos {
        tools::run_list_logos();
        return;
    }

    if cli.clean_cache {
        tools::run_clean_cache();
        return;
    }

    if cli.self_update {
        tools::run_self_update();
        return;
    }

    let allow_missing = cli.generate_config
        || cli.import_fastfetch
        || cli.setup
        || (cli.doctor && cli.config.is_none());
    let (config, config_path) = match resolve_config(&cli, allow_missing) {
        Ok(resolved) => resolved,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    if cli.dump_config {
        match config.to_toml_string() {
            Ok(toml) => out(&toml),
            Err(error) => {
                eprintln!("Failed to serialize active configuration: {error}");
                std::process::exit(1);
            }
        }
        return;
    }

    if cli.generate_config {
        if let Err(error) = generate_config_file(&cli, &config, &config_path) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    if cli.import_fastfetch {
        if let Err(error) = import_fastfetch_config(&config_path) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    if cli.doctor {
        tools::run_doctor(&config, &config_path, config.general.check_updates);
        return;
    }

    if cli.setup {
        if let Err(error) = tools::run_setup_tui(&config, &config_path) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    let is_interactive = stdout_is_tty() && std::io::stdin().is_terminal();
    let is_dumb = std::env::var("TERM")
        .map(|t| t == "dumb" || t.is_empty())
        .unwrap_or(false);
    let tty_ready = is_interactive && !is_dumb;

    // Animated 3D / live monitor / animations must never run without a TTY:
    // raw-mode loops would hang display managers, cron jobs and pipes.
    let is_3d_requested = cli.three_d
        || config.general.logo.three_d.enabled
        || config.general.three_d.unwrap_or(false);
    let is_anim_requested = cli.ascii_anim.is_some() || config.general.logo.animation.enabled;
    let is_live_requested = cli.live || config.general.live;

    let mut is_3d = is_3d_requested && tty_ready && !cli.keep_logo;
    let is_anim = is_anim_requested && tty_ready && !cli.keep_logo;
    let is_live = is_live_requested && tty_ready && !cli.keep_logo;

    if is_3d && (cli.image.is_some() || cli.no_logo) {
        if cli.no_logo {
            eprintln!(
                "warning: --3d is disabled because --no-logo was given; showing static output"
            );
        } else {
            eprintln!(
                "warning: --3d is disabled because --image was given; rendering the static image"
            );
        }
        is_3d = false;
    }

    if !is_3d && !tty_ready && cli.three_d {
        eprintln!("warning: --3d requires an interactive TTY; falling back to static output");
    }
    if !is_live && !tty_ready && cli.live {
        eprintln!("warning: --live requires an interactive TTY; falling back to static output");
    }
    if !is_anim && !tty_ready && cli.ascii_anim.is_some() {
        eprintln!(
            "warning: --ascii-anim requires an interactive TTY; falling back to static output"
        );
    }

    let is_waybar = cli.waybar
        || cli
            .format
            .as_deref()
            .map(|f| f.eq_ignore_ascii_case("waybar"))
            .unwrap_or(false);
    let is_json = cli.json
        || cli
            .format
            .as_deref()
            .map(|f| f.eq_ignore_ascii_case("json"))
            .unwrap_or(false);

    let requested_modes: [(&str, bool); 6] = [
        ("--benchmark", cli.benchmark),
        ("--waybar", is_waybar),
        ("--json", is_json),
        ("--3d", is_3d),
        ("--ascii-anim", is_anim),
        ("--live", is_live),
    ];
    for warning in mode_conflict_warnings(&requested_modes) {
        eprintln!("{warning}");
    }

    if tty_ready && (is_3d || is_anim || is_live) {
        install_terminal_panic_hook();
    }

    let (modules, configs) = build_modules(&config);

    let result = if cli.benchmark {
        run_benchmark(&modules)
    } else if is_waybar {
        run_waybar(&config, &modules, &configs)
    } else if is_json {
        run_json(&config, &modules, &configs)
    } else if is_3d {
        run_3d(&config, &cli, &modules, &configs)
    } else if is_anim {
        run_ascii_animation(&config, &cli, &modules, &configs)
    } else if is_live {
        run_live(&config, &cli, &modules, &configs)
    } else {
        run_once(&config, &cli, &modules, &configs)
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

pub(crate) struct LineWrapGuard {
    active: bool,
}

impl LineWrapGuard {
    pub(crate) fn new() -> Self {
        // Disabling line wrapping only makes sense on a real terminal; escape
        // sequences must never leak into pipes or files.
        let active = stdout_is_tty();
        if active {
            let _ = write_stdout("\x1b[?7l");
        }
        Self { active }
    }
}

impl Drop for LineWrapGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = write_stdout("\x1b[?7h");
        }
    }
}

pub(crate) const OUTPUT_ERROR: &str = "failed to write output to stdout";

pub(crate) fn run_once(
    config: &Config,
    cli: &Cli,
    modules: &[Box<dyn Module>],
    configs: &[rustfetch_core::config::ModuleConfig],
) -> Result<(), String> {
    let max_attempts = if cli.keep_logo { 4 } else { 1 };
    let mut result = Ok(());
    for _ in 0..max_attempts {
        if cli.keep_logo {
            let start = std::time::Instant::now();
            let mut last_size = layout::terminal_size();
            let mut stable_ms = 0u64;
            while stable_ms < 45 && start.elapsed() < std::time::Duration::from_millis(300) {
                std::thread::sleep(std::time::Duration::from_millis(15));
                let cur = layout::terminal_size();
                if cur == last_size {
                    stable_ms += 15;
                } else {
                    last_size = cur;
                    stable_ms = 0;
                }
            }
        }
        let size_before = layout::terminal_size();
        result = run_once_inner(config, cli, modules, configs);
        if result.is_err() || !cli.keep_logo || layout::terminal_size() == size_before {
            break;
        }
    }
    result
}

fn image_default_max_rows(effective_logo_pos: &str, term_rows: usize, card_rows: usize) -> usize {
    if effective_logo_pos == "top" || effective_logo_pos == "bottom" {
        (term_rows / 2).clamp(8, 18)
    } else {
        card_rows.clamp(12, 24)
    }
}

fn begin_keep_logo_frame(keep_logo: bool) {
    if keep_logo && stdout_is_tty() {
        let _ = write_stdout("\x1b[?2026h");
        if terminal_supports_kitty() {
            rustfetch_render::clear_kitty_images();
        }
        let _ = write_stdout("\x1b[2J\x1b[3J\x1b[H");
    }
}

fn end_keep_logo_frame(keep_logo: bool) {
    if keep_logo && stdout_is_tty() {
        let _ = write_stdout("\x1b[?2026l");
    }
}

pub(crate) fn build_configured_palette_lines(
    config: &Config,
    image_palette: Option<Vec<(u8, u8, u8)>>,
) -> Option<Vec<String>> {
    if !config.general.colors.enabled {
        return None;
    }

    let theme_palette = config.general.theme.as_deref().and_then(|t_name| {
        rustfetch_render::get_theme(t_name).map(|t| {
            vec![
                t.background,
                t.border,
                t.primary,
                t.secondary,
                t.accent,
                t.text,
            ]
        })
    });

    let custom_colors: Option<Vec<(u8, u8, u8)>> = config
        .general
        .colors
        .custom
        .as_ref()
        .map(|list| {
            list.iter()
                .filter_map(|s| rustfetch_render::parse_color(s))
                .collect()
        })
        .or(image_palette)
        .or(theme_palette);

    let mut lines = layout::build_color_palette(
        &config.general.colors.symbol,
        config.general.colors.block,
        custom_colors.as_deref(),
    );

    if config.general.colors.rows == 1 && lines.len() > 1 {
        lines.truncate(1);
    }

    Some(lines)
}

fn run_once_inner(
    config: &Config,
    cli: &Cli,
    modules: &[Box<dyn Module>],
    configs: &[rustfetch_core::config::ModuleConfig],
) -> Result<(), String> {
    let _wrap_guard = LineWrapGuard::new();
    if (cli.clear || config.general.clear_screen.unwrap_or(false)) && stdout_is_tty() {
        let _ = write_stdout("\x1b[2J\x1b[3J\x1b[H");
    }
    if !cli.no_logo && !cli.keep_logo && terminal_supports_kitty() {
        rustfetch_render::clear_kitty_images();
    }
    let mut resolved_image_path = None;
    let mut dynamic_color = None;
    let mut image_palette = None;

    let custom_ascii_path = cli
        .ascii
        .as_deref()
        .or(config.general.logo.ascii_path.as_deref());

    let is_3d_requested = cli.three_d
        || config.general.logo.three_d.enabled
        || config.general.three_d.unwrap_or(false);

    let show_logo = config.general.logo.enabled && !cli.no_logo;
    if (!is_3d_requested || cli.image.is_some())
        && (custom_ascii_path.is_none() || cli.image.is_some())
        && show_logo
    {
        if config.general.logo.random_image && cli.image.is_none() {
            let dir = config
                .general
                .logo
                .image_dir
                .clone()
                .unwrap_or_else(|| default_logo_dir().to_string_lossy().into_owned());
            let fastfetch_dir = fastfetch_logo_dir();
            resolved_image_path = pick_random_image(&dir, cli.keep_logo)
                .or_else(|| pick_random_image(&fastfetch_dir.to_string_lossy(), cli.keep_logo));
        }
        if resolved_image_path.is_none() {
            resolved_image_path = config.general.logo.image_path.clone();
        }

        if let Some(img_path) = &resolved_image_path {
            let expanded_img = tilde_expand(img_path).to_string();
            resolved_image_path = Some(expanded_img.clone());
            let (dom, pal) = get_image_color_and_palette(
                &expanded_img,
                config.general.logo.auto_color,
                config.general.colors.image_palette,
            );
            dynamic_color = dom;
            image_palette = pal;
        }
    }

    if let Some(c) = &cli.color {
        dynamic_color = Some(c.clone());
    } else if let Some(t) = &cli.theme {
        if let Some(th) = rustfetch_render::get_theme(t) {
            dynamic_color = Some(format!(
                "#{:02x}{:02x}{:02x}",
                th.primary.0, th.primary.1, th.primary.2
            ));
        }
    } else if dynamic_color.is_none()
        && let Some(t_name) = &config.general.theme
        && let Some(t) = rustfetch_render::get_theme(t_name)
    {
        dynamic_color = Some(format!(
            "#{:02x}{:02x}{:02x}",
            t.primary.0, t.primary.1, t.primary.2
        ));
    }

    let palette_lines = build_configured_palette_lines(config, image_palette);

    let (logo_block, distro_color) = if show_logo && resolved_image_path.is_none() {
        let (block, color) =
            build_ascii_logo_block(config, custom_ascii_path, None, dynamic_color.as_deref());
        (Some(block), color)
    } else {
        (None, None)
    };

    let header_color = dynamic_color.as_deref().or(distro_color);

    let mut info_lines = gather_info_lines(
        config,
        modules,
        configs,
        dynamic_color.as_deref(),
        distro_color,
    );

    let header = build_header();

    let active_border_images = config.general.active_border_images();

    let border_image_lines = if active_border_images.is_empty() {
        let border_image_path = cli
            .border_image
            .as_deref()
            .or(config.general.border_image.as_deref())
            .map(|p| tilde_expand(p).to_string());
        let border_image_width = cli
            .border_image_width
            .or(config.general.border_image_width)
            .unwrap_or(24);
        let border_image_height = cli
            .border_image_height
            .or(config.general.border_image_height);
        let preserve_aspect_ratio = cli
            .border_image_preserve_aspect_ratio
            .or(config.general.border_image_preserve_aspect_ratio)
            .unwrap_or(true);

        border_image_path.as_deref().and_then(|p| {
            rustfetch_render::render_halfblock_ex(
                p,
                border_image_width,
                border_image_height,
                preserve_aspect_ratio,
            )
        })
    } else {
        None
    };

    let logo_pos = cli
        .logo_position
        .as_deref()
        .or(config.general.logo.position.as_deref())
        .unwrap_or("left");

    let mut render_opts = layout::RenderOptions {
        header: header.as_deref(),
        header_color,
        separator: &config.general.separator,
        padding: config.general.padding,
        center: config.general.center,
        border: config.general.border,
        border_style: config.general.border_style.as_deref(),
        border_color: config.general.border_color.as_deref(),
        border_chars: config.general.border_chars.as_ref(),
        border_title: config.general.border_title.as_deref(),
        box_padding: config.general.box_padding,
        palette_lines: palette_lines.as_deref(),
        palette_position: &config.general.colors.position,
        icon_only: config.general.icon_only,
        key_type: config.general.key_type.as_deref(),
        border_image: border_image_lines.as_deref(),
        border_image_position: &config.general.border_image_position,
        border_images: &active_border_images,
        logo_position: logo_pos,
    };

    let (term_cols, term_rows) = layout::terminal_size();
    let is_compact_term = term_rows < 18 || term_cols < 65;
    let max_allowed_rows = term_rows.saturating_sub(2).max(6);
    if term_rows < 22 {
        render_opts.palette_lines = None;
    }
    if term_rows < 24 {
        let overhead = (if config.general.border { 2 } else { 0 })
            + (if header.is_some() { 2 } else { 0 })
            + (if render_opts.palette_lines.is_some() {
                2
            } else {
                0
            });
        let max_modules = max_allowed_rows.saturating_sub(overhead).max(3);
        if info_lines.len() > max_modules {
            let preserve_update = info_lines
                .last()
                .is_some_and(|l| layout::strip_ansi(&l.value).starts_with("Update available"));
            if preserve_update {
                let update_line = info_lines.pop();
                info_lines.truncate(max_modules.saturating_sub(1));
                if let Some(u) = update_line {
                    info_lines.push(u);
                }
            } else {
                info_lines.truncate(max_modules);
            }
        }
    }

    if is_compact_term && cli.image.is_none() && cli.ascii.is_none() {
        begin_keep_logo_frame(cli.keep_logo);
        if !layout::render(None, &info_lines, &render_opts) {
            end_keep_logo_frame(cli.keep_logo);
            return Err(OUTPUT_ERROR.to_string());
        }
        end_keep_logo_frame(cli.keep_logo);
        return Ok(());
    }

    if show_logo && let Some(image_path) = &resolved_image_path {
        let config_img_w = cli.width.unwrap_or(config.general.logo.image_width_cols);
        let mut target_cols = config_img_w;

        let right_preview = layout::format_info_lines(
            &info_lines,
            &config.general.separator,
            config.general.icon_only,
            config.general.key_type.as_deref(),
        );
        let max_right_w = right_preview
            .iter()
            .map(|l| layout::strip_ansi_width(l))
            .max()
            .unwrap_or(0);
        let header_w = header
            .as_ref()
            .map(|h| unicode_width::UnicodeWidthStr::width(h.as_str()))
            .unwrap_or(0);
        let needed_right_w = max_right_w.max(header_w) + if config.general.border { 4 } else { 0 };

        let pad_val = config.general.padding;
        let mut effective_logo_pos = logo_pos;

        // Responsive layout adaptation for tiling window managers (Hyprland, MangoWM, etc.)
        let min_card_w = 32;
        let min_image_w = 16;

        if effective_logo_pos == "left" || effective_logo_pos == "right" {
            let avail_for_both = term_cols.saturating_sub(layout::LOGO_GAP + pad_val * 2 + 1);
            if avail_for_both < min_image_w + min_card_w {
                if term_cols >= 32 {
                    effective_logo_pos = "top";
                    target_cols = config_img_w
                        .min(term_cols.saturating_sub(pad_val * 2 + 2))
                        .max(min_image_w);
                }
            } else if target_cols + min_card_w > avail_for_both {
                target_cols = avail_for_both.saturating_sub(min_card_w).max(min_image_w);
            }
        } else if effective_logo_pos == "top" || effective_logo_pos == "bottom" {
            target_cols = config_img_w
                .min(term_cols.saturating_sub(pad_val * 2 + 2))
                .max(min_image_w);
        }

        render_opts.logo_position = effective_logo_pos;

        let card_rows = info_lines.len()
            + if header.is_some() { 2 } else { 0 }
            + if render_opts.palette_lines.is_some() {
                2
            } else {
                0
            }
            + if config.general.border { 2 } else { 0 };
        let default_max_rows = image_default_max_rows(effective_logo_pos, term_rows, card_rows);
        let mut max_rows_val = cli
            .height
            .unwrap_or(default_max_rows)
            .min(term_rows.saturating_sub(2).max(1));
        let mut max_image_rows = Some(max_rows_val);

        if cli.live
            && let Ok((cols, _)) = crossterm::terminal::size()
        {
            let max_cols = (cols as usize) / 2;
            if target_cols > max_cols && max_cols > 5 {
                target_cols = max_cols;
            }
        }

        let protocol = cli
            .protocol
            .as_deref()
            .unwrap_or(&config.general.logo.protocol)
            .to_lowercase();
        let use_kitty = protocol == "kitty" || (protocol == "auto" && terminal_supports_kitty());
        let use_sixel = protocol == "sixel"
            || (protocol == "auto" && !use_kitty && rustfetch_render::supports_sixel());
        let use_iterm = protocol == "iterm"
            || (protocol == "auto"
                && !use_kitty
                && !use_sixel
                && rustfetch_render::supports_iterm());

        if (use_kitty || use_sixel || use_iterm) && matches!(effective_logo_pos, "bottom" | "right")
        {
            eprintln!(
                "warning: --logo-position {effective_logo_pos} is not supported for pixel image protocols; falling back to left"
            );
            effective_logo_pos = "left";
            render_opts.logo_position = "left";
            let avail_for_both = term_cols.saturating_sub(layout::LOGO_GAP + pad_val * 2 + 1);
            target_cols = config_img_w;
            if avail_for_both < min_image_w + min_card_w {
                if term_cols >= 32 {
                    effective_logo_pos = "top";
                    render_opts.logo_position = "top";
                    target_cols = config_img_w
                        .min(term_cols.saturating_sub(pad_val * 2 + 2))
                        .max(min_image_w);
                }
            } else if target_cols + min_card_w > avail_for_both {
                target_cols = avail_for_both.saturating_sub(min_card_w).max(min_image_w);
            }
            let updated_default_max_rows =
                image_default_max_rows(effective_logo_pos, term_rows, card_rows);
            max_rows_val = cli
                .height
                .unwrap_or(updated_default_max_rows)
                .min(term_rows.saturating_sub(2).max(1));
            max_image_rows = Some(max_rows_val);
            if cli.live
                && let Ok((cols, _)) = crossterm::terminal::size()
            {
                let max_cols = (cols as usize) / 2;
                if target_cols > max_cols && max_cols > 5 {
                    target_cols = max_cols;
                }
            }
        }

        if effective_logo_pos == "behind" && terminal_supports_kitty() {
            let card_w = max_right_w
                .max(header_w)
                .max(target_cols)
                .min(term_cols.saturating_sub(pad_val * 2 + 1));
            let card_h = (info_lines.len() + 4).min(max_allowed_rows);
            begin_keep_logo_frame(cli.keep_logo);
            let _ = rustfetch_render::render_background(image_path, card_w, card_h, -1);
            if !layout::render(None, &info_lines, &render_opts) {
                end_keep_logo_frame(cli.keep_logo);
                return Err(OUTPUT_ERROR.to_string());
            }
            end_keep_logo_frame(cli.keep_logo);
            return Ok(());
        }

        let (est_cols, est_rows) =
            rustfetch_render::image_display_dimensions(image_path, target_cols, max_image_rows)
                .unwrap_or((target_cols, max_rows_val));

        let bounded_card_w = if effective_logo_pos == "top" || effective_logo_pos == "bottom" {
            needed_right_w.min(term_cols.saturating_sub(pad_val * 2 + 1))
        } else {
            needed_right_w
                .min(term_cols.saturating_sub(est_cols + layout::LOGO_GAP + pad_val * 2 + 1))
        };

        let reserve_rows = est_rows.max(card_rows).min(term_rows.saturating_sub(1));

        if use_kitty {
            let total_fetch_w = if effective_logo_pos == "top" || effective_logo_pos == "bottom" {
                est_cols.max(bounded_card_w)
            } else {
                est_cols + layout::LOGO_GAP + bounded_card_w
            };
            let h_pad =
                layout::compute_h_pad(total_fetch_w, config.general.padding, config.general.center);

            if rustfetch_render::supports_kitty_placeholders() {
                if let Some((actual_cols, _actual_lines, placeholder_rows)) =
                    rustfetch_render::render_kitty_placeholder(
                        image_path,
                        target_cols,
                        max_image_rows,
                        cli.keep_logo,
                    )
                {
                    if !layout::render_with_kitty_placeholder(
                        &placeholder_rows,
                        actual_cols,
                        h_pad,
                        &info_lines,
                        &render_opts,
                    ) {
                        end_keep_logo_frame(cli.keep_logo);
                        return Err(OUTPUT_ERROR.to_string());
                    }
                    end_keep_logo_frame(cli.keep_logo);
                    return Ok(());
                }
                if cli.keep_logo {
                    return Ok(());
                }
            } else {
                let pre_scroll = if effective_logo_pos == "top" || effective_logo_pos == "bottom" {
                    0
                } else {
                    reserve_rows
                };

                if let Some((actual_cols, actual_lines)) = rustfetch_render::render_image_positioned(
                    image_path,
                    target_cols,
                    max_image_rows,
                    pre_scroll,
                    h_pad,
                    cli.keep_logo,
                ) {
                    if !layout::render_with_image(
                        actual_lines,
                        actual_cols,
                        h_pad,
                        &info_lines,
                        &render_opts,
                    ) {
                        end_keep_logo_frame(cli.keep_logo);
                        return Err(OUTPUT_ERROR.to_string());
                    }
                    end_keep_logo_frame(cli.keep_logo);
                    return Ok(());
                }
                if cli.keep_logo {
                    return Ok(());
                }
            }
        } else if use_sixel {
            let total_fetch_w = if effective_logo_pos == "top" || effective_logo_pos == "bottom" {
                est_cols.max(bounded_card_w)
            } else {
                est_cols + layout::LOGO_GAP + bounded_card_w
            };
            let h_pad =
                layout::compute_h_pad(total_fetch_w, config.general.padding, config.general.center);

            begin_keep_logo_frame(cli.keep_logo);
            if effective_logo_pos != "top" && effective_logo_pos != "bottom" && reserve_rows > 1 {
                let cursor_up = format!(
                    "{}\x1b[{}A\r",
                    "\n".repeat(reserve_rows - 1),
                    reserve_rows - 1
                );
                if !write_stdout(&cursor_up) {
                    end_keep_logo_frame(cli.keep_logo);
                    return Err(OUTPUT_ERROR.to_string());
                }
            }

            if h_pad > 0 && !write_stdout(&" ".repeat(h_pad)) {
                end_keep_logo_frame(cli.keep_logo);
                return Err(OUTPUT_ERROR.to_string());
            }

            if let Some((actual_cols, actual_lines)) =
                rustfetch_render::render_sixel_ex(image_path, target_cols, max_image_rows)
            {
                if !layout::render_with_image(
                    actual_lines,
                    actual_cols,
                    h_pad,
                    &info_lines,
                    &render_opts,
                ) {
                    end_keep_logo_frame(cli.keep_logo);
                    return Err(OUTPUT_ERROR.to_string());
                }
                end_keep_logo_frame(cli.keep_logo);
                return Ok(());
            }
            end_keep_logo_frame(cli.keep_logo);
        } else if use_iterm {
            let total_fetch_w = if effective_logo_pos == "top" || effective_logo_pos == "bottom" {
                est_cols.max(bounded_card_w)
            } else {
                est_cols + layout::LOGO_GAP + bounded_card_w
            };
            let h_pad =
                layout::compute_h_pad(total_fetch_w, config.general.padding, config.general.center);

            begin_keep_logo_frame(cli.keep_logo);
            if effective_logo_pos != "top" && effective_logo_pos != "bottom" && reserve_rows > 1 {
                let cursor_up = format!(
                    "{}\x1b[{}A\r",
                    "\n".repeat(reserve_rows - 1),
                    reserve_rows - 1
                );
                if !write_stdout(&cursor_up) {
                    end_keep_logo_frame(cli.keep_logo);
                    return Err(OUTPUT_ERROR.to_string());
                }
            }

            if h_pad > 0 && !write_stdout(&" ".repeat(h_pad)) {
                end_keep_logo_frame(cli.keep_logo);
                return Err(OUTPUT_ERROR.to_string());
            }

            if let Some((actual_cols, actual_lines)) =
                rustfetch_render::render_iterm_ex(image_path, target_cols, max_image_rows)
            {
                if !layout::render_with_image(
                    actual_lines,
                    actual_cols,
                    h_pad,
                    &info_lines,
                    &render_opts,
                ) {
                    end_keep_logo_frame(cli.keep_logo);
                    return Err(OUTPUT_ERROR.to_string());
                }
                end_keep_logo_frame(cli.keep_logo);
                return Ok(());
            }
            end_keep_logo_frame(cli.keep_logo);
        }

        if let Some(lines) = if protocol == "braille" {
            rustfetch_render::render_braille_ex(image_path, target_cols, max_image_rows, true)
        } else {
            rustfetch_render::render_halfblock_ex(image_path, target_cols, max_image_rows, true)
        } {
            let w = lines
                .first()
                .map(|l| layout::strip_ansi_width(l))
                .unwrap_or(target_cols);
            let logo_block = LogoBlock { lines, width: w };
            begin_keep_logo_frame(cli.keep_logo);
            if !layout::render(Some(&logo_block), &info_lines, &render_opts) {
                end_keep_logo_frame(cli.keep_logo);
                return Err(OUTPUT_ERROR.to_string());
            }
            end_keep_logo_frame(cli.keep_logo);
            return Ok(());
        }
    }

    begin_keep_logo_frame(cli.keep_logo);
    if !layout::render(logo_block.as_ref(), &info_lines, &render_opts) {
        end_keep_logo_frame(cli.keep_logo);
        return Err(OUTPUT_ERROR.to_string());
    }
    end_keep_logo_frame(cli.keep_logo);
    Ok(())
}

use json_export::run_json;

#[cfg(test)]
fn generate_waybar_json(
    config: &Config,
    modules: &[Box<dyn Module>],
    configs: &[rustfetch_core::config::ModuleConfig],
) -> String {
    let info_lines = gather_info_lines(config, modules, configs, None, None);
    let header = build_header();
    json_export::generate_waybar_json(config, &info_lines, header.as_deref())
}

fn run_waybar(
    config: &Config,
    modules: &[Box<dyn Module>],
    configs: &[rustfetch_core::config::ModuleConfig],
) -> Result<(), String> {
    let info_lines = gather_info_lines(config, modules, configs, None, None);
    let header = build_header();
    json_export::run_waybar(config, &info_lines, header.as_deref())
}

pub fn render_sparkline(samples: &[u8], colored: bool) -> String {
    const TICKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if samples.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(samples.len() * 16);
    for &pct in samples {
        let clamped = pct.min(100);
        let idx = ((clamped as usize) * 7) / 100;
        if colored {
            let col = if clamped < 60 {
                "\x1b[38;2;102;217;160m"
            } else if clamped < 85 {
                "\x1b[38;2;249;175;79m"
            } else {
                "\x1b[38;2;243;139;168m"
            };
            out.push_str(col);
            out.push(TICKS[idx]);
        } else {
            out.push(TICKS[idx]);
        }
    }
    if colored {
        out.push_str("\x1b[0m");
    }
    out
}

/// Canonical progress-bar glyph styles accepted by config bar_style values.
pub const BAR_STYLES: &[&str] = &["blocks", "line", "dots", "ascii"];

/// Resolves a configured bar style name; unknown styles fall back to "blocks".
fn bar_style_name(style: Option<&str>) -> &'static str {
    match style.map(str::trim).map(str::to_ascii_lowercase) {
        Some(name) if name == "line" => "line",
        Some(name) if name == "dots" => "dots",
        Some(name) if name == "ascii" => "ascii",
        _ => "blocks",
    }
}

/// Filled and empty glyphs for a resolved bar style.
fn bar_glyphs(style: &str) -> (char, char) {
    match style {
        "line" => ('━', '─'),
        "dots" => ('●', '○'),
        "ascii" => ('#', '-'),
        _ => ('■', '-'),
    }
}

fn format_bar(
    percent: u8,
    width: usize,
    colored: bool,
    invert_color: bool,
    style: Option<&str>,
) -> String {
    // Keep the rendered bar bounded regardless of config values.
    let width = width.clamp(1, 256);
    let percent = percent.min(100);
    let elapsed = ((percent as usize * width) + 50) / 100;
    let remaining = width.saturating_sub(elapsed);
    let (filled_glyph, track_glyph) = bar_glyphs(bar_style_name(style));

    let color_code = if !colored {
        ""
    } else if !invert_color {
        if percent < 60 {
            "\x1b[38;2;102;217;160m"
        } else if percent < 85 {
            "\x1b[38;2;249;175;79m"
        } else {
            "\x1b[38;2;243;139;168m"
        }
    } else if percent < 20 {
        "\x1b[38;2;243;139;168m"
    } else if percent < 50 {
        "\x1b[38;2;249;175;79m"
    } else {
        "\x1b[38;2;102;217;160m"
    };

    let dim_track = if colored {
        "\x1b[38;2;108;112;134m"
    } else {
        ""
    };
    let reset = if colored { "\x1b[0m" } else { "" };
    format!(
        "{}[{}{}{}{}]",
        color_code,
        filled_glyph.to_string().repeat(elapsed),
        dim_track,
        track_glyph.to_string().repeat(remaining),
        color_code
    ) + reset
}

pub(crate) fn gather_info_lines(
    config: &Config,
    modules: &[Box<dyn Module>],
    configs: &[rustfetch_core::config::ModuleConfig],
    dynamic_color: Option<&str>,
    distro_color: Option<&str>,
) -> Vec<InfoLine> {
    let results = detect_all(modules);
    let collected: Vec<(InfoLine, bool)> = results
        .into_iter()
        .zip(configs.iter())
        .filter_map(|(result, mc)| {
            let is_divider = mc.name == "break" || mc.name == "divider";
            if is_divider {
                let raw_label = if let Some(lbl) = &mc.label {
                    if !lbl.is_empty() {
                        format!("---DIVIDER:{lbl}---")
                    } else {
                        "---DIVIDER---".to_string()
                    }
                } else {
                    "---DIVIDER---".to_string()
                };
                return Some((
                    InfoLine {
                        label: text::sanitize_terminal(&raw_label),
                        value: String::new(),
                        icon: None,
                        color: String::new(),
                    },
                    false,
                ));
            }

            let info = result?;
            let raw_label = if mc.name == "custom" {
                mc.label.clone().unwrap_or_default()
            } else {
                mc.label.clone().unwrap_or(info.label)
            };
            // System/network/config supplied text must never inject terminal
            // escape sequences (USER, SSID, media titles, custom values, ...).
            let label = text::sanitize_terminal(&raw_label);

            let raw_value = if mc.name == "custom" {
                mc.value.clone().unwrap_or_default()
            } else {
                info.value
            };
            let mut value = text::sanitize_terminal(&raw_value);

            if let Some(fmt) = &mc.format {
                value = text::sanitize_terminal(&text::expand_value_template(fmt, &value));
            }

            let icon = if config.general.icons {
                if let Some(user_icon) = &mc.icon {
                    Some(text::sanitize_terminal(user_icon))
                } else if mc.name == "os" {
                    Some(rustfetch_core::distro_icon(&value).to_string())
                } else {
                    default_icon_for_module(&mc.name).map(str::to_string)
                }
            } else {
                None
            };

            if mc.bar.unwrap_or(false)
                && !value.is_empty()
                && let Some(pct) = extract_percentage(&value)
            {
                let width = mc.bar_width.unwrap_or(10);
                let invert = mc.name == "battery";
                let bar = format_bar(pct, width, true, invert, mc.bar_style.as_deref());
                if value.contains('%') {
                    if let Some(pct_idx) = value.rfind('(') {
                        value = format!("{}{} {}", &value[..pct_idx], bar, &value[pct_idx..]);
                    } else {
                        value = format!("{} {}", value, bar);
                    }
                } else {
                    value = format!("{} {} ({}%)", value, bar, pct);
                }
            }

            if mc.name == "update" {
                value = if let Some((_, target)) = value
                    .strip_prefix("Update available (")
                    .and_then(|s| s.strip_suffix(')'))
                    .and_then(|s| s.split_once(" -> "))
                {
                    format!("Update available: {target} (rustfetch --self-update)")
                } else {
                    format!("{value} (rustfetch --self-update)")
                };
                // Intentional OSC-8 hyperlink is added *after* sanitisation so that
                // only this well-known escape sequence reaches the terminal.
                if layout::colors_enabled() {
                    value = format!(
                        "\x1b]8;;https://github.com/MehmetCanWT/rustfetch\x07{value}\x1b]8;;\x07"
                    );
                }
            }

            let theme_color = config
                .general
                .theme
                .as_deref()
                .and_then(rustfetch_render::get_theme)
                .map(|t| format!("#{:02x}{:02x}{:02x}", t.primary.0, t.primary.1, t.primary.2));
            let has_explicit_color = mc.color.is_some() || mc.name == "update";
            let color_str = mc
                .color
                .as_deref()
                .or(if mc.name == "update" {
                    Some("bright_yellow")
                } else {
                    None
                })
                .or(dynamic_color)
                .or(theme_color.as_deref())
                .or(distro_color)
                .unwrap_or("blue");
            let color = color_code(color_str);
            Some((
                InfoLine {
                    label,
                    value,
                    icon,
                    color,
                },
                has_explicit_color,
            ))
        })
        .collect();

    let grad_rgb: Vec<(u8, u8, u8)> = config
        .general
        .gradient
        .as_ref()
        .map(|list| {
            list.iter()
                .filter_map(|s| rustfetch_render::parse_color(s))
                .collect()
        })
        .unwrap_or_default();

    let grad_target_count = collected
        .iter()
        .filter(|(line, explicit)| !explicit && !line.label.starts_with("---DIVIDER"))
        .count();

    let mut grad_idx = 0usize;
    collected
        .into_iter()
        .map(|(mut line, explicit)| {
            if !grad_rgb.is_empty() && !explicit && !line.label.starts_with("---DIVIDER") {
                let t = if grad_target_count <= 1 {
                    0.0
                } else {
                    grad_idx as f32 / (grad_target_count - 1) as f32
                };
                grad_idx += 1;
                let (r, g, b) = rustfetch_render::interpolate_gradient(&grad_rgb, t);
                line.color = color_code(&format!("#{:02x}{:02x}{:02x}", r, g, b));
            }
            line
        })
        .collect()
}

/// Sanitises the identity fields and formats them, or None when unusable.
fn build_header_from(user: Option<&str>, hostname: Option<&str>) -> Option<String> {
    let user = text::sanitize_terminal(user?).trim().to_string();
    let hostname = text::sanitize_terminal(hostname?).trim().to_string();
    if user.is_empty() || hostname.is_empty() {
        return None;
    }
    Some(format!("{user}@{hostname}"))
}

pub(crate) fn build_header() -> Option<String> {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .ok()?;
    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })?;
    build_header_from(Some(&user), Some(&hostname))
}

pub(crate) fn detect_distro_name() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find_map(|line| line.strip_prefix("ID="))
                .map(|val| val.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "linux".to_string())
}

fn terminal_supports_kitty() -> bool {
    rustfetch_render::supports_kitty()
}

fn run_benchmark(modules: &[Box<dyn Module>]) -> Result<(), String> {
    use std::fmt::Write as FmtWrite;
    use std::time::Instant;

    let colored = layout::colors_enabled();
    let (bold, cyan, green, yellow, dim, reset) = if colored {
        (
            "\x1b[1m", "\x1b[36m", "\x1b[32m", "\x1b[33m", "\x1b[90m", "\x1b[0m",
        )
    } else {
        ("", "", "", "", "", "")
    };

    let total_start = Instant::now();
    let mut module_timings = Vec::new();
    for m in modules {
        let start = Instant::now();
        let res = m.detect();
        let elapsed = start.elapsed();
        module_timings.push((m.name(), elapsed, res.is_some()));
    }
    let total_time = total_start.elapsed();

    let mut output = String::new();
    let _ = writeln!(output, "{bold}{cyan}RustFetch Module Benchmark{reset}");
    let _ = writeln!(output, "──────────────────────────────────────────────");

    for (name, dur, detected) in &module_timings {
        let us = dur.as_micros();
        let ms = dur.as_secs_f64() * 1000.0;
        let status = if *detected { "✓" } else { "-" };
        let bar_len = ((us as f64 / 1500.0) * 12.0).clamp(1.0, 24.0) as usize;
        let bar = "■".repeat(bar_len);
        if us < 1000 {
            let _ = writeln!(
                output,
                "  [{status}] {name:<12} : {green}{us:>6} µs{reset}  {dim}{bar}{reset}"
            );
        } else {
            let _ = writeln!(
                output,
                "  [{status}] {name:<12} : {yellow}{ms:>6.2} ms{reset}  {dim}{bar}{reset}"
            );
        }
    }

    let _ = writeln!(output, "──────────────────────────────────────────────");
    let total_us = total_time.as_micros();
    let total_ms = total_time.as_secs_f64() * 1000.0;
    let _ = writeln!(
        output,
        "  {bold}Total Time   : {cyan}{total_ms:.2} ms ({total_us} µs){reset} ⚡"
    );
    let _ = writeln!(output);

    if !write_stdout(&output) {
        return Err(OUTPUT_ERROR.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_waybar_json() {
        let config = Config::default();
        let (modules, configs): (Vec<_>, Vec<_>) = config
            .modules
            .iter()
            .filter_map(|mc| module_from_config(mc).map(|m| (m, mc.clone())))
            .unzip();
        let json_str = generate_waybar_json(&config, &modules, &configs);
        assert!(!json_str.is_empty());
        assert!(
            !json_str.contains('\n'),
            "Waybar output must be a single line"
        );
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");
        assert_eq!(parsed["class"], "rustfetch");
        assert!(parsed.get("text").is_some());
        assert!(parsed.get("tooltip").is_some());
        let tooltip = parsed["tooltip"].as_str().unwrap();
        assert!(
            !tooltip.contains("\x1b["),
            "ANSI escape codes must be stripped from tooltip"
        );
        let text = parsed["text"].as_str().unwrap();
        assert!(
            !text.contains("\x1b["),
            "ANSI escape codes must be stripped from text"
        );
        assert!(!text.is_empty());
    }

    fn parse_cli(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("valid CLI arguments")
    }

    #[test]
    fn test_format_bar_styles() {
        assert_eq!(format_bar(50, 10, false, false, None), "[■■■■■-----]");
        assert_eq!(
            format_bar(50, 10, false, false, Some("blocks")),
            "[■■■■■-----]"
        );
        assert_eq!(
            format_bar(50, 10, false, false, Some("line")),
            "[━━━━━─────]"
        );
        assert_eq!(
            format_bar(50, 10, false, false, Some("dots")),
            "[●●●●●○○○○○]"
        );
        assert_eq!(
            format_bar(50, 10, false, false, Some("ascii")),
            "[#####-----]"
        );
        // Unknown/empty styles fall back to the block style.
        assert_eq!(format_bar(0, 4, false, false, Some("nope")), "[----]");
        assert_eq!(format_bar(100, 4, false, false, Some("")), "[■■■■]");
    }

    #[test]
    fn test_format_bar_width_is_clamped() {
        // 0 -> 1 cell, 100000 -> 256 cells.
        assert_eq!(format_bar(0, 0, false, false, None), "[-]");
        assert_eq!(format_bar(50, 0, false, false, None), "[■]");
        let clamped = format_bar(100, 100_000, false, false, None);
        assert_eq!(clamped.chars().filter(|c| *c == '■').count(), 256);
    }

    #[test]
    fn test_shading_parsers() {
        for mode in SHADING_MODES {
            assert!(parse_shading_mode(mode).is_ok(), "mode {mode} accepted");
        }
        assert!(parse_shading_mode("ascii").unwrap() == "ascii");
        assert!(parse_shading_mode(" ASCII ").unwrap() == "ascii");
        assert!(parse_shading_mode("").is_err());
        assert!(parse_shading_mode("blocks2").is_err());
        assert!(parse_shading_chars(" .:-=+*#%@").is_ok());
        assert!(parse_shading_chars("").is_err());
        assert!(parse_shading_chars("   ").is_err());
        assert!(parse_shading_chars("\x1b[31m").is_err());
        assert_eq!(parse_shading_chars(" .,- ").unwrap(), ".,-");
    }

    #[test]
    fn test_resolve_shading_chars_ignores_empty_values() {
        assert_eq!(
            resolve_shading_chars(Some("+#"), None).as_deref(),
            Some("+#")
        );
        assert_eq!(resolve_shading_chars(None, Some("  ")), None);
        assert_eq!(
            resolve_shading_chars(Some(""), Some(" .")).as_deref(),
            Some(".")
        );
    }

    #[test]
    fn test_cli_bounds_are_enforced() {
        assert!(Cli::try_parse_from(["rustfetch", "--width", "1"]).is_ok());
        assert!(Cli::try_parse_from(["rustfetch", "--width", "512"]).is_ok());
        assert!(Cli::try_parse_from(["rustfetch", "--width", "0"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--width", "513"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--height", "0"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--border-image-width", "513"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--border-image-height", "0"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--border-image-offset-x", "-10000"]).is_ok());
        assert!(Cli::try_parse_from(["rustfetch", "--border-image-offset-x", "10001"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--speed", "nan"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--speed", "0"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--speed", "10"]).is_ok());
        assert!(Cli::try_parse_from(["rustfetch", "--size", "0.05"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--size", "3"]).is_ok());
        assert!(Cli::try_parse_from(["rustfetch", "--depth", "6"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--depth", "5"]).is_ok());
        assert!(Cli::try_parse_from(["rustfetch", "--fps", "0"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--fps", "121"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--fps", "120"]).is_ok());
        assert!(Cli::try_parse_from(["rustfetch", "--shading-mode", "bogus"]).is_err());
        assert!(Cli::try_parse_from(["rustfetch", "--shading-mode", "dot"]).is_ok());
        assert!(Cli::try_parse_from(["rustfetch", "--shading", ""]).is_err());
    }

    #[test]
    fn test_build_header_sanitizes_control_sequences() {
        let header = build_header_from(Some("evil\x1b]0;PWNED\x07user"), Some("host\x1b[31m"));
        assert_eq!(header.as_deref(), Some("eviluser@host"));
        assert!(!header.unwrap().contains('\x1b'));
        assert_eq!(build_header_from(Some(""), Some("host")), None);
        assert_eq!(build_header_from(Some("user"), Some("   ")), None);
        assert_eq!(build_header_from(None, Some("host")), None);
    }

    #[test]
    fn test_dump_config_uses_active_configuration() {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_dump_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        std::fs::write(&path, "[general]\nseparator = \"@@\"\n").unwrap();

        let cli = parse_cli(&["rustfetch", "-c", path.to_str().unwrap()]);
        let (config, resolved_path) = resolve_config(&cli, false).expect("config resolved");
        assert_eq!(resolved_path, path);
        assert_eq!(config.general.separator, "@@");
        let active_toml = config.to_toml_string().expect("active config serializes");
        assert!(active_toml.contains("@@"));
        // The template must stay the built-in default instead of the active values.
        let default_toml = Config::to_default_toml();
        assert!(!default_toml.contains("@@"));
        assert!(default_toml.contains("separator"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_unreadable_explicit_config_is_an_error() {
        let cli = parse_cli(&["rustfetch", "-c", "/definitely/not/a/real/config.toml"]);
        assert!(resolve_config(&cli, false).is_err());
        // --generate-config / --import-fastfetch may target a path that does
        // not exist yet, so they resolve with allow_missing_path = true.
        assert!(resolve_config(&cli, true).is_ok());
    }

    #[test]
    fn test_apply_cli_overrides_image_enables_logo() {
        let mut config = Config::default();
        config.general.logo.enabled = false;
        let cli = parse_cli(&["rustfetch", "-i", "/tmp/logo.png"]);
        apply_cli_overrides(&mut config, &cli);
        assert!(config.general.logo.enabled);
        assert_eq!(
            config.general.logo.image_path.as_deref(),
            Some("/tmp/logo.png")
        );
        assert!(!config.general.logo.random_image);
    }

    #[test]
    fn test_mode_conflict_warnings() {
        let warnings = mode_conflict_warnings(&[
            ("--benchmark", false),
            ("--waybar", false),
            ("--json", true),
            ("--3d", false),
            ("--ascii-anim", false),
            ("--live", true),
        ]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("--live"));
        assert!(warnings[0].contains("--json"));
        assert!(mode_conflict_warnings(&[("--json", false)]).is_empty());
    }

    #[test]
    fn test_no_update_check_disables_updates() {
        let cli = parse_cli(&["rustfetch", "--no-update-check"]);
        let (config, _) = resolve_config(&cli, true).expect("config resolved");
        assert!(!config.general.check_updates);
    }

    #[test]
    fn test_apply_cli_overrides_extended_flags() {
        let mut config = Config::default();
        let cli = parse_cli(&[
            "rustfetch",
            "--no-logo",
            "--rotate-x",
            "--fps",
            "90",
            "--protocol",
            "braille",
            "--width",
            "32",
            "--ascii",
            "/tmp/art.txt",
        ]);
        apply_cli_overrides(&mut config, &cli);
        assert!(!config.general.logo.enabled);
        assert!(config.general.logo.three_d.rotate_x);
        assert!(!config.general.logo.three_d.rotate_y);
        assert_eq!(config.general.logo.animation.fps, 90.0);
        assert_eq!(config.general.logo.protocol, "braille");
        assert_eq!(config.general.logo.image_width_cols, 32);
        assert_eq!(config.general.logo.three_d.width, Some(32));
        assert_eq!(
            config.general.logo.ascii_path.as_deref(),
            Some("/tmp/art.txt")
        );
    }
}
