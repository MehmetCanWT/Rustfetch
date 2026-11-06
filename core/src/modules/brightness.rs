use crate::{Info, Module};
use std::fs;
use std::path::{Path, PathBuf};

pub struct BrightnessModule;

impl Module for BrightnessModule {
    fn name(&self) -> &'static str {
        "brightness"
    }

    fn detect(&self) -> Option<Info> {
        detect_from_backlight_root(Path::new("/sys/class/backlight"))
    }
}

/// Returns the brightness percentage from the highest-priority backlight device.
///
/// Real laptops often expose both a firmware interface (acpi_video0) and the
/// real panel control (intel_backlight / amdgpu_bl0). The device with the best
/// rank wins and ties are broken by name, so the result never depends on the
/// directory iteration order.
pub fn detect_from_backlight_root(root: &Path) -> Option<Info> {
    let entries = fs::read_dir(root).ok()?;
    let mut devices: Vec<(u8, String, PathBuf)> = entries
        .flatten()
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (device_rank(&name), name, entry.path())
        })
        .collect();
    devices.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    for (_, _, path) in devices {
        if let Some(pct) = detect_brightness_in(&path) {
            return Some(Info::new("Brightness", format!("{pct}%")));
        }
    }
    None
}

/// Ranks backlight devices so the real panel control beats legacy interfaces.
pub fn device_rank(name: &str) -> u8 {
    let lower = name.to_ascii_lowercase();
    if lower.contains("intel_backlight") {
        0
    } else if lower.contains("amdgpu") {
        1
    } else if lower.contains("nouveau") {
        2
    } else if lower.starts_with("acpi_video") {
        8
    } else {
        4
    }
}

/// Computes the clamped brightness percentage for one backlight directory.
pub fn detect_brightness_in(dir: &Path) -> Option<u32> {
    let cur: f64 = read_number(&dir.join("brightness"))?;
    let max: f64 = read_number(&dir.join("max_brightness"))?;
    if max <= 0.0 || cur < 0.0 {
        return None;
    }
    let pct = ((cur / max) * 100.0).round() as u32;
    Some(pct.min(100))
}

fn read_number(path: &Path) -> Option<f64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_brightness_test_{tag}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn device(root: &Path, name: &str, cur: &str, max: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("brightness"), format!("{cur}\n")).unwrap();
        fs::write(dir.join("max_brightness"), format!("{max}\n")).unwrap();
    }

    #[test]
    fn test_brightness_module_name() {
        assert_eq!(BrightnessModule.name(), "brightness");
    }

    #[test]
    fn test_detect_from_backlight_dir_valid() {
        let temp = unique_dir("valid");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("brightness"), "14400\n").unwrap();
        fs::write(temp.join("max_brightness"), "19200\n").unwrap();
        assert_eq!(detect_brightness_in(&temp), Some(75));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_from_backlight_dir_zero_max() {
        let temp = unique_dir("zero_max");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("brightness"), "0\n").unwrap();
        fs::write(temp.join("max_brightness"), "0\n").unwrap();
        assert!(detect_brightness_in(&temp).is_none());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_brightness_is_clamped_to_100() {
        let temp = unique_dir("clamp");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("brightness"), "150\n").unwrap();
        fs::write(temp.join("max_brightness"), "100\n").unwrap();
        assert_eq!(detect_brightness_in(&temp), Some(100));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_device_rank_prefers_real_panel() {
        assert!(device_rank("intel_backlight") < device_rank("acpi_video0"));
        assert!(device_rank("amdgpu_bl1") < device_rank("acpi_video0"));
        assert!(device_rank("nouveau_bl") < device_rank("acpi_video0"));
        assert!(device_rank("acpi_video0") > device_rank("some_panel"));
    }

    #[test]
    fn test_intel_beats_acpi_regardless_of_creation_order() {
        let first = unique_dir("order_a");
        device(&first, "acpi_video0", "50", "100");
        device(&first, "intel_backlight", "10", "100");
        assert_eq!(
            detect_from_backlight_root(&first).map(|info| info.value),
            Some("10%".to_string())
        );
        let _ = fs::remove_dir_all(&first);

        let second = unique_dir("order_b");
        device(&second, "intel_backlight", "10", "100");
        device(&second, "acpi_video0", "50", "100");
        assert_eq!(
            detect_from_backlight_root(&second).map(|info| info.value),
            Some("10%".to_string())
        );
        let _ = fs::remove_dir_all(&second);
    }
}
