use super::Config;

/// Shading modes understood by the 3D renderer.
pub const SHADING_MODES: &[&str] = &[
    "ascii",
    "blocks",
    "sextants",
    "braille",
    "dot",
    "wireframe",
    "solid",
];

/// Progress-bar glyph styles understood by the layout engine.
pub const BAR_STYLES: &[&str] = &["blocks", "line", "dots", "ascii"];

/// Theme identifiers accepted by `rustfetch_render::get_theme`.
///
/// Kept here as a dependency-free copy (core must not depend on the render
/// crate); `render/src/color.rs` is the authority and this list must be kept
/// in sync with its canonical names and aliases.
pub(super) const KNOWN_THEMES: &[&str] = &[
    "catppuccin-mocha",
    "catppuccin",
    "mocha",
    "catppuccin-latte",
    "latte",
    "catppuccin-macchiato",
    "macchiato",
    "catppuccin-frappe",
    "frappe",
    "dracula",
    "tokyo-night",
    "tokyonight",
    "gruvbox",
    "gruvbox-dark",
    "nord",
    "rose-pine",
    "rosepine",
    "cyberpunk",
    "monokai",
    "onedark",
    "one-dark",
    "solarized-dark",
    "solarized",
];

/// Returns true when `name` is a module identifier accepted by [`crate::modules::module_by_name`]
/// (including the `disk:<mount>` form and supported aliases).
pub fn is_known_module_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    if let Some(mount) = trimmed.strip_prefix("disk:") {
        return !mount.trim().is_empty();
    }
    crate::modules::module_names().contains(&trimmed)
        || matches!(
            trimmed,
            "age" | "driver" | "power" | "network" | "net" | "initsystem" | "vm" | "separator"
        )
}

/// Maps 3D shading mode names and aliases to their canonical identifier.
pub fn canonical_shading_mode(name: &str) -> Option<&'static str> {
    match name.trim().to_lowercase().as_str() {
        "ascii" => Some("ascii"),
        "blocks" | "block" | "halfblock" | "halfblocks" => Some("blocks"),
        "sextants" | "sextant" | "quadrants" | "quadrant" => Some("sextants"),
        "braille" => Some("braille"),
        "dot" | "dots" | "dotted" => Some("dot"),
        "wireframe" | "wire" | "wire-frame" => Some("wireframe"),
        "solid" | "filled" => Some("solid"),
        _ => None,
    }
}

/// Returns true when `name` is a supported 3D shading mode or alias (case-insensitive).
pub fn is_known_shading_mode(name: &str) -> bool {
    canonical_shading_mode(name).is_some()
}

/// Returns true when `name` is a supported progress-bar style (case-insensitive).
pub fn is_known_bar_style(name: &str) -> bool {
    BAR_STYLES.contains(&name.trim().to_lowercase().as_str())
}

/// Returns true when `name` resolves to a theme in the render crate.
pub fn is_known_theme_name(name: &str) -> bool {
    let normalized = name.trim().to_lowercase().replace('_', "-");
    KNOWN_THEMES.contains(&normalized.as_str())
}

pub(super) fn push_warning(warnings: &mut Vec<String>, message: String) {
    if !warnings.contains(&message) {
        warnings.push(message);
    }
}

fn clamp_usize(value: &mut usize, min: usize, max: usize, path: &str, warnings: &mut Vec<String>) {
    if *value < min {
        push_warning(
            warnings,
            format!("{path} {value} is below the supported minimum {min}; clamped to {min}"),
        );
        *value = min;
    } else if *value > max {
        push_warning(
            warnings,
            format!("{path} {value} exceeds the supported maximum {max}; clamped to {max}"),
        );
        *value = max;
    }
}

fn clamp_optional_usize(
    value: &mut Option<usize>,
    min: usize,
    max: usize,
    path: &str,
    warnings: &mut Vec<String>,
) {
    if let Some(current) = value {
        if *current < min {
            push_warning(
                warnings,
                format!("{path} {current} is below the supported minimum {min}; clamped to {min}"),
            );
            *value = Some(min);
        } else if *current > max {
            push_warning(
                warnings,
                format!("{path} {current} exceeds the supported maximum {max}; clamped to {max}"),
            );
            *value = Some(max);
        }
    }
}

fn clamp_i32(value: &mut i32, min: i32, max: i32, path: &str, warnings: &mut Vec<String>) {
    if *value < min {
        push_warning(
            warnings,
            format!("{path} {value} is below the supported minimum {min}; clamped to {min}"),
        );
        *value = min;
    } else if *value > max {
        push_warning(
            warnings,
            format!("{path} {value} exceeds the supported maximum {max}; clamped to {max}"),
        );
        *value = max;
    }
}

fn clamp_optional_i32(
    value: &mut Option<i32>,
    min: i32,
    max: i32,
    path: &str,
    warnings: &mut Vec<String>,
) {
    if let Some(mut current) = *value {
        clamp_i32(&mut current, min, max, path, warnings);
        *value = Some(current);
    }
}

fn clamp_f32(
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    path: &str,
    warnings: &mut Vec<String>,
) {
    if !value.is_finite() {
        push_warning(
            warnings,
            format!("{path} is not a finite number; using the default {default}"),
        );
        *value = default;
    } else if *value < min {
        push_warning(
            warnings,
            format!("{path} {value} is below the supported minimum {min}; clamped to {min}"),
        );
        *value = min;
    } else if *value > max {
        push_warning(
            warnings,
            format!("{path} {value} exceeds the supported maximum {max}; clamped to {max}"),
        );
        *value = max;
    }
}

fn clamp_f64(
    value: &mut f64,
    min: f64,
    max: f64,
    default: f64,
    path: &str,
    warnings: &mut Vec<String>,
) {
    if !value.is_finite() {
        push_warning(
            warnings,
            format!("{path} is not a finite number; using the default {default}"),
        );
        *value = default;
    } else if *value < min {
        push_warning(
            warnings,
            format!("{path} {value} is below the supported minimum {min}; clamped to {min}"),
        );
        *value = min;
    } else if *value > max {
        push_warning(
            warnings,
            format!("{path} {value} exceeds the supported maximum {max}; clamped to {max}"),
        );
        *value = max;
    }
}

/// Single place where loaded configuration values are normalized and bounded.
///
/// Every load path (TOML, salvage recovery, Fastfetch import) funnels through
/// this function so no module ever sees a hostile value such as `padding = 99999`,
/// a NaN float or an unknown `shading_mode`.
pub(super) fn normalize_config_values(cfg: &mut Config, warnings: &mut Vec<String>) {
    clamp_usize(&mut cfg.general.padding, 0, 64, "general.padding", warnings);
    clamp_usize(
        &mut cfg.general.box_padding,
        0,
        16,
        "general.box_padding",
        warnings,
    );
    clamp_usize(
        &mut cfg.general.colors.rows,
        1,
        2,
        "general.colors.rows",
        warnings,
    );
    clamp_usize(
        &mut cfg.general.logo.image_width_cols,
        1,
        256,
        "general.logo.image_width_cols",
        warnings,
    );

    clamp_f32(
        &mut cfg.general.logo.three_d.speed,
        0.05,
        10.0,
        1.0,
        "general.logo.three_d.speed",
        warnings,
    );
    clamp_f32(
        &mut cfg.general.logo.three_d.size,
        0.1,
        3.0,
        1.0,
        "general.logo.three_d.size",
        warnings,
    );
    clamp_f32(
        &mut cfg.general.logo.three_d.depth,
        0.1,
        5.0,
        1.0,
        "general.logo.three_d.depth",
        warnings,
    );
    clamp_optional_usize(
        &mut cfg.general.logo.three_d.width,
        1,
        512,
        "general.logo.three_d.width",
        warnings,
    );
    clamp_optional_usize(
        &mut cfg.general.logo.three_d.height,
        1,
        512,
        "general.logo.three_d.height",
        warnings,
    );
    clamp_f64(
        &mut cfg.general.logo.animation.fps,
        1.0,
        120.0,
        15.0,
        "general.logo.animation.fps",
        warnings,
    );

    clamp_optional_usize(
        &mut cfg.general.border_image_width,
        1,
        512,
        "general.border_image_width",
        warnings,
    );
    clamp_optional_usize(
        &mut cfg.general.border_image_height,
        1,
        512,
        "general.border_image_height",
        warnings,
    );
    clamp_optional_i32(
        &mut cfg.general.border_image_offset_x,
        -10_000,
        10_000,
        "general.border_image_offset_x",
        warnings,
    );
    clamp_optional_i32(
        &mut cfg.general.border_image_offset_y,
        -10_000,
        10_000,
        "general.border_image_offset_y",
        warnings,
    );
    clamp_optional_i32(
        &mut cfg.general.border_image_z_index,
        -100,
        100,
        "general.border_image_z_index",
        warnings,
    );

    for (index, image) in cfg.general.border_images.iter_mut().enumerate() {
        let prefix = format!("general.border_images[{index}]");
        clamp_optional_usize(
            &mut image.width,
            1,
            512,
            &format!("{prefix}.width"),
            warnings,
        );
        clamp_optional_usize(
            &mut image.height,
            1,
            512,
            &format!("{prefix}.height"),
            warnings,
        );
        clamp_optional_i32(
            &mut image.offset_x,
            -10_000,
            10_000,
            &format!("{prefix}.offset_x"),
            warnings,
        );
        clamp_optional_i32(
            &mut image.offset_y,
            -10_000,
            10_000,
            &format!("{prefix}.offset_y"),
            warnings,
        );
        clamp_i32(
            &mut image.z_index,
            -100,
            100,
            &format!("{prefix}.z_index"),
            warnings,
        );
    }

    for module in &mut cfg.modules {
        module.name = module.name.trim().to_string();
        if let Some(width) = &mut module.bar_width {
            clamp_usize(
                width,
                1,
                256,
                &format!("module '{}'.bar_width", module.name),
                warnings,
            );
        }
        if let Some(style) = module.bar_style.take() {
            let trimmed = style.trim();
            if trimmed.is_empty() {
                continue;
            }
            let normalized = trimmed.to_lowercase();
            if is_known_bar_style(&normalized) {
                module.bar_style = Some(normalized);
            } else {
                push_warning(
                    warnings,
                    format!(
                        "module '{}' has unknown bar_style '{}'; expected one of {}",
                        module.name,
                        trimmed,
                        BAR_STYLES.join(", ")
                    ),
                );
            }
        }
    }

    if let Some(canonical) = canonical_shading_mode(&cfg.general.logo.three_d.shading_mode) {
        cfg.general.logo.three_d.shading_mode = canonical.to_string();
    } else {
        push_warning(
            warnings,
            format!(
                "unknown shading_mode '{}'; expected one of {}; using 'ascii'",
                cfg.general.logo.three_d.shading_mode.trim(),
                SHADING_MODES.join(", ")
            ),
        );
        cfg.general.logo.three_d.shading_mode = "ascii".to_string();
    }
}

/// Collects non-clamping diagnostics about the current configuration values.
pub(super) fn collect_config_warnings(cfg: &Config, warnings: &mut Vec<String>) {
    for module in &cfg.modules {
        let name = module.name.trim();
        if name.is_empty() {
            push_warning(
                warnings,
                "module entry without a name is ignored".to_string(),
            );
        } else if !is_known_module_name(name) {
            push_warning(
                warnings,
                format!(
                    "unknown module '{name}' is ignored; run `rustfetch --list-modules` for the supported names"
                ),
            );
        }
        if let Some(style) = &module.bar_style
            && !is_known_bar_style(style)
        {
            push_warning(
                warnings,
                format!(
                    "module '{name}' has unknown bar_style '{}'; expected one of {}",
                    style.trim(),
                    BAR_STYLES.join(", ")
                ),
            );
        }
    }

    if let Some(theme) = &cfg.general.theme {
        let trimmed = theme.trim();
        if !trimmed.is_empty() && !is_known_theme_name(trimmed) {
            push_warning(
                warnings,
                format!("unknown theme '{trimmed}' (ignored by the renderer)"),
            );
        }
    }

    if !is_known_shading_mode(&cfg.general.logo.three_d.shading_mode) {
        push_warning(
            warnings,
            format!(
                "unknown shading_mode '{}'; expected one of {}",
                cfg.general.logo.three_d.shading_mode.trim(),
                SHADING_MODES.join(", ")
            ),
        );
    }
}
