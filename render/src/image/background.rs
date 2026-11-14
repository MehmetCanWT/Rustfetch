//! Background image rendering for cards and text lines.

use fast_image_resize as fr;
use std::fmt::Write as _;
use std::io::Write;

use super::{clamp_pixel_dims, fit_dimensions, open_image_reader, resize_rgba};
use crate::kitty;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundOptions {
    pub col_offset: usize,
    pub card_w: usize,
    pub card_h: usize,
    pub target_cols: Option<usize>,
    pub target_rows: Option<usize>,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
    pub z_index: i32,
    pub preserve_aspect_ratio: bool,
}

pub fn render_background_ex(path: &str, opts: &BackgroundOptions) -> Option<usize> {
    let img_reader = open_image_reader(path)?;
    let rgba_img = img_reader.decode().ok()?.to_rgba8();
    let orig_width = rgba_img.width();
    let orig_height = rgba_img.height();
    if orig_width == 0 || orig_height == 0 || opts.card_w == 0 || opts.card_h == 0 {
        return None;
    }

    let (fit_cols, fit_rows) = fit_dimensions(
        orig_width,
        orig_height,
        opts.target_cols,
        opts.target_rows,
        opts.card_w,
        opts.card_h,
        opts.preserve_aspect_ratio,
    );
    let max_cols = opts.target_cols.unwrap_or(opts.card_w).max(opts.card_w);
    let max_rows = opts
        .target_rows
        .or(Some(opts.card_h))
        .map(|r| r.max(opts.card_h));
    let (fit_cols, fit_rows) = clamp_pixel_dims(fit_cols, fit_rows, max_cols, max_rows, 1, 1);

    // Calculate exact center alignment within the card:
    let center_pad_x = (opts.card_w as i32 - fit_cols as i32) / 2;
    let center_pad_y = (opts.card_h as i32 - fit_rows as i32) / 2;

    let eff_col =
        (opts.col_offset as i32 + center_pad_x + opts.offset_x.unwrap_or(0)).max(0) as usize;
    let eff_row = (center_pad_y + opts.offset_y.unwrap_or(0)).max(0) as usize;

    let max_px_w = 1920u32;
    let max_px_h = 1080u32;
    let (buf, w, h) = if orig_width > max_px_w || orig_height > max_px_h {
        let ratio_w = max_px_w as f64 / orig_width as f64;
        let ratio_h = max_px_h as f64 / orig_height as f64;
        let scale = ratio_w.min(ratio_h);
        let dst_w = ((orig_width as f64 * scale).round() as u32).max(1);
        let dst_h = ((orig_height as f64 * scale).round() as u32).max(1);
        let resized = resize_rgba(
            rgba_img.into_raw(),
            orig_width,
            orig_height,
            dst_w,
            dst_h,
            fr::FilterType::Lanczos3,
        )?;
        (resized, dst_w, dst_h)
    } else {
        (rgba_img.into_raw(), orig_width, orig_height)
    };

    print!("\x1b7");
    if eff_col > 0 {
        print!("\x1b[{}C", eff_col);
    }
    if eff_row > 0 {
        print!("\x1b[{}B", eff_row);
    }

    kitty::print_kitty_ex(
        &buf,
        w,
        h,
        fit_cols as u32,
        fit_rows as u32,
        opts.z_index,
        true,
    );

    print!("\x1b8");
    let _ = std::io::stdout().flush();
    Some(eff_row + fit_rows)
}

pub fn render_background(path: &str, target_cols: usize, target_rows: usize, z_index: i32) -> bool {
    let opts = BackgroundOptions {
        col_offset: 0,
        card_w: target_cols,
        card_h: target_rows,
        target_cols: Some(target_cols),
        target_rows: Some(target_rows),
        offset_x: None,
        offset_y: None,
        z_index,
        preserve_aspect_ratio: true,
    };
    render_background_ex(path, &opts).is_some()
}

pub fn apply_image_background_to_lines(
    lines: &[String],
    path: &str,
    target_cols: usize,
    target_rows: usize,
) -> Option<Vec<String>> {
    apply_image_background_to_lines_ex(lines, path, target_cols, target_rows, true, None, None)
}

pub fn apply_image_background_to_lines_ex(
    lines: &[String],
    path: &str,
    target_cols: usize,
    target_rows: usize,
    preserve_aspect_ratio: bool,
    offset_x: Option<i32>,
    offset_y: Option<i32>,
) -> Option<Vec<String>> {
    let img_reader = open_image_reader(path)?;
    let img = img_reader.decode().ok()?.to_rgba8();

    let orig_width = img.width();
    let orig_height = img.height();
    if orig_width == 0 || orig_height == 0 || target_cols == 0 || target_rows == 0 {
        return None;
    }

    let (fit_cols, fit_rows) = fit_dimensions(
        orig_width,
        orig_height,
        Some(target_cols),
        Some(target_rows),
        target_cols,
        target_rows,
        preserve_aspect_ratio,
    );
    let (fit_cols, fit_rows) =
        clamp_pixel_dims(fit_cols, fit_rows, target_cols, Some(target_rows), 1, 1);

    let raw = resize_rgba(
        img.into_raw(),
        orig_width,
        orig_height,
        fit_cols as u32,
        fit_rows as u32,
        fr::FilterType::Lanczos3,
    )?;
    let mut result = Vec::with_capacity(target_rows);

    let center_pad_x = (target_cols as i32 - fit_cols as i32) / 2;
    let center_pad_y = (target_rows as i32 - fit_rows as i32) / 2;
    let start_col = (center_pad_x + offset_x.unwrap_or(0)).max(0) as usize;
    let start_row = (center_pad_y + offset_y.unwrap_or(0)).max(0) as usize;

    for row in 0..target_rows {
        let content_line = lines.get(row).map(|s| s.as_str()).unwrap_or("");
        let mut row_pixels = Vec::with_capacity(target_cols);
        for col in 0..target_cols {
            if col >= start_col
                && col < start_col + fit_cols
                && row >= start_row
                && row < start_row + fit_rows
            {
                let idx = ((row - start_row) * fit_cols + (col - start_col)) * 4;
                row_pixels.push((raw[idx], raw[idx + 1], raw[idx + 2], raw[idx + 3]));
            } else {
                row_pixels.push((0, 0, 0, 0));
            }
        }
        result.push(apply_row_background(content_line, &row_pixels, target_cols));
    }

    Some(result)
}

fn apply_row_background(
    content: &str,
    row_pixels: &[(u8, u8, u8, u8)],
    target_width: usize,
) -> String {
    use unicode_width::UnicodeWidthChar;

    let mut out = String::with_capacity(content.len() + target_width * 20);
    let mut col = 0usize;
    let mut cur_bg: Option<(u8, u8, u8)> = None;
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            let mut seq = String::from('\x1b');
            match chars.peek().copied() {
                Some('[') => {
                    if let Some(bracket) = chars.next() {
                        seq.push(bracket);
                    }
                    let mut final_byte = None;
                    for c in chars.by_ref() {
                        seq.push(c);
                        if ('@'..='~').contains(&c) {
                            final_byte = Some(c);
                            break;
                        }
                    }
                    if final_byte == Some('m') && seq.len() >= 3 {
                        let params = &seq[2..seq.len() - 1];
                        let pvec: Vec<&str> = params.split(';').collect();
                        let mut pi = 0usize;
                        while pi < pvec.len() {
                            match pvec[pi] {
                                "" | "0" | "49" => cur_bg = None,
                                "38" | "48" | "58" => {
                                    if pvec.get(pi + 1) == Some(&"2") {
                                        pi += 4;
                                    } else if pvec.get(pi + 1) == Some(&"5") {
                                        pi += 2;
                                    }
                                }
                                _ => {}
                            }
                            pi += 1;
                        }
                    }
                }
                Some(']') => {
                    if let Some(bracket) = chars.next() {
                        seq.push(bracket);
                    }
                    while let Some(c) = chars.next() {
                        seq.push(c);
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' && chars.peek() == Some(&'\\') {
                            if let Some(st) = chars.next() {
                                seq.push(st);
                            }
                            break;
                        }
                    }
                }
                Some('P' | '_' | '^' | 'X') => {
                    if let Some(intro) = chars.next() {
                        seq.push(intro);
                    }
                    while let Some(c) = chars.next() {
                        seq.push(c);
                        if c == '\x1b' && chars.peek() == Some(&'\\') {
                            if let Some(st) = chars.next() {
                                seq.push(st);
                            }
                            break;
                        }
                    }
                }
                Some('(' | ')' | '*' | '+') => {
                    if let Some(intro) = chars.next() {
                        seq.push(intro);
                    }
                    if let Some(next_c) = chars.next() {
                        seq.push(next_c);
                    }
                }
                Some(_) => {
                    if let Some(next_c) = chars.next() {
                        seq.push(next_c);
                    }
                }
                None => {}
            }
            out.push_str(&seq);
            continue;
        }

        let ch_width = ch.width().unwrap_or(1);
        if col < target_width
            && let Some(&(r, g, b, a)) = row_pixels.get(col)
        {
            if a > 20 {
                if cur_bg != Some((r, g, b)) {
                    let _ = write!(out, "\x1b[48;2;{r};{g};{b}m");
                    cur_bg = Some((r, g, b));
                }
            } else if cur_bg.is_some() {
                out.push_str("\x1b[49m");
                cur_bg = None;
            }
        }
        out.push(ch);
        col += ch_width;
    }

    while col < target_width {
        if let Some(&(r, g, b, a)) = row_pixels.get(col) {
            if a > 20 {
                if cur_bg != Some((r, g, b)) {
                    let _ = write!(out, "\x1b[48;2;{r};{g};{b}m");
                    cur_bg = Some((r, g, b));
                }
                out.push(' ');
            } else {
                if cur_bg.is_some() {
                    out.push_str("\x1b[49m");
                    cur_bg = None;
                }
                out.push(' ');
            }
        } else {
            out.push(' ');
        }
        col += 1;
    }
    out.push_str("\x1b[0m");
    out
}
