use std::path::PathBuf;

use super::validation::normalize_config_values;
use super::{Config, ModuleConfig};

impl Config {
    pub fn find_fastfetch_config() -> Option<PathBuf> {
        let check_dir = |dir: PathBuf| {
            let jsonc = dir.join("fastfetch").join("config.jsonc");
            if jsonc.is_file() {
                return Some(jsonc);
            }
            let json = dir.join("fastfetch").join("config.json");
            if json.is_file() {
                return Some(json);
            }
            None
        };

        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let trimmed = xdg.trim();
            if !trimmed.is_empty()
                && let Some(p) = check_dir(PathBuf::from(trimmed))
            {
                return Some(p);
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let trimmed = home.trim();
            if !trimmed.is_empty()
                && let Some(p) = check_dir(PathBuf::from(trimmed).join(".config"))
            {
                return Some(p);
            }
        }
        None
    }

    pub fn import_from_fastfetch(content: &str) -> Option<Self> {
        let stripped = strip_jsonc(content);
        let val: serde_json::Value = serde_json::from_str(&stripped).ok()?;

        let mut config = Self::default();

        if let Some(sep) = val.pointer("/display/separator").and_then(|s| s.as_str()) {
            if sep == " " {
                config.general.separator = " ".to_string();
            } else {
                config.general.separator = sep.trim_end().to_string();
            }
        }

        if let Some(pad) = val
            .pointer("/display/key/paddingLeft")
            .and_then(|p| p.as_u64())
        {
            config.general.padding = pad as usize;
        }

        let mut center_detected = false;
        if let Some(logo) = val.get("logo").and_then(|l| l.as_object()) {
            if let Some(ltype) = logo.get("type").and_then(|t| t.as_str()) {
                if ltype == "none" {
                    config.general.logo.enabled = false;
                } else if ltype == "kitty" {
                    config.general.logo.protocol = "kitty".to_string();
                }
            }

            if let Some(pad_left) = logo
                .get("padding")
                .and_then(|p| p.get("left"))
                .and_then(|v| v.as_i64())
                && pad_left > 0
            {
                center_detected = true;
            }

            if let Some(width) = logo.get("width").and_then(|w| w.as_u64())
                && (15..=60).contains(&width)
            {
                config.general.logo.image_width_cols = width.min(45) as usize;
            }

            if let Some(src) = logo.get("source").and_then(|s| s.as_str())
                && !src.is_empty()
            {
                let is_file_type = logo.get("type").and_then(|t| t.as_str()) == Some("file");
                let path = PathBuf::from(src);
                if is_file_type {
                    config.general.logo.ascii_path = Some(src.to_string());
                } else if let Some(parent) = path.parent() {
                    let parent_str = parent.to_string_lossy();
                    if parent_str.ends_with("/logos") || parent_str.contains("/logos/") {
                        config.general.logo.image_dir = Some(parent_str.to_string());
                        config.general.logo.random_image = true;
                        config.general.logo.auto_color = true;
                    } else if path.is_dir() {
                        config.general.logo.image_dir = Some(src.to_string());
                        config.general.logo.random_image = true;
                        config.general.logo.auto_color = true;
                    } else {
                        config.general.logo.image_path = Some(src.to_string());
                        config.general.logo.auto_color = true;
                    }
                } else {
                    config.general.logo.image_path = Some(src.to_string());
                }
            }
        }

        if config.general.logo.enabled
            && config.general.logo.ascii_path.is_none()
            && config.general.logo.image_dir.is_none()
            && config.general.logo.image_path.is_none()
            && let Ok(home) = std::env::var("HOME")
        {
            let ff_logos = PathBuf::from(home).join(".config/fastfetch/logos");
            if ff_logos.is_dir() {
                config.general.logo.image_dir = Some("~/.config/fastfetch/logos".to_string());
                config.general.logo.random_image = true;
                config.general.logo.auto_color = true;
            }
        }

        if let Some(modules_arr) = val.get("modules").and_then(|m| m.as_array()) {
            let mut converted = Vec::new();
            let mut seen = std::collections::HashSet::new();

            for item in modules_arr {
                if let Some(mod_obj) = item.as_object() {
                    let mod_type = mod_obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if mod_type == "custom" {
                        if let Some(fmt) = mod_obj.get("format").and_then(|f| f.as_str())
                            && fmt.contains('\n')
                        {
                            center_detected = true;
                        }
                        let mut mc = ModuleConfig::new("custom");
                        if let Some(key) = mod_obj.get("key").and_then(|k| k.as_str()) {
                            mc.label = Some(key.to_string());
                        }
                        if let Some(fmt) = mod_obj.get("format").and_then(|f| f.as_str()) {
                            mc.value = Some(fmt.to_string());
                        }
                        if mc.label.is_some() || mc.value.is_some() {
                            converted.push(mc);
                        }
                        continue;
                    }

                    if let Some(mapped_name) = map_fastfetch_type(mod_type) {
                        let allow_multiple = matches!(
                            mapped_name,
                            "break"
                                | "custom"
                                | "divider"
                                | "packages"
                                | "gpu"
                                | "dgpu"
                                | "igpu"
                                | "disk"
                        );
                        if allow_multiple || seen.insert(mapped_name) {
                            let mut mc = ModuleConfig::new(mapped_name);
                            if let Some(icon) = mod_obj.get("keyIcon").and_then(|i| i.as_str())
                                && !icon.trim().is_empty()
                            {
                                mc.icon = Some(icon.to_string());
                            }
                            if let Some(key) = mod_obj.get("key").and_then(|k| k.as_str()) {
                                let k_trim = key.trim();
                                if !k_trim.is_empty()
                                    && !(mapped_name == "desktop"
                                        && (k_trim.eq_ignore_ascii_case("de")
                                            || k_trim.eq_ignore_ascii_case("wm")))
                                {
                                    mc.label = Some(key.to_string());
                                }
                            }
                            if let Some(color) = mod_obj.get("keyColor").and_then(|c| c.as_str()) {
                                mc.color = Some(color.to_string());
                            }
                            if let Some(fmt) = mod_obj.get("format").and_then(|f| f.as_str()) {
                                mc.format = Some(fmt.to_string());
                            }
                            converted.push(mc);
                        }
                    }
                } else if let Some(mod_name) = item.as_str()
                    && let Some(mapped_name) = map_fastfetch_type(mod_name)
                {
                    let allow_multiple = matches!(
                        mapped_name,
                        "break"
                            | "custom"
                            | "divider"
                            | "packages"
                            | "gpu"
                            | "dgpu"
                            | "igpu"
                            | "disk"
                    );
                    if allow_multiple || seen.insert(mapped_name) {
                        converted.push(ModuleConfig::new(mapped_name));
                    }
                }
            }

            if !converted.is_empty() {
                config.modules = converted;
            }
        }

        config.general.center = center_detected;

        let mut warnings = Vec::new();
        normalize_config_values(&mut config, &mut warnings);
        config.load_warnings = warnings;
        Some(config)
    }
}

fn map_fastfetch_type(ff_type: &str) -> Option<&'static str> {
    match ff_type.trim().to_ascii_lowercase().as_str() {
        "os" => Some("os"),
        "host" => Some("host"),
        "board" => Some("board"),
        "kernel" => Some("kernel"),
        "uptime" => Some("uptime"),
        "packages" => Some("packages"),
        "shell" => Some("shell"),
        "de" | "wm" => Some("desktop"),
        "font" | "terminalfont" => Some("font"),
        "terminal" => Some("terminal"),
        "cpu" => Some("cpu"),
        "cpu_usage" | "cpuusage" => Some("cpu_usage"),
        "gpu" => Some("gpu"),
        "dgpu" => Some("dgpu"),
        "igpu" => Some("igpu"),
        "memory" => Some("memory"),
        "swap" => Some("swap"),
        "disk" => Some("disk"),
        "display" | "monitor" => Some("display"),
        "battery" | "poweradapter" => Some("battery"),
        "localip" | "local_ip" => Some("local_ip"),
        "locale" => Some("locale"),
        "break" => Some("break"),
        "divider" | "separator" => Some("divider"),
        "custom" => Some("custom"),
        "media" | "player" => Some("media"),
        "sound" | "audio" => Some("sound"),
        "wifi" => Some("wifi"),
        "bluetooth" => Some("bluetooth"),
        "brightness" => Some("brightness"),
        "processes" => Some("processes"),
        "temp" | "temperature" => Some("temp"),
        "init" | "initsystem" => Some("init"),
        "container" | "vm" => Some("container"),
        "os_age" | "age" | "disk_age" => Some("os_age"),
        "power_profile" | "powerprofile" | "power" => Some("power_profile"),
        "gpu_driver" | "gpudriver" | "vulkan" | "opengl" => Some("gpu_driver"),
        "net_io" | "netio" | "network" => Some("net_io"),
        "update" | "updates" => Some("update"),
        _ => None,
    }
}

pub fn strip_jsonc(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    let mut in_str = false;
    let mut escape = false;

    while i < n {
        let c = chars[i];
        if in_str {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if c == '/' && i + 1 < n {
            if chars[i + 1] == '/' {
                i += 2;
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
                if i < n {
                    out.push('\n');
                }
            } else if chars[i + 1] == '*' {
                i += 2;
                while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 1;
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
        i += 1;
    }

    let out_chars: Vec<char> = out.chars().collect();
    let len = out_chars.len();
    let mut final_out = String::with_capacity(len);
    let mut j = 0;
    in_str = false;
    escape = false;

    while j < len {
        let c = out_chars[j];
        if in_str {
            final_out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            final_out.push(c);
        } else if c == ',' {
            let mut k = j + 1;
            while k < len && out_chars[k].is_whitespace() {
                k += 1;
            }
            if k < len && (out_chars[k] == '}' || out_chars[k] == ']') {
                // skip trailing comma
            } else {
                final_out.push(c);
            }
        } else {
            final_out.push(c);
        }
        j += 1;
    }

    final_out
}
