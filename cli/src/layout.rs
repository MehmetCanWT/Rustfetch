use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Upper bound for terminal dimensions accepted from any source (env, ioctl).
const MAX_TERM_DIMENSION: usize = 1000;
/// Fallback terminal size when nothing else is available.
const DEFAULT_TERM_WIDTH: usize = 80;
const DEFAULT_TERM_HEIGHT: usize = 24;
/// Minimum width reserved for the info card when it is placed beside a logo.
const MIN_INFO_CARD_WIDTH: usize = 34;
/// Upper bound for the user configured inner box padding.
const MAX_BOX_PADDING: usize = 16;

/// True when a configured border glyph occupies exactly one terminal cell.
fn is_single_cell(glyph: &str) -> bool {
    crate::text::visible_width(glyph) == 1
}

/// Replaces border glyphs that are not exactly one cell wide with the style
/// defaults: the box geometry repeats glyphs by count and would otherwise break.
fn normalize_border_chars(
    custom: Option<&rustfetch_core::BorderCharsConfig>,
    default: &rustfetch_core::BorderCharsConfig,
) -> rustfetch_core::BorderCharsConfig {
    let mut chars = custom.cloned().unwrap_or_else(|| default.clone());
    let fields: [(&mut String, &String); 11] = [
        (&mut chars.top_left, &default.top_left),
        (&mut chars.top, &default.top),
        (&mut chars.top_right, &default.top_right),
        (&mut chars.right, &default.right),
        (&mut chars.bottom_right, &default.bottom_right),
        (&mut chars.bottom, &default.bottom),
        (&mut chars.bottom_left, &default.bottom_left),
        (&mut chars.left, &default.left),
        (&mut chars.divider_left, &default.divider_left),
        (&mut chars.divider, &default.divider),
        (&mut chars.divider_right, &default.divider_right),
    ];
    for (glyph, fallback) in fields {
        if !is_single_cell(glyph) {
            *glyph = fallback.clone();
        }
    }
    chars
}

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

/// Decides whether ANSI colour may be emitted.
///
/// FORCE_COLOR/CLICOLOR_FORCE always win (unless explicitly "0"), NO_COLOR only
/// disables with a non-empty value, and a stdout that is not a terminal (pipe,
/// file, CI) never receives ANSI escapes.
pub fn colors_enabled() -> bool {
    let force = std::env::var("FORCE_COLOR")
        .map(|value| value != "0")
        .unwrap_or(false)
        || std::env::var("CLICOLOR_FORCE")
            .map(|value| value != "0")
            .unwrap_or(false);
    let no_color = std::env::var("NO_COLOR").ok();
    let term = std::env::var("TERM").ok();
    decide_colors(
        force,
        no_color.as_deref(),
        term.as_deref(),
        std::io::stdout().is_terminal(),
    )
}

/// Pure colour decision (unit-testable: no environment or TTY access).
fn decide_colors(
    force: bool,
    no_color: Option<&str>,
    term: Option<&str>,
    stdout_is_tty: bool,
) -> bool {
    if force {
        return true;
    }
    if no_color.is_some_and(|value| !value.is_empty()) {
        return false;
    }
    if term == Some("dumb") {
        return false;
    }
    stdout_is_tty
}

pub fn is_linux_console() -> bool {
    std::env::var("TERM").as_deref() == Ok("linux")
}

pub fn rgb_to_ansi(r: u8, g: u8, b: u8) -> &'static str {
    const BASIC: &[(u8, u8, u8, &str)] = &[
        (0, 0, 0, "\x1b[30m"),
        (205, 0, 0, "\x1b[31m"),
        (0, 205, 0, "\x1b[32m"),
        (205, 205, 0, "\x1b[33m"),
        (0, 0, 238, "\x1b[34m"),
        (205, 0, 205, "\x1b[35m"),
        (0, 205, 205, "\x1b[36m"),
        (229, 229, 229, "\x1b[37m"),
        (127, 127, 127, "\x1b[90m"),
        (255, 0, 0, "\x1b[91m"),
        (0, 255, 0, "\x1b[92m"),
        (255, 255, 0, "\x1b[93m"),
        (92, 92, 255, "\x1b[94m"),
        (255, 0, 255, "\x1b[95m"),
        (0, 255, 255, "\x1b[96m"),
        (255, 255, 255, "\x1b[97m"),
    ];

    let mut best_dist = u32::MAX;
    let mut best_code = "\x1b[37m";

    for &(br, bg, bb, code) in BASIC {
        let dr = (r as i32) - (br as i32);
        let dg = (g as i32) - (bg as i32);
        let db = (b as i32) - (bb as i32);
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best_code = code;
        }
    }
    best_code
}

pub fn color_code(name: &str) -> String {
    if name.starts_with("\x1b[") {
        return name.to_string();
    }
    if name.starts_with('#') {
        if let Some((r, g, b)) = rustfetch_render::parse_color(name) {
            if is_linux_console() {
                return rgb_to_ansi(r, g, b).to_string();
            }
            return format!("\x1b[38;2;{r};{g};{b}m");
        }
    }

    if let Some((r, g, b)) = rustfetch_render::parse_color(name) {
        return format!("\x1b[38;2;{r};{g};{b}m");
    }

    let code = match name.to_lowercase().as_str() {
        "bright_black" | "gray" | "grey" => "\x1b[38;2;140;140;140m",
        "bright_red" => "\x1b[38;2;255;82;82m",
        "bright_green" => "\x1b[38;2;105;240;174m",
        "bright_yellow" => "\x1b[38;2;255;215;64m",
        "bright_blue" => "\x1b[38;2;68;138;255m",
        "bright_magenta" => "\x1b[38;2;224;64;251m",
        "bright_cyan" => "\x1b[38;2;24;255;255m",
        "bright_white" => "\x1b[38;2;255;255;255m",
        _ => "\x1b[38;2;245;245;245m",
    };
    code.to_string()
}

pub struct InfoLine {
    pub label: String,
    pub value: String,
    pub icon: Option<String>,
    pub color: String,
}

pub struct LogoBlock {
    pub lines: Vec<String>,
    pub width: usize,
}

/// Sanitises a terminal dimension: drops zero/unset values, clamps to 1..=1000
/// and falls back to 80x24 when nothing usable was supplied.
fn sanitize_dimensions(cols: Option<usize>, rows: Option<usize>) -> (usize, usize) {
    let cols = cols
        .filter(|value| *value > 0)
        .map(|value| value.clamp(1, MAX_TERM_DIMENSION))
        .unwrap_or(DEFAULT_TERM_WIDTH);
    let rows = rows
        .filter(|value| *value > 0)
        .map(|value| value.clamp(1, MAX_TERM_DIMENSION))
        .unwrap_or(DEFAULT_TERM_HEIGHT);
    (cols, rows)
}

fn env_dimensions() -> (Option<usize>, Option<usize>) {
    let cols = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());
    let rows = std::env::var("LINES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());
    (cols, rows)
}

pub fn terminal_size() -> (usize, usize) {
    // A real terminal always wins over the environment: stale exported
    // COLUMNS/LINES values must not override the actual window size.
    if std::io::stdout().is_terminal() {
        if let Ok((width, height)) = crossterm::terminal::size() {
            if width > 0 && height > 0 {
                return sanitize_dimensions(Some(usize::from(width)), Some(usize::from(height)));
            }
        }
        #[cfg(unix)]
        {
            for &fd in &[libc::STDOUT_FILENO, libc::STDERR_FILENO, libc::STDIN_FILENO] {
                let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
                if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) } == 0
                    && ws.ws_col > 0
                    && ws.ws_row > 0
                {
                    return sanitize_dimensions(
                        Some(usize::from(ws.ws_col)),
                        Some(usize::from(ws.ws_row)),
                    );
                }
            }
        }
    }
    let (cols, rows) = env_dimensions();
    sanitize_dimensions(cols, rows)
}

pub fn terminal_width() -> usize {
    terminal_size().0
}

pub const LOGO_GAP: usize = 3;

pub fn compute_h_pad(total_content_cols: usize, padding: usize, center: bool) -> usize {
    if center {
        terminal_width().saturating_sub(total_content_cols) / 2
    } else {
        padding
    }
}

pub fn build_color_palette(
    symbol: &str,
    block: bool,
    custom_palette: Option<&[(u8, u8, u8)]>,
) -> Vec<String> {
    if let Some(colors) = custom_palette.filter(|colors| !colors.is_empty()) {
        let sym = if symbol.is_empty() { "●" } else { symbol };
        // The ANSI palette uses at most two rows. Longer custom lists are
        // distributed evenly instead of being silently dropped.
        let chunks: Vec<&[(u8, u8, u8)]> = if colors.len() <= 8 {
            vec![colors]
        } else if colors.len() <= 16 {
            vec![&colors[..8], &colors[8..]]
        } else {
            let per_row = colors.len().div_ceil(2);
            vec![&colors[..per_row], &colors[per_row..]]
        };
        let mut lines = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let mut row = String::new();
            for &(r, g, b) in chunk {
                if block {
                    row.push_str(&format!("\x1b[48;2;{r};{g};{b}m   "));
                } else {
                    row.push_str(&format!("\x1b[38;2;{r};{g};{b}m{sym} "));
                }
            }
            row.push_str(RESET);
            lines.push(row);
        }
        return lines;
    }

    let mut lines = Vec::with_capacity(2);
    if block {
        let mut r1 = String::new();
        let mut r2 = String::new();
        for i in 40..=47 {
            r1.push_str(&format!("\x1b[{}m   ", i));
        }
        r1.push_str(RESET);
        for i in 100..=107 {
            r2.push_str(&format!("\x1b[{}m   ", i));
        }
        r2.push_str(RESET);
        lines.push(r1);
        lines.push(r2);
    } else {
        let sym = if symbol.is_empty() { "●" } else { symbol };
        let mut r1 = String::new();
        let mut r2 = String::new();
        for i in 30..=37 {
            r1.push_str(&format!("\x1b[{}m{} ", i, sym));
        }
        r1.push_str(RESET);
        for i in 90..=97 {
            r2.push_str(&format!("\x1b[{}m{} ", i, sym));
        }
        r2.push_str(RESET);
        lines.push(r1);
        lines.push(r2);
    }
    lines
}

#[derive(Clone, Copy)]
pub struct RenderOptions<'a> {
    pub header: Option<&'a str>,
    pub header_color: Option<&'a str>,
    pub separator: &'a str,
    pub padding: usize,
    pub center: bool,
    pub border: bool,
    pub border_style: Option<&'a str>,
    pub border_color: Option<&'a str>,
    pub border_chars: Option<&'a rustfetch_core::BorderCharsConfig>,
    pub border_title: Option<&'a str>,
    pub box_padding: usize,
    pub palette_lines: Option<&'a [String]>,
    pub palette_position: &'a str,
    pub icon_only: bool,
    pub key_type: Option<&'a str>,
    pub border_image: Option<&'a [String]>,
    pub border_image_position: &'a str,
    pub border_images: &'a [rustfetch_core::BorderImageConfig],
    pub logo_position: &'a str,
}

/// One token of an ANSI string: an escape sequence kept verbatim or a visible char.
enum AnsiToken {
    Escape(String),
    Char(char),
}

/// Length in chars of the escape sequence starting at start (an ESC).
///
/// Mirrors crate::text's tokenizer; crate::text stays the single source of truth
/// for stripping/measuring, this copy only splits sequences verbatim so
/// truncate_ansi can preserve them.
fn escape_seq_len(chars: &[char], start: usize) -> usize {
    let n = chars.len();
    if start + 1 >= n {
        return 1;
    }
    match chars[start + 1] {
        '[' => {
            let mut j = start + 2;
            while j < n {
                let c = chars[j];
                if ('\x40'..='\x7e').contains(&c) {
                    return j - start + 1;
                }
                if c == '\x1b' {
                    return j - start;
                }
                j += 1;
            }
            n - start
        }
        ']' => {
            let mut j = start + 2;
            while j < n {
                let c = chars[j];
                if c == '\x07' {
                    return j - start + 1;
                }
                if c == '\x1b' {
                    if j + 1 < n && chars[j + 1] == '\\' {
                        return j - start + 2;
                    }
                    return j - start;
                }
                j += 1;
            }
            n - start
        }
        'P' | '^' | '_' | 'X' => {
            let mut j = start + 2;
            while j + 1 < n {
                if chars[j] == '\x1b' && chars[j + 1] == '\\' {
                    return j - start + 2;
                }
                j += 1;
            }
            n - start
        }
        _ => {
            let mut j = start + 1;
            while j < n && ('\x20'..='\x2f').contains(&chars[j]) {
                j += 1;
            }
            if j < n && ('\x30'..='\x7e').contains(&chars[j]) {
                j += 1;
            }
            j - start
        }
    }
}

/// Splits input into escape sequences and visible characters.
fn ansi_tokens(input: &str) -> Vec<AnsiToken> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' {
            let len = escape_seq_len(&chars, i).max(1);
            let end = (i + len).min(chars.len());
            tokens.push(AnsiToken::Escape(chars[i..end].iter().collect()));
            i = end;
        } else {
            tokens.push(AnsiToken::Char(chars[i]));
            i += 1;
        }
    }
    tokens
}

fn is_regional_indicator(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}

fn is_emoji_modifier(c: char) -> bool {
    ('\u{1F3FB}'..='\u{1F3FF}').contains(&c)
}

fn is_variation_selector(c: char) -> bool {
    c == '\u{FE0E}' || c == '\u{FE0F}'
}

fn is_tag_character(c: char) -> bool {
    ('\u{E0020}'..='\u{E007F}').contains(&c)
}

fn is_zero_width_char(c: char) -> bool {
    UnicodeWidthChar::width(c).is_some_and(|width| width == 0)
}

/// True when cur continues the grapheme cluster ending at prev.
fn continues_cluster(prev: char, cur: char) -> bool {
    cur == '\u{200D}'
        || prev == '\u{200D}'
        || is_zero_width_char(cur)
        || is_variation_selector(cur)
        || is_emoji_modifier(cur)
        || is_tag_character(cur)
}

/// Flags marking grapheme cluster starts inside chars (starts[i] starts one),
/// so a cut never lands in the middle of an emoji/ZWJ/combining sequence.
fn cluster_start_flags(chars: &[char]) -> Vec<bool> {
    let mut starts = vec![true; chars.len() + 1];
    if chars.is_empty() {
        return starts;
    }
    let mut regional_run = usize::from(is_regional_indicator(chars[0]));
    for i in 1..chars.len() {
        let prev = chars[i - 1];
        let cur = chars[i];
        let mut starts_new = !continues_cluster(prev, cur);
        if is_regional_indicator(prev) && is_regional_indicator(cur) {
            // Flags are pairs of regional indicators.
            starts_new = regional_run % 2 == 0;
        }
        starts[i] = starts_new;
        regional_run = if is_regional_indicator(cur) {
            regional_run + 1
        } else {
            0
        };
    }
    starts[0] = true;
    starts[chars.len()] = true;
    starts
}

/// Truncates s to at most max_width visible columns, keeping ANSI escape
/// sequences and appending an ellipsis. The visible width of the result never
/// exceeds max_width (measured with grapheme/emoji aware UnicodeWidthStr).
pub fn truncate_ansi(s: &str, max_width: usize) -> String {
    if crate::text::visible_width(s) <= max_width {
        return s.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }
    let target_width = max_width - 1;
    let tokens = ansi_tokens(s);
    let mut plain: Vec<char> = Vec::new();
    let mut token_index: Vec<usize> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if let AnsiToken::Char(c) = token {
            plain.push(*c);
            token_index.push(index);
        }
    }
    let mut plain_string = String::with_capacity(plain.len());
    let mut byte_ends: Vec<usize> = Vec::with_capacity(plain.len() + 1);
    byte_ends.push(0);
    for c in &plain {
        plain_string.push(*c);
        byte_ends.push(plain_string.len());
    }
    let starts = cluster_start_flags(&plain);
    let mut best_end = 0usize;
    let mut best_width = 0usize;
    for end in 1..=plain.len() {
        if !starts[end] {
            continue;
        }
        let width = UnicodeWidthStr::width(&plain_string[..byte_ends[end]]);
        if width > target_width {
            break;
        }
        best_end = end;
        best_width = width;
    }
    let last_token = if best_end == 0 {
        0
    } else {
        token_index[best_end - 1] + 1
    };
    let mut out = String::with_capacity(s.len());
    for token in &tokens[..last_token] {
        match token {
            AnsiToken::Escape(escape) => out.push_str(escape),
            AnsiToken::Char(c) => out.push(*c),
        }
    }
    if best_width < target_width {
        out.push_str(&" ".repeat(target_width - best_width));
    }
    out.push('…');
    if tokens
        .iter()
        .any(|token| matches!(token, AnsiToken::Escape(_)))
    {
        out.push_str(RESET);
    }
    if s.contains("\x1b]8;") {
        out.push_str("\x1b]8;;\x07");
    }
    out
}

pub fn wrap_in_box_bounded(
    lines: &[String],
    style: Option<&str>,
    color: Option<&str>,
    custom_chars: Option<&rustfetch_core::BorderCharsConfig>,
    title: Option<&str>,
    box_padding: usize,
    max_box_width: Option<usize>,
) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }

    let default_chars = rustfetch_core::BorderCharsConfig::from_style(style.unwrap_or("rounded"));
    let chars = normalize_border_chars(custom_chars, &default_chars);
    let chars = &chars;

    let border_color_code = color.map(color_code).unwrap_or_default();
    let reset_border = if color.is_some() { RESET } else { "" };

    let content_lines: Vec<&String> = lines
        .iter()
        .filter(|l| !l.starts_with("---DIVIDER"))
        .collect();

    let mut max_w = content_lines
        .iter()
        .map(|l| strip_ansi_width(l))
        .max()
        .unwrap_or(0);

    if let Some(t) = title.filter(|t| !t.is_empty()) {
        max_w = max_w.max(strip_ansi_width(t) + 4);
    }
    for line in lines {
        if line.starts_with("---DIVIDER:") {
            let section_title = line
                .strip_prefix("---DIVIDER:")
                .unwrap_or("")
                .trim_end_matches("---");
            let st_w = strip_ansi_width(section_title);
            max_w = max_w.max(st_w + 4);
        }
    }

    let mut pad_len = box_padding.clamp(1, MAX_BOX_PADDING);

    if let Some(mbw) = max_box_width {
        if mbw < 15 {
            pad_len = 0;
        } else if mbw < 35 && pad_len > 1 {
            pad_len = 1;
        }
        let border_overhead = 2 + (pad_len * 2);
        let avail_content_w = mbw.saturating_sub(border_overhead);
        if max_w > avail_content_w {
            max_w = avail_content_w;
        }
    }

    let pad_str = " ".repeat(pad_len);
    let inner_w = max_w + (pad_len * 2);

    let mut boxed = Vec::with_capacity(lines.len() + 2);

    // Top border line
    if let Some(t) = title.filter(|t| !t.is_empty()) {
        let title_w = strip_ansi_width(t);
        let top_rep = chars.top.as_str();
        if inner_w > title_w + 4 {
            let remain = inner_w - (title_w + 4);
            boxed.push(format!(
                "{border_color_code}{tl}{top}{top} {BOLD}{t}{RESET}{border_color_code} {rem}{tr}{reset_border}",
                tl = chars.top_left,
                top = top_rep,
                rem = top_rep.repeat(remain),
                tr = chars.top_right
            ));
        } else if inner_w > 4 {
            let avail_t = inner_w.saturating_sub(4);
            let trunc_t = truncate_ansi(t, avail_t);
            let trunc_w = strip_ansi_width(&trunc_t);
            let remain = inner_w.saturating_sub(trunc_w + 4);
            boxed.push(format!(
                "{border_color_code}{tl}{top}{top} {BOLD}{trunc_t}{RESET}{border_color_code} {rem}{tr}{reset_border}",
                tl = chars.top_left,
                top = top_rep,
                rem = top_rep.repeat(remain),
                tr = chars.top_right
            ));
        } else {
            boxed.push(format!(
                "{border_color_code}{tl}{rep}{tr}{reset_border}",
                tl = chars.top_left,
                rep = chars.top.repeat(inner_w),
                tr = chars.top_right
            ));
        }
    } else {
        boxed.push(format!(
            "{border_color_code}{tl}{rep}{tr}{reset_border}",
            tl = chars.top_left,
            rep = chars.top.repeat(inner_w),
            tr = chars.top_right
        ));
    }

    // Inner lines
    for line in lines {
        if line.starts_with("---DIVIDER:") {
            let section_title = line
                .strip_prefix("---DIVIDER:")
                .unwrap_or("")
                .trim_end_matches("---");
            let st_w = strip_ansi_width(section_title);
            let div_rep = chars.divider.as_str();
            if inner_w > st_w + 2 {
                let remain = inner_w - (st_w + 2);
                let left_side = remain / 2;
                let right_side = remain - left_side;
                boxed.push(format!(
                    "{border_color_code}{dl}{left_r} {BOLD}{section_title}{RESET}{border_color_code} {right_r}{dr}{reset_border}",
                    dl = chars.divider_left,
                    left_r = div_rep.repeat(left_side),
                    right_r = div_rep.repeat(right_side),
                    dr = chars.divider_right
                ));
            } else if inner_w > 4 {
                let trunc_st = truncate_ansi(section_title, inner_w.saturating_sub(2));
                let trunc_w = strip_ansi_width(&trunc_st);
                let remain = inner_w.saturating_sub(trunc_w + 2);
                let left_side = remain / 2;
                let right_side = remain - left_side;
                boxed.push(format!(
                    "{border_color_code}{dl}{left_r} {BOLD}{trunc_st}{RESET}{border_color_code} {right_r}{dr}{reset_border}",
                    dl = chars.divider_left,
                    left_r = div_rep.repeat(left_side),
                    right_r = div_rep.repeat(right_side),
                    dr = chars.divider_right
                ));
            } else {
                boxed.push(format!(
                    "{border_color_code}{dl}{rep}{dr}{reset_border}",
                    dl = chars.divider_left,
                    rep = chars.divider.repeat(inner_w),
                    dr = chars.divider_right
                ));
            }
        } else if line == "---DIVIDER---" {
            boxed.push(format!(
                "{border_color_code}{dl}{rep}{dr}{reset_border}",
                dl = chars.divider_left,
                rep = chars.divider.repeat(inner_w),
                dr = chars.divider_right
            ));
        } else {
            let processed_line = if strip_ansi_width(line) > max_w {
                truncate_ansi(line, max_w)
            } else {
                line.clone()
            };
            let visual_w = strip_ansi_width(&processed_line);
            let line_pad = " ".repeat(max_w.saturating_sub(visual_w));
            boxed.push(format!(
                "{border_color_code}{l}{reset_border}{pad_str}{processed_line}{line_pad}{pad_str}{border_color_code}{r}{reset_border}",
                l = chars.left,
                r = chars.right
            ));
        }
    }

    // Bottom border line
    boxed.push(format!(
        "{border_color_code}{bl}{rep}{br}{reset_border}",
        bl = chars.bottom_left,
        rep = chars.bottom.repeat(inner_w),
        br = chars.bottom_right
    ));

    boxed
}

pub fn build_right_lines(info_lines: &[InfoLine], opts: &RenderOptions) -> Vec<String> {
    build_right_lines_bounded(info_lines, opts, None)
}

pub fn build_right_lines_bounded(
    info_lines: &[InfoLine],
    opts: &RenderOptions,
    max_card_width: Option<usize>,
) -> Vec<String> {
    let mut right_lines: Vec<String> = Vec::new();

    if opts.palette_position == "top" || opts.palette_position == "both" {
        if let Some(pal) = opts.palette_lines {
            right_lines.extend(pal.iter().cloned());
            right_lines.push(String::new());
        }
    }

    if let Some(h) = opts.header {
        let col = opts.header_color.unwrap_or("cyan");
        let h_str = if let Some(mcw) = max_card_width {
            let max_h = if opts.border {
                mcw.saturating_sub(opts.box_padding.max(1) * 2 + 2)
            } else {
                mcw
            };
            if strip_ansi_width(h) > max_h && max_h > 0 {
                truncate_ansi(h, max_h)
            } else {
                h.to_string()
            }
        } else {
            h.to_string()
        };
        let h_w = strip_ansi_width(&h_str);
        right_lines.push(format!("{BOLD}{}{h_str}{RESET}", color_code(col)));
        right_lines.push(format!("{DIM}{}{RESET}", "─".repeat(h_w)));
    }

    right_lines.extend(format_info_lines(
        info_lines,
        opts.separator,
        opts.icon_only,
        opts.key_type,
    ));

    if opts.palette_position == "bottom" || opts.palette_position == "both" {
        if let Some(pal) = opts.palette_lines {
            right_lines.push(String::new());
            right_lines.extend(pal.iter().cloned());
        }
    }

    if opts.border {
        right_lines = wrap_in_box_bounded(
            &right_lines,
            opts.border_style,
            opts.border_color,
            opts.border_chars,
            opts.border_title,
            opts.box_padding,
            max_card_width,
        );
    } else {
        let mut content_w = right_lines
            .iter()
            .filter(|l| !l.starts_with("---DIVIDER"))
            .map(|l| strip_ansi_width(l))
            .max()
            .unwrap_or(24)
            .max(20);

        if let Some(mcw) = max_card_width {
            content_w = content_w.min(mcw);
        }

        right_lines = right_lines
            .into_iter()
            .map(|line| {
                if line.starts_with("---DIVIDER:") {
                    let section_title = line
                        .strip_prefix("---DIVIDER:")
                        .unwrap_or("")
                        .trim_end_matches("---");
                    let st_w = strip_ansi_width(section_title);
                    if st_w > 0 && content_w > st_w + 4 {
                        let remain = content_w.saturating_sub(st_w + 4);
                        format!(
                            "{DIM}── {RESET}{BOLD}{section_title}{RESET} {DIM}{}{RESET}",
                            "─".repeat(remain)
                        )
                    } else if st_w > 0 {
                        format!("{BOLD}{section_title}{RESET}")
                    } else {
                        String::new()
                    }
                } else if line == "---DIVIDER---" {
                    String::new()
                } else if let Some(mcw) = max_card_width {
                    if strip_ansi_width(&line) > mcw {
                        truncate_ansi(&line, mcw)
                    } else {
                        line
                    }
                } else {
                    line
                }
            })
            .collect();
    }

    if !opts.border_images.is_empty() {
        // Compose non-background border images first (frames, left, right, top, bottom)
        for img in opts.border_images {
            if !img.is_background() {
                let expanded = crate::tilde_expand(&img.path);
                let target_w = img.width.unwrap_or(24);
                if let Some(rasterized) = rustfetch_render::render_halfblock_ex(
                    expanded.as_ref(),
                    target_w,
                    img.height,
                    img.preserve_aspect_ratio,
                ) {
                    right_lines =
                        compose_with_border_image(&right_lines, &rasterized, &img.position);
                }
            }
        }

        // For background images, if not using Kitty (e.g. halfblock/fallback), apply TrueColor ANSI background
        for img in opts.border_images {
            if img.is_background() && !should_use_kitty(img) {
                let expanded = crate::tilde_expand(&img.path);
                let card_w = right_lines
                    .iter()
                    .map(|l| strip_ansi_width(l))
                    .max()
                    .unwrap_or(0);
                let card_h = right_lines.len();
                let target_cols = img.width.unwrap_or(card_w).max(1);
                let target_rows = img.height.unwrap_or(card_h).max(1);
                if let Some(bg_applied) = rustfetch_render::apply_image_background_to_lines_ex(
                    &right_lines,
                    expanded.as_ref(),
                    target_cols,
                    target_rows,
                    img.preserve_aspect_ratio,
                    img.offset_x,
                    img.offset_y,
                ) {
                    right_lines = bg_applied;
                }
            }
        }
    } else if let Some(border_img) = opts.border_image.filter(|b| !b.is_empty()) {
        return compose_with_border_image(&right_lines, border_img, opts.border_image_position);
    }

    right_lines
}

pub fn should_use_kitty(img_config: &rustfetch_core::BorderImageConfig) -> bool {
    let proto = img_config.protocol.to_ascii_lowercase();
    proto == "kitty" || (proto == "auto" && rustfetch_render::supports_kitty())
}

pub fn emit_kitty_background(
    img_config: &rustfetch_core::BorderImageConfig,
    col_offset: usize,
    card_w: usize,
    card_h: usize,
) -> Option<usize> {
    let expanded = crate::tilde_expand(&img_config.path).to_string();
    let z_index = img_config.effective_z_index();

    let opts = rustfetch_render::BackgroundOptions {
        col_offset,
        card_w,
        card_h,
        target_cols: img_config.width,
        target_rows: img_config.height,
        offset_x: img_config.offset_x,
        offset_y: img_config.offset_y,
        z_index,
        preserve_aspect_ratio: img_config.preserve_aspect_ratio,
    };

    rustfetch_render::render_background_ex(&expanded, &opts)
}

pub fn emit_all_kitty_backgrounds(
    border_images: &[rustfetch_core::BorderImageConfig],
    col_offset: usize,
    card_w: usize,
    card_h: usize,
) -> usize {
    let mut max_lines = card_h;
    for img in border_images {
        if img.is_background() && should_use_kitty(img) {
            if let Some(bottom_row) = emit_kitty_background(img, col_offset, card_w, card_h) {
                max_lines = max_lines.max(bottom_row);
            }
        }
    }
    max_lines
}

/// Places a rasterized decoration around the telemetry card. `left` preserves
/// the original layout; `right`, `top`, `bottom`, and `frame` support the
/// common ricing compositions without requiring duplicate image files.
pub fn compose_with_border_image(
    content: &[String],
    border_image: &[String],
    position: &str,
) -> Vec<String> {
    match position.to_ascii_lowercase().as_str() {
        "top" => [border_image, content].concat(),
        "bottom" => [content, border_image].concat(),
        "right" => compose_image_columns(content, border_image, false, true),
        "frame" | "both" => compose_image_columns(content, border_image, true, true),
        "background" | "behind" | "bg" => content.to_vec(),
        _ => compose_image_columns(content, border_image, true, false),
    }
}

fn compose_image_columns(
    content: &[String],
    border_image: &[String],
    left: bool,
    right: bool,
) -> Vec<String> {
    let image_width = border_image
        .iter()
        .map(|line| strip_ansi_width(line))
        .max()
        .unwrap_or(0);
    let content_width = content
        .iter()
        .map(|line| strip_ansi_width(line))
        .max()
        .unwrap_or(0);
    let total_rows = border_image.len().max(content.len());
    let mut combined = Vec::with_capacity(total_rows);

    for index in 0..total_rows {
        let image = border_image.get(index).map(String::as_str).unwrap_or("");
        let content_line = content.get(index).map(String::as_str).unwrap_or("");
        let image_pad = " ".repeat(image_width.saturating_sub(strip_ansi_width(image)));
        let content_pad = " ".repeat(content_width.saturating_sub(strip_ansi_width(content_line)));
        let row = match (left, right) {
            (true, true) => format!("{image}{image_pad}   {content_line}{content_pad}   {image}"),
            (true, false) => format!("{image}{image_pad}   {content_line}"),
            _ => format!("{content_line}{content_pad}   {image}"),
        };
        combined.push(row);
    }
    combined
}

fn effective_padding(padding: usize, term_w: usize) -> usize {
    padding.min(term_w / 4)
}

/// Emits a one-off diagnostic for a degraded rendering path.
fn warn_once(message: &str) {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!("rustfetch: {message}");
    }
}

/// Clamps the card width. A card that cannot fit at all is drawn without its
/// border (plus a single warning) instead of silently dropping the content.
fn card_max_width(opts: &mut RenderOptions, avail: usize) -> Option<usize> {
    if avail == 0 {
        warn_once("terminal is too narrow for a bordered card; drawing without a border");
        opts.border = false;
        None
    } else {
        Some(avail)
    }
}

fn max_line_width(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| strip_ansi_width(line))
        .max()
        .unwrap_or(0)
}

fn block_pad(center: bool, term_w: usize, content_w: usize, pad: &str) -> String {
    if center && term_w > content_w {
        " ".repeat((term_w - content_w) / 2)
    } else {
        pad.to_string()
    }
}

fn render_line<'a>(line: &'a str, keep_colors: bool) -> std::borrow::Cow<'a, str> {
    if keep_colors {
        std::borrow::Cow::Borrowed(line)
    } else {
        std::borrow::Cow::Owned(crate::text::strip_ansi(line))
    }
}

fn truncate_to_width(line: &str, max_width: usize) -> String {
    if strip_ansi_width(line) > max_width {
        truncate_ansi(line, max_width)
    } else {
        line.to_string()
    }
}

fn write_frame(frame: &str) -> bool {
    crate::text::write_stdout(frame)
}

/// ASCII-logo ordering helper: is the logo printed before the modules?
fn ascii_logo_first(position: &str) -> bool {
    !matches!(position.trim().to_ascii_lowercase().as_str(), "bottom")
}

pub fn render(logo: Option<&LogoBlock>, info_lines: &[InfoLine], opts: &RenderOptions) -> bool {
    let term_w = terminal_width();
    let mut opts = *opts;
    opts.padding = effective_padding(opts.padding, term_w);
    let pad = " ".repeat(opts.padding);
    let position = opts.logo_position.to_ascii_lowercase();
    let logo = logo.filter(|_| position != "none" && position != "hidden" && term_w >= 35);
    let Some(logo) = logo else {
        return build_without_logo(info_lines, &pad, term_w, &opts);
    };
    let is_vertical = position == "top" || position == "bottom";
    if is_vertical {
        // A logo wider than the terminal would wrap and destroy the layout.
        if term_w < logo.width + opts.padding * 2 {
            return build_without_logo(info_lines, &pad, term_w, &opts);
        }
        return build_with_logo(logo, info_lines, &pad, term_w, &opts);
    }
    let needed_side_by_side = logo.width + LOGO_GAP + MIN_INFO_CARD_WIDTH + opts.padding * 2;
    if term_w >= needed_side_by_side {
        return build_with_logo(logo, info_lines, &pad, term_w, &opts);
    }
    if term_w >= 45 {
        let mut top_opts = opts;
        top_opts.logo_position = "top";
        return build_with_logo(logo, info_lines, &pad, term_w, &top_opts);
    }
    build_without_logo(info_lines, &pad, term_w, &opts)
}

pub fn format_info_lines(
    info_lines: &[InfoLine],
    separator: &str,
    icon_only: bool,
    key_type: Option<&str>,
) -> Vec<String> {
    let key_type_owned = key_type.map(str::to_ascii_lowercase);
    let key_type = key_type_owned.as_deref();
    let key_type_none = key_type == Some("none");
    let show_icon = key_type != Some("title") && !key_type_none;
    let show_label = !icon_only && key_type != Some("icon") && !key_type_none;

    let label_width = if show_label {
        info_lines
            .iter()
            .filter(|line| {
                !line.label.is_empty()
                    && !line.value.is_empty()
                    && !line.label.starts_with("---DIVIDER")
            })
            .map(|line| {
                let icon_w = if show_icon {
                    line.icon
                        .as_deref()
                        .map(|icon| crate::text::visible_width(icon) + 1)
                        .unwrap_or(0)
                } else {
                    0
                };
                crate::text::visible_width(&line.label) + icon_w
            })
            .max()
            .unwrap_or(0)
    } else {
        0
    };

    let sep_trimmed = separator.trim();
    let is_sep_empty = sep_trimmed.is_empty();

    info_lines
        .iter()
        .map(|line| {
            if line.label.starts_with("---DIVIDER") {
                return line.label.clone();
            }

            if line.label.is_empty() && line.icon.is_none() {
                if line.value.is_empty() {
                    return String::new();
                }
                return format!("{}{}{}", line.color, line.value, RESET);
            } else if line.value.is_empty() {
                let icon_str = if show_icon {
                    line.icon
                        .as_deref()
                        .map(|icon| format!("{icon} "))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                return format!("{BOLD}{}{icon_str}{}{RESET}", line.color, line.label);
            }

            let icon_str = if show_icon {
                line.icon
                    .as_deref()
                    .map(|icon| format!("{icon} "))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            if !show_label || line.label.is_empty() {
                let icon_display = if show_icon && line.icon.is_some() {
                    format!(
                        "{BOLD}{}{}{RESET}",
                        line.color,
                        line.icon.as_deref().unwrap_or("")
                    )
                } else {
                    String::new()
                };

                if is_sep_empty {
                    if icon_display.is_empty() {
                        line.value.clone()
                    } else {
                        format!("{icon_display}  {}", line.value)
                    }
                } else if icon_display.is_empty() {
                    line.value.clone()
                } else {
                    format!("{icon_display} {sep_trimmed} {}", line.value)
                }
            } else {
                let icon_w = if show_icon {
                    line.icon
                        .as_deref()
                        .map(|icon| crate::text::visible_width(icon) + 1)
                        .unwrap_or(0)
                } else {
                    0
                };
                let cur_label_w = crate::text::visible_width(&line.label) + icon_w;
                let label_pad = " ".repeat(label_width.saturating_sub(cur_label_w));

                if is_sep_empty {
                    format!(
                        "{BOLD}{color}{icon_str}{label}{RESET}{label_pad}  {value}",
                        color = line.color,
                        label = line.label,
                        value = line.value
                    )
                } else {
                    format!(
                        "{BOLD}{color}{icon_str}{label}{RESET}{label_pad} {sep_trimmed} {value}",
                        color = line.color,
                        label = line.label,
                        value = line.value
                    )
                }
            }
        })
        .collect()
}

/// Placement plan for an image logo that the caller already rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
enum ImageCardPlacement {
    /// Image on the left, info card to its right (logo_position "left", default).
    LeftOfImage,
    /// Image on top, info card below it (logo_position "top").
    TopOfImage,
    /// Image behind the card (logo_position "behind"); same safe layout as left.
    BehindImage,
}

/// Pure placement plan for image logos (unit tested without a terminal).
///
/// Returns the placement plus whether an unsupported image position (bottom /
/// right) had to fall back to the safe default, so exactly one warning is shown.
fn image_card_placement(position: &str) -> (ImageCardPlacement, bool) {
    match position.trim().to_ascii_lowercase().as_str() {
        "top" => (ImageCardPlacement::TopOfImage, false),
        "behind" => (ImageCardPlacement::BehindImage, false),
        "left" => (ImageCardPlacement::LeftOfImage, false),
        "bottom" | "right" => (ImageCardPlacement::LeftOfImage, true),
        _ => (ImageCardPlacement::LeftOfImage, false),
    }
}

/// Maximum usable card width for an image placement, in terminal columns.
fn image_card_max_width(
    placement: ImageCardPlacement,
    term_w: usize,
    image_cols: usize,
    h_pad: usize,
) -> usize {
    match placement {
        ImageCardPlacement::TopOfImage => term_w.saturating_sub(h_pad * 2 + 1),
        ImageCardPlacement::LeftOfImage | ImageCardPlacement::BehindImage => {
            term_w.saturating_sub(h_pad + image_cols + LOGO_GAP + 1)
        }
    }
}

fn frame_without_logo(
    info_lines: &[InfoLine],
    pad: &str,
    term_w: usize,
    opts: &RenderOptions,
) -> (String, usize, usize, usize) {
    let mut opts = *opts;
    let avail = term_w.saturating_sub(pad.len() * 2 + 1);
    let max_w = card_max_width(&mut opts, avail);
    let right_lines = build_right_lines_bounded(info_lines, &opts, max_w);
    let content_w = max_line_width(&right_lines);
    let h_pad = block_pad(opts.center, term_w, content_w, pad);
    let keep_colors = colors_enabled();
    let mut frame = String::new();
    for line in &right_lines {
        frame.push_str(&format!("{h_pad}{}\n", render_line(line, keep_colors)));
    }
    (frame, h_pad.len(), content_w, right_lines.len())
}

fn build_without_logo(
    info_lines: &[InfoLine],
    pad: &str,
    term_w: usize,
    opts: &RenderOptions,
) -> bool {
    let (frame, col_offset, content_w, rows) = frame_without_logo(info_lines, pad, term_w, opts);
    let bg_rows = emit_all_kitty_backgrounds(opts.border_images, col_offset, content_w, rows);
    let mut ok = write_frame(&frame);
    if bg_rows > rows {
        ok &= write_frame(&"\n".repeat(bg_rows - rows));
    }
    ok
}

/// Horizontal ASCII layout: logo left (default) or logo right ("right").
fn frame_horizontal(
    logo_lines: &[String],
    logo_width: usize,
    right_lines: &[String],
    pad: &str,
    term_w: usize,
    opts: &RenderOptions,
    position: &str,
) -> (String, usize, usize, usize) {
    let keep_colors = colors_enabled();
    let mut right_lines: Vec<String> = right_lines.to_vec();
    if opts.center && logo_lines.len() > right_lines.len() {
        let v_offset = (logo_lines.len() - right_lines.len()) / 2;
        let mut padded: Vec<String> = vec![String::new(); v_offset];
        padded.append(&mut right_lines);
        right_lines = padded;
    }
    let max_right_w = max_line_width(&right_lines);
    let logo_h = logo_lines.len();
    let total_w = if position == "right" {
        max_right_w + LOGO_GAP + logo_width
    } else {
        logo_width + LOGO_GAP + max_right_w
    };
    let h_pad = block_pad(opts.center, term_w, total_w, pad);
    let col_offset = if position == "right" {
        h_pad.len() + max_right_w + LOGO_GAP
    } else {
        h_pad.len() + logo_width + LOGO_GAP
    };
    let mut frame = String::new();
    let logo_v_offset = if right_lines.len() > logo_h {
        (right_lines.len() - logo_h) / 2
    } else {
        0
    };
    let logo_pad = " ".repeat(logo_width);
    let gap = " ".repeat(LOGO_GAP);
    let total_rows = logo_h.max(right_lines.len());
    for i in 0..total_rows {
        let right_line = right_lines.get(i).map(String::as_str).unwrap_or("");
        let right_clean = render_line(right_line, keep_colors);
        let logo_idx =
            (i >= logo_v_offset && i < logo_v_offset + logo_h).then(|| i - logo_v_offset);
        if position == "right" {
            let right_trail =
                " ".repeat(max_right_w.saturating_sub(strip_ansi_width(&right_clean)));
            match logo_idx.and_then(|idx| logo_lines.get(idx)) {
                Some(logo_line) => frame.push_str(&format!(
                    "{h_pad}{right_clean}{right_trail}{gap}{logo_line}\n"
                )),
                None => frame.push_str(&format!("{h_pad}{right_clean}\n")),
            }
        } else {
            match logo_idx.and_then(|idx| logo_lines.get(idx)) {
                Some(logo_line) => {
                    let trail = " ".repeat(logo_width.saturating_sub(strip_ansi_width(logo_line)));
                    frame.push_str(&format!("{h_pad}{logo_line}{trail}{gap}{right_clean}\n"));
                }
                None => frame.push_str(&format!("{h_pad}{logo_pad}{gap}{right_clean}\n")),
            }
        }
    }
    (frame, col_offset, max_right_w, right_lines.len())
}

fn frame_vertical_logo(
    logo_lines: &[String],
    logo_width: usize,
    pad: &str,
    term_w: usize,
    opts: &RenderOptions,
) -> String {
    let logo_pad = block_pad(opts.center, term_w, logo_width, pad);
    let keep_colors = colors_enabled();
    let mut frame = String::new();
    for line in logo_lines {
        frame.push_str(&format!("{logo_pad}{}\n", render_line(line, keep_colors)));
    }
    frame
}

fn frame_vertical_modules(
    right_lines: &[String],
    pad: &str,
    term_w: usize,
    opts: &RenderOptions,
) -> (String, usize, usize, usize) {
    let content_w = max_line_width(right_lines);
    let modules_pad = block_pad(opts.center, term_w, content_w, pad);
    let keep_colors = colors_enabled();
    let mut frame = String::new();
    for line in right_lines {
        frame.push_str(&format!(
            "{modules_pad}{}\n",
            render_line(line, keep_colors)
        ));
    }
    (frame, modules_pad.len(), content_w, right_lines.len())
}

fn build_with_logo(
    logo: &LogoBlock,
    info_lines: &[InfoLine],
    pad: &str,
    term_w: usize,
    opts: &RenderOptions,
) -> bool {
    let mut opts = *opts;
    let position = opts.logo_position.to_ascii_lowercase();
    let is_vertical = position == "top" || position == "bottom";
    let avail = if is_vertical {
        term_w.saturating_sub(pad.len() * 2 + 1)
    } else {
        term_w.saturating_sub(pad.len() + logo.width + LOGO_GAP + 1)
    };
    let max_w = card_max_width(&mut opts, avail);
    let right_lines = build_right_lines_bounded(info_lines, &opts, max_w);
    let keep_colors = colors_enabled();
    let logo_lines: Vec<String> = logo
        .lines
        .iter()
        .map(|line| {
            let clean = render_line(line, keep_colors).into_owned();
            truncate_to_width(&clean, term_w)
        })
        .collect();
    let logo_width = max_line_width(&logo_lines);

    if is_vertical {
        let logo_frame = frame_vertical_logo(&logo_lines, logo_width, pad, term_w, &opts);
        let (modules_frame, modules_pad, modules_w, rows) =
            frame_vertical_modules(&right_lines, pad, term_w, &opts);
        let mut ok = true;
        if ascii_logo_first(&position) {
            ok &= write_frame(&logo_frame);
            let bg_rows =
                emit_all_kitty_backgrounds(opts.border_images, modules_pad, modules_w, rows);
            ok &= write_frame(&modules_frame);
            if bg_rows > rows {
                ok &= write_frame(&"\n".repeat(bg_rows - rows));
            }
        } else {
            let bg_rows =
                emit_all_kitty_backgrounds(opts.border_images, modules_pad, modules_w, rows);
            ok &= write_frame(&modules_frame);
            if bg_rows > rows {
                ok &= write_frame(&"\n".repeat(bg_rows - rows));
            }
            ok &= write_frame("\n");
            ok &= write_frame(&logo_frame);
        }
        return ok;
    }

    let (frame, col_offset, card_w, rows) = frame_horizontal(
        &logo_lines,
        logo_width,
        &right_lines,
        pad,
        term_w,
        &opts,
        &position,
    );
    let bg_rows = emit_all_kitty_backgrounds(opts.border_images, col_offset, card_w, rows);
    let mut ok = write_frame(&frame);
    if bg_rows > rows {
        ok &= write_frame(&"\n".repeat(bg_rows - rows));
    }
    ok
}

pub fn render_with_image(
    image_lines: usize,
    image_cols: usize,
    h_pad: usize,
    info_lines: &[InfoLine],
    opts: &RenderOptions,
) -> bool {
    let term_w = terminal_width();
    let mut opts = *opts;
    opts.padding = effective_padding(opts.padding, term_w);
    // Image logos only support left/top/behind: unsupported requests (bottom /
    // right) are reported once and fall back to the safe left-hand layout
    // instead of silently producing a broken placement.
    let (placement, unsupported) = image_card_placement(opts.logo_position);
    if unsupported {
        warn_once(
            "logo position bottom/right is only supported for ASCII logos; using left for image logo",
        );
    }
    let avail = image_card_max_width(placement, term_w, image_cols, h_pad);
    let max_w = card_max_width(&mut opts, avail);
    let right_lines = build_right_lines_bounded(info_lines, &opts, max_w);
    let keep_colors = colors_enabled();
    let content_w = max_line_width(&right_lines);
    let rows = right_lines.len();
    let right_indent = h_pad + image_cols + LOGO_GAP;
    let mut ok = true;
    match placement {
        ImageCardPlacement::TopOfImage => {
            let bg_rows = emit_all_kitty_backgrounds(opts.border_images, h_pad, content_w, rows);
            let mut frame = String::from("\n");
            for line in &right_lines {
                frame.push_str(&format!(
                    "{}{}\n",
                    " ".repeat(h_pad),
                    render_line(line, keep_colors)
                ));
            }
            ok &= write_frame(&frame);
            if bg_rows > rows {
                ok &= write_frame(&"\n".repeat(bg_rows - rows));
            }
        }
        ImageCardPlacement::LeftOfImage | ImageCardPlacement::BehindImage => {
            let mut prefix = String::new();
            if image_lines > 1 {
                prefix.push_str(&format!("\r\x1b[{}A", image_lines - 1));
            } else {
                prefix.push('\r');
            }
            ok &= write_frame(&prefix);
            let bg_rows =
                emit_all_kitty_backgrounds(opts.border_images, right_indent, content_w, rows);
            let mut frame = String::new();
            for line in &right_lines {
                frame.push_str(&format!(
                    "\r\x1b[{}C{}\x1b[K\n",
                    right_indent,
                    render_line(line, keep_colors)
                ));
            }
            ok &= write_frame(&frame);
            if bg_rows > rows {
                ok &= write_frame(&"\n".repeat(bg_rows - rows));
            }
        }
    }
    ok
}

/// Removes ANSI escape sequences (single source of truth: crate::text).
pub fn strip_ansi(s: &str) -> String {
    crate::text::strip_ansi(s)
}

/// Visible column width after stripping ANSI, grapheme/emoji aware.
pub fn strip_ansi_width(s: &str) -> usize {
    crate::text::visible_width(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_line(label: &str, value: &str) -> InfoLine {
        InfoLine {
            label: label.to_string(),
            value: value.to_string(),
            icon: None,
            color: String::new(),
        }
    }

    fn opts(border: bool) -> RenderOptions<'static> {
        RenderOptions {
            header: None,
            header_color: None,
            separator: ":",
            padding: 1,
            center: false,
            border,
            border_style: Some("rounded"),
            border_color: None,
            border_chars: None,
            border_title: None,
            box_padding: 1,
            palette_lines: None,
            palette_position: "none",
            icon_only: false,
            key_type: None,
            border_image: None,
            border_image_position: "left",
            border_images: &[],
            logo_position: "left",
        }
    }

    #[test]
    fn test_color_code() {
        assert_eq!(color_code("red"), "\x1b[38;2;239;83;80m");
        assert_eq!(color_code("#ff007f"), "\x1b[38;2;255;0;127m");
        assert!(color_code("\x1b[31m").starts_with('\x1b'));
    }

    #[test]
    fn decide_colors_never_colours_non_tty_output() {
        assert!(!decide_colors(false, None, Some("xterm-256color"), false));
        assert!(decide_colors(false, None, Some("xterm-256color"), true));
        assert!(decide_colors(true, Some("1"), Some("dumb"), false));
        assert!(!decide_colors(false, Some("1"), Some("xterm"), true));
        assert!(decide_colors(false, Some(""), Some("xterm"), true));
        assert!(!decide_colors(false, None, Some("dumb"), true));
    }

    #[test]
    fn sanitize_dimensions_clamps_and_defaults() {
        assert_eq!(
            sanitize_dimensions(Some(120), Some(40)),
            (120, 40),
            "valid dimensions pass through"
        );
        assert_eq!(
            sanitize_dimensions(Some(0), Some(10_000)),
            (DEFAULT_TERM_WIDTH, MAX_TERM_DIMENSION)
        );
        assert_eq!(
            sanitize_dimensions(None, None),
            (DEFAULT_TERM_WIDTH, DEFAULT_TERM_HEIGHT)
        );
    }

    #[test]
    fn test_terminal_width() {
        assert!(terminal_width() > 0);
        let (width, height) = terminal_size();
        assert!((1..=MAX_TERM_DIMENSION).contains(&width));
        assert!((1..=MAX_TERM_DIMENSION).contains(&height));
    }

    #[test]
    fn test_strip_ansi_width() {
        assert_eq!(strip_ansi_width(""), 0);
        assert_eq!(strip_ansi_width("hello world"), 11);
        assert_eq!(strip_ansi_width("\x1b[31mhello\x1b[0m"), 5);
        assert_eq!(strip_ansi_width("\x1b7hello\x1b8"), 5);
        assert_eq!(strip_ansi_width("\x1b_Ga=T,f=100;ABCDEF\x1b\\world"), 5);
        assert_eq!(
            strip_ansi_width("\x1b]8;;https://example.com\x07link\x1b]8;;\x07"),
            4
        );
        assert_eq!(strip_ansi_width("\x1b]0;terminal title\x1b\\window"), 6);
        assert_eq!(strip_ansi_width("🦀 rust"), 7);
        assert_eq!(strip_ansi_width("\x1b[32m你好世界\x1b[0m"), 8);
        assert_eq!(strip_ansi_width("\x1b["), 0);
        // Two-byte escape: ESC ( B is a charset designation, "B" is not visible.
        assert_eq!(strip_ansi_width("\x1b(Bhi"), 2);
        assert_eq!(strip_ansi_width("\x1b)0hi"), 2);
    }

    #[test]
    fn visible_width_handles_emoji_grapheme_clusters() {
        assert_eq!(strip_ansi_width("👨‍👩‍👧"), 2);
        assert_eq!(strip_ansi_width("❤️"), 2);
        assert_eq!(strip_ansi_width("👍🏽"), 2);
        assert_eq!(strip_ansi_width("1️⃣"), 2);
        assert_eq!(strip_ansi_width("🇹🇷"), 2);
    }

    #[test]
    fn test_truncate_ansi() {
        assert_eq!(truncate_ansi("hello world", 15), "hello world");
        assert_eq!(truncate_ansi("hello world", 5), "hell…");
        assert_eq!(strip_ansi_width(&truncate_ansi("hello world", 5)), 5);
        assert_eq!(truncate_ansi("hello", 0), "");
        assert_eq!(truncate_ansi("hello", 1), "…");

        let colored = "\x1b[31mhello world\x1b[0m";
        let truncated = truncate_ansi(colored, 6);
        assert!(truncated.contains('…'));
        assert!(truncated.contains("\x1b[31m"));
        assert!(truncated.ends_with("\x1b[0m"));
        assert_eq!(strip_ansi_width(&truncated), 6);
    }

    #[test]
    fn truncate_ansi_never_exceeds_max_width() {
        let samples = [
            "AAAA❤️BBBB",
            "❤️❤️❤️❤️",
            "👨‍👩‍👧👨‍👩‍👧👨‍👩‍👧",
            "a\tb\tc",
            "\x1b(Bhello",
            "\x1b]0;title\x07hello",
            "\x1b]8;;https://example.com\x07link\x1b]8;;\x07",
            "🇹🇷🇹🇷🇹🇷",
            "👍🏽👍🏽",
            "e\u{301}eeee",
            "plain ascii text",
        ];
        for sample in samples {
            for max_width in 0..=12 {
                let out = truncate_ansi(sample, max_width);
                let width = strip_ansi_width(&out);
                assert!(
                    width <= max_width,
                    "truncate_ansi({sample:?}, {max_width}) -> {out:?} has width {width}"
                );
                assert!(
                    !out.contains("\r\n"),
                    "truncate output must not contain CRLF"
                );
            }
        }
    }

    #[test]
    fn truncate_ansi_keeps_whole_grapheme_clusters() {
        let out = truncate_ansi("AAAA❤️BBBB", 9);
        assert!(
            out.contains("❤️"),
            "emoji cluster must stay intact: {out:?}"
        );
        assert_eq!(strip_ansi_width(&out), 9);
        let family = truncate_ansi("x👨‍👩‍👧y", 3);
        assert_eq!(strip_ansi_width(&family), 3);
        assert!(!family.contains('\u{200D}') || family.contains("👨‍👩‍👧"));
    }

    #[test]
    fn test_build_color_palette() {
        let dots = build_color_palette("●", false, None);
        assert_eq!(dots.len(), 2);
        assert!(dots[0].contains('●'));
        assert_eq!(strip_ansi_width(&dots[0]), 16);

        let blocks = build_color_palette("", true, None);
        assert_eq!(blocks.len(), 2);
        assert_eq!(strip_ansi_width(&blocks[0]), 24);

        let custom = [(255u8, 100u8, 50u8), (100, 200, 255)];
        let custom_dots = build_color_palette("●", false, Some(&custom));
        assert_eq!(custom_dots.len(), 1);
        assert!(custom_dots[0].contains("\x1b[38;2;255;100;50m"));
    }

    #[test]
    fn palette_distributes_long_custom_lists() {
        let many: Vec<(u8, u8, u8)> = (0..20u8).map(|i| (i * 10, 0, 0)).collect();
        let lines = build_color_palette("●", false, Some(&many));
        assert_eq!(lines.len(), 2);
        let symbols: usize = lines.iter().map(|line| line.matches('●').count()).sum();
        assert_eq!(symbols, 20, "all custom colours must be shown");
        assert!(!lines.iter().any(|line| strip_ansi_width(line) == 0));
    }

    #[test]
    fn test_format_info_lines_modes() {
        let lines = vec![
            info_line("OS", "Linux"),
            info_line("CPU", "8 cores"),
            info_line("", ""),
            info_line("", "Raw Text"),
        ];
        let formatted = format_info_lines(&lines, ":", false, None);
        assert_eq!(formatted.len(), 4);
        assert!(formatted[0].contains("OS"));
        assert!(formatted[2].is_empty());
        assert!(formatted[3].contains("Raw Text"));

        let icon_only = format_info_lines(&lines, "•", true, None);
        assert!(!icon_only[0].contains("OS"));

        // key_type = "none" prints only the value: no label, separator or icon.
        let values_only = format_info_lines(&lines, ":", false, Some("none"));
        assert_eq!(values_only[0], "Linux");
        assert_eq!(values_only[1], "8 cores");
        assert!(!values_only[0].contains(':'));

        let titles = format_info_lines(&lines, ":", false, Some("title"));
        assert!(titles[0].contains("OS"));
        assert!(titles[0].contains("Linux"));
    }

    #[test]
    fn test_wrap_in_box() {
        let lines = vec!["Hello".to_string(), "World!".to_string()];
        let boxed = wrap_in_box_bounded(&lines, None, None, None, None, 1, None);
        assert_eq!(boxed.len(), 4);
        assert!(boxed[0].starts_with('╭'));
        assert!(boxed[0].ends_with('╮'));
        assert!(boxed[3].starts_with('╰'));
        let width = strip_ansi_width(&boxed[0]);
        for line in &boxed {
            assert_eq!(strip_ansi_width(line), width);
        }
    }

    #[test]
    fn test_wrap_in_box_advanced_with_title_and_divider() {
        let lines = vec![
            "---DIVIDER:System---".to_string(),
            "OS: Linux".to_string(),
            "---DIVIDER---".to_string(),
            "CPU: 8 cores".to_string(),
        ];
        let boxed = wrap_in_box_bounded(
            &lines,
            Some("double"),
            Some("cyan"),
            None,
            Some("Specs"),
            1,
            None,
        );
        assert_eq!(boxed.len(), 6);
        assert!(boxed[0].contains("Specs"));
        assert!(boxed[1].contains('╠'));
        let width = strip_ansi_width(&boxed[0]);
        for line in &boxed {
            assert_eq!(strip_ansi_width(line), width);
        }
    }

    #[test]
    fn non_single_cell_border_chars_fall_back_to_style_defaults() {
        let mut chars = rustfetch_core::BorderCharsConfig::from_style("rounded");
        let wide = "\u{1F525}";
        chars.top_left = wide.to_string();
        chars.top = wide.to_string();
        chars.top_right = wide.to_string();
        chars.left = wide.to_string();
        chars.right = wide.to_string();
        chars.bottom = wide.to_string();
        chars.bottom_left = wide.to_string();
        chars.bottom_right = wide.to_string();
        chars.divider = wide.to_string();
        chars.divider_left = wide.to_string();
        chars.divider_right = wide.to_string();
        let lines = vec!["OS: Linux".to_string(), "CPU: intel".to_string()];
        let boxed = wrap_in_box_bounded(&lines, None, None, Some(&chars), None, 1, None);
        let width = strip_ansi_width(&boxed[0]);
        for line in &boxed {
            assert_eq!(strip_ansi_width(line), width);
            assert!(!line.contains('\u{1F525}'));
        }
    }

    #[test]
    fn box_padding_is_clamped() {
        let lines = vec!["OS: Linux".to_string()];
        let wide = wrap_in_box_bounded(&lines, None, None, None, None, 100, None);
        let clamped = wrap_in_box_bounded(&lines, None, None, None, None, MAX_BOX_PADDING, None);
        assert_eq!(wide, clamped);
    }

    #[test]
    fn test_wrap_in_box_bounded_responsive() {
        let lines = vec![
            "OS: Arch Linux x86_64".to_string(),
            "CPU: 12th Gen Intel(R) Core(TM) i5-12400F (12) @ 4.40 GHz".to_string(),
        ];
        let max_w = 40;
        let boxed = wrap_in_box_bounded(
            &lines,
            Some("rounded"),
            None,
            None,
            Some("Info"),
            1,
            Some(max_w),
        );
        assert!(boxed.len() >= 4);
        for line in &boxed {
            assert!(
                strip_ansi_width(line) <= max_w,
                "line exceeded max_w: {line}"
            );
        }
        let width = strip_ansi_width(&boxed[0]);
        for line in &boxed {
            assert_eq!(strip_ansi_width(line), width);
        }
    }

    #[test]
    fn test_compose_with_border_image_positions() {
        let content = vec!["CPU: 8 cores".to_string(), "RAM: 16 GiB".to_string()];
        let image = vec!["<top>".to_string(), "<bottom>".to_string()];

        let left = compose_with_border_image(&content, &image, "left");
        assert!(left[0].starts_with("<top>"));
        assert!(left[0].contains("CPU: 8 cores"));

        let right = compose_with_border_image(&content, &image, "right");
        assert!(right[0].starts_with("CPU: 8 cores"));
        assert!(right[0].ends_with("<top>"));

        let top = compose_with_border_image(&content, &image, "top");
        assert_eq!(top, ["<top>", "<bottom>", "CPU: 8 cores", "RAM: 16 GiB"]);

        let background = compose_with_border_image(&content, &image, "background");
        assert_eq!(background, content);
    }

    #[test]
    fn test_build_right_lines_header_color() {
        let info = vec![info_line("OS", "Linux")];
        let mut options = opts(false);
        options.header = Some("user@host");
        options.header_color = Some("#ff007f");
        let lines = build_right_lines(&info, &options);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("\x1b[38;2;255;0;127m"));
        assert!(lines[0].contains("user@host"));
    }

    #[test]
    fn horizontal_frames_use_lf_only() {
        let info = vec![info_line("OS", "Linux"), info_line("CPU", "intel")];
        let options = opts(false);
        let (frame, _, _, _) = frame_without_logo(&info, " ", 80, &options);
        assert!(
            !frame.contains('\r'),
            "frame must not contain CR: {frame:?}"
        );
        assert!(!frame.contains("\r\n"));
        assert!(frame.ends_with('\n'));

        let right_lines = build_right_lines(&info, &options);
        let logo_lines = vec!["/\\".to_string(), "\\/".to_string()];
        let (frame, _, _, _) =
            frame_horizontal(&logo_lines, 2, &right_lines, " ", 80, &options, "left");
        assert!(!frame.contains('\r'));
    }

    #[test]
    fn image_card_placement_plan() {
        assert_eq!(
            image_card_placement("top"),
            (ImageCardPlacement::TopOfImage, false)
        );
        assert_eq!(
            image_card_placement("LEFT"),
            (ImageCardPlacement::LeftOfImage, false)
        );
        assert_eq!(
            image_card_placement("behind"),
            (ImageCardPlacement::BehindImage, false)
        );
        // Unsupported for image logos: reported once and fallen back to left.
        assert_eq!(
            image_card_placement("bottom"),
            (ImageCardPlacement::LeftOfImage, true)
        );
        assert_eq!(
            image_card_placement("right"),
            (ImageCardPlacement::LeftOfImage, true)
        );
        assert_eq!(
            image_card_placement(""),
            (ImageCardPlacement::LeftOfImage, false)
        );
    }

    #[test]
    fn image_card_widths_respect_placement() {
        let term_w = 120;
        let top = image_card_max_width(ImageCardPlacement::TopOfImage, term_w, 20, 1);
        let left = image_card_max_width(ImageCardPlacement::LeftOfImage, term_w, 20, 1);
        let behind = image_card_max_width(ImageCardPlacement::BehindImage, term_w, 20, 1);
        assert_eq!(top, 117);
        assert_eq!(left, 95);
        assert_eq!(behind, 95);
        assert_eq!(
            image_card_max_width(ImageCardPlacement::TopOfImage, 10, 20, 8),
            0
        );
    }

    #[test]
    fn ascii_logo_positions_are_applied() {
        // Vertical order: "top" prints the logo first, "bottom" prints it last.
        assert!(ascii_logo_first("top"));
        assert!(!ascii_logo_first("bottom"));
        assert!(ascii_logo_first("left"));

        let info = vec![info_line("OS", "Linux")];
        let options = opts(false);
        let right_lines = build_right_lines(&info, &options);
        let logo_lines = vec!["LOGO".to_string(), "LOGO".to_string()];

        let (left_frame, _, _, _) =
            frame_horizontal(&logo_lines, 4, &right_lines, " ", 80, &options, "left");
        let (right_frame, _, _, _) =
            frame_horizontal(&logo_lines, 4, &right_lines, " ", 80, &options, "right");
        for line in left_frame.lines() {
            if line.contains("LOGO") && line.contains("OS") {
                let logo_at = line.find("LOGO").unwrap_or(usize::MAX);
                let card_at = line.find("OS").unwrap_or(0);
                assert!(logo_at < card_at, "logo must be left of the card: {line:?}");
            }
        }
        for line in right_frame.lines() {
            if line.contains("LOGO") {
                assert!(
                    line.ends_with("LOGO"),
                    "logo must be right of the card: {line:?}"
                );
            }
        }
    }
}
