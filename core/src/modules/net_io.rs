use crate::cache::Cache;
use crate::{Info, Module};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct NetIo;

/// Cumulative interface counters read from sysfs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceBytes {
    pub iface: String,
    pub rx: u64,
    pub tx: u64,
}

/// Previous sample persisted in the cache, used to compute a throughput delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrevSample {
    pub iface: String,
    pub rx: u64,
    pub tx: u64,
    pub timestamp_secs: u64,
}

impl Module for NetIo {
    fn name(&self) -> &'static str {
        "net_io"
    }

    fn detect(&self) -> Option<Info> {
        let value = generate_info(Path::new("/sys/class/net"), Path::new("/proc/net/route"))?;
        Some(Info::new(format!("Network ({})", value.iface), value.value))
    }
}

/// Formatted network row (interface plus display value).
pub struct NetIoInfo {
    pub iface: String,
    pub value: String,
}

pub fn generate_info(net_root: &Path, route_path: &Path) -> Option<NetIoInfo> {
    generate_info_with_cache(net_root, route_path, &Cache::new("net_io"))
}

/// Generates the network row, using the cached previous counters for a live
/// rate when the previous sample is at most 10 seconds old.
pub fn generate_info_with_cache(
    net_root: &Path,
    route_path: &Path,
    cache: &Cache,
) -> Option<NetIoInfo> {
    let route_content = fs::read_to_string(route_path).ok();
    let current = select_interface(net_root, route_content.as_deref())?;
    let now = unix_secs();
    let previous = cache.read_stale().and_then(|text| parse_prev_sample(&text));

    let current_sample = PrevSample {
        iface: current.iface.clone(),
        rx: current.rx,
        tx: current.tx,
        timestamp_secs: now,
    };

    let value = match &previous {
        Some(prev) => match throughput_delta(prev, &current_sample) {
            Some((rx_per_sec, tx_per_sec)) => format_rate_line(rx_per_sec, tx_per_sec),
            None => format_net_totals(current.rx, current.tx),
        },
        None => format_net_totals(current.rx, current.tx),
    };

    cache.write(&serialize_prev_sample(&current_sample));
    Some(NetIoInfo {
        iface: current.iface,
        value,
    })
}

pub fn format_bytes_human(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const TIB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;
    if b < KIB {
        return format!("{bytes} B");
    }

    let (mut value, mut unit, mut decimals) = if b >= TIB {
        (b / TIB, "TiB", 2)
    } else if b >= GIB {
        (b / GIB, "GiB", 2)
    } else if b >= MIB {
        (b / MIB, "MiB", 1)
    } else {
        (b / KIB, "KiB", 1)
    };

    // Promote before rounding so 1048575 B is not printed as "1024.0 KiB".
    if value >= 1023.95 {
        let promoted = match unit {
            "KiB" => Some((b / MIB, "MiB", 1)),
            "MiB" => Some((b / GIB, "GiB", 2)),
            "GiB" => Some((b / TIB, "TiB", 2)),
            _ => None,
        };
        if let Some((next_value, next_unit, next_decimals)) = promoted {
            value = next_value;
            unit = next_unit;
            decimals = next_decimals;
        }
    }

    format!("{value:.decimals$} {unit}")
}

pub fn format_net_totals(rx_bytes: u64, tx_bytes: u64) -> String {
    format!(
        "↓ {} / ↑ {}",
        format_bytes_human(rx_bytes),
        format_bytes_human(tx_bytes)
    )
}

pub fn format_rate_line(rx_per_sec: u64, tx_per_sec: u64) -> String {
    format!(
        "↓ {}/s / ↑ {}/s",
        format_bytes_human(rx_per_sec),
        format_bytes_human(tx_per_sec)
    )
}

/// Returns true for interfaces that must not be reported as the active NIC.
pub fn is_virtual_interface(name: &str) -> bool {
    if name == "lo" {
        return true;
    }
    const PREFIXES: &[&str] = &[
        "docker",
        "veth",
        "br-",
        "virbr",
        "tun",
        "tap",
        "wg",
        "tailscale",
        "zt",
        "vboxnet",
        "vmnet",
        "podman",
        "cni",
        "flannel",
        "kube",
        "lxc",
        "lxdbr",
    ];
    PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

/// Parses /proc/net/route and returns the interface of the default route.
pub fn parse_default_route_interface(content: &str) -> Option<String> {
    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 {
            continue;
        }
        let iface = fields[0];
        let destination = fields[1];
        if destination != "00000000" {
            continue;
        }
        if let Ok(flags) = u32::from_str_radix(fields[3], 16) {
            if flags & 0x1 != 0 {
                return Some(iface.to_string());
            }
        }
    }
    None
}

fn read_interface_bytes(path: &Path) -> Option<(u64, u64)> {
    let rx = fs::read_to_string(path.join("statistics/rx_bytes"))
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    let tx = fs::read_to_string(path.join("statistics/tx_bytes"))
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    Some((rx, tx))
}

/// Selects the interface to report: the default-route interface when it is a
/// physical device, otherwise the busiest interface that is up.
pub fn select_interface(net_root: &Path, route_content: Option<&str>) -> Option<InterfaceBytes> {
    let route_iface = route_content.and_then(parse_default_route_interface);
    let entries = fs::read_dir(net_root).ok()?;

    let mut candidates: Vec<(bool, InterfaceBytes)> = Vec::new();
    for entry in entries.flatten() {
        let iface = entry.file_name().to_string_lossy().into_owned();
        if is_virtual_interface(&iface) {
            continue;
        }
        let path = entry.path();
        let is_up = fs::read_to_string(path.join("operstate"))
            .map(|state| state.trim() == "up")
            .unwrap_or(false);
        let Some((rx, tx)) = read_interface_bytes(&path) else {
            // Missing statistics: skip this interface instead of reporting zeros.
            continue;
        };
        if !is_up && rx == 0 && tx == 0 {
            continue;
        }
        candidates.push((is_up, InterfaceBytes { iface, rx, tx }));
    }

    if candidates.is_empty() {
        return None;
    }

    if let Some(route) = route_iface {
        if let Some((_, found)) = candidates.iter().find(|(_, bytes)| bytes.iface == route) {
            return Some(found.clone());
        }
    }

    candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| (b.1.rx + b.1.tx).cmp(&(a.1.rx + a.1.tx)))
            .then_with(|| a.1.iface.cmp(&b.1.iface))
    });
    candidates.into_iter().next().map(|(_, bytes)| bytes)
}

/// Returns per-second rates when the previous sample is recent and consistent.
pub fn throughput_delta(prev: &PrevSample, current: &PrevSample) -> Option<(u64, u64)> {
    if prev.iface != current.iface {
        return None;
    }
    if current.timestamp_secs <= prev.timestamp_secs {
        return None;
    }
    let elapsed = current.timestamp_secs - prev.timestamp_secs;
    if elapsed > 10 {
        return None;
    }
    if current.rx < prev.rx || current.tx < prev.tx {
        return None;
    }
    let seconds = elapsed as f64;
    let rx = ((current.rx - prev.rx) as f64 / seconds).round() as u64;
    let tx = ((current.tx - prev.tx) as f64 / seconds).round() as u64;
    Some((rx, tx))
}

pub fn serialize_prev_sample(sample: &PrevSample) -> String {
    format!(
        "{}\n{}\n{}\n{}\n",
        sample.iface, sample.rx, sample.tx, sample.timestamp_secs
    )
}

pub fn parse_prev_sample(text: &str) -> Option<PrevSample> {
    let mut lines = text.lines();
    let iface = lines.next()?.trim().to_string();
    let rx = lines.next()?.trim().parse().ok()?;
    let tx = lines.next()?.trim().parse().ok()?;
    let timestamp_secs = lines.next()?.trim().parse().ok()?;
    if iface.is_empty() || is_virtual_interface(&iface) {
        return None;
    }
    Some(PrevSample {
        iface,
        rx,
        tx,
        timestamp_secs,
    })
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rustfetch_netio_test_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn interface(root: &Path, name: &str, up: bool, rx: u64, tx: u64) {
        let dir = root.join(name);
        fs::create_dir_all(dir.join("statistics")).unwrap();
        fs::write(dir.join("operstate"), if up { "up\n" } else { "down\n" }).unwrap();
        fs::write(dir.join("statistics/rx_bytes"), format!("{rx}\n")).unwrap();
        fs::write(dir.join("statistics/tx_bytes"), format!("{tx}\n")).unwrap();
    }

    const ROUTE: &str =
        "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0
eth0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0
";

    #[test]
    fn test_net_io_module_name() {
        assert_eq!(NetIo.name(), "net_io");
    }

    #[test]
    fn test_format_net_totals() {
        assert_eq!(
            format_net_totals(2 * 1024 * 1024 * 1024, 512 * 1024 * 1024),
            "↓ 2.00 GiB / ↑ 512.0 MiB"
        );
        assert_eq!(format_net_totals(500, 1024), "↓ 500 B / ↑ 1.0 KiB");
    }

    #[test]
    fn test_format_bytes_human_boundaries() {
        assert_eq!(format_bytes_human(1023), "1023 B");
        assert_eq!(format_bytes_human(1024), "1.0 KiB");
        assert_eq!(format_bytes_human(1536), "1.5 KiB");
        // Regression: must not print "1024.0 KiB".
        assert_eq!(format_bytes_human(1048575), "1.0 MiB");
        assert_eq!(format_bytes_human(1048576), "1.0 MiB");
    }

    #[test]
    fn test_parse_default_route_interface() {
        assert_eq!(
            parse_default_route_interface(ROUTE),
            Some("eth0".to_string())
        );
        assert_eq!(parse_default_route_interface(""), None);
        assert_eq!(
            parse_default_route_interface(
                "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n"
            ),
            None
        );
    }

    #[test]
    fn test_virtual_interface_filter() {
        for name in [
            "lo",
            "docker0",
            "veth1234",
            "br-abc",
            "virbr0",
            "tun0",
            "tap0",
            "wg0",
            "tailscale0",
            "ztabc",
        ] {
            assert!(is_virtual_interface(name), "{name} must be virtual");
        }
        for name in ["eth0", "enp2s0", "wlan0", "wlp3s0", "bond0"] {
            assert!(!is_virtual_interface(name), "{name} must not be virtual");
        }
    }

    #[test]
    fn test_select_interface_prefers_default_route() {
        let root = unique_dir("route_pref");
        interface(&root, "eth0", true, 1_000_000, 500_000);
        interface(&root, "wlan0", true, 112_000_000, 9_000_000);
        let selected = select_interface(&root, Some(ROUTE)).unwrap();
        assert_eq!(selected.iface, "eth0");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_select_interface_ignores_vpn_and_busiest_wins() {
        let root = unique_dir("vpn");
        interface(&root, "eth0", true, 1_000_000, 500_000);
        interface(&root, "tun0", true, 900_000_000, 100_000_000);
        interface(&root, "docker0", true, 123_456_789, 123_456_789);
        let route =
            "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
tun0\t00000000\t00000000\t0003\t0\t0\t0\t00000000\t0\t0\t0
";
        let selected = select_interface(&root, Some(route)).unwrap();
        assert_eq!(selected.iface, "eth0");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_select_interface_skips_unreadable_statistics() {
        let root = unique_dir("broken");
        fs::create_dir_all(root.join("broken0")).unwrap();
        fs::write(root.join("broken0/operstate"), "up\n").unwrap();
        assert!(select_interface(&root, None).is_none());
        interface(&root, "eth0", true, 10, 20);
        let selected = select_interface(&root, None).unwrap();
        assert_eq!(selected.iface, "eth0");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_select_interface_prefers_up_over_down() {
        let root = unique_dir("up");
        interface(&root, "enp2s0", false, 9_000_000, 9_000_000);
        interface(&root, "wlan0", true, 100, 100);
        let selected = select_interface(&root, None).unwrap();
        assert_eq!(selected.iface, "wlan0");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_throughput_delta() {
        let prev = PrevSample {
            iface: "eth0".to_string(),
            rx: 1_000,
            tx: 500,
            timestamp_secs: 100,
        };
        let current = PrevSample {
            iface: "eth0".to_string(),
            rx: 3_000,
            tx: 1_500,
            timestamp_secs: 102,
        };
        assert_eq!(throughput_delta(&prev, &current), Some((1_000, 500)));

        let stale = PrevSample {
            timestamp_secs: 120,
            ..current.clone()
        };
        assert_eq!(throughput_delta(&prev, &stale), None);

        let other = PrevSample {
            iface: "wlan0".to_string(),
            ..current.clone()
        };
        assert_eq!(throughput_delta(&prev, &other), None);

        let wrapped = PrevSample {
            rx: 10,
            tx: 10,
            ..current.clone()
        };
        assert_eq!(throughput_delta(&prev, &wrapped), None);
    }

    #[test]
    fn test_prev_sample_roundtrip() {
        let sample = PrevSample {
            iface: "eth0".to_string(),
            rx: 42,
            tx: 43,
            timestamp_secs: 44,
        };
        let text = serialize_prev_sample(&sample);
        assert_eq!(parse_prev_sample(&text), Some(sample));
        assert_eq!(parse_prev_sample("garbage"), None);
        assert_eq!(
            parse_prev_sample("lo\n1\n2\n3\n"),
            None,
            "virtual interfaces are not valid samples"
        );
    }

    #[test]
    fn test_generate_info_uses_cache_for_rate() {
        let root = unique_dir("cache");
        interface(&root, "eth0", true, 5_000, 1_000);
        let route = root.join("route");
        fs::write(&route, ROUTE).unwrap();
        let cache_path = root.join("cache_net_io");
        let cache = Cache::for_path(cache_path);

        let now = unix_secs();
        let prev = PrevSample {
            iface: "eth0".to_string(),
            rx: 1_000,
            tx: 500,
            timestamp_secs: now.saturating_sub(2),
        };
        cache.write(&serialize_prev_sample(&prev));
        let info = generate_info_with_cache(&root, &route, &cache).unwrap();
        assert_eq!(info.iface, "eth0");
        assert!(
            info.value.contains("/s"),
            "expected a rate value, got {}",
            info.value
        );
        // The new sample must have been persisted as the new baseline.
        let stored = parse_prev_sample(&cache.read_stale().unwrap()).unwrap();
        assert_eq!(stored.rx, 5_000);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_generate_info_without_previous_sample_reports_totals() {
        let root = unique_dir("no_cache");
        interface(&root, "eth0", true, 5_000, 1_000);
        let route = root.join("route");
        fs::write(&route, ROUTE).unwrap();
        let cache = Cache::for_path(root.join("cache_net_io"));
        let info = generate_info_with_cache(&root, &route, &cache).unwrap();
        assert_eq!(info.value, format_net_totals(5_000, 1_000));
        let _ = fs::remove_dir_all(&root);
    }
}
