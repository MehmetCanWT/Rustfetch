//! Image decoding, aspect-ratio fitting, and raster/Kitty/halfblock renderers.

mod background;

#[cfg(test)]
mod tests;

pub use background::{
    BackgroundOptions, apply_image_background_to_lines, apply_image_background_to_lines_ex,
    render_background, render_background_ex,
};

use fast_image_resize as fr;
use fast_image_resize::Image;
use image::{AnimationDecoder, ImageReader};
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufReader, Write};
use std::num::NonZeroU32;

use crate::{kitty, terminal};

/// Hard cap on decoded image dimensions (protects against decompression bombs).
pub(crate) const MAX_IMAGE_DIMENSION: u32 = 8192;

/// Hard cap on decoded frames in a single animated GIF.
pub(crate) const MAX_GIF_FRAMES: usize = 512;

/// Default maximum number of terminal rows used when the caller does not limit it.
pub(crate) const DEFAULT_IMAGE_ROWS: usize = 24;

/// Hard pixel budget for rasterization buffers (2000 x 4000 pixels).
pub(crate) const MAX_IMAGE_PIXEL_WIDTH: usize = 2000;
pub(crate) const MAX_IMAGE_PIXEL_HEIGHT: usize = 4000;

/// Reads image dimensions from the file header, rejecting oversized images.
pub(crate) fn image_dimensions_checked(path: &str) -> Option<(u32, u32)> {
    let (width, height) = image::image_dimensions(path).ok()?;
    if width == 0 || height == 0 || width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return None;
    }
    Some((width, height))
}

/// Opens an image reader after applying the shared dimension cap.
pub(crate) fn open_image_reader(path: &str) -> Option<ImageReader<BufReader<File>>> {
    image_dimensions_checked(path)?;
    ImageReader::open(path).ok()?.with_guessed_format().ok()
}

/// Clamps raster dimensions to the requested terminal box and the global pixel
/// budget, rounding down to whole sub-pixel units so callers never index out of
/// bounds. Shared by the half-block and braille renderers.
pub(crate) fn clamp_pixel_dims(
    pixel_cols: usize,
    pixel_rows: usize,
    target_cols: usize,
    target_rows: Option<usize>,
    px_per_col: usize,
    px_per_row: usize,
) -> (usize, usize) {
    let px_per_col = px_per_col.max(1);
    let px_per_row = px_per_row.max(1);
    let rows_limit = target_rows.unwrap_or(DEFAULT_IMAGE_ROWS).max(1);
    let max_cols = (target_cols.max(1) * px_per_col)
        .min(MAX_IMAGE_PIXEL_WIDTH)
        .max(px_per_col);
    let max_rows = (rows_limit * px_per_row)
        .min(MAX_IMAGE_PIXEL_HEIGHT)
        .max(px_per_row);
    let cols = pixel_cols.max(px_per_col).min(max_cols);
    let rows = pixel_rows.max(px_per_row).min(max_rows);
    let cols = (cols / px_per_col).max(1) * px_per_col;
    let rows = (rows / px_per_row).max(1) * px_per_row;
    (cols, rows)
}

pub(crate) fn resize_rgba(
    mut raw: Vec<u8>,
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    filter: fr::FilterType,
) -> Option<Vec<u8>> {
    // Premultiply alpha before convolution to prevent dark fringes around transparent borders
    let mut has_transparency = false;
    for chunk in raw.as_chunks_mut::<4>().0 {
        let a = chunk[3] as u32;
        if a < 255 {
            has_transparency = true;
            chunk[0] = ((chunk[0] as u32 * a + 127) / 255) as u8;
            chunk[1] = ((chunk[1] as u32 * a + 127) / 255) as u8;
            chunk[2] = ((chunk[2] as u32 * a + 127) / 255) as u8;
        }
    }

    let src_image = Image::from_vec_u8(
        NonZeroU32::new(src_w)?,
        NonZeroU32::new(src_h)?,
        raw,
        fr::PixelType::U8x4,
    )
    .ok()?;
    let mut dst_image = Image::new(
        NonZeroU32::new(dst_w)?,
        NonZeroU32::new(dst_h)?,
        fr::PixelType::U8x4,
    );
    fr::Resizer::new(fr::ResizeAlg::Convolution(filter))
        .resize(&src_image.view(), &mut dst_image.view_mut())
        .ok()?;

    let mut dst_vec = dst_image.into_vec();
    if has_transparency {
        for chunk in dst_vec.as_chunks_mut::<4>().0 {
            let a = chunk[3] as u32;
            if a > 0 && a < 255 {
                chunk[0] = (((chunk[0] as u32 * 255 + a / 2) / a).min(255)) as u8;
                chunk[1] = (((chunk[1] as u32 * 255 + a / 2) / a).min(255)) as u8;
                chunk[2] = (((chunk[2] as u32 * 255 + a / 2) / a).min(255)) as u8;
            }
        }
    }
    Some(dst_vec)
}

enum PreparedKittyImage {
    Static {
        cols: usize,
        rows: usize,
        px_w: u32,
        px_h: u32,
        rgba: Vec<u8>,
    },
    Gif {
        cols: usize,
        rows: usize,
        px_w: u32,
        px_h: u32,
        frames: Vec<(Vec<u8>, u32)>,
    },
}

impl PreparedKittyImage {
    fn dimensions(&self) -> (usize, usize) {
        match self {
            Self::Static { cols, rows, .. } | Self::Gif { cols, rows, .. } => (*cols, *rows),
        }
    }
}

fn prepare_kitty_image(
    path: &str,
    target_cols: usize,
    max_rows: Option<usize>,
) -> Option<PreparedKittyImage> {
    if target_cols == 0 {
        return None;
    }
    let (cell_w, cell_h) = terminal::cell_size_or_default();
    let img_reader = open_image_reader(path)?;

    if img_reader.format() == Some(image::ImageFormat::Gif) {
        let (orig_width, orig_height) = image_dimensions_checked(path)?;
        let (fit_cols, fit_rows) =
            fit_cell_dimensions(orig_width, orig_height, target_cols, max_rows);
        let (target_px_width, target_px_height) = clamp_pixel_dims(
            fit_cols * cell_w as usize,
            fit_rows * cell_h as usize,
            target_cols,
            max_rows,
            cell_w as usize,
            cell_h as usize,
        );
        let target_px_width = target_px_width as u32;
        let target_px_height = target_px_height as u32;

        if let Ok(file) = File::open(path)
            && let Ok(decoder) = image::codecs::gif::GifDecoder::new(BufReader::new(file))
        {
            let mut resized_buffers = Vec::new();
            for frame_res in decoder.into_frames().take(MAX_GIF_FRAMES) {
                let frame = frame_res.ok()?;
                let (num, den) = frame.delay().numer_denom_ms();
                let delay_ms = num.checked_div(den).unwrap_or(0);
                let raw_buf = frame.into_buffer();
                let fw = raw_buf.width();
                let fh = raw_buf.height();
                let buf = resize_rgba(
                    raw_buf.into_raw(),
                    fw,
                    fh,
                    target_px_width,
                    target_px_height,
                    fr::FilterType::Lanczos3,
                )?;
                resized_buffers.push((buf, delay_ms));
            }
            if !resized_buffers.is_empty() {
                return Some(PreparedKittyImage::Gif {
                    cols: fit_cols,
                    rows: fit_rows,
                    px_w: target_px_width,
                    px_h: target_px_height,
                    frames: resized_buffers,
                });
            }
        }
    }

    let img = img_reader.decode().ok()?;
    let rgba_img = img.to_rgba8();
    let orig_width = rgba_img.width();
    let orig_height = rgba_img.height();
    if orig_width == 0 || orig_height == 0 {
        return None;
    }

    let (fit_cols, fit_rows) = fit_cell_dimensions(orig_width, orig_height, target_cols, max_rows);
    let (target_px_width, target_px_height) = clamp_pixel_dims(
        fit_cols * cell_w as usize,
        fit_rows * cell_h as usize,
        target_cols,
        max_rows,
        cell_w as usize,
        cell_h as usize,
    );
    let target_px_width = target_px_width as u32;
    let target_px_height = target_px_height as u32;

    let rgba = resize_rgba(
        rgba_img.into_raw(),
        orig_width,
        orig_height,
        target_px_width,
        target_px_height,
        fr::FilterType::Lanczos3,
    )?;

    Some(PreparedKittyImage::Static {
        cols: fit_cols,
        rows: fit_rows,
        px_w: target_px_width,
        px_h: target_px_height,
        rgba,
    })
}

pub fn render_ex(
    path: &str,
    target_cols: usize,
    max_rows: Option<usize>,
) -> Option<(usize, usize)> {
    render_positioned(path, target_cols, max_rows, 0, 0, false)
}

pub fn render_positioned(
    path: &str,
    target_cols: usize,
    max_rows: Option<usize>,
    reserve_rows: usize,
    h_pad: usize,
    clear_before_emit: bool,
) -> Option<(usize, usize)> {
    let expected_winsize = terminal::get_winsize_cells();
    let prepared = prepare_kitty_image(path, target_cols, max_rows)?;
    let (_fit_cols, fit_rows) = prepared.dimensions();

    if clear_before_emit && expected_winsize != terminal::get_winsize_cells() {
        return None;
    }
    if clear_before_emit {
        print!("\x1b[?2026h");
        kitty::clear_kitty_images();
        print!("\x1b[2J\x1b[3J\x1b[H");
    }
    if reserve_rows > 0 {
        let effective_reserve = reserve_rows.max(fit_rows);
        if effective_reserve > 1 {
            print!(
                "{}\x1b[{}A\r",
                "\n".repeat(effective_reserve - 1),
                effective_reserve - 1
            );
        }
    }
    if h_pad > 0 {
        print!("{}", " ".repeat(h_pad));
    }
    let _ = std::io::stdout().flush();

    match prepared {
        PreparedKittyImage::Gif {
            cols,
            rows,
            px_w,
            px_h,
            frames,
        } => {
            let kitty_frames: Vec<_> = frames
                .iter()
                .map(|(buf, delay)| kitty::KittyFrame {
                    rgba_data: buf,
                    delay_ms: *delay,
                })
                .collect();
            kitty::print_kitty_animation(&kitty_frames, px_w, px_h, cols as u32, rows as u32);
            Some((cols, rows))
        }
        PreparedKittyImage::Static {
            cols,
            rows,
            px_w,
            px_h,
            rgba,
        } => {
            kitty::print_kitty(&rgba, px_w, px_h, cols as u32, rows as u32);
            Some((cols, rows))
        }
    }
}

/// Transmits a static image or animated GIF as a Kitty virtual placement (`U=1`)
/// and returns `(cols, rows, placeholder_rows)` so the caller can lay out the
/// image cells sequentially alongside system telemetry without cursor-up jumps.
pub fn render_kitty_placeholder(
    path: &str,
    target_cols: usize,
    max_rows: Option<usize>,
    clear_before_emit: bool,
) -> Option<(usize, usize, Vec<String>)> {
    let expected_winsize = terminal::get_winsize_cells();
    let prepared = prepare_kitty_image(path, target_cols, max_rows)?;

    if clear_before_emit && expected_winsize != terminal::get_winsize_cells() {
        return None;
    }
    if clear_before_emit {
        print!("\x1b[?2026h");
        kitty::clear_kitty_images();
        print!("\x1b[2J\x1b[3J\x1b[H");
        let _ = std::io::stdout().flush();
    }

    match prepared {
        PreparedKittyImage::Gif {
            cols,
            rows,
            px_w,
            px_h,
            frames,
        } => {
            let kitty_frames: Vec<_> = frames
                .iter()
                .map(|(buf, delay)| kitty::KittyFrame {
                    rgba_data: buf,
                    delay_ms: *delay,
                })
                .collect();
            let (_id, placeholder_rows) = kitty::print_kitty_animation_virtual(
                &kitty_frames,
                px_w,
                px_h,
                cols as u32,
                rows as u32,
            )?;
            Some((cols, rows, placeholder_rows))
        }
        PreparedKittyImage::Static {
            cols,
            rows,
            px_w,
            px_h,
            rgba,
        } => {
            let (_id, placeholder_rows) =
                kitty::print_kitty_virtual(&rgba, px_w, px_h, cols as u32, rows as u32)?;
            Some((cols, rows, placeholder_rows))
        }
    }
}

pub(crate) fn fit_cell_dimensions(
    orig_w: u32,
    orig_h: u32,
    target_cols: usize,
    max_rows: Option<usize>,
) -> (usize, usize) {
    let (fit_cols, fit_rows) = fit_dimensions(
        orig_w,
        orig_h,
        Some(target_cols),
        max_rows,
        target_cols,
        max_rows.unwrap_or(24),
        true,
    );
    (fit_cols.max(1), fit_rows.max(1))
}

pub fn image_display_dimensions(
    path: &str,
    target_cols: usize,
    max_rows: Option<usize>,
) -> Option<(usize, usize)> {
    if target_cols == 0 {
        return None;
    }
    let (orig_width, orig_height) = image_dimensions_checked(path)?;
    Some(fit_cell_dimensions(
        orig_width,
        orig_height,
        target_cols,
        max_rows,
    ))
}

/// Computes target (cols, rows) in terminal character cells while preserving image aspect ratio.
/// Takes into account the terminal cell aspect ratio (typically 1:2, where 1 col is half as wide as 1 row is tall).
pub fn fit_dimensions(
    orig_w: u32,
    orig_h: u32,
    target_cols: Option<usize>,
    target_rows: Option<usize>,
    container_cols: usize,
    container_rows: usize,
    preserve_aspect_ratio: bool,
) -> (usize, usize) {
    if orig_w == 0 || orig_h == 0 || !preserve_aspect_ratio {
        return (
            target_cols.unwrap_or(container_cols).max(1),
            target_rows.unwrap_or(container_rows).max(1),
        );
    }

    let (cell_w, cell_h) = terminal::cell_size_or_default();
    let cell_ratio = cell_w as f64 / cell_h as f64; // typically ~0.5
    let img_ratio = orig_w as f64 / orig_h as f64;
    // How many terminal columns correspond to 1 terminal row for this image:
    let natural_cols_per_row = (img_ratio / cell_ratio).max(0.1);

    match (target_cols, target_rows) {
        (Some(c), None) => {
            let cols = c.max(1);
            let rows = (cols as f64 / natural_cols_per_row).round().max(1.0) as usize;
            (cols, rows)
        }
        (None, Some(r)) => {
            let rows = r.max(1);
            let cols = (rows as f64 * natural_cols_per_row).round().max(1.0) as usize;
            (cols, rows)
        }
        (c, r) => {
            let box_c = c.unwrap_or(container_cols).max(1) as f64;
            let box_r = r.unwrap_or(container_rows).max(1) as f64;
            if box_c / natural_cols_per_row <= box_r {
                let cols = box_c as usize;
                let rows = (box_c / natural_cols_per_row).round().max(1.0) as usize;
                (cols, rows)
            } else {
                let rows = box_r as usize;
                let cols = (box_r * natural_cols_per_row).round().max(1.0) as usize;
                (cols, rows)
            }
        }
    }
}

pub fn render_halfblock_ex(
    path: &str,
    target_cols: usize,
    target_rows: Option<usize>,
    preserve_aspect_ratio: bool,
) -> Option<Vec<String>> {
    let img_reader = open_image_reader(path)?;
    let img = img_reader.decode().ok()?.to_rgba8();

    let orig_width = img.width();
    let orig_height = img.height();
    if orig_width == 0 || orig_height == 0 || target_cols == 0 {
        return None;
    }

    let img_ratio = orig_width as f64 / orig_height as f64;

    // In halfblocks, each cell character has 2 vertical pixels (1 column = 1 px wide, 1 row = 2 px high).
    // Because a terminal cell is ~10px wide x 20px high, each half-block pixel is ~10px x 10px (square!).
    let (pixel_cols, pixel_rows) = match target_rows {
        Some(rows) if rows > 0 => {
            let max_c = target_cols;
            let max_r_px = rows * 2;
            if preserve_aspect_ratio {
                let from_w = (max_c as f64 / img_ratio).round() as usize;
                if from_w <= max_r_px {
                    (max_c, from_w.max(2))
                } else {
                    let from_h = (max_r_px as f64 * img_ratio).round() as usize;
                    (from_h.max(1), max_r_px)
                }
            } else {
                (max_c, max_r_px.max(2))
            }
        }
        _ => {
            let px_h = ((target_cols as f64 / img_ratio).round() as usize).max(2);
            (target_cols, px_h)
        }
    };

    // Shared clamp keeps the output bounded even for extreme aspect ratios.
    let (pixel_cols, pixel_rows) =
        clamp_pixel_dims(pixel_cols, pixel_rows, target_cols, target_rows, 1, 2);

    let raw = resize_rgba(
        img.into_raw(),
        orig_width,
        orig_height,
        pixel_cols as u32,
        pixel_rows as u32,
        fr::FilterType::Lanczos3,
    )?;
    let width = pixel_cols;
    let height = pixel_rows;
    let mut lines = Vec::with_capacity(height / 2);

    for y in (0..height).step_by(2) {
        let mut line = String::with_capacity(width * 20);
        let mut cur_fg: Option<(u8, u8, u8)> = None;
        let mut cur_bg: Option<(u8, u8, u8)> = None;

        for x in 0..width {
            let idx_top = (y * width + x) * 4;
            let idx_bot = ((y + 1) * width + x) * 4;

            let (r1, g1, b1, a1) = (
                raw[idx_top],
                raw[idx_top + 1],
                raw[idx_top + 2],
                raw[idx_top + 3],
            );
            let (r2, g2, b2, a2) = (
                raw[idx_bot],
                raw[idx_bot + 1],
                raw[idx_bot + 2],
                raw[idx_bot + 3],
            );

            if a1 < 30 && a2 < 30 {
                if cur_bg.is_some() {
                    line.push_str("\x1b[49m");
                    cur_bg = None;
                }
                line.push(' ');
            } else if a1 < 30 {
                if cur_bg.is_some() {
                    line.push_str("\x1b[49m");
                    cur_bg = None;
                }
                if cur_fg != Some((r2, g2, b2)) {
                    let _ = write!(line, "\x1b[38;2;{r2};{g2};{b2}m");
                    cur_fg = Some((r2, g2, b2));
                }
                line.push('▄');
            } else if a2 < 30 {
                if cur_bg.is_some() {
                    line.push_str("\x1b[49m");
                    cur_bg = None;
                }
                if cur_fg != Some((r1, g1, b1)) {
                    let _ = write!(line, "\x1b[38;2;{r1};{g1};{b1}m");
                    cur_fg = Some((r1, g1, b1));
                }
                line.push('▀');
            } else {
                if cur_fg != Some((r1, g1, b1)) {
                    let _ = write!(line, "\x1b[38;2;{r1};{g1};{b1}m");
                    cur_fg = Some((r1, g1, b1));
                }
                if cur_bg != Some((r2, g2, b2)) {
                    let _ = write!(line, "\x1b[48;2;{r2};{g2};{b2}m");
                    cur_bg = Some((r2, g2, b2));
                }
                line.push('▀');
            }
        }
        if cur_fg.is_some() || cur_bg.is_some() {
            line.push_str("\x1b[0m");
        }
        lines.push(line);
    }

    Some(lines)
}

pub fn render_halfblock(path: &str, target_cols: usize) -> Option<Vec<String>> {
    render_halfblock_ex(path, target_cols, None, true)
}
