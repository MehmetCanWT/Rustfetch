//! Full-screen `ratatui` live system monitor (`--live`) and shared TUI styling helpers.
//!
//! Uses `CrosstermBackend` in the alternate screen with responsive layout tiers,
//! live `Sparkline` CPU/RAM widgets, theme hot-swapping, and clean Kitty image
//! overlay synchronization (clearing and re-fitting on resize).

use crate::args::Cli;
use crate::image_picker::{
    default_logo_dir, fastfetch_logo_dir, get_image_color_and_palette, pick_random_image,
    tilde_expand,
};
use crate::json_export::extract_percentage;
use crate::layout::{InfoLine, strip_ansi};
use crate::logos;
use crate::tools;
use crate::{
    build_header, detect_distro_name, gather_info_lines, install_terminal_panic_hook,
    install_termination_handlers, reset_termination_flag, resolve_distro_logo, terminate_requested,
};
use crossterm::{
    cursor::{MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Sparkline},
};
use rustfetch_core::Module;
use rustfetch_core::config::{Config, ModuleConfig};
use std::io::{self, IsTerminal};
use std::time::{Duration, Instant};

/// Resolved RGB palette for `ratatui` views (`--live` and `--setup`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct TuiPalette {
    pub(crate) primary: Color,
    pub(crate) secondary: Color,
    pub(crate) accent: Color,
    pub(crate) border: Color,
    pub(crate) fg: Color,
    pub(crate) muted: Color,
    pub(crate) highlight_bg: Color,
}

impl TuiPalette {
    /// Builds a `ratatui` palette from a built-in theme name or optional hex accent.
    pub(crate) fn resolve(theme_name: Option<&str>, accent_hex: Option<&str>) -> Self {
        if let Some(theme) = theme_name.and_then(rustfetch_render::get_theme) {
            return Self {
                primary: Color::Rgb(theme.primary.0, theme.primary.1, theme.primary.2),
                secondary: Color::Rgb(theme.secondary.0, theme.secondary.1, theme.secondary.2),
                accent: Color::Rgb(theme.accent.0, theme.accent.1, theme.accent.2),
                border: Color::Rgb(theme.border.0, theme.border.1, theme.border.2),
                fg: Color::Reset,
                muted: Color::DarkGray,
                highlight_bg: Color::Rgb(
                    theme.border.0 / 3,
                    theme.border.1 / 3,
                    theme.border.2 / 3,
                ),
            };
        }

        let primary = accent_hex
            .and_then(rustfetch_render::parse_color)
            .map(|(r, g, b)| Color::Rgb(r, g, b))
            .unwrap_or(Color::Cyan);

        Self {
            primary,
            secondary: Color::LightCyan,
            accent: Color::Yellow,
            border: primary,
            fg: Color::Reset,
            muted: Color::DarkGray,
            highlight_bg: Color::Rgb(42, 46, 56),
        }
    }
}

/// Maps a config border style string to the closest `ratatui` `BorderType`.
pub(crate) fn border_type_from_style(style: Option<&str>) -> BorderType {
    match style.unwrap_or("rounded").to_ascii_lowercase().as_str() {
        "double" => BorderType::Double,
        "thick" | "heavy" => BorderType::Thick,
        "single" | "light" => BorderType::Plain,
        "quadrant" | "block" => BorderType::QuadrantOutside,
        _ => BorderType::Rounded,
    }
}

/// Converts a single ANSI-colored line (standard SGR or 24-bit TrueColor) into
/// a styled `ratatui::text::Line` so colored ASCII logos and half-block images
/// render cleanly inside `ratatui` widgets without raw escape bleed.
pub(crate) fn ansi_to_ratatui_line(input: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut style = Style::default();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.peek().copied() {
                Some('[') => {
                    chars.next();
                    let mut seq = String::new();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            if c == 'm' {
                                if !buf.is_empty() {
                                    spans.push(Span::styled(std::mem::take(&mut buf), style));
                                }
                                style = apply_sgr_params(style, &seq);
                            }
                            break;
                        }
                        seq.push(c);
                    }
                }
                Some(']') => {
                    // Skip OSC sequences (e.g. OSC-8 hyperlinks) cleanly.
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if ch != '\r' && ch != '\n' {
            buf.push(ch);
        }
    }

    if !buf.is_empty() {
        spans.push(Span::styled(buf, style));
    }
    Line::from(spans)
}

fn apply_sgr_params(mut style: Style, params: &str) -> Style {
    if params.is_empty() {
        return Style::default();
    }
    let nums: Vec<u16> = params
        .split(';')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .collect();
    let mut i = 0;
    while i < nums.len() {
        match nums[i] {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            2 => style = style.add_modifier(Modifier::DIM),
            3 => style = style.add_modifier(Modifier::ITALIC),
            4 => style = style.add_modifier(Modifier::UNDERLINED),
            22 => {
                style = style
                    .remove_modifier(Modifier::BOLD)
                    .remove_modifier(Modifier::DIM)
            }
            30 => style = style.fg(Color::Black),
            31 => style = style.fg(Color::Red),
            32 => style = style.fg(Color::Green),
            33 => style = style.fg(Color::Yellow),
            34 => style = style.fg(Color::Blue),
            35 => style = style.fg(Color::Magenta),
            36 => style = style.fg(Color::Cyan),
            37 => style = style.fg(Color::Gray),
            39 => style = style.fg(Color::Reset),
            40 => style = style.bg(Color::Black),
            41 => style = style.bg(Color::Red),
            42 => style = style.bg(Color::Green),
            43 => style = style.bg(Color::Yellow),
            44 => style = style.bg(Color::Blue),
            45 => style = style.bg(Color::Magenta),
            46 => style = style.bg(Color::Cyan),
            47 => style = style.bg(Color::Gray),
            49 => style = style.bg(Color::Reset),
            90 => style = style.fg(Color::DarkGray),
            91 => style = style.fg(Color::LightRed),
            92 => style = style.fg(Color::LightGreen),
            93 => style = style.fg(Color::LightYellow),
            94 => style = style.fg(Color::LightBlue),
            95 => style = style.fg(Color::LightMagenta),
            96 => style = style.fg(Color::LightCyan),
            97 => style = style.fg(Color::White),
            38 if i + 1 < nums.len() => match nums[i + 1] {
                2 if i + 4 < nums.len() => {
                    style = style.fg(Color::Rgb(
                        nums[i + 2] as u8,
                        nums[i + 3] as u8,
                        nums[i + 4] as u8,
                    ));
                    i += 4;
                }
                5 if i + 2 < nums.len() => {
                    style = style.fg(Color::Indexed(nums[i + 2] as u8));
                    i += 2;
                }
                _ => {}
            },
            48 if i + 1 < nums.len() => match nums[i + 1] {
                2 if i + 4 < nums.len() => {
                    style = style.bg(Color::Rgb(
                        nums[i + 2] as u8,
                        nums[i + 3] as u8,
                        nums[i + 4] as u8,
                    ));
                    i += 4;
                }
                5 if i + 2 < nums.len() => {
                    style = style.bg(Color::Indexed(nums[i + 2] as u8));
                    i += 2;
                }
                _ => {}
            },
            _ => {}
        }
        i += 1;
    }
    style
}

struct LiveTuiGuard;

impl Drop for LiveTuiGuard {
    fn drop(&mut self) {
        rustfetch_render::clear_kitty_images();
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

struct LiveState {
    config: Config,
    module_configs: Vec<ModuleConfig>,
    theme_choices: Vec<Option<&'static str>>,
    theme_idx: usize,
    header: String,
    distro_name: String,
    image_path: Option<String>,
    dynamic_accent: Option<String>,
    kitty_supported: bool,
    last_kitty_rect: Option<Rect>,
    cached_halfblock: Option<(u16, u16, Vec<Line<'static>>)>,
    ascii_logo: Vec<Line<'static>>,
    info_lines: Vec<InfoLine>,
    cpu_history: Vec<u64>,
    ram_history: Vec<u64>,
    prev_cpu_sample: Option<(u64, u64)>,
    last_graph_sample: Option<Instant>,
    last_info_refresh: Option<Instant>,
    update_hint: Option<String>,
    pending_kitty_rect: Option<Rect>,
}

const SPARKLINE_CAPACITY: usize = 60;

impl LiveState {
    fn new(config: &Config, cli: &Cli, configs: &[ModuleConfig]) -> Self {
        let mut live_config = config.clone();
        let mut live_configs = configs.to_vec();
        for mc in &mut live_configs {
            if matches!(
                mc.name.as_str(),
                "memory" | "disk" | "swap" | "battery" | "cpu_usage" | "temp"
            ) && mc.bar.is_none()
            {
                mc.bar = Some(true);
            }
        }

        let image_path = resolve_live_image(&mut live_config, cli);
        let dynamic_accent = if live_config.general.logo.auto_color {
            image_path
                .as_deref()
                .and_then(|p| get_image_color_and_palette(p, true, false).0)
        } else {
            None
        };

        let mut theme_choices: Vec<Option<&'static str>> =
            Vec::with_capacity(rustfetch_render::THEME_NAMES.len() + 1);
        theme_choices.push(None);
        theme_choices.extend(rustfetch_render::THEME_NAMES.iter().copied().map(Some));

        let theme_idx = live_config
            .general
            .theme
            .as_deref()
            .and_then(|active| {
                theme_choices
                    .iter()
                    .position(|choice| choice.is_some_and(|c| c.eq_ignore_ascii_case(active)))
            })
            .unwrap_or(0);

        let distro_name = if live_config.general.logo.distro == "auto" {
            detect_distro_name()
        } else {
            live_config.general.logo.distro.clone()
        };

        let logo = resolve_distro_logo(&distro_name, 0);
        let ascii_logo = logo
            .colored_lines()
            .into_iter()
            .map(|l| ansi_to_ratatui_line(&l))
            .collect();

        let header = build_header().unwrap_or_else(|| "RustFetch".to_string());
        let protocol = live_config.general.logo.protocol.to_ascii_lowercase();
        let kitty_supported = image_path.is_some()
            && (protocol == "kitty" || (protocol == "auto" && rustfetch_render::supports_kitty()));

        let mut prev_cpu_sample = rustfetch_core::modules::cpu_usage::sample_proc_stat();
        let mut cpu_history = Vec::with_capacity(SPARKLINE_CAPACITY);
        if let Some((t1, i1)) = prev_cpu_sample {
            std::thread::sleep(Duration::from_millis(20));
            if let Some((t2, i2)) = rustfetch_core::modules::cpu_usage::sample_proc_stat() {
                if let Some(pct) = rustfetch_core::modules::cpu_usage::usage_percent(t1, i1, t2, i2)
                {
                    cpu_history.push(pct as u64);
                }
                prev_cpu_sample = Some((t2, i2));
            }
        }

        Self {
            config: live_config,
            module_configs: live_configs,
            theme_choices,
            theme_idx,
            header,
            distro_name,
            image_path,
            dynamic_accent,
            kitty_supported,
            last_kitty_rect: None,
            cached_halfblock: None,
            ascii_logo,
            info_lines: Vec::new(),
            cpu_history,
            ram_history: Vec::with_capacity(SPARKLINE_CAPACITY),
            prev_cpu_sample,
            last_graph_sample: None,
            last_info_refresh: None,
            update_hint: None,
            pending_kitty_rect: None,
        }
    }

    fn active_theme_label(&self) -> &str {
        self.config.general.theme.as_deref().unwrap_or("default")
    }

    fn palette(&self) -> TuiPalette {
        TuiPalette::resolve(
            self.config.general.theme.as_deref(),
            self.dynamic_accent.as_deref(),
        )
    }

    fn sample_graphs(&mut self) {
        let current_sample = rustfetch_core::modules::cpu_usage::sample_proc_stat();
        if let (Some((t1, i1)), Some((t2, i2))) = (self.prev_cpu_sample, current_sample)
            && let Some(pct) = rustfetch_core::modules::cpu_usage::usage_percent(t1, i1, t2, i2)
        {
            push_capped(&mut self.cpu_history, pct as u64, SPARKLINE_CAPACITY);
        }
        self.prev_cpu_sample = current_sample;

        if let Some(pct) = rustfetch_core::modules::memory::Memory
            .detect()
            .and_then(|i| extract_percentage(&i.value))
        {
            push_capped(&mut self.ram_history, pct as u64, SPARKLINE_CAPACITY);
        }
        self.last_graph_sample = Some(Instant::now());
    }

    fn refresh_info(&mut self, modules: &[Box<dyn Module>]) {
        let distro_color = logos::get_logo(&self.distro_name).color;
        self.info_lines = gather_info_lines(
            &self.config,
            modules,
            &self.module_configs,
            self.dynamic_accent.as_deref(),
            Some(distro_color),
        );
        self.update_hint = self.info_lines.iter().find_map(|line| {
            let plain = strip_ansi(&line.value);
            if line.label.eq_ignore_ascii_case("update") && plain.contains("Update available") {
                Some(plain)
            } else {
                None
            }
        });
        self.last_info_refresh = Some(Instant::now());
    }

    fn cycle_theme(&mut self) {
        self.theme_idx = (self.theme_idx + 1) % self.theme_choices.len();
        self.config.general.theme = self.theme_choices[self.theme_idx].map(str::to_string);
        self.config.general.logo.auto_color = self.config.general.theme.is_none();
        if let Some(theme) = self
            .config
            .general
            .theme
            .as_deref()
            .and_then(rustfetch_render::get_theme)
        {
            self.config.general.border_color = Some(format!(
                "#{:02x}{:02x}{:02x}",
                theme.border.0, theme.border.1, theme.border.2
            ));
        }
    }

    fn clear_kitty(&mut self) {
        if self.kitty_supported && self.last_kitty_rect.is_some() {
            rustfetch_render::clear_kitty_images();
            self.last_kitty_rect = None;
        }
    }

    fn sync_kitty(&mut self) {
        if !self.kitty_supported {
            return;
        }
        match (self.image_path.as_deref(), self.pending_kitty_rect) {
            (Some(path), Some(rect)) if rect.width >= 4 && rect.height >= 4 => {
                if self.last_kitty_rect != Some(rect)
                    && rustfetch_render::render_kitty_in_rect(
                        path,
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                    )
                    .is_some()
                {
                    self.last_kitty_rect = Some(rect);
                }
            }
            _ => {
                self.clear_kitty();
            }
        }
    }

    fn halfblock_lines_for(&mut self, width: u16, height: u16) -> Option<&[Line<'static>]> {
        let path = self.image_path.as_deref()?;
        if width < 4 || height < 2 {
            return None;
        }
        let needs_rebuild = match &self.cached_halfblock {
            Some((w, h, _)) => *w != width || *h != height,
            None => true,
        };
        if needs_rebuild {
            let rendered = rustfetch_render::render_halfblock_ex(
                path,
                width as usize,
                Some(height as usize),
                true,
            )?;
            let lines: Vec<Line<'static>> = rendered
                .into_iter()
                .map(|l| ansi_to_ratatui_line(&l))
                .collect();
            self.cached_halfblock = Some((width, height, lines));
        }
        self.cached_halfblock
            .as_ref()
            .map(|(_, _, lines)| lines.as_slice())
    }
}

fn push_capped(history: &mut Vec<u64>, value: u64, cap: usize) {
    if history.len() >= cap {
        history.remove(0);
    }
    history.push(value);
}

fn resolve_live_image(config: &mut Config, cli: &Cli) -> Option<String> {
    if !config.general.logo.enabled || cli.no_logo {
        return None;
    }
    if let Some(path) = cli.image.as_deref() {
        let expanded = tilde_expand(path).into_owned();
        if std::path::Path::new(&expanded).is_file() {
            return Some(expanded);
        }
    }
    if let Some(path) = config.general.logo.image_path.as_deref() {
        let expanded = tilde_expand(path).into_owned();
        if std::path::Path::new(&expanded).is_file() {
            return Some(expanded);
        }
    }
    if config.general.logo.random_image {
        let dir = config
            .general
            .logo
            .image_dir
            .clone()
            .unwrap_or_else(|| default_logo_dir().to_string_lossy().into_owned());
        let fastfetch_dir = fastfetch_logo_dir();
        if let Some(picked) = pick_random_image(&dir, cli.keep_logo)
            .or_else(|| pick_random_image(&fastfetch_dir.to_string_lossy(), cli.keep_logo))
        {
            config.general.logo.image_path = Some(picked.clone());
            config.general.logo.random_image = false;
            return Some(picked);
        }
    }
    None
}

/// Computes whether the logo pane should be shown and its width in columns.
fn compute_logo_pane_width(area: Rect, show_logo: bool, preferred_cols: usize) -> Option<u16> {
    if !show_logo || area.width < 76 || area.height < 14 {
        return None;
    }
    let max_logo = (area.width * 42 / 100).max(24);
    let target = (preferred_cols as u16 + 4).clamp(24, max_logo);
    if area.width.saturating_sub(target) < 42 {
        return None;
    }
    Some(target)
}

fn render_live_frame(frame: &mut Frame<'_>, state: &mut LiveState) {
    let area = frame.area();
    let palette = state.palette();
    let show_borders = state.config.general.border;
    let border_type = border_type_from_style(state.config.general.border_style.as_deref());
    state.pending_kitty_rect = None;

    let spark_h = if area.height >= 16 { 5 } else { 3 };
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(spark_h),
            Constraint::Length(1),
        ])
        .split(area);

    // 1. Main area: Logo/Artwork (left, when wide enough) + Telemetry card (right)
    let show_logo = state.config.general.logo.enabled;
    let preferred_w = state.config.general.logo.image_width_cols.clamp(24, 52);
    if let Some(logo_w) = compute_logo_pane_width(root[0], show_logo, preferred_w) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(logo_w), Constraint::Min(40)])
            .split(root[0]);
        render_logo_pane(frame, cols[0], state, palette, show_borders, border_type);
        render_info_pane(frame, cols[1], state, palette, show_borders, border_type);
    } else {
        render_info_pane(frame, root[0], state, palette, show_borders, border_type);
    }

    // 2. Live CPU & RAM Sparklines
    let graph_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[1]);
    render_sparkline_pane(
        frame,
        graph_cols[0],
        "CPU",
        &state.cpu_history,
        palette.primary,
        palette.border,
        show_borders,
        border_type,
    );
    render_sparkline_pane(
        frame,
        graph_cols[1],
        "RAM",
        &state.ram_history,
        palette.secondary,
        palette.border,
        show_borders,
        border_type,
    );

    // 3. Footer / Controls bar
    render_footer(frame, root[2], state, palette);
}

fn render_logo_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &mut LiveState,
    palette: TuiPalette,
    show_borders: bool,
    border_type: BorderType,
) {
    let inner = if show_borders {
        let title = if state.image_path.is_some() {
            " Artwork "
        } else {
            " Logo "
        };
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(Style::default().fg(palette.border));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    } else {
        Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(1),
        }
    };

    if inner.width < 4 || inner.height < 2 {
        return;
    }

    if state.kitty_supported && state.image_path.is_some() {
        // Reserve exact inner rectangle; Kitty graphics are flushed right after
        // `terminal.draw(...)` so they align cleanly with the ratatui frame.
        state.pending_kitty_rect = Some(inner);
        return;
    }

    if state.image_path.is_some()
        && let Some(lines) = state.halfblock_lines_for(inner.width, inner.height)
    {
        let pad_top = inner.height.saturating_sub(lines.len() as u16) / 2;
        let render_area = Rect {
            x: inner.x,
            y: inner.y + pad_top,
            width: inner.width,
            height: inner.height.saturating_sub(pad_top),
        };
        frame.render_widget(
            Paragraph::new(lines.to_vec()).alignment(Alignment::Center),
            render_area,
        );
        return;
    }

    let pad_top = inner.height.saturating_sub(state.ascii_logo.len() as u16) / 2;
    let render_area = Rect {
        x: inner.x,
        y: inner.y + pad_top,
        width: inner.width,
        height: inner.height.saturating_sub(pad_top),
    };
    frame.render_widget(
        Paragraph::new(state.ascii_logo.clone()).alignment(Alignment::Center),
        render_area,
    );
}

fn render_info_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &LiveState,
    palette: TuiPalette,
    show_borders: bool,
    border_type: BorderType,
) {
    let version = rustfetch_core::CURRENT_VERSION;
    let codename = rustfetch_core::codename_for_version(version);
    let inner = if show_borders {
        let title = format!(" {} — RustFetch v{version} \"{codename}\" ", state.header);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(Style::default().fg(palette.border));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    } else {
        Rect {
            x: area.x.saturating_add(1),
            y: area.y,
            width: area.width.saturating_sub(2),
            height: area.height,
        }
    };

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(state.info_lines.len() + 3);
    if !show_borders {
        lines.push(Line::from(Span::styled(
            state.header.clone(),
            Style::default()
                .fg(palette.primary)
                .add_modifier(Modifier::BOLD),
        )));
        let sep_len = state.header.chars().count().min(inner.width as usize);
        lines.push(Line::from(Span::styled(
            "─".repeat(sep_len),
            Style::default().fg(palette.muted),
        )));
    }

    let is_divider_line = |l: &InfoLine| l.label.starts_with("---DIVIDER");
    let max_label_w = state
        .info_lines
        .iter()
        .filter(|l| !is_divider_line(l))
        .map(|l| l.label.chars().count())
        .max()
        .unwrap_or(8)
        .min(18);
    let sep = &state.config.general.separator;
    let show_icons = state.config.general.icons;

    for info in &state.info_lines {
        if is_divider_line(info) {
            let rule_w = inner.width.saturating_sub(2) as usize;
            let section = info
                .label
                .strip_prefix("---DIVIDER:")
                .map(|s| s.trim_end_matches("---"))
                .filter(|s| !s.is_empty());
            if let Some(title) = section {
                let side = rule_w.saturating_sub(title.chars().count() + 2) / 2;
                lines.push(Line::from(vec![
                    Span::styled("─".repeat(side), Style::default().fg(palette.muted)),
                    Span::styled(
                        format!(" {title} "),
                        Style::default()
                            .fg(palette.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("─".repeat(side), Style::default().fg(palette.muted)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "─".repeat(rule_w),
                    Style::default().fg(palette.muted),
                )));
            }
            continue;
        }

        let is_update = info.label.eq_ignore_ascii_case("update");
        let label_color = if is_update {
            Color::LightYellow
        } else {
            palette.primary
        };

        let mut spans: Vec<Span<'static>> = Vec::with_capacity(6);
        spans.push(Span::raw(" "));
        if show_icons && let Some(icon) = &info.icon {
            spans.push(Span::styled(
                format!("{icon} "),
                Style::default().fg(label_color),
            ));
        }
        if !state.config.general.icon_only {
            spans.push(Span::styled(
                format!("{:<width$}", info.label, width = max_label_w),
                Style::default()
                    .fg(label_color)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {sep} "),
                Style::default().fg(palette.muted),
            ));
        }

        let value_line = ansi_to_ratatui_line(&info.value);
        if value_line.spans.is_empty() {
            spans.push(Span::styled(
                strip_ansi(&info.value),
                Style::default().fg(palette.fg),
            ));
        } else {
            spans.extend(value_line.spans);
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

#[allow(clippy::too_many_arguments)]
fn render_sparkline_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    name: &str,
    data: &[u64],
    color: Color,
    border_color: Color,
    show_borders: bool,
    border_type: BorderType,
) {
    let latest = data.last().copied().unwrap_or(0);
    let title = format!(" {name} {latest:>3}% ");
    let spark = Sparkline::default()
        .data(data)
        .max(100)
        .style(Style::default().fg(color));

    if show_borders {
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(Style::default().fg(border_color));
        frame.render_widget(spark.block(block), area);
    } else {
        let block = Block::default()
            .title(title)
            .borders(Borders::TOP)
            .border_style(Style::default().fg(border_color));
        frame.render_widget(spark.block(block), area);
    }
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &LiveState, palette: TuiPalette) {
    let border_state = if state.config.general.border {
        "on"
    } else {
        "off"
    };
    let compact = area.width < 76;
    let theme_label = if compact {
        " Theme ".to_string()
    } else {
        format!(" Theme ({}) ", state.active_theme_label())
    };
    let border_label = if compact {
        " Border ".to_string()
    } else {
        format!(" Border ({border_state}) ")
    };
    let mut spans = vec![
        Span::styled(
            " [t]",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(theme_label, Style::default().fg(palette.muted)),
        Span::styled(
            "[b]",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(border_label, Style::default().fg(palette.muted)),
        Span::styled(
            "[r]",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Refresh ", Style::default().fg(palette.muted)),
        Span::styled(
            "[u]",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Update ", Style::default().fg(palette.muted)),
        Span::styled(
            "[q]",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Quit", Style::default().fg(palette.muted)),
    ];

    if state.update_hint.is_some() {
        spans.push(Span::styled(
            "  • Update available (press u)",
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Runs the `ratatui`-powered interactive live monitor (`rustfetch --live`).
pub(crate) fn run_live_tui(
    config: &Config,
    cli: &Cli,
    modules: &[Box<dyn Module>],
    configs: &[ModuleConfig],
) -> Result<(), String> {
    if !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        return Err("Live monitor (--live) requires an interactive terminal (TTY)".to_string());
    }

    install_termination_handlers();
    reset_termination_flag();
    install_terminal_panic_hook();

    let mut state = LiveState::new(config, cli, configs);
    state.sample_graphs();
    state.refresh_info(modules);

    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
    let mut stdout = io::stdout();
    if let Err(e) = execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        MoveTo(0, 0)
    ) {
        let _ = disable_raw_mode();
        return Err(format!("Failed to enter alternate screen: {e}"));
    }
    let guard = LiveTuiGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Failed to initialize TUI terminal: {e}"))?;

    loop {
        if terminate_requested() {
            break;
        }

        if state
            .last_graph_sample
            .is_none_or(|t| t.elapsed() >= Duration::from_millis(950))
        {
            state.sample_graphs();
        }
        if state
            .last_info_refresh
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(2))
        {
            state.refresh_info(modules);
        }

        terminal
            .draw(|f| render_live_frame(f, &mut state))
            .map_err(|e| format!("Failed to draw live TUI frame: {e}"))?;
        state.sync_kitty();

        if let Ok(true) = event::poll(Duration::from_millis(250))
            && let Ok(ev) = event::read()
        {
            match ev {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        state.cycle_theme();
                        state.refresh_info(modules);
                    }
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        state.config.general.border = !state.config.general.border;
                        state.clear_kitty();
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        state.sample_graphs();
                        state.refresh_info(modules);
                    }
                    KeyCode::Char('u') | KeyCode::Char('U') => {
                        drop(terminal);
                        drop(guard);
                        return tools::self_update();
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {
                    state.clear_kitty();
                    let _ = terminal.clear();
                }
                _ => {}
            }
        }
    }

    drop(terminal);
    drop(guard);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_to_ratatui_line_parses_truecolor_and_strips_osc8() {
        let input =
            "\x1b[1;38;2;255;128;64mHello\x1b[0m \x1b]8;;https://example.com\x07Link\x1b]8;;\x07";
        let line = ansi_to_ratatui_line(input);
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "Hello");
        assert_eq!(line.spans[0].style.fg, Some(Color::Rgb(255, 128, 64)));
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(line.spans[1].content, " Link");
        assert_eq!(line.spans[1].style.fg, None);
    }

    #[test]
    fn logo_pane_hides_on_narrow_or_short_terminals() {
        assert_eq!(
            compute_logo_pane_width(Rect::new(0, 0, 60, 30), true, 40),
            None
        );
        assert_eq!(
            compute_logo_pane_width(Rect::new(0, 0, 120, 10), true, 40),
            None
        );
        assert_eq!(
            compute_logo_pane_width(Rect::new(0, 0, 140, 35), false, 40),
            None
        );
        let wide = compute_logo_pane_width(Rect::new(0, 0, 140, 35), true, 36);
        assert!(wide.is_some_and(|w| (24..=58).contains(&w)));
    }

    #[test]
    fn tui_palette_resolves_builtin_themes_and_hex_fallback() {
        let cat = TuiPalette::resolve(Some("catppuccin"), None);
        assert!(matches!(cat.primary, Color::Rgb(_, _, _)));
        let custom = TuiPalette::resolve(None, Some("#ff8800"));
        assert_eq!(custom.primary, Color::Rgb(255, 136, 0));
    }

    #[test]
    fn push_capped_respects_capacity() {
        let mut hist = Vec::new();
        for i in 0..10 {
            push_capped(&mut hist, i, 4);
        }
        assert_eq!(hist, vec![6, 7, 8, 9]);
    }

    #[test]
    fn live_tui_renders_cleanly_at_wide_and_compact_sizes() {
        use clap::Parser;
        use ratatui::backend::TestBackend;

        let config = Config::default();
        let cli = Cli::try_parse_from(["rustfetch", "--live", "--no-logo", "--no-update-check"])
            .expect("valid cli");
        let mut state = LiveState::new(&config, &cli, &config.modules);
        state.cpu_history = vec![12, 24, 38, 19];
        state.ram_history = vec![42, 43, 44, 45];
        state.info_lines = vec![
            InfoLine {
                label: "OS".to_string(),
                value: "CachyOS".to_string(),
                icon: Some("󰣇".to_string()),
                color: String::new(),
            },
            InfoLine {
                label: "Memory".to_string(),
                value: "6.0 GiB / 15.4 GiB".to_string(),
                icon: None,
                color: String::new(),
            },
        ];

        for (w, h) in [(130, 32), (68, 16)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("test terminal");
            terminal
                .draw(|f| render_live_frame(f, &mut state))
                .expect("render frame");
            let buf = terminal.backend().buffer();
            let text: String = buf.content().iter().map(|c| c.symbol()).collect();
            assert!(text.contains("CachyOS"), "missing OS row at {w}x{h}");
            assert!(text.contains("CPU"), "missing CPU sparkline at {w}x{h}");
            assert!(text.contains("RAM"), "missing RAM sparkline at {w}x{h}");
            assert!(text.contains("Quit"), "missing footer at {w}x{h}");
        }
    }
}
