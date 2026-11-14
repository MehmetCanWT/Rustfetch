use fast_image_resize as fr;
use std::collections::HashMap;
use std::fmt::Write as _;

use crate::image::open_image_reader;

/// Aggregated colour statistics for one sampled image.
struct ColorSample {
    counts: HashMap<(u8, u8, u8), u32>,
    avg_lum: u8,
}

fn load_and_sample_image(path: &str) -> Option<ColorSample> {
    let img_reader = open_image_reader(path)?;
    let img = img_reader.decode().ok()?;
    let rgba_img = img.to_rgba8();

    let orig_w = rgba_img.width();
    let orig_h = rgba_img.height();
    if orig_w == 0 || orig_h == 0 {
        return None;
    }

    let buffer = crate::image::resize_rgba(
        rgba_img.into_raw(),
        orig_w,
        orig_h,
        60,
        60,
        fr::FilterType::Bilinear,
    )?;
    let mut counts: HashMap<(u8, u8, u8), u32> = HashMap::new();
    let mut lum_sum: u64 = 0;
    let mut pixel_count: u64 = 0;
    for &[r, g, b, a] in buffer.as_chunks::<4>().0 {
        if a < 32 {
            continue;
        }
        let qr = (r / 4) * 4;
        let qg = (g / 4) * 4;
        let qb = (b / 4) * 4;
        *counts.entry((qr, qg, qb)).or_insert(0) += 1;
        lum_sum += (r as u64 * 299 + g as u64 * 587 + b as u64 * 114) / 1000;
        pixel_count += 1;
    }
    let avg_lum = lum_sum.checked_div(pixel_count).unwrap_or(128).min(255) as u8;
    Some(ColorSample { counts, avg_lum })
}

fn ensure_visible(c: (u8, u8, u8)) -> (u8, u8, u8) {
    let lum = 0.299 * c.0 as f64 + 0.587 * c.1 as f64 + 0.114 * c.2 as f64;
    if lum < 85.0 {
        let factor = 110.0 / lum.max(1.0);
        (
            ((c.0 as f64 * factor).round() as u32).min(255) as u8,
            ((c.1 as f64 * factor).round() as u32).min(255) as u8,
            ((c.2 as f64 * factor).round() as u32).min(255) as u8,
        )
    } else {
        c
    }
}

fn rgb_sv_lum(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r_f = r as f64 / 255.0;
    let g_f = g as f64 / 255.0;
    let b_f = b as f64 / 255.0;
    let max = r_f.max(g_f).max(b_f);
    let min = r_f.min(g_f).min(b_f);
    let s = if max > 0.0 { (max - min) / max } else { 0.0 };
    let lum = (0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) / 255.0;
    (s, max, lum)
}

fn dominant_from_counts(sample: &ColorSample) -> String {
    let mut candidates: Vec<(f64, (u8, u8, u8))> = Vec::new();
    for (&(r, g, b), &count) in &sample.counts {
        let (s, v, lum) = rgb_sv_lum(r, g, b);
        if s > 0.25 && v > 0.35 && v < 0.98 {
            let score = (count as f64).powf(0.6)
                * s.powf(1.6)
                * v.powf(0.8)
                * (1.0 - (lum - 0.6).abs() * 0.7);
            candidates.push((score, (r, g, b)));
        }
    }

    candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if let Some(&(_, (mut r, mut g, mut b))) = candidates.first() {
        let lum = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
        if lum < 110.0 {
            let factor = 135.0 / lum.max(1.0);
            r = ((r as f64 * factor).round() as u32).min(255) as u8;
            g = ((g as f64 * factor).round() as u32).min(255) as u8;
            b = ((b as f64 * factor).round() as u32).min(255) as u8;
        }
        return format!("#{:02x}{:02x}{:02x}", r, g, b);
    }

    // No saturated candidate: derive a neutral grey from the image luminance
    // instead of returning an unrelated hard-coded blue.
    let v = sample.avg_lum.clamp(32, 224);
    format!("#{v:02x}{v:02x}{v:02x}")
}

/// Even grey ramp used when an image has no saturated colours at all.
fn monochrome_palette(count: usize, avg_lum: u8) -> Vec<(u8, u8, u8)> {
    let count = count.max(1);
    let base = avg_lum as i32;
    let spread = 70i32;
    let mut palette = Vec::with_capacity(count);
    for index in 0..count {
        let t = if count == 1 {
            0.5
        } else {
            index as f64 / (count - 1) as f64
        };
        let offset = ((spread * 2) as f64 * t).round() as i32;
        let value = (base - spread + offset).clamp(24, 235) as u8;
        palette.push((value, value, value));
    }
    palette
}

fn palette_from_counts(sample: &ColorSample, count: usize) -> Option<Vec<(u8, u8, u8)>> {
    if count == 0 {
        return Some(Vec::new());
    }
    if sample.counts.is_empty() {
        return Some(monochrome_palette(count, sample.avg_lum));
    }

    let mut scored_colors: Vec<(f64, (u8, u8, u8))> = Vec::new();
    for (&(r, g, b), &cnt) in &sample.counts {
        let (s, v, lum) = rgb_sv_lum(r, g, b);
        if v > 0.12 && lum < 0.96 {
            let score = (cnt as f64).powf(0.5) * (1.0 + s * 1.5) * (1.0 - (lum - 0.55).abs() * 0.5);
            scored_colors.push((score, (r, g, b)));
        }
    }

    scored_colors.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Achromatic images (grey/white/black) get an even grey ramp instead of a
    // handful of lightened duplicates of the same grey.
    let has_saturation = scored_colors.iter().any(|(_, (r, g, b))| {
        let (saturation, _, _) = rgb_sv_lum(*r, *g, *b);
        saturation > 0.08
    });
    if scored_colors.is_empty() || !has_saturation {
        return Some(monochrome_palette(count, sample.avg_lum));
    }

    let mut palette: Vec<(u8, u8, u8)> = Vec::with_capacity(count);
    palette.push(ensure_visible(scored_colors[0].1));

    let color_dist = |c1: (u8, u8, u8), c2: (u8, u8, u8)| -> f64 {
        let dr = c1.0 as f64 - c2.0 as f64;
        let dg = c1.1 as f64 - c2.1 as f64;
        let db = c1.2 as f64 - c2.2 as f64;
        (2.0 * dr * dr + 4.0 * dg * dg + 3.0 * db * db).sqrt()
    };

    while palette.len() < count {
        let mut best_candidate = None;
        let mut best_min_dist = -1.0;

        for &(_, candidate) in scored_colors.iter().take(250) {
            let visible = ensure_visible(candidate);
            let min_dist = palette
                .iter()
                .map(|&p| color_dist(p, visible))
                .fold(f64::INFINITY, f64::min);

            if min_dist > best_min_dist && min_dist > 35.0 {
                best_min_dist = min_dist;
                best_candidate = Some(visible);
            }
        }

        if let Some(cand) = best_candidate {
            palette.push(cand);
        } else {
            break;
        }
    }

    let mut i = 0;
    while palette.len() < count && i < palette.len() {
        let base = palette[i];
        let lightened = (
            ((base.0 as f64 * 1.35).min(255.0)) as u8,
            ((base.1 as f64 * 1.35).min(255.0)) as u8,
            ((base.2 as f64 * 1.35).min(255.0)) as u8,
        );
        if !palette.contains(&lightened) {
            palette.push(lightened);
        }
        i += 1;
    }

    let mut i = 0;
    while palette.len() < count && i < palette.len() {
        let base = palette[i];
        let darkened = (
            (base.0 as f64 * 0.72).round() as u8,
            (base.1 as f64 * 0.72).round() as u8,
            (base.2 as f64 * 0.72).round() as u8,
        );
        if !palette.contains(&darkened) {
            palette.push(darkened);
        }
        i += 1;
    }

    if !palette.is_empty() {
        let seed_len = palette.len();
        let mut idx = 0;
        while palette.len() < count {
            palette.push(palette[idx % seed_len]);
            idx += 1;
        }
    }

    Some(palette)
}

pub fn extract_dominant_color(path: &str) -> Option<String> {
    let sample = load_and_sample_image(path)?;
    Some(dominant_from_counts(&sample))
}

pub fn extract_color_palette(path: &str, count: usize) -> Option<Vec<(u8, u8, u8)>> {
    let sample = load_and_sample_image(path)?;
    palette_from_counts(&sample, count)
}

pub type RgbPalette = Vec<(u8, u8, u8)>;

pub fn extract_dominant_and_palette(
    path: &str,
    count: usize,
) -> Option<(String, Option<RgbPalette>)> {
    let sample = load_and_sample_image(path)?;
    Some((
        dominant_from_counts(&sample),
        palette_from_counts(&sample, count),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub primary: (u8, u8, u8),
    pub secondary: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub background: (u8, u8, u8),
    pub border: (u8, u8, u8),
    pub text: (u8, u8, u8),
}

/// Canonical theme identifiers understood by [`get_theme`], in display order.
///
/// This is the single source of truth for `--list-themes`, the interactive
/// setup wizard and the documentation consistency tests.
pub const THEME_NAMES: &[&str] = &[
    "catppuccin",
    "catppuccin-latte",
    "catppuccin-macchiato",
    "catppuccin-frappe",
    "dracula",
    "tokyo-night",
    "gruvbox",
    "nord",
    "rose-pine",
    "cyberpunk",
    "monokai",
    "onedark",
    "solarized",
];

/// Returns the canonical theme identifier list.
pub fn theme_names() -> &'static [&'static str] {
    THEME_NAMES
}

pub fn get_theme(name: &str) -> Option<Theme> {
    match name.to_lowercase().replace('_', "-").as_str() {
        "catppuccin-mocha" | "catppuccin" | "mocha" => Some(Theme {
            name: "Catppuccin Mocha",
            primary: (203, 166, 247),   // Mauve
            secondary: (137, 180, 250), // Blue
            accent: (166, 227, 161),    // Green
            background: (30, 30, 46),   // Base
            border: (180, 190, 254),    // Lavender
            text: (205, 214, 244),      // Text
        }),
        "catppuccin-latte" | "latte" => Some(Theme {
            name: "Catppuccin Latte",
            primary: (136, 57, 239),
            secondary: (30, 102, 245),
            accent: (64, 160, 43),
            background: (239, 241, 245),
            border: (114, 135, 253),
            text: (76, 79, 105),
        }),
        "catppuccin-macchiato" | "macchiato" => Some(Theme {
            name: "Catppuccin Macchiato",
            primary: (198, 160, 246),
            secondary: (138, 173, 244),
            accent: (166, 218, 149),
            background: (36, 39, 58),
            border: (183, 189, 248),
            text: (202, 211, 245),
        }),
        "catppuccin-frappe" | "frappe" => Some(Theme {
            name: "Catppuccin Frappé",
            primary: (202, 158, 230),
            secondary: (140, 170, 238),
            accent: (166, 209, 137),
            background: (48, 52, 70),
            border: (186, 187, 241),
            text: (198, 208, 245),
        }),
        "dracula" => Some(Theme {
            name: "Dracula",
            primary: (189, 147, 249),
            secondary: (255, 121, 198),
            accent: (80, 250, 123),
            background: (40, 42, 54),
            border: (98, 114, 164),
            text: (248, 248, 242),
        }),
        "tokyo-night" | "tokyonight" => Some(Theme {
            name: "Tokyo Night",
            primary: (122, 162, 247),
            secondary: (125, 207, 255),
            accent: (187, 154, 247),
            background: (26, 27, 38),
            border: (86, 95, 137),
            text: (192, 202, 245),
        }),
        "gruvbox" | "gruvbox-dark" => Some(Theme {
            name: "Gruvbox Dark",
            primary: (254, 128, 25),
            secondary: (250, 189, 47),
            accent: (184, 187, 38),
            background: (40, 40, 40),
            border: (102, 92, 84),
            text: (235, 219, 178),
        }),
        "nord" => Some(Theme {
            name: "Nord",
            primary: (136, 192, 208),
            secondary: (129, 161, 193),
            accent: (163, 190, 140),
            background: (46, 52, 64),
            border: (76, 86, 106),
            text: (236, 239, 244),
        }),
        "rose-pine" | "rosepine" => Some(Theme {
            name: "Rosé Pine",
            primary: (235, 188, 186),
            secondary: (156, 207, 216),
            accent: (246, 193, 119),
            background: (25, 23, 36),
            border: (110, 106, 134),
            text: (224, 222, 244),
        }),
        "cyberpunk" => Some(Theme {
            name: "Cyberpunk",
            primary: (252, 238, 10),
            secondary: (0, 240, 255),
            accent: (255, 0, 60),
            background: (8, 9, 12),
            border: (113, 28, 145),
            text: (234, 234, 234),
        }),
        "monokai" => Some(Theme {
            name: "Monokai",
            primary: (249, 38, 114),
            secondary: (166, 226, 46),
            accent: (102, 217, 239),
            background: (39, 40, 34),
            border: (117, 113, 94),
            text: (248, 248, 242),
        }),
        "onedark" | "one-dark" => Some(Theme {
            name: "One Dark",
            primary: (97, 175, 239),
            secondary: (198, 120, 221),
            accent: (152, 195, 121),
            background: (40, 44, 52),
            border: (92, 99, 112),
            text: (171, 178, 191),
        }),
        "solarized-dark" | "solarized" => Some(Theme {
            name: "Solarized Dark",
            primary: (38, 139, 210),
            secondary: (42, 161, 152),
            accent: (133, 153, 0),
            background: (0, 43, 54),
            border: (88, 110, 117),
            text: (131, 148, 150),
        }),
        _ => None,
    }
}

pub fn parse_color(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim();
    if s.starts_with('#') {
        let hex = s.trim_start_matches('#');
        if hex.len() == 6 && hex.is_ascii() {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some((r, g, b));
        } else if hex.len() == 3 && hex.is_ascii() {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some((r, g, b));
        }
    }
    match s.to_lowercase().as_str() {
        "black" => Some((20, 20, 20)),
        "red" => Some((239, 83, 80)),
        "green" => Some((76, 175, 80)),
        "yellow" => Some((255, 193, 7)),
        "blue" => Some((66, 165, 245)),
        "magenta" | "pink" => Some((186, 104, 200)),
        "cyan" => Some((38, 198, 218)),
        "white" => Some((245, 245, 245)),
        "orange" => Some((233, 84, 32)),
        _ => None,
    }
}

pub fn interpolate_color(c1: (u8, u8, u8), c2: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let r = (c1.0 as f32 * (1.0 - t) + c2.0 as f32 * t).round() as u8;
    let g = (c1.1 as f32 * (1.0 - t) + c2.1 as f32 * t).round() as u8;
    let b = (c1.2 as f32 * (1.0 - t) + c2.2 as f32 * t).round() as u8;
    (r, g, b)
}

pub fn interpolate_gradient(colors: &[(u8, u8, u8)], t: f32) -> (u8, u8, u8) {
    if colors.is_empty() {
        return (255, 255, 255);
    }
    if colors.len() == 1 {
        return colors[0];
    }
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let segments = colors.len() - 1;
    let scaled = t * segments as f32;
    let idx = (scaled.floor() as usize).min(segments - 1);
    let local_t = scaled - idx as f32;
    interpolate_color(colors[idx], colors[idx + 1], local_t)
}

pub fn apply_gradient_to_text(text: &str, colors: &[(u8, u8, u8)]) -> String {
    if colors.is_empty() || text.is_empty() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    if n <= 1 {
        let (r, g, b) = colors[0];
        return format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m");
    }
    let mut out = String::with_capacity(text.len() * 20);
    for (i, &ch) in chars.iter().enumerate() {
        let t = i as f32 / (n - 1) as f32;
        let (r, g, b) = interpolate_gradient(colors, t);
        let _ = write!(out, "\x1b[38;2;{r};{g};{b}m{ch}");
    }
    out.push_str("\x1b[0m");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dominant_color_nonexistent() {
        assert!(extract_dominant_color("nonexistent_image_12345.png").is_none());
    }

    #[test]
    fn test_extract_dominant_color_real_image() {
        let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
        let color = extract_dominant_color(&image).expect("repository image must yield a colour");
        assert!(color.starts_with('#'));
        assert_eq!(color.len(), 7);
    }

    fn write_mono_png(name: &str, rgb: [u8; 3]) -> String {
        let path = std::env::temp_dir().join(format!(
            "rustfetch_color_test_{}_{}.png",
            name,
            std::process::id()
        ));
        let img = image::RgbaImage::from_pixel(16, 16, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
        img.save(&path).expect("write colour test png");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn test_monochrome_dominant_color_is_gray() {
        let path = write_mono_png("uniform_dark", [30, 30, 30]);
        let color = extract_dominant_color(&path).expect("must extract");
        let _ = std::fs::remove_file(&path);
        let rgb = parse_color(&color).expect("valid hex colour");
        assert_eq!(
            rgb.0, rgb.1,
            "grey dominant colour must have equal channels"
        );
        assert_eq!(rgb.1, rgb.2);
        assert_ne!(color, "#5485b6", "hard-coded blue fallback was removed");
    }

    #[test]
    fn test_monochrome_palette_is_gray_ramp() {
        let path = write_mono_png("uniform_gray", [96, 96, 96]);
        let palette = extract_color_palette(&path, 8).expect("must yield palette");
        let _ = std::fs::remove_file(&path);
        assert_eq!(palette.len(), 8);
        for (r, g, b) in &palette {
            assert_eq!(r, g);
            assert_eq!(g, b);
        }
    }

    #[test]
    fn test_zero_palette_count_returns_empty() {
        let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
        assert_eq!(extract_color_palette(&image, 0), Some(Vec::new()));
    }

    #[test]
    fn test_get_theme_and_parse_color() {
        let mocha = get_theme("catppuccin-mocha").unwrap();
        assert_eq!(mocha.name, "Catppuccin Mocha");

        let dracula = get_theme("dracula").unwrap();
        assert_eq!(dracula.name, "Dracula");

        let col = parse_color("#ff00ff").unwrap();
        assert_eq!(col, (255, 0, 255));

        let col_short = parse_color("#f0f").unwrap();
        assert_eq!(col_short, (255, 0, 255));

        let named = parse_color("cyan").unwrap();
        assert_eq!(named, (38, 198, 218));
    }

    #[test]
    fn test_gradient_interpolation() {
        let c1 = (0, 0, 0);
        let c2 = (200, 100, 50);
        let mid = interpolate_color(c1, c2, 0.5);
        assert_eq!(mid, (100, 50, 25));

        let text = "HELLO";
        let grad = apply_gradient_to_text(text, &[c1, c2]);
        assert!(grad.contains("\x1b[38;2;"));
        assert!(grad.contains('H'));
        assert!(grad.contains('O'));
    }
}
