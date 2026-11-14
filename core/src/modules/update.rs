use crate::proc::NETWORK_TIMEOUT;
use crate::proc::run_with_timeout;
use crate::{Info, Module, cache::Cache};
use std::process::Command;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const CAT_BREEDS: &[&str] = &[
    "Turkish Van",
    "Turkish Angora",
    "Bengal",
    "Maine Coon",
    "Siamese",
    "Ragdoll",
    "Persian",
    "Birman",
    "Chartreux",
    "Korat",
    "Abyssinian",
    "British Shorthair",
    "Scottish Fold",
    "Russian Blue",
    "Norwegian Forest",
    "Siberian",
    "Savannah",
    "Ocicat",
    "Somali",
    "Sphynx",
    "Bombay",
    "Burmese",
    "Manx",
    "Anatolian",
];

/// Returns the official cat breed codename for a given semantic version.
pub fn codename_for_version(version: &str) -> &'static str {
    let Some((major, minor, patch)) = parse_semver(version) else {
        return "Ragdoll";
    };
    match (major, minor, patch) {
        (0, 1, 0) => "Turkish Van",
        (0, 1, 1) => "Turkish Angora",
        (0, 1, 2) => "Bengal",
        (0, 1, 3) => "Maine Coon",
        (0, 1, 4) => "Siamese",
        (0, 1, 5) => "Ragdoll",
        (0, 1, 6) => "Persian",
        (0, 1, 7) => "Birman",
        (0, 1, 8) => "Chartreux",
        (0, 1, 9) => "Korat",
        (0, 2, 0) => "Abyssinian",
        (0, 2, 1) => "British Shorthair",
        (0, 2, 2) => "Scottish Fold",
        (0, 2, 3) => "Russian Blue",
        (0, 2, 4) => "Norwegian Forest",
        (0, 2, 5) => "Siberian",
        (0, 3, 0) => "Savannah",
        (0, 3, 1) => "Ocicat",
        (0, 3, 2) => "Somali",
        (0, 4, 0) => "Sphynx",
        (1, 0, 0) => "Anatolian",
        _ => {
            // Wrapping arithmetic: a hostile/typo'd version string stored in the
            // cache or environment must never panic (debug builds panic on
            // overflow) only produce a deterministic codename.
            let mixed = major
                .wrapping_mul(31)
                .wrapping_add(minor.wrapping_mul(10))
                .wrapping_add(patch);
            let idx = (mixed % CAT_BREEDS.len() as u64) as usize;
            CAT_BREEDS[idx]
        }
    }
}

/// Formats a version string with its cat breed codename, e.g. `v0.1.2 "Bengal"`.
pub fn format_version_with_codename(version: &str) -> String {
    let clean = version.trim().trim_start_matches(['v', 'V']);
    let codename = codename_for_version(clean);
    format!("v{clean} \"{codename}\"")
}

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/MehmetCanWT/rustfetch/releases/latest";
const GITHUB_MAIN_CARGO_TOML_URL: &str =
    "https://raw.githubusercontent.com/MehmetCanWT/rustfetch/main/Cargo.toml";
const UPDATE_CACHE_TTL: Duration = Duration::from_secs(6 * 3600);
const UPDATE_CHECK_THROTTLE: Duration = Duration::from_secs(3600);

pub struct Update;

impl Module for Update {
    fn name(&self) -> &'static str {
        "update"
    }

    fn detect(&self) -> Option<Info> {
        if std::env::var("RUSTFETCH_DISABLE_UPDATE_CHECK")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        {
            return None;
        }

        let current = CURRENT_VERSION.trim().trim_start_matches(['v', 'V']);

        if let Ok(forced_latest) = std::env::var("RUSTFETCH_LATEST_VERSION") {
            let (latest, remote_code) = split_cached_version_codename(&forced_latest);
            if is_newer_version(current, &latest) {
                return Some(Info::new(
                    "Update",
                    format_update_message_with_codename(current, &latest, remote_code.as_deref()),
                ));
            }
            return None;
        }

        let cache = Cache::new("update");
        // Fast path: use fresh cached version (< 6 hours).
        let cached_entry = if let Some(cached) = cache.read(UPDATE_CACHE_TTL) {
            Some(cached.trim().to_string())
        } else {
            // Cache is expired or missing. Never block the interactive fetch:
            // a detached background process refreshes the cache.
            trigger_background_update_check();
            // Fall back to stale cache if available, so existing update notification is not lost
            cache.read_stale().map(|s| s.trim().to_string())
        };

        let cached_entry = cached_entry.unwrap_or_else(|| current.to_string());
        let (latest, remote_codename) = split_cached_version_codename(&cached_entry);
        if is_newer_version(current, &latest) {
            Some(Info::new(
                "Update",
                format_update_message_with_codename(current, &latest, remote_codename.as_deref()),
            ))
        } else {
            None
        }
    }
}

/// Checks whether the IPv4 routing table contains a default gateway.
pub fn parse_has_default_ipv4_route(routes: &str) -> bool {
    for line in routes.lines().skip(1) {
        let mut fields = line.split_whitespace();
        let _iface = fields.next();
        let dest = fields.next();
        let _gateway = fields.next();
        let flags = fields.next();
        if let (Some(dest), Some(flags)) = (dest, flags)
            && dest == "00000000"
            && let Ok(f) = u32::from_str_radix(flags, 16)
            && f & 0x0001 != 0
        {
            return true;
        }
    }
    false
}

/// Checks whether the IPv6 routing table contains a default gateway (non-loopback ::/0).
pub fn parse_has_default_ipv6_route(routes_v6: &str) -> bool {
    for line in routes_v6.lines() {
        let mut fields = line.split_whitespace();
        let dest = fields.next();
        let prefix = fields.next();
        let _src = fields.next();
        let _src_prefix = fields.next();
        let _nexthop = fields.next();
        let _metric = fields.next();
        let _ref = fields.next();
        let _use = fields.next();
        let _flags = fields.next();
        let iface = fields.next();
        if let (Some(dest), Some(prefix)) = (dest, prefix)
            && dest == "00000000000000000000000000000000"
            && prefix == "00"
        {
            if let Some(iface) = iface {
                if iface != "lo" {
                    return true;
                }
            } else {
                return true;
            }
        }
    }
    false
}

/// Checks whether the system has an active default network gateway routing route (IPv4 or IPv6).
/// Takes ~5 microseconds (pure /proc/net read, 0 network packets).
pub fn has_default_gateway() -> bool {
    let mut inspected_any = false;
    if let Ok(routes) = std::fs::read_to_string("/proc/net/route") {
        inspected_any = true;
        if parse_has_default_ipv4_route(&routes) {
            return true;
        }
    }
    if let Ok(routes_v6) = std::fs::read_to_string("/proc/net/ipv6_route") {
        inspected_any = true;
        if parse_has_default_ipv6_route(&routes_v6) {
            return true;
        }
    }
    // Fall back to true if /proc/net is inaccessible (e.g. non-Linux or restricted container)
    !inspected_any
}

/// Spawns a detached background process to refresh the update cache without blocking the shell.
///
/// The throttle marker is written only after the gateway check succeeds, so an
/// offline first run (laptop boot before Wi-Fi is up) does not suppress the next
/// check for an hour. If the child cannot be spawned the marker is rolled back.
pub fn trigger_background_update_check() {
    if !has_default_gateway() {
        return;
    }
    let throttle_cache = Cache::new("update_throttle");
    if throttle_cache.read(UPDATE_CHECK_THROTTLE).is_some() {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    throttle_cache.write("1");

    let mut command = Command::new(exe);
    command
        .arg("--check-updates-bg")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    command.process_group(0);

    match command.spawn() {
        Ok(mut child) => {
            // Best-effort reap: the parent usually exits first, but a long-lived
            // --live session must not accumulate zombies.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(_) => throttle_cache.remove(),
    }
}

/// Executed in the background process to fetch the latest version and update the disk cache.
///
/// A failed fetch must not refresh the success TTL: a stale-but-valid cached
/// version stays usable and the throttle marker decides when to retry next.
pub fn refresh_update_cache() {
    if !has_default_gateway() {
        return;
    }
    let cache = Cache::new("update");
    if let Some(fetched) = fetch_latest_github_version() {
        cache.write(&fetched);
    }
}

pub fn format_update_message(current: &str, latest: &str) -> String {
    let (latest_ver, remote_code) = split_cached_version_codename(latest);
    format_update_message_with_codename(current, &latest_ver, remote_code.as_deref())
}

pub fn format_update_message_with_codename(
    current: &str,
    latest: &str,
    remote_codename: Option<&str>,
) -> String {
    let cur = current.trim().trim_start_matches(['v', 'V']);
    let lat = latest.trim().trim_start_matches(['v', 'V']);
    let lat_code = remote_codename
        .and_then(sanitize_codename)
        .unwrap_or_else(|| codename_for_version(lat).to_string());
    format!("Update available (v{cur} -> v{lat} \"{lat_code}\")")
}

/// Keeps only characters that are safe to print on a single terminal line.
///
/// Release titles and cache entries are remote/user data; ANSI escape sequences,
/// control characters and bidi overrides are stripped so a spoofed codename
/// cannot drive the terminal (OSC/CSI injection, title changes, ...).
pub fn sanitize_codename(raw: &str) -> Option<String> {
    let mut cleaned = String::new();
    let mut kept = 0usize;
    for ch in raw.chars() {
        if kept >= 48 {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '.' | '\'' | '-') {
            cleaned.push(ch);
            kept += 1;
        }
    }
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn split_cached_version_codename(entry: &str) -> (String, Option<String>) {
    let (ver, code) = match entry.split_once('|') {
        Some((ver, code)) => (ver.trim(), Some(code.trim())),
        None => (entry.trim(), None),
    };
    let version = ver.trim_start_matches(['v', 'V']).to_string();
    let codename = code.and_then(sanitize_codename);
    (version, codename)
}

/// Extracts a cat breed codename from a GitHub Release title if present,
/// e.g. `RustFetch v0.2.0 — "Abyssinian"` -> `Some("Abyssinian")`.
///
/// The extracted text is sanitized to printable characters only.
pub fn extract_codename_from_release_title(title: &str) -> Option<String> {
    if let Some(start) = title.find('"') {
        let rest = &title[start + 1..];
        if let Some(end) = rest.find('"')
            && let Some(clean) = sanitize_codename(rest[..end].trim())
        {
            return Some(clean);
        }
    }
    for sep in ["—", "–", " - "] {
        if let Some((_, after)) = title.split_once(sep) {
            let candidate = after.trim().trim_matches(['"', '\'', '(', ')']);
            if parse_semver(candidate).is_none()
                && let Some(clean) = sanitize_codename(candidate)
            {
                return Some(clean);
            }
        }
    }
    None
}

pub fn parse_semver(version: &str) -> Option<(u64, u64, u64)> {
    let cleaned = version.trim().trim_start_matches(['v', 'V']);
    let core_part = cleaned.split(['-', '+']).next()?;
    let mut parts = core_part.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let Some(cur) = parse_semver(current) else {
        return false;
    };
    let Some(lat) = parse_semver(latest) else {
        return false;
    };
    lat > cur
}

pub fn parse_github_release_info(json_str: &str) -> Option<(String, Option<String>)> {
    let val: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let tag = val.get("tag_name")?.as_str()?.trim();
    let cleaned = tag.trim_start_matches(['v', 'V']);
    if parse_semver(cleaned).is_some() {
        let title_codename = val
            .get("name")
            .and_then(|n| n.as_str())
            .and_then(extract_codename_from_release_title);
        Some((cleaned.to_string(), title_codename))
    } else {
        None
    }
}

fn curl_binary() -> &'static str {
    if std::path::Path::new("/usr/bin/curl").is_file() {
        "/usr/bin/curl"
    } else {
        "curl"
    }
}

fn wget_binary() -> &'static str {
    if std::path::Path::new("/usr/bin/wget").is_file() {
        "/usr/bin/wget"
    } else {
        "wget"
    }
}

/// Fetches a URL through a bounded external HTTP client.
///
/// curl is preferred and invoked with -q (ignore ~/.curlrc), a strict
/// HTTPS-only protocol allowlist and a maximum response size; wget is only
/// tried when the curl binary could not be spawned at all. Both have a hard
/// deadline, so a stuck network can never hang the caller.
fn fetch_url(url: &str) -> Option<String> {
    if !has_default_gateway() {
        return None;
    }

    let curl = run_with_timeout(
        curl_binary(),
        &[
            "-q",
            "-fsSL",
            "--proto",
            "=https",
            "--connect-timeout",
            "2",
            "--max-time",
            "4",
            "--max-filesize",
            "1048576",
            "-H",
            "User-Agent: rustfetch",
            url,
        ],
        NETWORK_TIMEOUT,
    );
    match curl {
        Some(output) => {
            if !output.success() {
                return None;
            }
            let text = output.stdout_string();
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
        None => {
            let wget = run_with_timeout(
                wget_binary(),
                &[
                    "-q",
                    "-O-",
                    "--timeout=3",
                    "--tries=1",
                    "--max-redirect=5",
                    "--https-only",
                    url,
                ],
                NETWORK_TIMEOUT,
            )?;
            if !wget.success() {
                return None;
            }
            let text = wget.stdout_string();
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
    }
}

pub fn check_latest_release() -> Option<(String, String)> {
    if let Ok(forced) = std::env::var("RUSTFETCH_LATEST_VERSION") {
        let (version, codename) = split_cached_version_codename(&forced);
        if parse_semver(&version).is_some() {
            let code = codename.unwrap_or_else(|| codename_for_version(&version).to_string());
            return Some((version, code));
        }
    }
    let raw = fetch_latest_github_version()?;
    let (version, codename) = split_cached_version_codename(&raw);
    let code = codename.unwrap_or_else(|| codename_for_version(&version).to_string());
    Some((version, code))
}

/// Parses `[workspace.package].version` or `[package].version` from a `Cargo.toml`
/// manifest fetched from the repository.
pub fn parse_cargo_toml_version(toml_text: &str) -> Option<String> {
    let mut in_pkg_section = false;
    for raw_line in toml_text.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let section = line[1..line.len() - 1].trim();
            in_pkg_section = section == "workspace.package" || section == "package";
            continue;
        }
        if !in_pkg_section {
            continue;
        }
        if let Some((key, rest)) = line.split_once('=')
            && key.trim() == "version"
        {
            let raw_val = rest.trim().trim_matches(|c| c == '"' || c == '\'');
            let clean = raw_val.trim().trim_start_matches(['v', 'V']);
            if parse_semver(clean).is_some() {
                return Some(clean.to_string());
            }
        }
    }
    None
}

/// Chooses the highest valid semver candidate between a GitHub release entry and
/// `Cargo.toml` on `main`.
pub fn pick_newer_candidate(
    release: Option<(String, Option<String>)>,
    main_cargo: Option<String>,
) -> Option<(String, Option<String>)> {
    match (release, main_cargo) {
        (Some((rel_ver, rel_code)), Some(cargo_ver)) => {
            if is_newer_version(&rel_ver, &cargo_ver) {
                Some((cargo_ver, None))
            } else {
                Some((rel_ver, rel_code))
            }
        }
        (Some(rel), None) => Some(rel),
        (None, Some(cargo_ver)) => Some((cargo_ver, None)),
        (None, None) => None,
    }
}

/// Fetches the latest version across GitHub Releases and `Cargo.toml` on `main`.
pub fn fetch_latest_github_version() -> Option<String> {
    let release =
        fetch_url(GITHUB_LATEST_RELEASE_URL).and_then(|body| parse_github_release_info(&body));
    let main_cargo =
        fetch_url(GITHUB_MAIN_CARGO_TOML_URL).and_then(|body| parse_cargo_toml_version(&body));
    let (version, codename) = pick_newer_candidate(release, main_cargo)?;
    Some(match codename {
        Some(code) => format!("{version}|{code}"),
        None => version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_module_name() {
        assert_eq!(Update.name(), "update");
    }

    #[test]
    fn test_parse_semver_and_is_newer() {
        assert_eq!(parse_semver("0.1.2"), Some((0, 1, 2)));
        assert_eq!(parse_semver("v1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_semver("invalid"), None);

        assert!(is_newer_version("0.1.2", "0.1.3"));
        assert!(is_newer_version("0.1.2", "v0.2.0"));
        assert!(is_newer_version("0.1.2", "1.0.0"));
        assert!(!is_newer_version("0.1.2", "0.1.2"));
        assert!(!is_newer_version("0.1.2", "0.1.1"));
    }

    #[test]
    fn test_parse_github_release_info() {
        let json = r#"{"tag_name": "v0.3.0", "name": "RustFetch v0.3.0 — \"Van Kedisi\""}"#;
        assert_eq!(
            parse_github_release_info(json),
            Some(("0.3.0".to_string(), Some("Van Kedisi".to_string())))
        );
    }

    #[test]
    fn test_format_update_message() {
        assert_eq!(
            format_update_message("0.1.2", "v0.2.0"),
            "Update available (v0.1.2 -> v0.2.0 \"Abyssinian\")"
        );
        assert_eq!(
            format_update_message("0.1.2", "0.2.0|Tekir"),
            "Update available (v0.1.2 -> v0.2.0 \"Tekir\")"
        );
        assert_eq!(codename_for_version("0.1.0"), "Turkish Van");
        assert_eq!(codename_for_version("0.1.1"), "Turkish Angora");
        assert_eq!(codename_for_version("0.1.2"), "Bengal");
        assert_eq!(codename_for_version("0.1.3"), "Maine Coon");
        assert_eq!(codename_for_version("0.1.4"), "Siamese");
        assert_eq!(codename_for_version("0.1.5"), "Ragdoll");
    }

    #[test]
    fn test_parse_default_ipv4_and_ipv6_routes() {
        let v4_table = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\nwlp0s20f3\t00000000\t0101A8C0\t0003\t0\t0\t600\t00000000\t0\t0\t0\n";
        assert!(parse_has_default_ipv4_route(v4_table));

        let v4_table_no_gw = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\nwlp0s20f3\t0001A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\t0\t0\t0\n";
        assert!(!parse_has_default_ipv4_route(v4_table_no_gw));

        let v6_table = "00000000000000000000000000000000 00 00000000000000000000000000000000 00 fe800000000000003e64cffffe7229e7 00000258 00000001 00000000 00000003 eth0\n";
        assert!(parse_has_default_ipv6_route(v6_table));

        let v6_loopback_only = "00000000000000000000000000000000 00 00000000000000000000000000000000 00 00000000000000000000000000000000 ffffffff 00000001 00000000 00200200 lo\n";
        assert!(!parse_has_default_ipv6_route(v6_loopback_only));
    }

    #[test]
    fn test_codename_overflow_is_safe() {
        // A hostile or corrupted version string must never panic in debug builds
        // (naive usize multiplication overflows there).
        let code = codename_for_version("18446744073709551615.0.0");
        assert!(!code.is_empty());
        let message = format_update_message("0.1.2", "18446744073709551615.0.0");
        assert!(message.starts_with("Update available (v0.1.2 -> v18446744073709551615.0.0"));
    }

    #[test]
    fn test_codename_sanitizer_strips_terminal_escapes() {
        assert_eq!(
            sanitize_codename("Van\u{1b}]0;x\u{7}Kedisi").as_deref(),
            Some("Van0xKedisi")
        );
        assert_eq!(sanitize_codename("\u{202e}evil").as_deref(), Some("evil"));
        assert_eq!(sanitize_codename("   "), None);
        assert_eq!(
            sanitize_codename("Abyssinian").as_deref(),
            Some("Abyssinian")
        );

        // Release titles are remote input: ESC sequences are removed before display.
        let hostile = "RustFetch v0.3.0 — \"\u{1b}[31mSiamese\u{1b}[0m\"";
        assert_eq!(
            extract_codename_from_release_title(hostile).as_deref(),
            Some("31mSiamese0m")
        );
    }

    #[test]
    fn test_cached_codename_is_sanitized() {
        let message = format_update_message("0.1.2", "0.2.0|Si\u{1b}[2Jamese");
        assert_eq!(message, "Update available (v0.1.2 -> v0.2.0 \"Si2Jamese\")");
    }

    #[test]
    fn test_parse_cargo_toml_version_and_pick_newer_candidate() {
        let manifest = r#"
[workspace]
members = ["core", "cli"]

[workspace.package]
version = "0.1.6" # current main bump
edition = "2024"

[dependencies]
serde = "1"
"#;
        assert_eq!(parse_cargo_toml_version(manifest).as_deref(), Some("0.1.6"));
        assert_eq!(
            parse_cargo_toml_version("[workspace]\nversion = \"1.0.0\"\n"),
            None
        );

        // main Cargo.toml newer than release -> picks Cargo.toml version
        assert_eq!(
            pick_newer_candidate(
                Some(("0.1.5".to_string(), Some("Ragdoll".to_string()))),
                Some("0.1.6".to_string())
            ),
            Some(("0.1.6".to_string(), None))
        );

        // release equals or beats main Cargo.toml -> preserves release codename
        assert_eq!(
            pick_newer_candidate(
                Some(("0.1.6".to_string(), Some("Van Kedisi".to_string()))),
                Some("0.1.6".to_string())
            ),
            Some(("0.1.6".to_string(), Some("Van Kedisi".to_string())))
        );

        // release missing -> falls back to main Cargo.toml
        assert_eq!(
            pick_newer_candidate(None, Some("0.2.0".to_string())),
            Some(("0.2.0".to_string(), None))
        );
    }
}
