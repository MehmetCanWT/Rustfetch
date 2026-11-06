use crate::{Info, Module};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Temp;

impl Module for Temp {
    fn name(&self) -> &'static str {
        "temp"
    }

    fn detect(&self) -> Option<Info> {
        let temp = detect_cpu_temp()?;
        Some(Info::new("Temperature", format!("{temp}°C")))
    }
}

/// Plausible CPU temperature window in millidegrees Celsius (inclusive).
const MIN_TEMP_MDEG: i32 = -40_000;
const MAX_TEMP_MDEG: i32 = 150_000;

pub fn detect_cpu_temp() -> Option<i32> {
    detect_cpu_temp_from(
        Path::new("/sys/class/hwmon"),
        Path::new("/sys/class/thermal"),
    )
}

/// Picks the highest-priority CPU temperature sensor below the given roots.
///
/// Priority: package temperature (coretemp/k10temp/x86_pkg_temp) > per-core >
/// nvme > acpitz. Ties are broken by a stable sensor key, so repeated runs on
/// the same machine always report the same sensor.
pub fn detect_cpu_temp_from(hwmon_root: &Path, thermal_root: &Path) -> Option<i32> {
    let mut candidates: Vec<(u8, String, i32)> = Vec::new();
    collect_hwmon_candidates(hwmon_root, &mut candidates);
    collect_thermal_candidates(thermal_root, &mut candidates);
    candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    candidates.first().map(|(_, _, mdeg)| mdeg / 1000)
}

fn collect_hwmon_candidates(root: &Path, out: &mut Vec<(u8, String, i32)>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    dirs.sort();

    for dir in dirs {
        let chip = read_trimmed(&dir.join("name"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let chip_rank = match chip.as_str() {
            "coretemp" | "k10temp" | "zenpower" | "cpu_thermal" | "soc_thermal" => 0u8,
            "nvme" => 4,
            _ => continue,
        };

        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .collect();
        files.sort();

        let mut saw_label = false;
        for file in files {
            let Some(fname) = file.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(stem) = fname.strip_suffix("_label") else {
                continue;
            };
            saw_label = true;
            let label = read_trimmed(&file).unwrap_or_default();
            let label_lower = label.to_ascii_lowercase();
            let priority = if chip_rank == 4 {
                4
            } else if label_lower.starts_with("package id") || label == "Tdie" || label == "Tctl" {
                chip_rank
            } else if label_lower.starts_with("core") || label_lower.starts_with("cpu") {
                chip_rank + 1
            } else {
                chip_rank + 2
            };
            let input = dir.join(format!("{stem}_input"));
            if let Some(temp) = read_temp_file(&input) {
                out.push((priority, format!("{chip}:{label}:{stem}"), temp));
            }
        }

        // Some drivers expose no labels at all; temp1_input is then the package
        // (or the only) sensor.
        if !saw_label {
            if let Some(temp) = read_temp_file(&dir.join("temp1_input")) {
                out.push((chip_rank + 2, format!("{chip}:unlabelled:temp1"), temp));
            }
        }
    }
}

fn collect_thermal_candidates(root: &Path, out: &mut Vec<(u8, String, i32)>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    dirs.sort();

    for dir in dirs {
        let zone = read_trimmed(&dir.join("type"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let priority = match zone.as_str() {
            "x86_pkg_temp" => 0u8,
            "tcpu" | "cpu-thermal" | "cpu_thermal" | "soc-thermal" | "soc_thermal" => 2,
            "acpitz" => 5,
            _ => continue,
        };
        if let Some(temp) = read_temp_file(&dir.join("temp")) {
            let sensor = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push((priority, format!("thermal:{zone}:{sensor}"), temp));
        }
    }
}

/// Reads a millidegree temperature file, rejecting implausible values.
///
/// Zero and negative temperatures are legitimate; only values outside the
/// -40.0..=150.0 °C window are ignored.
pub fn read_temp_file(path: &Path) -> Option<i32> {
    let content = fs::read_to_string(path).ok()?;
    let val: i32 = content.trim().parse().ok()?;
    if (MIN_TEMP_MDEG..=MAX_TEMP_MDEG).contains(&val) {
        Some(val)
    } else {
        None
    }
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rustfetch_temp_test_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn hwmon_chip(root: &Path, dir: &str, chip: &str, sensors: &[(&str, &str)]) {
        let chip_dir = root.join(dir);
        fs::create_dir_all(&chip_dir).unwrap();
        fs::write(chip_dir.join("name"), format!("{chip}\n")).unwrap();
        for (filename, value) in sensors {
            fs::write(chip_dir.join(filename), format!("{value}\n")).unwrap();
        }
    }

    fn thermal_zone(root: &Path, dir: &str, zone_type: &str, value: i32) {
        let zone_dir = root.join(dir);
        fs::create_dir_all(&zone_dir).unwrap();
        fs::write(zone_dir.join("type"), format!("{zone_type}\n")).unwrap();
        fs::write(zone_dir.join("temp"), format!("{value}\n")).unwrap();
    }

    #[test]
    fn test_temp_module_name() {
        assert_eq!(Temp.name(), "temp");
    }

    #[test]
    fn test_read_temp_file_edge_cases() {
        assert!(read_temp_file(Path::new("/nonexistent")).is_none());
    }

    #[test]
    fn test_package_beats_core_and_acpitz() {
        let hwmon = unique_dir("pkg_hwmon");
        let thermal = unique_dir("pkg_thermal");
        hwmon_chip(
            &hwmon,
            "hwmon5",
            "coretemp",
            &[
                ("temp1_input", "70000"),
                ("temp1_label", "Package id 0"),
                ("temp2_input", "60000"),
                ("temp2_label", "Core 0"),
            ],
        );
        thermal_zone(&thermal, "thermal_zone0", "acpitz", 50_000);
        assert_eq!(detect_cpu_temp_from(&hwmon, &thermal), Some(70));
        let _ = fs::remove_dir_all(&hwmon);
        let _ = fs::remove_dir_all(&thermal);
    }

    #[test]
    fn test_core_used_without_package() {
        let hwmon = unique_dir("core_hwmon");
        let thermal = unique_dir("core_thermal");
        hwmon_chip(
            &hwmon,
            "hwmon5",
            "coretemp",
            &[
                ("temp2_input", "60000"),
                ("temp2_label", "Core 0"),
                ("temp3_input", "61000"),
                ("temp3_label", "Core 1"),
            ],
        );
        thermal_zone(&thermal, "thermal_zone0", "acpitz", 50_000);
        assert_eq!(detect_cpu_temp_from(&hwmon, &thermal), Some(60));
        let _ = fs::remove_dir_all(&hwmon);
        let _ = fs::remove_dir_all(&thermal);
    }

    #[test]
    fn test_acpitz_fallback_when_no_cpu_sensor() {
        let hwmon = unique_dir("acpi_hwmon");
        let thermal = unique_dir("acpi_thermal");
        fs::create_dir_all(&hwmon).unwrap();
        thermal_zone(&thermal, "thermal_zone0", "acpitz", 50_000);
        assert_eq!(detect_cpu_temp_from(&hwmon, &thermal), Some(50));
        let _ = fs::remove_dir_all(&hwmon);
        let _ = fs::remove_dir_all(&thermal);
    }

    #[test]
    fn test_negative_and_zero_temperatures_are_kept() {
        let hwmon = unique_dir("neg_hwmon");
        let thermal = unique_dir("neg_thermal");
        hwmon_chip(
            &hwmon,
            "hwmon5",
            "coretemp",
            &[("temp1_input", "-5000"), ("temp1_label", "Package id 0")],
        );
        fs::create_dir_all(&hwmon).unwrap();
        assert_eq!(detect_cpu_temp_from(&hwmon, &thermal), Some(-5));

        let hwmon2 = unique_dir("zero_hwmon");
        hwmon_chip(
            &hwmon2,
            "hwmon5",
            "coretemp",
            &[("temp1_input", "0"), ("temp1_label", "Package id 0")],
        );
        assert_eq!(detect_cpu_temp_from(&hwmon2, &thermal), Some(0));
        let _ = fs::remove_dir_all(&hwmon);
        let _ = fs::remove_dir_all(&hwmon2);
        let _ = fs::remove_dir_all(&thermal);
    }

    #[test]
    fn test_implausible_values_are_rejected() {
        assert!(read_temp_file(&unique_file("hot", "150001")).is_none());
        assert!(read_temp_file(&unique_file("cold", "-40001")).is_none());
        assert_eq!(read_temp_file(&unique_file("max", "150000")), Some(150_000));
        assert_eq!(read_temp_file(&unique_file("min", "-40000")), Some(-40_000));
    }

    #[test]
    fn test_thermal_zone_priority_and_determinism() {
        let hwmon = unique_dir("tz_hwmon");
        let thermal = unique_dir("tz_thermal");
        fs::create_dir_all(&hwmon).unwrap();
        thermal_zone(&thermal, "thermal_zone0", "acpitz", 62_000);
        thermal_zone(&thermal, "thermal_zone1", "x86_pkg_temp", 45_000);
        assert_eq!(detect_cpu_temp_from(&hwmon, &thermal), Some(45));
        // Stable across repeated calls (sorted sensor keys, no readdir order).
        for _ in 0..5 {
            assert_eq!(detect_cpu_temp_from(&hwmon, &thermal), Some(45));
        }
        let _ = fs::remove_dir_all(&hwmon);
        let _ = fs::remove_dir_all(&thermal);
    }

    #[test]
    fn test_nvme_ranks_above_acpitz() {
        let hwmon = unique_dir("nvme_hwmon");
        let thermal = unique_dir("nvme_thermal");
        hwmon_chip(
            &hwmon,
            "hwmon3",
            "nvme",
            &[("temp1_input", "80000"), ("temp1_label", "Composite")],
        );
        thermal_zone(&thermal, "thermal_zone0", "acpitz", 40_000);
        // Priority: package/core > nvme > acpitz.
        assert_eq!(detect_cpu_temp_from(&hwmon, &thermal), Some(80));
        let _ = fs::remove_dir_all(&hwmon);
        let _ = fs::remove_dir_all(&thermal);
    }

    fn unique_file(tag: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "rustfetch_temp_test_file_{tag}_{}",
            std::process::id()
        ));
        fs::write(&path, content).unwrap();
        path
    }
}
