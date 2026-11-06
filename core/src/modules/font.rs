use crate::{Info, Module};
use std::fs;
use std::path::PathBuf;

pub struct FontModule;

impl Module for FontModule {
    fn name(&self) -> &'static str {
        "font"
    }

    fn detect(&self) -> Option<Info> {
        let config_dir = config_dir_from(
            env_value("XDG_CONFIG_HOME").as_deref(),
            env_value("HOME").as_deref(),
        )?;

        if let Ok(content) = fs::read_to_string(config_dir.join("kdeglobals")) {
            if let Some(font) = parse_kde_font(&content) {
                return Some(Info::new("Font", font));
            }
        }

        for gtk_ver in ["gtk-4.0", "gtk-3.0"] {
            if let Ok(content) = fs::read_to_string(config_dir.join(gtk_ver).join("settings.ini")) {
                if let Some(font) = parse_gtk_font(&content) {
                    return Some(Info::new("Font", format!("{font} [GTK]")));
                }
            }
        }

        None
    }
}

fn env_value(name: &str) -> Option<String> {
    std::env::var_os(name)
        .map(|value| value.to_string_lossy().trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Resolves the XDG config directory from already-trimmed environment values.
///
/// An *empty but set* `XDG_CONFIG_HOME` must fall back to `$HOME/.config`;
/// using it verbatim would make the module look in `/kdeglobals` and silently
/// disappear (this was a real bug).
pub fn config_dir_from(xdg_config_home: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    if let Some(xdg) = xdg_config_home.map(str::trim).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(xdg));
    }
    if let Some(home) = home.map(str::trim).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(home).join(".config"));
    }
    None
}

/// Parses the Qt/KDE UI font from `kdeglobals` (`font=Noto Sans,10,-1,...`).
pub fn parse_kde_font(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("font=") {
            let parts: Vec<&str> = rest.split(',').collect();
            let name = parts.first().map(|part| part.trim()).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            let point_size = parts.get(1).map(|part| part.trim()).unwrap_or("");
            let value = if point_size.is_empty() {
                format!("{name} [Qt]")
            } else {
                format!("{name} ({point_size}pt) [Qt]")
            };
            return Some(value);
        }
    }
    None
}

/// Parses the GTK UI font from `settings.ini` (`gtk-font-name=Noto Sans, 10`).
pub fn parse_gtk_font(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(font) = line.strip_prefix("gtk-font-name=") {
            let cleaned = font
                .replace(',', " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_module_name() {
        assert_eq!(FontModule.name(), "font");
    }

    #[test]
    fn test_config_dir_falls_back_when_xdg_is_empty() {
        assert_eq!(
            config_dir_from(Some("   "), Some("/home/user")),
            Some(PathBuf::from("/home/user/.config"))
        );
        assert_eq!(
            config_dir_from(Some(""), Some("/home/user")),
            Some(PathBuf::from("/home/user/.config"))
        );
        assert_eq!(
            config_dir_from(Some("/custom/config"), Some("/home/user")),
            Some(PathBuf::from("/custom/config"))
        );
        assert_eq!(config_dir_from(None, None), None);
    }

    #[test]
    fn test_parse_kde_font() {
        let content = "[General]\nfont=Noto Sans,10,-1,5,50,0,0,0,0,0\n";
        assert_eq!(
            parse_kde_font(content).as_deref(),
            Some("Noto Sans (10pt) [Qt]")
        );
        assert_eq!(parse_kde_font("font=,10"), None);
        assert_eq!(parse_kde_font("[General]\n"), None);
    }

    #[test]
    fn test_parse_gtk_font() {
        let content = "[Settings]\ngtk-font-name=Noto Sans,  10\n";
        assert_eq!(parse_gtk_font(content).as_deref(), Some("Noto Sans 10"));
        assert_eq!(parse_gtk_font("gtk-font-name=\n"), None);
        assert_eq!(parse_gtk_font(""), None);
    }
}
