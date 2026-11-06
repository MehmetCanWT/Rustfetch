use crate::modules::net_io::{is_virtual_interface, parse_default_route_interface};
use crate::proc::{run_with_timeout, OPTIONAL_SOURCE_TIMEOUT};
use crate::{Info, Module};
use std::fs;
use std::path::Path;

pub struct Wifi;

impl Module for Wifi {
    fn name(&self) -> &'static str {
        "wifi"
    }

    fn is_slow(&self) -> bool {
        true
    }

    fn detect(&self) -> Option<Info> {
        let details = get_wifi_details()?;
        let mut parts = Vec::new();
        if let Some(dbm) = details.dbm {
            if let Some(sig) = details.signal_pct {
                parts.push(format!("{dbm} dBm, {sig}%"));
            } else {
                parts.push(format!("{dbm} dBm"));
            }
        } else if let Some(sig) = details.signal_pct {
            parts.push(format!("{sig}%"));
        }

        let sig_str = if parts.is_empty() {
            String::new()
        } else {
            format!(" ({})", parts.join(", "))
        };

        let band_str = details.band.map(|b| format!(" [{b}]")).unwrap_or_default();

        let val = format!("{}{sig_str}{band_str}", details.ssid);
        Some(Info::new("Wi-Fi", val))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiDetails {
    pub ssid: String,
    pub signal_pct: Option<u8>,
    pub dbm: Option<i32>,
    pub band: Option<String>,
}

pub fn get_wifi_details() -> Option<WifiDetails> {
    get_wifi_details_from(Path::new("/sys/class/net"), Path::new("/proc/net/route"))
}

/// Reads Wi-Fi details for the active wireless interface.
///
/// All external tools are bounded by OPTIONAL_SOURCE_TIMEOUT so a stuck
/// NetworkManager or iw never blocks the fetch.
pub fn get_wifi_details_from(net_root: &Path, route_path: &Path) -> Option<WifiDetails> {
    let route_iface = fs::read_to_string(route_path)
        .ok()
        .and_then(|content| parse_default_route_interface(&content));
    let iface = find_wireless_interface_from(net_root, route_iface.as_deref())?;

    if let Some(output) = run_with_timeout("iw", &["dev", &iface, "link"], OPTIONAL_SOURCE_TIMEOUT)
    {
        if output.success() {
            let text = output.stdout_string();
            if let Some(details) = parse_iw_link_details(&text) {
                return Some(details);
            }
        }
    }

    if let Some(output) = run_with_timeout(
        "nmcli",
        &[
            "-t",
            "-g",
            "IN-USE,SSID,SIGNAL,FREQ",
            "dev",
            "wifi",
            "list",
            "--rescan",
            "no",
        ],
        OPTIONAL_SOURCE_TIMEOUT,
    ) {
        if output.success() {
            let text = output.stdout_string();
            if let Some(details) = parse_nmcli_dev_wifi(&text) {
                return Some(details);
            }
        }
    }

    if let Some(output) = run_with_timeout("iwgetid", &["-r", &iface], OPTIONAL_SOURCE_TIMEOUT) {
        if output.success() {
            let ssid = output.stdout_string().trim().to_string();
            if !ssid.is_empty() {
                return Some(WifiDetails {
                    ssid,
                    signal_pct: None,
                    dbm: None,
                    band: None,
                });
            }
        }
    }

    None
}

pub fn find_wireless_interface() -> Option<String> {
    find_wireless_interface_from(Path::new("/sys/class/net"), None)
}

/// Finds the wireless interface, preferring the default-route interface.
///
/// Interfaces are sorted by name so the choice never depends on readdir order.
pub fn find_wireless_interface_from(net_root: &Path, preferred: Option<&str>) -> Option<String> {
    let entries = fs::read_dir(net_root).ok()?;
    let mut up: Vec<String> = Vec::new();
    let mut all: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_virtual_interface(&name) {
            continue;
        }
        let path = entry.path();
        let wireless = path.join("wireless").exists()
            || path.join("phy80211").exists()
            || name.starts_with("wl");
        if !wireless {
            continue;
        }
        all.push(name.clone());
        if fs::read_to_string(path.join("operstate"))
            .map(|state| state.trim() == "up")
            .unwrap_or(false)
        {
            up.push(name);
        }
    }

    up.sort();
    all.sort();

    if let Some(pref) = preferred {
        if up.iter().any(|name| name == pref) || all.iter().any(|name| name == pref) {
            return Some(pref.to_string());
        }
    }

    up.into_iter().next().or_else(|| all.into_iter().next())
}

fn freq_to_band(freq: f64) -> Option<String> {
    if freq >= 5925.0 {
        Some("6 GHz".to_string())
    } else if freq >= 4900.0 {
        Some("5 GHz".to_string())
    } else if freq >= 2400.0 {
        Some("2.4 GHz".to_string())
    } else {
        None
    }
}

pub fn parse_iw_link_details(text: &str) -> Option<WifiDetails> {
    if !text.contains("Connected to") {
        return None;
    }

    let mut ssid = None;
    let mut signal_pct = None;
    let mut dbm_val = None;
    let mut band = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("SSID:") {
            let s = rest.trim();
            if !s.is_empty() {
                ssid = Some(s.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("signal:") {
            if let Some(dbm) = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<i32>().ok())
            {
                // Standard dBm to percentage mapping: -50 dBm is 100%, -100 dBm is 0%
                signal_pct = Some((2 * (dbm + 100)).clamp(0, 100) as u8);
                dbm_val = Some(dbm);
            }
        } else if let Some(rest) = trimmed.strip_prefix("freq:") {
            band = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .and_then(freq_to_band);
        }
    }

    ssid.map(|s| WifiDetails {
        ssid: s,
        signal_pct,
        dbm: dbm_val,
        band,
    })
}

/// Active-value tokens accepted from nmcli across locales and versions.
const NMCLI_ACTIVE_TOKENS: &[&str] = &[
    "*", "yes", "evet", "ja", "true", "oui", "sí", "sim", "tak", "da",
];

pub fn parse_nmcli_dev_wifi(text: &str) -> Option<WifiDetails> {
    for line in text.lines() {
        let Some((active_raw, rest)) = line.split_once(':') else {
            continue;
        };
        let Some((rest, freq_raw)) = rest.rsplit_once(':') else {
            continue;
        };
        let Some((ssid_raw, sig_raw)) = rest.rsplit_once(':') else {
            continue;
        };
        let active = active_raw.trim().to_ascii_lowercase();
        if !NMCLI_ACTIVE_TOKENS.contains(&active.as_str()) {
            continue;
        }
        let ssid = unescape_nmcli(ssid_raw.trim());
        if ssid.is_empty() {
            continue;
        }
        let signal_pct = sig_raw
            .trim()
            .parse::<u8>()
            .ok()
            .map(|value| value.min(100));
        let band = freq_raw
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .and_then(freq_to_band);
        return Some(WifiDetails {
            ssid,
            signal_pct,
            dbm: None,
            band,
        });
    }
    None
}

/// Unescapes nmcli terse output (backslash escapes any following character).
pub fn unescape_nmcli(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rustfetch_wifi_test_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn wireless(root: &Path, name: &str, up: bool) {
        let dir = root.join(name);
        fs::create_dir_all(dir.join("wireless")).unwrap();
        fs::write(dir.join("operstate"), if up { "up\n" } else { "down\n" }).unwrap();
    }

    #[test]
    fn test_wifi_module_name() {
        assert_eq!(Wifi.name(), "wifi");
    }

    #[test]
    fn test_parse_iw_link_details_with_freq() {
        let sample = "Connected to 3c:64:cf:72:29:e6 (on wlp0s20f3)\n\tSSID: MyNetwork 5G\n\tfreq: 5200.0\n\tsignal: -48 dBm\n";
        let details = parse_iw_link_details(sample).unwrap();
        assert_eq!(details.ssid, "MyNetwork 5G");
        assert_eq!(details.signal_pct, Some(100));
        assert_eq!(details.dbm, Some(-48));
        assert_eq!(details.band, Some("5 GHz".to_string()));
    }

    #[test]
    fn test_parse_iw_link_disconnected() {
        assert_eq!(parse_iw_link_details("Not connected."), None);
    }

    #[test]
    fn test_parse_iw_link_6ghz() {
        let sample =
            "Connected to aa:bb (on wlan0)\n\tSSID: X\n\tfreq: 6135.0\n\tsignal: -70 dBm\n";
        let details = parse_iw_link_details(sample).unwrap();
        assert_eq!(details.band, Some("6 GHz".to_string()));
        assert_eq!(details.signal_pct, Some(60));
    }

    #[test]
    fn test_parse_nmcli_dev_wifi() {
        let sample = "hayır:OtherNet:60:2437 MHz\nevet:Home\\:Fiber:95:5200 MHz\n";
        let details = parse_nmcli_dev_wifi(sample).unwrap();
        assert_eq!(details.ssid, "Home:Fiber");
        assert_eq!(details.signal_pct, Some(95));
        assert_eq!(details.band, Some("5 GHz".to_string()));
    }

    #[test]
    fn test_parse_nmcli_star_active_and_backslash_escape() {
        let sample = "*:My\\\\Net:80:2412 MHz\n";
        let details = parse_nmcli_dev_wifi(sample).unwrap();
        assert_eq!(details.ssid, "My\\Net");
        assert_eq!(details.band, Some("2.4 GHz".to_string()));

        let localized = "oui:Reseau:70:5180 MHz\n";
        assert_eq!(
            parse_nmcli_dev_wifi(localized).map(|details| details.ssid),
            Some("Reseau".to_string())
        );
    }

    #[test]
    fn test_parse_nmcli_no_active_or_malformed() {
        assert_eq!(parse_nmcli_dev_wifi("no:Net:70:5180 MHz\n"), None);
        assert_eq!(parse_nmcli_dev_wifi("yes:Net:70\n"), None);
        assert_eq!(parse_nmcli_dev_wifi(""), None);
    }

    #[test]
    fn test_unescape_nmcli() {
        assert_eq!(unescape_nmcli("a\\:b"), "a:b");
        assert_eq!(unescape_nmcli("a\\\\b"), "a\\b");
        assert_eq!(unescape_nmcli("trailing\\"), "trailing");
        assert_eq!(unescape_nmcli("plain"), "plain");
    }

    #[test]
    fn test_find_wireless_interface_from_prefers_route() {
        let root = unique_dir("iface");
        wireless(&root, "wlan0", true);
        wireless(&root, "wlan1", true);
        // Without a preference the sorted name wins deterministically.
        assert_eq!(
            find_wireless_interface_from(&root, None),
            Some("wlan0".to_string())
        );
        assert_eq!(
            find_wireless_interface_from(&root, Some("wlan1")),
            Some("wlan1".to_string())
        );
        assert_eq!(
            find_wireless_interface_from(&root, Some("eth0")),
            Some("wlan0".to_string())
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_find_wireless_interface_prefers_up() {
        let root = unique_dir("up");
        wireless(&root, "wlan0", false);
        wireless(&root, "wlan1", true);
        assert_eq!(
            find_wireless_interface_from(&root, None),
            Some("wlan1".to_string())
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_find_wireless_interface_missing_root() {
        assert_eq!(
            find_wireless_interface_from(Path::new("/nonexistent-net"), None),
            None
        );
    }
}
