use super::*;

#[test]
fn render_halfblock_rasterizes_a_repository_image() {
    let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
    let lines = render_halfblock(&image, 4).expect("repository image must render");

    assert!(!lines.is_empty());
    assert!(lines.iter().any(|line| line.contains("\x1b[")));
}

#[test]
fn test_apply_image_background_to_lines() {
    let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
    let input_lines = vec![
        "\x1b[34mOS:\x1b[0m Arch Linux".to_string(),
        "\x1b[32mHost:\x1b[0m Laptop".to_string(),
    ];
    let bg_lines = apply_image_background_to_lines(&input_lines, &image, 30, 4)
        .expect("should apply background");
    assert_eq!(bg_lines.len(), 4);
    for line in &bg_lines {
        assert!(line.contains("\x1b[48;2;"));
        assert!(line.ends_with("\x1b[0m"));
    }
    fn strip_ansi(s: &str) -> String {
        let mut in_escape = false;
        let mut clean = String::new();
        for ch in s.chars() {
            if ch == '\x1b' {
                in_escape = true;
                continue;
            }
            if in_escape {
                if ch.is_ascii_alphabetic() {
                    in_escape = false;
                }
                continue;
            }
            clean.push(ch);
        }
        clean
    }

    let stripped0 = strip_ansi(&bg_lines[0]);
    assert!(stripped0.contains("OS: Arch Linux"));
    let stripped1 = strip_ansi(&bg_lines[1]);
    assert!(stripped1.contains("Host: Laptop"));
}

#[test]
fn test_render_background_valid() {
    let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
    assert!(render_background(&image, 40, 10, -1));
}

#[test]
fn test_render_ex_respects_max_rows() {
    let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
    if let Some((cols, rows)) = render_ex(&image, 40, Some(10)) {
        assert!(rows <= 10, "Rendered rows {} should be <= 10", rows);
        assert!(cols <= 40, "Rendered cols {} should be <= 40", cols);
    }
}

fn write_temp_png(name: &str, width: u32, height: u32) -> String {
    let path = std::env::temp_dir().join(format!(
        "rustfetch_image_test_{}_{}.png",
        name,
        std::process::id()
    ));
    let img = image::RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, 255]));
    img.save(&path).expect("write test png");
    path.to_string_lossy().into_owned()
}

#[test]
fn clamp_pixel_dims_enforces_budget_and_parity() {
    assert_eq!(clamp_pixel_dims(10, 10, 8, None, 1, 2), (8, 10));
    assert_eq!(
        clamp_pixel_dims(5000, 10_000, 2000, Some(24), 1, 2),
        (2000, 48)
    );
    let (cols, rows) = clamp_pixel_dims(0, 0, 4, None, 1, 2);
    assert!(cols >= 1 && rows >= 2 && rows % 2 == 0);
    let (cols, rows) = clamp_pixel_dims(9999, 9999, 1000, Some(0), 1, 2);
    assert!(cols <= MAX_IMAGE_PIXEL_WIDTH);
    assert!(rows <= 2);
    assert_eq!(rows % 2, 0);
}

#[test]
fn tall_image_is_rejected_without_expanding() {
    // 1 x 200000 previously expanded to 2400000 output lines (26 s, ~1.4 GB).
    // The shared 8192 px dimension cap must reject it immediately.
    let path = write_temp_png("tall_halfblock", 1, 200_000);
    let start = std::time::Instant::now();
    let rendered = render_halfblock_ex(&path, 24, None, true);
    let _ = std::fs::remove_file(&path);
    assert!(rendered.is_none(), "oversized tall image must be rejected");
    assert!(
        start.elapsed() < std::time::Duration::from_secs(5),
        "rejection must be immediate"
    );
}

#[test]
fn very_tall_but_allowed_image_is_row_bounded() {
    // 1 x 8000 passes the 8192 px dimension cap but would expand to 192000
    // pixel rows without the shared clamp.
    let path = write_temp_png("tall_allowed", 1, 8000);
    let start = std::time::Instant::now();
    let lines = render_halfblock_ex(&path, 24, None, true).expect("must render");
    let _ = std::fs::remove_file(&path);
    assert!(!lines.is_empty());
    assert!(lines.len() <= DEFAULT_IMAGE_ROWS);
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn clamp_pixel_dims_bounds_extreme_aspect_ratios() {
    // Same math the half-block renderer applies before rasterizing.
    let (cols, rows) = clamp_pixel_dims(24, 4_800_000, 24, None, 1, 2);
    assert_eq!((cols, rows), (24, 48));
    let (cols, rows) = clamp_pixel_dims(48, 4_800_000, 24, None, 2, 4);
    assert_eq!((cols, rows), (48, 96));
}

#[test]
fn zero_target_rows_is_clamped() {
    let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
    let lines = render_halfblock_ex(&image, 8, Some(0), true).expect("must render");
    assert!(!lines.is_empty());
}

#[test]
fn oversized_image_dimensions_are_rejected() {
    let path = write_temp_png("oversized", 9000, 1);
    assert!(
        open_image_reader(&path).is_none(),
        "images wider than the 8192 px cap must be rejected"
    );
    assert!(render_halfblock_ex(&path, 8, Some(4), true).is_none());
    assert!(image_display_dimensions(&path, 40, Some(10)).is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn image_reader_accepts_normal_images() {
    let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
    assert!(open_image_reader(&image).is_some());
    assert!(image_dimensions_checked(&image).is_some());
}

#[test]
fn animated_gif_uses_kitty_animation_path() {
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame, RgbaImage};

    let path = std::env::temp_dir().join(format!("rustfetch_gif_test_{}.gif", std::process::id()));
    {
        let file = std::fs::File::create(&path).expect("create gif");
        let mut encoder = GifEncoder::new(file);
        let frames: Vec<Frame> = (0..2)
            .map(|index| {
                Frame::from_parts(
                    RgbaImage::from_pixel(4, 4, image::Rgba([index * 100, 10, 10, 255])),
                    0,
                    0,
                    Delay::from_numer_denom_ms(50, 1),
                )
            })
            .collect();
        encoder.encode_frames(frames).expect("encode gif");
    }
    let rendered = render_positioned(path.to_str().expect("utf8 path"), 4, Some(4), 0, 0, false);
    let placeholder =
        render_kitty_placeholder(path.to_str().expect("utf8 path"), 4, Some(4), false);
    let _ = std::fs::remove_file(&path);
    let (cols, rows) = rendered.expect("animated GIF must render");
    assert!(cols >= 1 && rows >= 1);
    assert!(cols <= 4 && rows <= 4);

    let (pcols, prows, prows_vec) = placeholder.expect("animated GIF placeholder must render");
    assert_eq!(pcols, cols);
    assert_eq!(prows, rows);
    assert_eq!(prows_vec.len(), prows);
    assert!(prows_vec[0].contains(crate::kitty::PLACEHOLDER_CHAR));
}

#[test]
fn static_png_renders_kitty_placeholder_rows() {
    let image = format!("{}/../images/arch.png", env!("CARGO_MANIFEST_DIR"));
    let (cols, rows, lines) =
        render_kitty_placeholder(&image, 8, Some(6), false).expect("static placeholder renders");
    assert!((1..=8).contains(&cols));
    assert!((1..=6).contains(&rows));
    assert_eq!(lines.len(), rows);
    assert!(
        lines
            .iter()
            .all(|line| line.contains(crate::kitty::PLACEHOLDER_CHAR))
    );
}

#[test]
fn test_apply_image_background_preserves_osc8_hyperlink() {
    let mut path = std::env::temp_dir();
    path.push(format!("rustfetch_osc8_bg_{}.png", std::process::id()));
    {
        use image::{ImageFormat, RgbaImage};
        let img = RgbaImage::from_pixel(2, 2, image::Rgba([30, 60, 90, 255]));
        img.save_with_format(&path, ImageFormat::Png)
            .expect("write png");
    }
    let line = "\x1b]8;;https://example.com\x07Update available\x1b]8;;\x07".to_string();
    let rendered =
        apply_image_background_to_lines(&[line], path.to_str().expect("utf8 path"), 20, 1)
            .expect("background rendering succeeds");
    let _ = std::fs::remove_file(&path);
    assert_eq!(rendered.len(), 1);
    assert!(
        rendered[0].contains("\x1b]8;;https://example.com\x07"),
        "missing OSC-8 start in {:?}",
        rendered[0]
    );
    assert!(
        rendered[0].contains("\x1b]8;;\x07"),
        "missing OSC-8 end in {:?}",
        rendered[0]
    );
    assert!(
        !rendered[0].contains("\x1b]8;;\x1b["),
        "background SGR injected inside OSC sequence: {:?}",
        rendered[0]
    );
}
