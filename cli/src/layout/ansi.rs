use std::io::IsTerminal;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
pub(super) fn decide_colors(
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
    if let Some((r, g, b)) = rustfetch_render::parse_color(name) {
        if is_linux_console() {
            return rgb_to_ansi(r, g, b).to_string();
        }
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

/// Removes ANSI escape sequences (single source of truth: crate::text).
pub fn strip_ansi(s: &str) -> String {
    crate::text::strip_ansi(s)
}

/// Visible column width after stripping ANSI, grapheme/emoji aware.
pub fn strip_ansi_width(s: &str) -> usize {
    crate::text::visible_width(s)
}
