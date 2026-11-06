use crate::{Info, Module};
use std::fs;
use std::path::{Path, PathBuf};

pub struct BatteryModule;

impl Module for BatteryModule {
    fn name(&self) -> &'static str {
        "battery"
    }

    fn detect(&self) -> Option<Info> {
        detect_from_root(Path::new("/sys/class/power_supply"))
    }
}

/// Reads every battery below the given power_supply root, in name order.
pub fn detect_from_root(root: &Path) -> Option<Info> {
    let dir = fs::read_dir(root).ok()?;
    let mut entries: Vec<PathBuf> = dir.flatten().map(|entry| entry.path()).collect();
    entries.sort();

    let mut batteries = Vec::new();
    for path in entries {
        if fs::read_to_string(path.join("type"))
            .is_ok_and(|text| text.trim().eq_ignore_ascii_case("Battery"))
        {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(info) = detect_from_dir(&path, &name) {
                batteries.push(info);
            }
        }
    }

    if batteries.is_empty() {
        None
    } else if batteries.len() == 1 {
        batteries.pop()
    } else {
        let value = batteries
            .into_iter()
            .map(|battery| format!("{}: {}", battery.label, battery.value))
            .collect::<Vec<_>>()
            .join(" | ");
        Some(Info::new("Battery".to_string(), value))
    }
}

/// Health percentage from one consistent unit family (charge or energy).
///
/// Mixing a charge value with an energy design value would silently produce a
/// bogus percentage, so both members must come from the same family.
pub fn health_percent(
    charge_full: Option<f64>,
    charge_design: Option<f64>,
    energy_full: Option<f64>,
    energy_design: Option<f64>,
) -> Option<u32> {
    let (full, design) = match (charge_full, charge_design) {
        (Some(full), Some(design)) => (full, design),
        _ => match (energy_full, energy_design) {
            (Some(full), Some(design)) => (full, design),
            _ => return None,
        },
    };
    if design <= 0.0 || !full.is_finite() || !design.is_finite() {
        return None;
    }
    let health = (full / design * 100.0).round();
    if health.is_finite() {
        Some(health.clamp(0.0, 100.0) as u32)
    } else {
        None
    }
}

fn detect_from_dir(path: &Path, name: &str) -> Option<Info> {
    let capacity = read_number(path.join("capacity"))?;
    let capacity_pct = (capacity.round().clamp(0.0, 100.0)) as u32;

    let status = read_trimmed(path.join("status"));
    let manufacturer = read_trimmed(path.join("manufacturer"));

    let ac_status = match status.as_deref() {
        Some(value)
            if value.eq_ignore_ascii_case("charging") || value.eq_ignore_ascii_case("full") =>
        {
            "AC Connected".to_string()
        }
        Some(value) if !value.is_empty() => value.to_string(),
        _ => "Unknown".to_string(),
    };

    let cycles = fs::read_to_string(path.join("cycle_count"))
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        .filter(|cycles| *cycles > 0);

    let health = health_percent(
        read_number(path.join("charge_full")),
        read_number(path.join("charge_full_design")),
        read_number(path.join("energy_full")),
        read_number(path.join("energy_full_design")),
    );

    let mut extra = Vec::new();
    if let Some(health) = health {
        extra.push(format!("Health: {health}%"));
    }
    if let Some(cycles) = cycles {
        extra.push(format!("{cycles} cycles"));
    }

    let extra_str = if extra.is_empty() {
        String::new()
    } else {
        format!(" ({})", extra.join(", "))
    };

    let value = match manufacturer {
        Some(manufacturer) => format!("{manufacturer} - {capacity_pct}% [{ac_status}]{extra_str}"),
        None => format!("{capacity_pct}% [{ac_status}]{extra_str}"),
    };

    Some(Info::new(format!("Battery ({name})"), value))
}

fn read_number(path: PathBuf) -> Option<f64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_trimmed(path: PathBuf) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_battery_test_{tag}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_battery_module_name() {
        assert_eq!(BatteryModule.name(), "battery");
    }

    #[test]
    fn test_detect_from_dir_full() {
        let temp = unique_dir("full");
        fs::create_dir_all(&temp).unwrap();

        fs::write(temp.join("capacity"), "95\n").unwrap();
        fs::write(temp.join("status"), "Full\n").unwrap();
        fs::write(temp.join("manufacturer"), "SMP\n").unwrap();
        fs::write(temp.join("cycle_count"), "120\n").unwrap();
        fs::write(temp.join("charge_full"), "4000\n").unwrap();
        fs::write(temp.join("charge_full_design"), "5000\n").unwrap();

        let info = detect_from_dir(&temp, "BAT0").unwrap();
        assert_eq!(info.label, "Battery (BAT0)");
        assert_eq!(
            info.value,
            "SMP - 95% [AC Connected] (Health: 80%, 120 cycles)"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_from_dir_no_extra() {
        let temp = unique_dir("simple");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("capacity"), "42\n").unwrap();
        fs::write(temp.join("status"), "Discharging\n").unwrap();

        let info = detect_from_dir(&temp, "BAT1").unwrap();
        assert_eq!(info.label, "Battery (BAT1)");
        assert_eq!(info.value, "42% [Discharging]");
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_missing_status_is_unknown() {
        let temp = unique_dir("no_status");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("capacity"), "55\n").unwrap();
        let info = detect_from_dir(&temp, "BAT0").unwrap();
        assert_eq!(info.value, "55% [Unknown]");
        assert!(!info.value.contains("[]"));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_invalid_capacity_is_rejected() {
        let temp = unique_dir("bad_capacity");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("capacity"), "abc\n").unwrap();
        assert!(detect_from_dir(&temp, "BAT0").is_none());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_capacity_clamped() {
        let temp = unique_dir("clamp");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("capacity"), "255\n").unwrap();
        fs::write(temp.join("status"), "Charging\n").unwrap();
        let info = detect_from_dir(&temp, "BAT0").unwrap();
        assert_eq!(info.value, "100% [AC Connected]");
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_health_does_not_mix_unit_families() {
        // charge_full without charge_full_design must not fall back to energy.
        assert_eq!(health_percent(Some(4000.0), None, None, Some(5000.0)), None);
        assert_eq!(
            health_percent(Some(4000.0), Some(5000.0), None, Some(9000.0)),
            Some(80)
        );
        assert_eq!(
            health_percent(None, None, Some(48_000.0), Some(60_000.0)),
            Some(80)
        );
        assert_eq!(health_percent(Some(1.0), Some(0.0), None, None), None);
        assert_eq!(
            health_percent(Some(120.0), Some(100.0), None, None),
            Some(100)
        );
    }

    #[test]
    fn test_detect_from_root_orders_batteries_and_skips_missing_capacity() {
        let root = unique_dir("multi");
        for (name, capacity) in [("BAT1", Some("10")), ("BAT0", Some("90")), ("BAT2", None)] {
            let dir = root.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("type"), "Battery\n").unwrap();
            if let Some(capacity) = capacity {
                fs::write(dir.join("capacity"), format!("{capacity}\n")).unwrap();
            }
        }
        let ac = root.join("AC");
        fs::create_dir_all(&ac).unwrap();
        fs::write(ac.join("type"), "Mains\n").unwrap();

        let info = detect_from_root(&root).unwrap();
        assert_eq!(info.label, "Battery");
        assert!(info.value.starts_with("Battery (BAT0): 90%"));
        assert!(info.value.contains("Battery (BAT1): 10%"));
        assert!(!info.value.contains("BAT2"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_detect_from_root_without_batteries() {
        let root = unique_dir("empty");
        fs::create_dir_all(root.join("AC")).unwrap();
        fs::write(root.join("AC/type"), "Mains\n").unwrap();
        assert!(detect_from_root(&root).is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
