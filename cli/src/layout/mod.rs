mod ansi;
mod card;
#[cfg(test)]
mod tests;

use std::fmt::Write as _;
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use ansi::decide_colors;
pub use ansi::{
    BOLD, DIM, RESET, build_color_palette, color_code, colors_enabled, strip_ansi,
    strip_ansi_width, truncate_ansi,
};
use card::MIN_INFO_CARD_WIDTH;
#[cfg(test)]
use card::{MAX_BOX_PADDING, compose_with_border_image, wrap_in_box_bounded};
pub use card::{
    build_right_lines, build_right_lines_bounded, emit_all_kitty_backgrounds, format_info_lines,
};

/// Upper bound for terminal dimensions accepted from any source (env, ioctl).
const MAX_TERM_DIMENSION: usize = 1000;
/// Fallback terminal size when nothing else is available.
const DEFAULT_TERM_WIDTH: usize = 80;
const DEFAULT_TERM_HEIGHT: usize = 24;

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
        if let Ok((width, height)) = crossterm::terminal::size()
            && width > 0
            && height > 0
        {
            return sanitize_dimensions(Some(usize::from(width)), Some(usize::from(height)));
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

pub(super) fn max_line_width(lines: &[String]) -> usize {
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
    let logo = logo.filter(|l| {
        !l.lines.is_empty()
            && l.width > 0
            && position != "none"
            && position != "hidden"
            && term_w >= 35
    });
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
        h_pad.len()
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
            let target_bottom = bg_rows.max(image_lines);
            if target_bottom > rows {
                ok &= write_frame(&"\n".repeat(target_bottom - rows));
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
            let target_bottom = bg_rows.max(image_lines);
            if target_bottom > rows {
                ok &= write_frame(&"\n".repeat(target_bottom - rows));
            }
        }
    }
    ok
}

fn build_kitty_placeholder_horizontal_frame(
    placeholder_rows: &[String],
    image_cols: usize,
    h_pad: usize,
    right_lines: &[String],
    center: bool,
    keep_colors: bool,
) -> (String, usize) {
    let mut padded_right: Vec<String> = right_lines.to_vec();
    if center && placeholder_rows.len() > padded_right.len() {
        let v_offset = (placeholder_rows.len() - padded_right.len()) / 2;
        let mut padded = vec![String::new(); v_offset];
        padded.append(&mut padded_right);
        padded_right = padded;
    }

    let img_h = placeholder_rows.len();
    let image_v_offset = if padded_right.len() > img_h {
        (padded_right.len() - img_h) / 2
    } else {
        0
    };
    let total_rows = img_h.max(padded_right.len());
    let pad = " ".repeat(h_pad);
    let img_blank = " ".repeat(image_cols);
    let gap = " ".repeat(LOGO_GAP);
    let mut frame = String::new();

    for i in 0..total_rows {
        let img_part = if i >= image_v_offset && i < image_v_offset + img_h {
            placeholder_rows[i - image_v_offset].as_str()
        } else {
            img_blank.as_str()
        };
        let right_line = padded_right.get(i).map(String::as_str).unwrap_or("");
        let right_clean = render_line(right_line, keep_colors);
        let _ = writeln!(frame, "{pad}{img_part}{gap}{right_clean}\x1b[K");
    }

    (frame, total_rows)
}

/// Renders Kitty Unicode placeholder rows alongside (`left`) or above (`top`)
/// system information telemetry using strictly sequential line output so scrollback
/// and terminal resizes keep the image and text grid locked together.
pub fn render_with_kitty_placeholder(
    placeholder_rows: &[String],
    image_cols: usize,
    h_pad: usize,
    info_lines: &[InfoLine],
    opts: &RenderOptions,
) -> bool {
    let term_w = terminal_width();
    let mut opts = *opts;
    opts.padding = effective_padding(opts.padding, term_w);
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
            let pad = " ".repeat(h_pad);
            let mut img_frame = String::new();
            for row in placeholder_rows {
                let _ = writeln!(img_frame, "{pad}{row}\x1b[K");
            }
            ok &= write_frame(&img_frame);

            let bg_rows = emit_all_kitty_backgrounds(opts.border_images, h_pad, content_w, rows);
            let mut card_frame = String::new();
            for line in &right_lines {
                let _ = writeln!(card_frame, "{pad}{}\x1b[K", render_line(line, keep_colors));
            }
            ok &= write_frame(&card_frame);
            if bg_rows > rows {
                ok &= write_frame(&"\n".repeat(bg_rows - rows));
            }
        }
        ImageCardPlacement::LeftOfImage | ImageCardPlacement::BehindImage => {
            let (frame, total_rows) = build_kitty_placeholder_horizontal_frame(
                placeholder_rows,
                image_cols,
                h_pad,
                &right_lines,
                opts.center,
                keep_colors,
            );
            let bg_rows =
                emit_all_kitty_backgrounds(opts.border_images, right_indent, content_w, total_rows);
            ok &= write_frame(&frame);
            if bg_rows > total_rows {
                ok &= write_frame(&"\n".repeat(bg_rows - total_rows));
            }
        }
    }
    ok
}
