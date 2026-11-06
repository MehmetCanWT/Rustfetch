use crate::{Info, Module};
use std::fs;
use std::path::Path;

pub struct PowerProfile;

impl Module for PowerProfile {
    fn name(&self) -> &'static str {
        "power_profile"
    }

    fn detect(&self) -> Option<Info> {
        detect_from(
            Path::new("/sys/devices/system/cpu"),
            Path::new("/sys/firmware/acpi/platform_profile"),
        )
    }
}

/// Reads the power profile from the given cpu and platform-profile roots.
pub fn detect_from(cpu_root: &Path, platform_profile_path: &Path) -> Option<Info> {
    let cpu = first_online_cpu(cpu_root).unwrap_or(0);
    let base = cpu_root.join(format!("cpu{cpu}"));
    let platform = fs::read_to_string(platform_profile_path).ok();
    let governor = fs::read_to_string(base.join("cpufreq/scaling_governor")).ok();
    let epp = fs::read_to_string(base.join("cpufreq/energy_performance_preference")).ok();

    let value = format_power_profile(platform.as_deref(), governor.as_deref(), epp.as_deref())?;
    Some(Info::new("Power Profile", value))
}

/// Returns the lowest CPU index listed in /sys/devices/system/cpu/online.
pub fn first_online_cpu(cpu_root: &Path) -> Option<u32> {
    let content = fs::read_to_string(cpu_root.join("online")).ok()?;
    let first = content.trim().split(',').next()?.trim();
    let start = first.split('-').next()?.trim();
    start.parse::<u32>().ok()
}

pub fn format_power_profile(
    platform_profile: Option<&str>,
    governor: Option<&str>,
    epp: Option<&str>,
) -> Option<String> {
    let prof = platform_profile
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(prettify_profile);
    let gov = governor
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(prettify_profile);
    let epp_raw = epp
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "default");

    let mut base = match (&prof, &gov) {
        (Some(p), Some(g)) => {
            if p.eq_ignore_ascii_case(g) {
                Some(p.clone())
            } else {
                Some(format!("{p} ({g})"))
            }
        }
        (Some(p), None) => Some(p.clone()),
        (None, Some(g)) => Some(g.clone()),
        (None, None) => None,
    };

    if let Some(e) = epp_raw {
        base = match base {
            Some(b) => Some(format!("{b} [EPP: {e}]")),
            None => Some(prettify_profile(e)),
        };
    }

    base
}

fn prettify_profile(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "performance" => "Performance".to_string(),
        "balanced" | "balance_performance" => "Balanced".to_string(),
        "balance_power" => "Balanced Power".to_string(),
        "low-power" | "power" | "powersave" => "Power Saver".to_string(),
        "quiet" => "Quiet".to_string(),
        "schedutil" => "Schedutil".to_string(),
        "ondemand" => "OnDemand".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rustfetch_power_test_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_power_profile_module_name() {
        assert_eq!(PowerProfile.name(), "power_profile");
    }

    #[test]
    fn test_format_power_profile() {
        assert_eq!(
            format_power_profile(Some("balanced\n"), Some("powersave\n"), None),
            Some("Balanced (Power Saver)".to_string())
        );
        assert_eq!(
            format_power_profile(Some("performance\n"), Some("performance\n"), None),
            Some("Performance".to_string())
        );
        assert_eq!(
            format_power_profile(None, Some("schedutil\n"), Some("balance_performance\n")),
            Some("Schedutil [EPP: balance_performance]".to_string())
        );
        assert_eq!(format_power_profile(None, None, None), None);
    }

    #[test]
    fn test_format_power_profile_keeps_epp_with_platform_profile() {
        assert_eq!(
            format_power_profile(Some("balanced"), Some("powersave"), Some("performance")),
            Some("Balanced (Power Saver) [EPP: performance]".to_string())
        );
        assert_eq!(
            format_power_profile(None, None, Some("power")),
            Some("Power Saver".to_string())
        );
    }

    #[test]
    fn test_first_online_cpu() {
        let dir = unique_dir("online");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("online"), "1-3,5\n").unwrap();
        assert_eq!(first_online_cpu(&dir), Some(1));
        fs::write(dir.join("online"), "0-11\n").unwrap();
        assert_eq!(first_online_cpu(&dir), Some(0));
        fs::write(dir.join("online"), "4\n").unwrap();
        assert_eq!(first_online_cpu(&dir), Some(4));
        fs::remove_file(dir.join("online")).unwrap();
        assert_eq!(first_online_cpu(&dir), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_from_uses_first_online_cpu() {
        let root = unique_dir("detect");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("online"), "1\n").unwrap();
        let cpu = root.join("cpu1");
        fs::create_dir_all(cpu.join("cpufreq")).unwrap();
        fs::write(cpu.join("cpufreq/scaling_governor"), "performance\n").unwrap();
        fs::write(cpu.join("cpufreq/energy_performance_preference"), "power\n").unwrap();
        let platform = root.join("platform_profile");
        fs::write(&platform, "balanced\n").unwrap();
        let info = detect_from(&root, &platform).unwrap();
        assert_eq!(
            info.value,
            "Balanced (Performance) [EPP: power]".to_string()
        );
        let _ = fs::remove_dir_all(&root);
    }
}
