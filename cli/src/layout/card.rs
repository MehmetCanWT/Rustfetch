use super::ansi::{BOLD, DIM, RESET, color_code, strip_ansi_width, truncate_ansi};
use super::{InfoLine, RenderOptions};

pub(super) const MIN_INFO_CARD_WIDTH: usize = 34;
/// Upper bound for the user configured inner box padding.
pub(super) const MAX_BOX_PADDING: usize = 16;

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

    if (opts.palette_position == "top" || opts.palette_position == "both")
        && let Some(pal) = opts.palette_lines
    {
        right_lines.extend(pal.iter().cloned());
        right_lines.push(String::new());
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

    if (opts.palette_position == "bottom" || opts.palette_position == "both")
        && let Some(pal) = opts.palette_lines
    {
        right_lines.push(String::new());
        right_lines.extend(pal.iter().cloned());
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
        if img.is_background()
            && should_use_kitty(img)
            && let Some(bottom_row) = emit_kitty_background(img, col_offset, card_w, card_h)
        {
            max_lines = max_lines.max(bottom_row);
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
