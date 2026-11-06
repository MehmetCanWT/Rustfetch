use crate::{Info, Module};
use std::env;

pub struct DesktopModule;

impl Module for DesktopModule {
    fn name(&self) -> &'static str {
        "desktop"
    }

    fn detect(&self) -> Option<Info> {
        let de = desktop_environment();
        let wm_type = env::var("WAYLAND_DISPLAY")
            .map(|_| "Wayland".to_string())
            .or_else(|_| env::var("XDG_SESSION_TYPE"))
            .unwrap_or_default();

        let wm_name = detect_wm_name(&de);

        let value = format_desktop(&de, &wm_name, &wm_type)?;

        Some(Info::new("Desktop", value))
    }
}

/// Resolves the desktop environment name.
///
/// `XDG_CURRENT_DESKTOP` is a colon-separated list (`ubuntu:GNOME`), so the most
/// specific token is used; `DESKTOP_SESSION` is often a full absolute path
/// (`/usr/share/wayland-sessions/plasma.desktop`) and is reduced to its stem.
fn desktop_environment() -> String {
    if let Ok(value) = env::var("XDG_CURRENT_DESKTOP") {
        if let Some(name) = preferred_desktop_from_list(&value) {
            return name;
        }
    }
    if let Ok(value) = env::var("DESKTOP_SESSION") {
        if let Some(name) = normalize_desktop_name(&value) {
            return name;
        }
    }
    String::new()
}

/// Picks the most specific (last non-empty) entry of a colon-separated
/// `XDG_CURRENT_DESKTOP` value.
pub fn preferred_desktop_from_list(raw: &str) -> Option<String> {
    raw.split(':')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(normalize_desktop_name)
        .next_back()
}

/// Normalizes a desktop/session identifier: strips any directory path and the
/// `.desktop` suffix, then maps common identifiers to display names
/// (`plasma` -> `KDE`, `gnome` -> `GNOME`, ...).
pub fn normalize_desktop_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let stem = std::path::Path::new(trimmed)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(trimmed)
        .trim();
    if stem.is_empty() {
        return None;
    }
    let key = stem.to_lowercase();
    let display = match key.as_str() {
        "plasma" | "plasmawayland" | "plasma-x11" | "kde" | "kde-plasma" | "kde5" | "kde6" => "KDE",
        "gnome" | "gnome-xorg" | "gnome-wayland" | "gnome-classic" | "ubuntu" => "GNOME",
        "xfce" | "xfce4" | "xfce-session" => "XFCE",
        "cinnamon" | "cinnamon2d" => "Cinnamon",
        "mate" => "MATE",
        "lxqt" => "LXQt",
        "lxde" => "LXDE",
        "budgie" | "budgie-desktop" => "Budgie",
        "pantheon" => "Pantheon",
        "cosmic" | "cosmic-session" => "COSMIC",
        "sway" => "Sway",
        "hyprland" => "Hyprland",
        "niri" => "Niri",
        "i3" => "i3",
        "openbox" => "Openbox",
        "fluxbox" => "Fluxbox",
        "deepin" => "Deepin",
        "enlightenment" => "Enlightenment",
        "icewm" => "IceWM",
        "ukui" => "UKUI",
        "unity" => "Unity",
        _ => return Some(stem.to_string()),
    };
    Some(display.to_string())
}

pub fn format_desktop(de: &str, wm_name: &str, wm_type: &str) -> Option<String> {
    let de = de.trim();
    let wm_name = wm_name.trim();
    let wm_type = wm_type.trim();

    if de.is_empty() && wm_name.is_empty() {
        return None;
    }

    let same_de_wm = !de.is_empty() && !wm_name.is_empty() && de.eq_ignore_ascii_case(wm_name);

    let value = match (
        de.is_empty(),
        wm_name.is_empty() || same_de_wm,
        wm_type.is_empty(),
    ) {
        (false, true, true) => de.to_string(),
        (false, true, false) => format!("{de} ({wm_type})"),
        (true, false, true) => wm_name.to_string(),
        (true, false, false) => format!("{wm_name} ({wm_type})"),
        (false, false, true) => format!("{de} ({wm_name})"),
        (false, false, false) => format!("{de} ({wm_name} {wm_type})"),
        (true, true, _) => return None,
    };

    Some(value)
}

fn detect_wm_name(de: &str) -> String {
    if let Some(wm) = get_wm_from_env(de) {
        return wm;
    }
    get_wm_name_from_proc()
}

pub fn get_wm_from_env(de: &str) -> Option<String> {
    if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return Some("Hyprland".to_string());
    }
    if env::var("SWAYSOCK").is_ok() {
        return Some("Sway".to_string());
    }
    if env::var("I3SOCK").is_ok() {
        return Some("i3".to_string());
    }
    if env::var("RIVER_RUNTIME_DIR").is_ok() {
        return Some("River".to_string());
    }

    let session = env::var("DESKTOP_SESSION").unwrap_or_default();
    get_wm_from_names(de, &session)
}

fn match_wm_name(s: &str) -> Option<&'static str> {
    let lower = s.to_lowercase();
    if lower.contains("kde") || lower.contains("plasma") {
        Some("KWin")
    } else if lower.contains("gnome") {
        Some("Mutter")
    } else if lower.contains("cinnamon") || lower.contains("muffin") {
        Some("Muffin")
    } else if lower.contains("mate") || lower.contains("marco") {
        Some("Marco")
    } else if lower.contains("hyprland") {
        Some("Hyprland")
    } else if lower.contains("sway") {
        Some("Sway")
    } else if lower.contains("mangowm") || lower.contains("mango") {
        Some("MangoWM")
    } else if lower.contains("niri") {
        Some("Niri")
    } else if lower.contains("river") {
        Some("River")
    } else if lower.contains("dwl") {
        Some("dwl")
    } else if lower.contains("wayfire") {
        Some("Wayfire")
    } else if lower.contains("labwc") {
        Some("labwc")
    } else if lower.contains("cosmic") {
        Some("COSMIC")
    } else if lower.contains("xfce") || lower.contains("xfwm") {
        Some("Xfwm4")
    } else if lower.contains("i3") {
        Some("i3")
    } else if lower.contains("bspwm") {
        Some("bspwm")
    } else if lower.contains("awesome") {
        Some("awesome")
    } else if lower.contains("qtile") {
        Some("Qtile")
    } else if lower.contains("openbox") {
        Some("Openbox")
    } else if lower.contains("dwm") {
        Some("dwm")
    } else {
        None
    }
}

pub fn get_wm_from_names(de: &str, session: &str) -> Option<String> {
    match_wm_name(de)
        .or_else(|| match_wm_name(session))
        .map(|s| s.to_string())
}

fn get_wm_name_from_proc() -> String {
    if let Ok(dir) = std::fs::read_dir("/proc") {
        for entry in dir.flatten() {
            let fname = entry.file_name();
            let Some(pid_str) = fname.to_str() else {
                continue;
            };
            if !pid_str.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid_str}/comm")) {
                let name = match comm.trim() {
                    "kwin_wayland" | "kwin_x11" => "KWin",
                    "mutter" | "gnome-shell" => "Mutter",
                    "cinnamon" | "muffin" => "Muffin",
                    "marco" => "Marco",
                    "sway" => "Sway",
                    "hyprland" | "Hyprland" => "Hyprland",
                    "mangowm" | "mango" => "MangoWM",
                    "niri" => "Niri",
                    "river" => "River",
                    "dwl" => "dwl",
                    "wayfire" => "Wayfire",
                    "labwc" => "labwc",
                    "cosmic-comp" => "COSMIC",
                    "xfwm4" => "Xfwm4",
                    "openbox" => "Openbox",
                    "i3" => "i3",
                    "bspwm" => "bspwm",
                    "awesome" => "awesome",
                    "qtile" => "Qtile",
                    "dwm" => "dwm",
                    _ => continue,
                };
                return name.to_string();
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_desktop_deduplicates_de_and_wm() {
        assert_eq!(
            format_desktop("Hyprland", "Hyprland", "Wayland"),
            Some("Hyprland (Wayland)".to_string())
        );
        assert_eq!(
            format_desktop("KDE", "KWin", "Wayland"),
            Some("KDE (KWin Wayland)".to_string())
        );
        assert_eq!(
            format_desktop("MangoWM", "", "Wayland"),
            Some("MangoWM (Wayland)".to_string())
        );
        assert_eq!(format_desktop("", "", "Wayland"), None);
    }

    #[test]
    fn test_get_wm_from_names() {
        assert_eq!(get_wm_from_names("KDE", ""), Some("KWin".to_string()));
        assert_eq!(get_wm_from_names("GNOME", ""), Some("Mutter".to_string()));
        assert_eq!(
            get_wm_from_names("Hyprland", ""),
            Some("Hyprland".to_string())
        );
        assert_eq!(get_wm_from_names("sway", ""), Some("Sway".to_string()));
        assert_eq!(
            get_wm_from_names("MangoWM", ""),
            Some("MangoWM".to_string())
        );
        assert_eq!(
            get_wm_from_names("Cinnamon", ""),
            Some("Muffin".to_string())
        );
        assert_eq!(get_wm_from_names("MATE", ""), Some("Marco".to_string()));
        assert_eq!(
            get_wm_from_names("", "plasma.desktop"),
            Some("KWin".to_string())
        );
        assert_eq!(get_wm_from_names("custom_nonexistent", "other"), None);
    }

    #[test]
    fn test_normalize_desktop_name() {
        assert_eq!(
            normalize_desktop_name("/usr/share/wayland-sessions/plasma.desktop").as_deref(),
            Some("KDE")
        );
        assert_eq!(normalize_desktop_name("plasma").as_deref(), Some("KDE"));
        assert_eq!(normalize_desktop_name("gnome").as_deref(), Some("GNOME"));
        assert_eq!(normalize_desktop_name("xfce").as_deref(), Some("XFCE"));
        assert_eq!(normalize_desktop_name("sway").as_deref(), Some("Sway"));
        assert_eq!(
            normalize_desktop_name("/usr/share/xsessions/i3.desktop").as_deref(),
            Some("i3")
        );
        assert_eq!(
            normalize_desktop_name("unknown-de").as_deref(),
            Some("unknown-de")
        );
        assert_eq!(normalize_desktop_name("   "), None);
    }

    #[test]
    fn test_preferred_desktop_from_list() {
        assert_eq!(
            preferred_desktop_from_list("ubuntu:GNOME").as_deref(),
            Some("GNOME")
        );
        assert_eq!(preferred_desktop_from_list("KDE").as_deref(), Some("KDE"));
        assert_eq!(preferred_desktop_from_list("  : ").as_deref(), None);
    }
}
