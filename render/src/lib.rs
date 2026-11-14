pub mod braille;
mod color;
mod image;
pub mod iterm;
mod kitty;
pub mod sixel;
mod terminal;
pub mod three_d;

pub use braille::{render_braille, render_braille_ex};
pub use color::{
    THEME_NAMES, Theme, apply_gradient_to_text, extract_color_palette,
    extract_dominant_and_palette, extract_dominant_color, get_theme, interpolate_color,
    interpolate_gradient, parse_color, theme_names,
};
pub use image::{
    BackgroundOptions, apply_image_background_to_lines, apply_image_background_to_lines_ex,
    fit_dimensions, image_display_dimensions, render_halfblock, render_halfblock_ex,
};
pub use iterm::{encode_iterm, render_iterm, render_iterm_ex};
pub use kitty::{clear_kitty_images, print_kitty, print_kitty_ex};
pub use sixel::{encode_sixel, render_sixel_ex};
pub use terminal::{
    cell_size_or_default, get_cell_size, is_tmux, supports_iterm, supports_kitty,
    supports_kitty_placeholders, supports_sixel, wrap_tmux_passthrough,
    wrap_tmux_passthrough_always,
};

pub fn render_image_ex(
    path: &str,
    target_cols: usize,
    max_rows: Option<usize>,
) -> Option<(usize, usize)> {
    // These APIs emit raw kitty graphics escape sequences, so silently do
    // nothing unless the terminal actually speaks the protocol.
    if !terminal::supports_kitty() {
        return None;
    }
    image::render_ex(path, target_cols, max_rows)
}

pub fn render_kitty_placeholder(
    path: &str,
    target_cols: usize,
    max_rows: Option<usize>,
    clear_before_emit: bool,
) -> Option<(usize, usize, Vec<String>)> {
    if !terminal::supports_kitty() {
        return None;
    }
    image::render_kitty_placeholder(path, target_cols, max_rows, clear_before_emit)
}

pub fn render_image_positioned(
    path: &str,
    target_cols: usize,
    max_rows: Option<usize>,
    reserve_rows: usize,
    h_pad: usize,
    clear_before_emit: bool,
) -> Option<(usize, usize)> {
    if !terminal::supports_kitty() {
        return None;
    }
    image::render_positioned(
        path,
        target_cols,
        max_rows,
        reserve_rows,
        h_pad,
        clear_before_emit,
    )
}

pub fn render_background(path: &str, target_cols: usize, target_rows: usize, z_index: i32) -> bool {
    if !terminal::supports_kitty() {
        return false;
    }
    image::render_background(path, target_cols, target_rows, z_index)
}

pub fn render_background_ex(path: &str, opts: &BackgroundOptions) -> Option<usize> {
    if !terminal::supports_kitty() {
        return None;
    }
    image::render_background_ex(path, opts)
}
