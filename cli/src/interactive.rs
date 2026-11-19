use crate::args::{Cli, resolve_shading_chars};
use crate::image_picker::tilde_expand;
use crate::layout::{self, LogoBlock};
use crate::logos;
use crate::text::write_stdout;
use crate::{
    LineWrapGuard, OUTPUT_ERROR, build_configured_palette_lines, build_header, detect_distro_name,
    gather_info_lines, install_termination_handlers, reset_termination_flag, resolve_distro_logo,
    terminate_requested,
};
use rustfetch_core::Module;
use rustfetch_core::config::Config;
use rustfetch_core::modules::module_from_config;
use std::io::Write;

struct TermGuard {
    #[cfg(unix)]
    orig_termios: Option<libc::termios>,
}

impl TermGuard {
    fn enter() -> Self {
        use crossterm::{
            cursor::{Hide, MoveTo},
            execute,
            terminal::{Clear, ClearType},
        };
        install_termination_handlers();
        reset_termination_flag();
        #[cfg(unix)]
        let mut orig_termios = None;
        #[cfg(unix)]
        unsafe {
            let mut orig: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut orig) == 0 {
                orig_termios = Some(orig);
                let mut raw = orig;
                // Retain ISIG so Ctrl+C still raises SIGINT instead of being
                // buffered as an input byte.
                raw.c_lflag &= !(libc::ICANON | libc::ECHO);
                raw.c_cc[libc::VMIN] = 0;
                raw.c_cc[libc::VTIME] = 0;
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &raw);
            }
        }
        #[cfg(not(unix))]
        let _ = crossterm::terminal::enable_raw_mode();

        let mut stdout = std::io::stdout();
        let _ = execute!(stdout, Hide, Clear(ClearType::All), MoveTo(0, 0));
        let _ = stdout.flush();

        Self {
            #[cfg(unix)]
            orig_termios,
        }
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            if let Some(ref orig) = self.orig_termios {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, orig);
            }
        }
        #[cfg(not(unix))]
        {
            let _ = crossterm::terminal::disable_raw_mode();
        }
        let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
        let _ = std::io::stdout().flush();
    }
}

fn compose_animated_frame(
    config: &Config,
    left_lines: &[String],
    (left_width, left_rows): (usize, usize),
    pad_short_left_lines: bool,
    (right_lines, right_width): (&[String], usize),
    (term_cols, term_rows): (usize, usize),
) -> String {
    let layout_stacked = term_cols < left_width + layout::LOGO_GAP + right_width;
    let max_w = term_cols.saturating_sub(1).max(1);
    let max_r = term_rows.saturating_sub(1).max(1);
    let mut rows_written = 0usize;
    let mut frame_buf = String::with_capacity(term_cols * term_rows * 4 + 64);
    frame_buf.push_str("\x1b[H");

    let push_bounded = |buf: &mut String, row: String, rows_written: &mut usize| {
        if *rows_written >= max_r {
            return;
        }
        if layout::strip_ansi_width(&row) > max_w {
            buf.push_str(&layout::truncate_ansi(&row, max_w));
        } else {
            buf.push_str(&row);
        }
        buf.push_str("\x1b[K\r\n");
        *rows_written += 1;
    };

    if layout_stacked {
        let left_pad = if config.general.center {
            " ".repeat(term_cols.saturating_sub(left_width) / 2)
        } else {
            " ".repeat(config.general.padding)
        };
        for line in left_lines {
            push_bounded(
                &mut frame_buf,
                format!("{left_pad}{line}"),
                &mut rows_written,
            );
        }
        push_bounded(&mut frame_buf, String::new(), &mut rows_written);
        let info_pad = if config.general.center {
            " ".repeat(term_cols.saturating_sub(right_width) / 2)
        } else {
            " ".repeat(config.general.padding)
        };
        for line in right_lines {
            push_bounded(
                &mut frame_buf,
                format!("{info_pad}{line}"),
                &mut rows_written,
            );
        }
    } else {
        let total_content_w = left_width + layout::LOGO_GAP + right_width;
        let left_pad = if config.general.center {
            " ".repeat(term_cols.saturating_sub(total_content_w) / 2)
        } else {
            " ".repeat(config.general.padding)
        };
        let total_rows = left_rows.max(right_lines.len());
        let gap = " ".repeat(layout::LOGO_GAP);

        for r in 0..total_rows {
            let mut row = String::with_capacity(total_content_w + 64);
            row.push_str(&left_pad);
            if let Some(line) = left_lines.get(r) {
                row.push_str(line);
                if pad_short_left_lines {
                    let line_w = layout::strip_ansi_width(line);
                    if line_w < left_width {
                        row.push_str(&" ".repeat(left_width - line_w));
                    }
                }
            } else {
                row.push_str(&" ".repeat(left_width));
            }
            row.push_str(&gap);
            if let Some(right_line) = right_lines.get(r) {
                row.push_str(right_line);
            }
            push_bounded(&mut frame_buf, row, &mut rows_written);
        }
    }
    frame_buf.push_str("\x1b[J");
    frame_buf
}

pub(crate) fn run_3d(
    config: &Config,
    cli: &Cli,
    modules: &[Box<dyn Module>],
    configs: &[rustfetch_core::config::ModuleConfig],
) -> Result<(), String> {
    use std::time::Duration;

    let distro_name = if config.general.logo.distro == "auto" {
        detect_distro_name()
    } else {
        config.general.logo.distro.clone()
    };
    let logo = logos::get_logo(&distro_name);

    let shading_mode_str = cli
        .shading_mode
        .as_deref()
        .unwrap_or(&config.general.logo.three_d.shading_mode);
    let shading_mode = rustfetch_render::three_d::ShadingMode::from_str_loose(shading_mode_str);

    let speed = cli.speed.unwrap_or(config.general.logo.three_d.speed);
    let size_scale = cli.size.unwrap_or(config.general.logo.three_d.size);
    let depth_scale = cli.depth.unwrap_or(config.general.logo.three_d.depth);

    let (rotate_x, rotate_y) = match (cli.rotate_x, cli.rotate_y) {
        (true, true) => (true, true),
        (true, false) => (true, false),
        (false, true) => (false, true),
        (false, false) => (
            config.general.logo.three_d.rotate_x,
            config.general.logo.three_d.rotate_y,
        ),
    };

    // Reject empty/whitespace-only custom ramps so the renderer never indexes
    // into an empty character set (used to panic with index out of bounds).
    let shading_chars = resolve_shading_chars(
        cli.shading.as_deref(),
        config.general.logo.three_d.shading.as_deref(),
    );

    let custom_ascii_path = cli
        .ascii
        .as_deref()
        .or(config.general.logo.ascii_path.as_deref());

    let (model, distro_outer, distro_inner) = custom_ascii_path
        .and_then(|ascii_p| {
            let expanded = tilde_expand(ascii_p);
            rustfetch_core::ascii::load_ascii_lines(std::path::Path::new(expanded.as_ref())).ok()
        })
        .filter(|lines| !lines.is_empty())
        .map(|custom_lines| {
            let m = rustfetch_render::three_d::Model3D::from_ascii_lines(
                &custom_lines,
                size_scale,
                depth_scale,
                shading_mode,
            );
            (m, "\x1b[38;2;81;190;240m", "\x1b[38;2;255;255;255m")
        })
        .unwrap_or_else(|| {
            let colored_logo_lines = logo.colored_lines();
            let m = rustfetch_render::three_d::Model3D::from_ascii_lines(
                &colored_logo_lines,
                size_scale,
                depth_scale,
                shading_mode,
            );
            let (d_outer, d_inner) = rustfetch_render::three_d::distro_3d_colors(&distro_name);
            (m, d_outer, d_inner)
        });

    let mut renderer =
        rustfetch_render::three_d::Renderer3D::new(shading_mode, shading_chars.as_deref());
    renderer.set_light_direction(&config.general.logo.three_d.light);
    renderer.size_scale = size_scale;
    renderer.speed = speed;
    renderer.outer_color = config
        .general
        .logo
        .three_d
        .outer_color
        .as_deref()
        .map(rustfetch_render::three_d::parse_color_to_ansi)
        .unwrap_or_else(|| distro_outer.to_string());
    renderer.inner_color = config
        .general
        .logo
        .three_d
        .inner_color
        .as_deref()
        .map(rustfetch_render::three_d::parse_color_to_ansi)
        .unwrap_or_else(|| distro_inner.to_string());

    let palette_lines = build_configured_palette_lines(config, None);

    let header = build_header();
    let header_color = cli
        .color
        .as_deref()
        .or(config.general.logo.three_d.outer_color.as_deref())
        .or(Some(distro_outer));

    let render_opts = layout::RenderOptions {
        header: header.as_deref(),
        header_color,
        separator: &config.general.separator,
        padding: config.general.padding,
        center: config.general.center,
        border: config.general.border,
        border_style: config.general.border_style.as_deref(),
        border_color: config.general.border_color.as_deref(),
        border_chars: config.general.border_chars.as_ref(),
        border_title: config.general.border_title.as_deref(),
        box_padding: config.general.box_padding,
        palette_lines: palette_lines.as_deref(),
        palette_position: &config.general.colors.position,
        icon_only: config.general.icon_only,
        key_type: config.general.key_type.as_deref(),
        border_image: None,
        border_image_position: &config.general.border_image_position,
        border_images: &[],
        logo_position: "left",
    };

    let mut info_lines = gather_info_lines(config, modules, configs, None, Some(distro_outer));
    let mut right_lines = layout::build_right_lines(&info_lines, &render_opts);

    let _guard = TermGuard::enter();

    let mut angle_x = 0.0f32;
    let mut angle_y = 0.0f32;
    let mut frame_idx = 0usize;

    let (telemetry_tx, telemetry_rx) = std::sync::mpsc::channel();
    let bg_config = config.clone();
    let bg_configs = configs.to_vec();
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_bg = running.clone();

    let _bg_handle = std::thread::spawn(move || {
        let (bg_modules, bg_configs): (Vec<Box<dyn Module>>, Vec<_>) = bg_configs
            .into_iter()
            .filter_map(|mc| module_from_config(&mc).map(|m| (m, mc)))
            .unzip();
        while running_bg.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(1000));
            if !running_bg.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let new_info = gather_info_lines(
                &bg_config,
                &bg_modules,
                &bg_configs,
                None,
                Some(distro_outer),
            );
            if telemetry_tx.send(new_info).is_err() {
                break;
            }
        }
    });

    let mut speed_multiplier = 1.0f32;
    let mut paused = false;
    let target_fps = cli.fps.unwrap_or(30.0).clamp(1.0, 120.0);
    let poll_timeout = ((1000.0 / target_fps) as i32).max(1);

    let max_frames = if cli.infinite || config.general.logo.three_d.infinite.unwrap_or(false) {
        None
    } else {
        cli.frames.or(config.general.logo.three_d.frames)
    };

    let hold = cli.hold || config.general.logo.three_d.hold.unwrap_or(false);
    let exit_on_key =
        !hold && (cli.exit_on_key || config.general.logo.three_d.exit_on_key.unwrap_or(true));
    let mut prev_term_size = layout::terminal_size();
    let mut write_failed = false;

    loop {
        if terminate_requested() {
            break;
        }
        if let Some(limit) = max_frames
            && frame_idx >= limit
        {
            break;
        }

        let (term_cols, term_rows) = layout::terminal_size();
        if (term_cols, term_rows) != prev_term_size {
            prev_term_size = (term_cols, term_rows);
            if !write_stdout("\x1b[2J\x1b[H") {
                write_failed = true;
                break;
            }
            right_lines = layout::build_right_lines(&info_lines, &render_opts);
        }

        // Receive non-blocking background telemetry updates
        while let Ok(new_info) = telemetry_rx.try_recv() {
            info_lines = new_info;
            right_lines = layout::build_right_lines(&info_lines, &render_opts);
        }

        // Compute layout
        let right_width = right_lines
            .iter()
            .map(|l| layout::strip_ansi_width(l))
            .max()
            .unwrap_or(30);

        let target_width = cli
            .width
            .or(config.general.logo.three_d.width)
            .unwrap_or(config.general.logo.image_width_cols.min(42))
            .max(25);

        let needed_cols = target_width + layout::LOGO_GAP + right_width;
        let layout_stacked = term_cols < needed_cols;

        let canvas_cols = if layout_stacked {
            target_width.min(term_cols.max(20))
        } else {
            target_width
        };

        let canvas_rows = if layout_stacked {
            let max_stacked_h = (canvas_cols * 3 / 5).clamp(14, 24);
            cli.height
                .or(config.general.logo.three_d.height)
                .unwrap_or(max_stacked_h)
                .min(term_rows.saturating_sub(4).max(8))
        } else {
            let ideal_h = (canvas_cols * 3 / 5).clamp(18, 24);
            cli.height
                .or(config.general.logo.three_d.height)
                .unwrap_or(ideal_h.max(right_lines.len().min(24)))
                .min(term_rows.saturating_sub(2).max(8))
        };

        // Render 3D Logo
        let logo_3d = renderer.render_frame(&model, angle_x, angle_y, canvas_cols, canvas_rows);

        // Update rotation
        if !paused {
            if rotate_x {
                angle_x += 0.04 * speed * speed_multiplier;
            }
            if rotate_y {
                angle_y += 0.06 * speed * speed_multiplier;
            }
        }

        let frame_buf = compose_animated_frame(
            config,
            &logo_3d,
            (canvas_cols, canvas_rows),
            false,
            (&right_lines, right_width),
            (term_cols, term_rows),
        );

        if !write_stdout(&frame_buf) {
            write_failed = true;
            break;
        }

        frame_idx += 1;

        // Poll input: exit on keypress or if hold mode is active, handle controls
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = std::io::stdin().as_raw_fd();
            let mut pfd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pfd, 1, poll_timeout) };
            if ret > 0 {
                if (pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL)) != 0 {
                    running.store(false, std::sync::atomic::Ordering::Relaxed);
                    break;
                }
                if (pfd.revents & libc::POLLIN) != 0 {
                    let mut avail: libc::c_int = 0;
                    unsafe { libc::ioctl(fd, libc::FIONREAD, &mut avail) };
                    if avail <= 0 {
                        running.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                    if exit_on_key {
                        // User started typing: leave all bytes in stdin for the shell and exit immediately!
                        running.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }

                    let mut ibuf = [0u8; 128];
                    let to_read = (avail as usize).min(ibuf.len());
                    let n =
                        unsafe { libc::read(fd, ibuf.as_mut_ptr() as *mut libc::c_void, to_read) };
                    if n > 0 {
                        let bytes = &ibuf[..n as usize];
                        let mut should_exit = false;
                        for &b in bytes {
                            match b {
                                b'q' | b'Q' | 3 => {
                                    should_exit = true;
                                    break;
                                }
                                b'w' | b'W' => angle_x += 0.2,
                                b's' | b'S' => angle_x -= 0.2,
                                b'a' | b'A' => angle_y -= 0.2,
                                b'd' | b'D' => angle_y += 0.2,
                                b'+' | b'=' => {
                                    speed_multiplier = (speed_multiplier * 1.25).min(5.0)
                                }
                                b'-' | b'_' => speed_multiplier = (speed_multiplier * 0.8).max(0.2),
                                b' ' => paused = !paused,
                                _ => {}
                            }
                        }
                        if should_exit {
                            running.store(false, std::sync::atomic::Ordering::Relaxed);
                            break;
                        }
                    }
                }
            }
        }
        #[cfg(not(unix))]
        {
            if let Ok(ready) = crossterm::event::poll(Duration::from_millis(poll_timeout as u64)) {
                if ready {
                    if exit_on_key {
                        let _ = crossterm::event::read();
                        running.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                    if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                        match key.code {
                            crossterm::event::KeyCode::Char('q')
                            | crossterm::event::KeyCode::Char('Q') => {
                                running.store(false, std::sync::atomic::Ordering::Relaxed);
                                break;
                            }
                            crossterm::event::KeyCode::Char('c')
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
                            {
                                running.store(false, std::sync::atomic::Ordering::Relaxed);
                                break;
                            }
                            crossterm::event::KeyCode::Char('w')
                            | crossterm::event::KeyCode::Char('W') => angle_x += 0.2,
                            crossterm::event::KeyCode::Char('s')
                            | crossterm::event::KeyCode::Char('S') => angle_x -= 0.2,
                            crossterm::event::KeyCode::Char('a')
                            | crossterm::event::KeyCode::Char('A') => angle_y -= 0.2,
                            crossterm::event::KeyCode::Char('d')
                            | crossterm::event::KeyCode::Char('D') => angle_y += 0.2,
                            crossterm::event::KeyCode::Char('+')
                            | crossterm::event::KeyCode::Char('=') => {
                                speed_multiplier = (speed_multiplier * 1.25).min(5.0)
                            }
                            crossterm::event::KeyCode::Char('-')
                            | crossterm::event::KeyCode::Char('_') => {
                                speed_multiplier = (speed_multiplier * 0.8).max(0.2)
                            }
                            crossterm::event::KeyCode::Char(' ') => paused = !paused,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    running.store(false, std::sync::atomic::Ordering::Relaxed);
    drop(_guard);

    if write_failed {
        return Err(OUTPUT_ERROR.to_string());
    }

    // Fallback immediately from 3D canvas to static 2D ASCII logo of the same distro
    if !write_stdout("\x1b[2J\x1b[H") {
        return Err(OUTPUT_ERROR.to_string());
    }

    let _wrap_guard = LineWrapGuard::new();
    let (logo_block, _) = build_ascii_logo_block(
        config,
        custom_ascii_path,
        Some(&distro_name),
        cli.color.as_deref(),
    );

    if !layout::render(Some(&logo_block), &info_lines, &render_opts) {
        return Err(OUTPUT_ERROR.to_string());
    }
    Ok(())
}

pub(crate) fn build_ascii_logo_block(
    config: &Config,
    custom_ascii_path: Option<&str>,
    distro_override: Option<&str>,
    dynamic_color: Option<&str>,
) -> (LogoBlock, Option<&'static str>) {
    if let Some(raw) = custom_ascii_path
        .and_then(|ascii_p| {
            let expanded = tilde_expand(ascii_p);
            std::fs::read_to_string(expanded.as_ref()).ok()
        })
        .filter(|raw| !raw.is_empty())
    {
        let processed = if let Some(colors) = &config.general.logo.colors {
            let codes: Vec<String> = colors.iter().map(|c| layout::color_code(c)).collect();
            let refs: Vec<&str> = codes.iter().map(|s| s.as_str()).collect();
            rustfetch_core::apply_neofetch_colors(&raw, &refs)
        } else {
            raw
        };
        let lines: Vec<String> = processed.lines().map(|s| s.to_string()).collect();
        let width = lines
            .iter()
            .map(|l| layout::strip_ansi_width(l))
            .max()
            .unwrap_or(0);
        return (LogoBlock { lines, width }, None);
    }

    let distro_name = distro_override.map(str::to_string).unwrap_or_else(|| {
        if config.general.logo.distro == "auto" {
            detect_distro_name()
        } else {
            config.general.logo.distro.clone()
        }
    });
    let logo = resolve_distro_logo(&distro_name, config.general.padding);
    let logo_colors = config.general.logo.colors.as_ref().map(|colors| {
        colors
            .iter()
            .map(|c| layout::color_code(c))
            .collect::<Vec<String>>()
    });
    let colored_lines = if let Some(colors) = &logo_colors {
        let refs: Vec<&str> = colors.iter().map(|s| s.as_str()).collect();
        logo.colored_lines_with_custom(None, Some(&refs))
    } else if let Some(dc) = dynamic_color {
        let dc_code = layout::color_code(dc);
        logo.colored_lines_with_custom(Some(&dc_code), None)
    } else {
        logo.colored_lines()
    };
    (
        LogoBlock {
            lines: colored_lines,
            width: logo.width,
        },
        Some(logo.color),
    )
}

pub(crate) fn run_ascii_animation(
    config: &Config,
    cli: &Cli,
    modules: &[Box<dyn Module>],
    configs: &[rustfetch_core::config::ModuleConfig],
) -> Result<(), String> {
    use std::time::{Duration, Instant};

    let anim_path = cli
        .ascii_anim
        .as_deref()
        .or(config.general.logo.animation.path.as_deref())
        .or(cli.ascii.as_deref())
        .or(config.general.logo.ascii_path.as_deref());

    let path_str = match anim_path {
        Some(p) => p,
        None => {
            return Err("No ASCII animation path specified. Use --ascii-anim <PATH> or set general.logo.animation.path in config.toml".to_string());
        }
    };

    let expanded = tilde_expand(path_str);
    let p = std::path::Path::new(expanded.as_ref());
    let frames = match rustfetch_core::ascii::load_ascii_frames(p) {
        Ok(f) if !f.is_empty() => f,
        Ok(_) => return Err(format!("No ASCII frames found in {}", p.display())),
        Err(e) => {
            return Err(format!(
                "Error reading ASCII frames from {}: {e}",
                p.display()
            ));
        }
    };

    let fps = cli
        .fps
        .unwrap_or(config.general.logo.animation.fps)
        .clamp(1.0, 120.0);
    let frame_interval = Duration::from_secs_f64(1.0 / fps);

    let palette_lines = build_configured_palette_lines(config, None);

    let distro_name = if config.general.logo.distro == "auto" {
        detect_distro_name()
    } else {
        config.general.logo.distro.clone()
    };
    let anim_distro_color = logos::get_logo(&distro_name).color;

    let header = build_header();
    let header_color = cli.color.as_deref().or(Some(anim_distro_color));

    let render_opts = layout::RenderOptions {
        header: header.as_deref(),
        header_color,
        separator: &config.general.separator,
        padding: config.general.padding,
        center: config.general.center,
        border: config.general.border,
        border_style: config.general.border_style.as_deref(),
        border_color: config.general.border_color.as_deref(),
        border_chars: config.general.border_chars.as_ref(),
        border_title: config.general.border_title.as_deref(),
        box_padding: config.general.box_padding,
        palette_lines: palette_lines.as_deref(),
        palette_position: &config.general.colors.position,
        icon_only: config.general.icon_only,
        key_type: config.general.key_type.as_deref(),
        border_image: None,
        border_image_position: &config.general.border_image_position,
        border_images: &[],
        logo_position: "left",
    };

    let mut info_lines = gather_info_lines(config, modules, configs, None, Some(anim_distro_color));
    let mut right_lines = layout::build_right_lines(&info_lines, &render_opts);

    let _guard = TermGuard::enter();

    let mut frame_idx = 0usize;
    let mut last_metric_refresh = Instant::now();

    let max_frames = if cli.infinite || config.general.logo.animation.infinite {
        None
    } else {
        cli.frames.or(config.general.logo.animation.frames)
    };

    let exit_on_key = !cli.hold
        && !config.general.logo.animation.hold.unwrap_or(false)
        && (cli.exit_on_key || config.general.logo.animation.exit_on_key.unwrap_or(true));

    let mut write_failed = false;

    loop {
        if terminate_requested() {
            break;
        }
        if let Some(limit) = max_frames
            && frame_idx >= limit
        {
            break;
        }

        if last_metric_refresh.elapsed() >= Duration::from_millis(1000) {
            info_lines = gather_info_lines(config, modules, configs, None, Some(anim_distro_color));
            right_lines = layout::build_right_lines(&info_lines, &render_opts);
            last_metric_refresh = Instant::now();
        }

        let (term_cols, term_rows) = layout::terminal_size();

        let current_frame = &frames[frame_idx % frames.len()];
        let frame_width = current_frame
            .iter()
            .map(|l| layout::strip_ansi_width(l))
            .max()
            .unwrap_or(0);

        let right_width = right_lines
            .iter()
            .map(|l| layout::strip_ansi_width(l))
            .max()
            .unwrap_or(30);

        let frame_buf = compose_animated_frame(
            config,
            current_frame,
            (frame_width, current_frame.len()),
            true,
            (&right_lines, right_width),
            (term_cols, term_rows),
        );

        if !write_stdout(&frame_buf) {
            write_failed = true;
            break;
        }

        frame_idx += 1;

        let wait_ms = (frame_interval.as_millis().max(1) as i32).min(1000);

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = std::io::stdin().as_raw_fd();
            let mut pfd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pfd, 1, wait_ms) };
            if ret > 0 {
                if (pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL)) != 0 {
                    break;
                }
                if (pfd.revents & libc::POLLIN) != 0 {
                    let mut avail: libc::c_int = 0;
                    unsafe { libc::ioctl(fd, libc::FIONREAD, &mut avail) };
                    if avail <= 0 {
                        break;
                    }
                    if exit_on_key {
                        break;
                    }
                    let mut ibuf = [0u8; 128];
                    let to_read = (avail as usize).min(ibuf.len());
                    let n =
                        unsafe { libc::read(fd, ibuf.as_mut_ptr() as *mut libc::c_void, to_read) };
                    if n > 0 {
                        let bytes = &ibuf[..n as usize];
                        if bytes.iter().any(|&b| b == b'q' || b == b'Q' || b == 3) {
                            break;
                        }
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            if let Ok(ready) = crossterm::event::poll(Duration::from_millis(wait_ms as u64)) {
                if ready {
                    if exit_on_key {
                        let _ = crossterm::event::read();
                        break;
                    }
                    if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                        if (key.code == crossterm::event::KeyCode::Char('c')
                            && key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL))
                            || key.code == crossterm::event::KeyCode::Char('q')
                            || key.code == crossterm::event::KeyCode::Char('Q')
                        {
                            break;
                        }
                    }
                }
            }
        }
    }

    drop(_guard);
    if write_failed {
        return Err(OUTPUT_ERROR.to_string());
    }
    Ok(())
}
