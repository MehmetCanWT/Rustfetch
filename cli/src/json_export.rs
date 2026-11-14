//! JSON and Waybar structured export formats.
//!
//! All output goes through crate::text::write_stdout_line so that a full disk
//! or a closed pipe never turns into a print! panic.

use crate::layout::{self, InfoLine};
use crate::text;
use rustfetch_core::config::{Config, ModuleConfig, default_icon_for_module};
use rustfetch_core::{Module, detect_all};
use serde::Serialize;

#[derive(Serialize)]
pub struct JsonModuleOutput {
    pub name: String,
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

pub fn run_json(
    config: &Config,
    modules: &[Box<dyn Module>],
    configs: &[ModuleConfig],
) -> Result<(), String> {
    let results = detect_all(modules);
    let mut outputs = Vec::new();

    for (result, mc) in results.into_iter().zip(configs.iter()) {
        if mc.name == "break" || mc.name == "divider" {
            continue;
        }
        let (label, mut value) = if mc.name == "custom" {
            (
                mc.label.clone().unwrap_or_else(|| "custom".to_string()),
                mc.value.clone().unwrap_or_default(),
            )
        } else if let Some(info) = result {
            (mc.label.clone().unwrap_or(info.label), info.value)
        } else {
            continue;
        };

        if let Some(fmt) = &mc.format {
            value = text::expand_value_template(fmt, &value);
        }

        let icon = if config.general.icons {
            if let Some(user_icon) = &mc.icon {
                Some(user_icon.clone())
            } else if mc.name == "os" {
                Some(rustfetch_core::distro_icon(&value).to_string())
            } else {
                default_icon_for_module(&mc.name).map(str::to_string)
            }
        } else {
            None
        };

        outputs.push(JsonModuleOutput {
            name: mc.name.clone(),
            label,
            value,
            icon,
        });
    }

    let json_str = serde_json::to_string_pretty(&outputs)
        .map_err(|error| format!("failed to serialize JSON output: {error}"))?;
    if !text::write_stdout_line(&json_str) {
        return Err("failed to write output to stdout".to_string());
    }
    Ok(())
}

#[derive(Serialize)]
pub struct WaybarOutput {
    pub text: String,
    pub tooltip: String,
    pub class: &'static str,
}

pub fn generate_waybar_json(
    config: &Config,
    info_lines: &[InfoLine],
    header: Option<&str>,
) -> String {
    let render_opts = layout::RenderOptions {
        header,
        header_color: None,
        separator: &config.general.separator,
        padding: 0,
        center: false,
        border: false,
        border_style: None,
        border_color: None,
        border_chars: None,
        border_title: None,
        box_padding: 0,
        palette_lines: None,
        palette_position: "none",
        icon_only: false,
        key_type: None,
        border_image: None,
        border_image_position: "none",
        border_images: &[],
        logo_position: "none",
    };

    let right_lines = layout::build_right_lines(info_lines, &render_opts);
    let tooltip_lines: Vec<String> = right_lines
        .into_iter()
        .map(|line| layout::strip_ansi(&line))
        .collect();
    let tooltip = tooltip_lines.join("\n").trim().to_string();

    let raw_text = if let Some(os_line) = info_lines
        .iter()
        .find(|line| line.label.eq_ignore_ascii_case("os"))
    {
        let clean_val = layout::strip_ansi(&os_line.value);
        if let Some(icon) = &os_line.icon {
            format!("{icon} {clean_val}")
        } else {
            clean_val
        }
    } else if let Some(first_line) = info_lines
        .iter()
        .find(|line| !line.value.is_empty() && !line.label.starts_with("---DIVIDER"))
    {
        let clean_val = layout::strip_ansi(&first_line.value);
        if let Some(icon) = &first_line.icon {
            format!("{icon} {clean_val}")
        } else {
            clean_val
        }
    } else {
        "rustfetch".to_string()
    };
    let text = layout::strip_ansi(&raw_text).trim().to_string();

    let output = WaybarOutput {
        text,
        tooltip,
        class: "rustfetch",
    };

    serde_json::to_string(&output).unwrap_or_default()
}

pub fn run_waybar(
    config: &Config,
    info_lines: &[InfoLine],
    header: Option<&str>,
) -> Result<(), String> {
    let json_str = generate_waybar_json(config, info_lines, header);
    if json_str.is_empty() {
        return Err("failed to serialize waybar output".to_string());
    }
    if !text::write_stdout_line(&json_str) {
        return Err("failed to write output to stdout".to_string());
    }
    Ok(())
}

/// Unit multipliers (in bytes) for the size units emitted by the modules.
fn unit_multiplier(unit: &str) -> f64 {
    if unit.starts_with("kib") {
        1024.0
    } else if unit.starts_with("mib") {
        1024.0 * 1024.0
    } else if unit.starts_with("gib") {
        1024.0 * 1024.0 * 1024.0
    } else if unit.starts_with("tib") {
        1024.0 * 1024.0 * 1024.0 * 1024.0
    } else if unit.starts_with("kb") {
        1000.0
    } else if unit.starts_with("mb") {
        1000.0 * 1000.0
    } else if unit.starts_with("gb") {
        1000.0 * 1000.0 * 1000.0
    } else if unit.starts_with("tb") {
        1000.0 * 1000.0 * 1000.0 * 1000.0
    } else {
        1.0
    }
}

pub fn extract_percentage(val: &str) -> Option<u8> {
    if let Some(pos) = val.find('%') {
        let prefix = &val[..pos];
        let num_str: String = prefix
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let num_str: String = num_str.chars().rev().collect();
        if let Ok(p) = num_str.parse::<f64>()
            && p.is_finite()
            && p >= 0.0
        {
            return Some(p.round().clamp(0.0, 100.0) as u8);
        }
    }

    if let Some((left, right)) = val.split_once('/') {
        let parse_num = |s: &str| -> Option<f64> {
            let mut words = s.split_whitespace();
            let n = words.next()?.parse::<f64>().ok()?;
            let unit = words.next().map(|u| u.to_lowercase()).unwrap_or_default();
            Some(n * unit_multiplier(&unit))
        };
        if let (Some(used), Some(total)) = (parse_num(left.trim()), parse_num(right.trim()))
            && total > 0.0
        {
            let pct = ((used / total) * 100.0).round();
            return Some((pct as u8).min(100));
        }
    }

    if let Some(pos) = val.find("°C") {
        let prefix = &val[..pos];
        let num_str: String = prefix
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let num_str: String = num_str.chars().rev().collect();
        if let Ok(temp_c) = num_str.parse::<f64>() {
            return Some((temp_c.round() as u8).min(100));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_percentage() {
        assert_eq!(extract_percentage("Battery: 85%"), Some(85));
        assert_eq!(extract_percentage("100%"), Some(100));
        assert_eq!(extract_percentage("0%"), Some(0));
        assert_eq!(extract_percentage("99.0%"), Some(99));
        assert_eq!(extract_percentage("12.6%"), Some(13));
        assert_eq!(extract_percentage("4.0 GiB / 16.0 GiB"), Some(25));
        assert_eq!(extract_percentage("512 MiB / 1024 MiB"), Some(50));
        assert_eq!(extract_percentage("CPU: 45.2°C"), Some(45));
        assert_eq!(extract_percentage("No number"), None);
    }

    #[test]
    fn test_extract_percentage_mixed_units() {
        // Binary and decimal units must be scaled to bytes before dividing.
        assert_eq!(extract_percentage("512 MiB / 1 GiB"), Some(50));
        assert_eq!(extract_percentage("1 GiB / 2 GiB"), Some(50));
        assert_eq!(extract_percentage("1 GB / 2 GB"), Some(50));
        assert_eq!(extract_percentage("500 MB / 1 GiB"), Some(47));
        assert_eq!(extract_percentage("1 KiB / 1 MiB"), Some(0));
    }

    #[test]
    fn test_generate_waybar_json() {
        let config = Config::default();
        let lines = vec![
            InfoLine {
                label: "OS".into(),
                value: "Arch Linux".into(),
                icon: Some("x".into()),
                color: "".into(),
            },
            InfoLine {
                label: "Uptime".into(),
                value: "2h 15m".into(),
                icon: None,
                color: "".into(),
            },
        ];
        let json_str = generate_waybar_json(&config, &lines, Some("user@host"));
        assert!(json_str.contains("\"class\":\"rustfetch\""));
        assert!(json_str.contains("Arch Linux"));
        assert!(json_str.contains("2h 15m"));
        // Waybar requires a single line of JSON.
        assert!(!json_str.contains('\n'));
    }
}
