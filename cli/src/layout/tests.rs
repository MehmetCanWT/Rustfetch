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

#[test]
fn kitty_placeholder_frame_is_sequential_with_no_cursor_up() {
    let info = vec![info_line("OS", "Arch Linux"), info_line("Kernel", "6.12")];
    let options = opts(false);
    let right_lines = build_right_lines(&info, &options);
    let placeholders = vec![
        "\x1b[38;2;0;0;1m\u{10EEEE}\u{0305}\u{0305}\u{10EEEE}\u{0305}\u{030D}\x1b[39m".to_string(),
        "\x1b[38;2;0;0;1m\u{10EEEE}\u{030D}\u{0305}\u{10EEEE}\u{030D}\u{030D}\x1b[39m".to_string(),
        "\x1b[38;2;0;0;1m\u{10EEEE}\u{030E}\u{0305}\u{10EEEE}\u{030E}\u{030D}\x1b[39m".to_string(),
        "\x1b[38;2;0;0;1m\u{10EEEE}\u{0310}\u{0305}\u{10EEEE}\u{0310}\u{030D}\x1b[39m".to_string(),
    ];

    let (frame, total_rows) =
        build_kitty_placeholder_horizontal_frame(&placeholders, 2, 1, &right_lines, true, false);
    assert_eq!(total_rows, placeholders.len().max(right_lines.len()));
    let lines: Vec<&str> = frame.lines().collect();
    assert_eq!(lines.len(), total_rows);
    assert!(
        lines
            .iter()
            .any(|l| l.contains('\u{10EEEE}') && l.contains("Arch Linux")),
        "placeholder row and card row must be emitted on the same sequential line: {frame:?}"
    );
    for csi in frame.split("\x1b[").skip(1) {
        let final_byte = csi.bytes().find(|b| (0x40..=0x7E).contains(b)).unwrap_or(0);
        assert!(
            !matches!(final_byte, b'A' | b'B' | b'C' | b'D'),
            "placeholder frame must not emit cursor-movement CSI escapes: {frame:?}"
        );
    }
}
