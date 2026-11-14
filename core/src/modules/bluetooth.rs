use crate::proc::{OPTIONAL_SOURCE_TIMEOUT, run_with_timeout};
use crate::{Info, Module};
use std::fs;
use std::path::{Path, PathBuf};

pub struct BluetoothModule;

impl Module for BluetoothModule {
    fn name(&self) -> &'static str {
        "bluetooth"
    }

    fn is_slow(&self) -> bool {
        true
    }

    fn detect(&self) -> Option<Info> {
        detect_from_root(Path::new("/sys/class/power_supply"))
    }
}

/// Collects connected Bluetooth peripherals: sysfs HID batteries first, then
/// (bounded) bluetoothctl output.
pub fn detect_from_root(root: &Path) -> Option<Info> {
    let mut devices = Vec::new();
    collect_sysfs_hid_devices(root, &mut devices);
    collect_bluetoothctl_devices(&mut devices);

    if devices.is_empty() {
        None
    } else {
        Some(Info::new("Bluetooth", devices.join(", ")))
    }
}

fn collect_sysfs_hid_devices(root: &Path, devices: &mut Vec<String>) {
    let Ok(dir) = fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<PathBuf> = dir.flatten().map(|entry| entry.path()).collect();
    entries.sort();

    for path in entries {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !is_hid_power_supply(&name) {
            continue;
        }
        // A HID power supply under a USB path is a wired / USB-receiver
        // peripheral, not a Bluetooth device.
        if let Ok(canonical) = fs::canonicalize(&path)
            && !is_bluetooth_sysfs_path(&canonical.to_string_lossy())
        {
            continue;
        }
        let model = read_trimmed(&path.join("model_name"))
            .filter(|model| !model.is_empty())
            .unwrap_or_else(|| name.clone());
        let capacity = read_trimmed(&path.join("capacity"))
            .and_then(|value| value.parse::<u8>().ok())
            .map(|value| value.min(100));
        let desc = match capacity {
            Some(pct) => format!("{model} ({pct}%)"),
            None => model,
        };
        if !devices.contains(&desc) {
            devices.push(desc);
        }
    }
}

fn collect_bluetoothctl_devices(devices: &mut Vec<String>) {
    let Some(output) = run_with_timeout(
        "bluetoothctl",
        &["devices", "Connected"],
        OPTIONAL_SOURCE_TIMEOUT,
    ) else {
        return;
    };
    if !output.success() {
        return;
    }
    for line in output.stdout_string().lines() {
        if let Some((mac, name)) = parse_bluetoothctl_device_line(line) {
            let battery = get_device_battery(&mac);
            let desc = match battery {
                Some(pct) => format!("{name} ({pct}%)"),
                None => name,
            };
            if !devices.contains(&desc) {
                devices.push(desc);
            }
        }
    }
}

pub fn is_hid_power_supply(name: &str) -> bool {
    name.starts_with("hid-") || name.starts_with("hidpp")
}

/// A HID battery is Bluetooth when located on a Bluetooth bus/UHID (`0005:`) and not under USB/I2C.
pub fn is_bluetooth_sysfs_path(canonical_path: &str) -> bool {
    if canonical_path.contains("/bluetooth/") || canonical_path.contains("0005:") {
        return true;
    }
    !canonical_path
        .split(['/', '\\'])
        .any(|segment| segment == "usb" || segment.starts_with("usb") || segment.starts_with("i2c"))
}

pub fn parse_bluetoothctl_device_line(line: &str) -> Option<(String, String)> {
    // Format: "Device 70:26:05:11:22:33 Sony WH-1000XM5"
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("Device ")?.trim();
    let (mac, name) = rest.split_once(' ')?;
    let mac = mac.trim().to_string();
    let name = name.trim().to_string();
    if mac.is_empty() || name.is_empty() {
        return None;
    }
    Some((mac, name))
}

pub fn parse_bluetoothctl_info_battery(output: &str) -> Option<u8> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Battery Percentage:") {
            let rest = rest.trim();
            if let Some((_, after_open)) = rest.split_once('(')
                && let Some((inner, _)) = after_open.split_once(')')
                && let Ok(pct) = inner.trim().parse::<u8>()
            {
                return Some(pct.min(100));
            }
            let cleaned = rest.trim_end_matches('%').trim();
            if let Ok(pct) = cleaned.parse::<u8>() {
                return Some(pct.min(100));
            }
        }
    }
    None
}

fn get_device_battery(mac: &str) -> Option<u8> {
    let output = run_with_timeout("bluetoothctl", &["info", mac], OPTIONAL_SOURCE_TIMEOUT)?;
    if !output.success() {
        return None;
    }
    parse_bluetoothctl_info_battery(&output.stdout_string())
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
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_bluetooth_test_{tag}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_bluetooth_module_name() {
        assert_eq!(BluetoothModule.name(), "bluetooth");
    }

    #[test]
    fn test_parse_bluetoothctl_device_line() {
        let line = "Device 70:26:05:AA:BB:CC Sony WH-1000XM5";
        let (mac, name) = parse_bluetoothctl_device_line(line).unwrap();
        assert_eq!(mac, "70:26:05:AA:BB:CC");
        assert_eq!(name, "Sony WH-1000XM5");
    }

    #[test]
    fn test_parse_bluetoothctl_info_battery_parens() {
        let info = "Device 70:26:05:AA:BB:CC\n\tName: Earbuds\n\tConnected: yes\n\tBattery Percentage: 0x55 (85)\n";
        assert_eq!(parse_bluetoothctl_info_battery(info), Some(85));
    }

    #[test]
    fn test_parse_bluetoothctl_info_battery_percent() {
        let info = "Device 70:26:05:AA:BB:CC\n\tBattery Percentage: 90%\n";
        assert_eq!(parse_bluetoothctl_info_battery(info), Some(90));
    }

    #[test]
    fn test_parse_bluetoothctl_info_battery_clamped() {
        let info = "\tBattery Percentage: 0x7f (150)\n";
        assert_eq!(parse_bluetoothctl_info_battery(info), Some(100));
    }

    #[test]
    fn test_parse_bluetoothctl_info_no_battery() {
        let info = "Device 70:26:05:AA:BB:CC\n\tName: Keyboard\n\tConnected: yes\n";
        assert_eq!(parse_bluetoothctl_info_battery(info), None);
    }

    #[test]
    fn test_unreadable_line_is_skipped() {
        assert_eq!(
            parse_bluetoothctl_device_line("Device AA:BB:CC:DD:EE:FF"),
            None
        );
        assert_eq!(parse_bluetoothctl_device_line("Controller XX"), None);
    }

    #[test]
    fn test_usb_hid_power_supply_is_not_bluetooth() {
        assert!(!is_bluetooth_sysfs_path(
            "/sys/devices/pci0000:00/0000:00:14.0/usb1/1-1/1-1:1.0/0003:046D:C52B.0001/power_supply/hid-0003:046D:C52B.0001"
        ));
        assert!(is_bluetooth_sysfs_path(
            "/sys/devices/virtual/misc/uhid/0005:054C:0CE6.0001/power_supply/hid-0005:054C:0CE6.0001"
        ));
    }

    #[test]
    fn test_collect_sysfs_excludes_wired_usb_receiver() {
        let root = unique_dir("sysfs");
        let bt = root.join("hid-0005:054C:0CE6.0001");
        fs::create_dir_all(&bt).unwrap();
        fs::write(bt.join("model_name"), "DualSense\n").unwrap();
        fs::write(bt.join("capacity"), "77\n").unwrap();

        let mut devices = Vec::new();
        collect_sysfs_hid_devices(&root, &mut devices);
        assert_eq!(devices, vec!["DualSense (77%)".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_collect_sysfs_skips_non_hid_entries() {
        let root = unique_dir("non_hid");
        let battery = root.join("BAT0");
        fs::create_dir_all(&battery).unwrap();
        fs::write(battery.join("capacity"), "50\n").unwrap();
        let mut devices = Vec::new();
        collect_sysfs_hid_devices(&root, &mut devices);
        assert!(devices.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_is_hid_power_supply() {
        assert!(is_hid_power_supply("hid-0005:054C:0CE6.0001"));
        assert!(is_hid_power_supply("hidpp_battery_0"));
        assert!(!is_hid_power_supply("BAT0"));
    }
}
