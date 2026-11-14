use std::ffi::CString;
use std::mem::MaybeUninit;

use crate::{Info, Module};

pub struct Disk {
    pub mount_point: Option<String>,
}

/// Filesystem capacity information in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub avail_bytes: u64,
}

impl Default for Disk {
    fn default() -> Self {
        Self::new()
    }
}

impl Disk {
    pub const fn new() -> Self {
        Self { mount_point: None }
    }

    pub fn with_mount(mount_point: impl Into<String>) -> Self {
        Self {
            mount_point: Some(mount_point.into()),
        }
    }
}

impl Module for Disk {
    fn name(&self) -> &'static str {
        "disk"
    }

    fn detect(&self) -> Option<Info> {
        if let Some(mount) = &self.mount_point {
            let trimmed = mount.trim();
            if !trimmed.is_empty() && trimmed != "/" {
                let value = disk_value_for(trimmed)?;
                return Some(Info::new(format!("Disk ({trimmed})"), value));
            }
        }
        let (label, value) = read_disk_with_label()?;
        Some(Info::new(label, value))
    }
}

fn disk_value_for(path: &str) -> Option<String> {
    format_disk_stats(get_disk_stats(path)?)
}

/// Formats filesystem usage using the same formula as df: the denominator is
/// used + available-for-unprivileged-users (not the raw block count).
pub fn format_disk_stats(stats: DiskStats) -> Option<String> {
    if stats.total_bytes == 0 || stats.avail_bytes > stats.total_bytes {
        return None;
    }

    let used_bytes = stats.used_bytes.min(stats.total_bytes);
    let gib = 1024.0 * 1024.0 * 1024.0;
    let used_gib = used_bytes as f64 / gib;
    let total_gib = stats.total_bytes as f64 / gib;
    let denominator = used_bytes.saturating_add(stats.avail_bytes);
    let pct = if denominator == 0 {
        0
    } else {
        (used_bytes as f64 / denominator as f64 * 100.0).round() as u64
    };

    Some(format!("{used_gib:.1} GiB / {total_gib:.1} GiB ({pct}%)"))
}

pub fn format_disk_usage(total_bytes: u64, free_bytes: u64) -> Option<String> {
    if total_bytes == 0 || free_bytes > total_bytes {
        return None;
    }
    let used_bytes = total_bytes - free_bytes;
    format_disk_stats(DiskStats {
        total_bytes,
        used_bytes,
        avail_bytes: free_bytes,
    })
}

/// Queries the filesystem using libc::statvfs for the given path.
pub fn get_disk_stats(path: &str) -> Option<DiskStats> {
    let c_path = CString::new(path).ok()?;
    let mut stat = MaybeUninit::<libc::statvfs>::zeroed();

    // Call libc::statvfs safely
    let ret = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if ret != 0 {
        return None;
    }

    let stat = unsafe { stat.assume_init() };

    let block_size = if stat.f_frsize > 0 {
        stat.f_frsize
    } else if stat.f_bsize > 0 {
        stat.f_bsize
    } else {
        return None;
    };

    let total_bytes = stat.f_blocks.checked_mul(block_size)?;
    let free_bytes = stat.f_bfree.checked_mul(block_size)?;
    let avail_bytes = stat.f_bavail.checked_mul(block_size)?;
    if free_bytes > total_bytes || avail_bytes > total_bytes {
        return None;
    }

    Some(DiskStats {
        total_bytes,
        used_bytes: total_bytes - free_bytes,
        avail_bytes,
    })
}

/// Legacy helper returning (total, free) for callers that only need two values.
pub fn get_disk_usage(path: &str) -> Option<(u64, u64)> {
    let stats = get_disk_stats(path)?;
    Some((stats.total_bytes, stats.total_bytes - stats.used_bytes))
}

/// Reads disk usage for the root filesystem, falling back to /sysroot or /var
/// on immutable ostree/composefs systems where / is a tiny read-only mount.
pub fn read_disk_with_label() -> Option<(String, String)> {
    read_disk_from(get_disk_stats)
}

/// Testable variant of read_disk_with_label: the stat provider is injected.
pub fn read_disk_from<F>(mut stat: F) -> Option<(String, String)>
where
    F: FnMut(&str) -> Option<DiskStats>,
{
    let one_gib = 1024 * 1024 * 1024;
    let mut chosen = "/";
    let mut stats = stat("/")?;
    if stats.total_bytes < one_gib {
        for fallback in ["/sysroot", "/var"] {
            if let Some(candidate) = stat(fallback)
                && candidate.total_bytes >= one_gib
            {
                chosen = fallback;
                stats = candidate;
                break;
            }
        }
    }
    let value = format_disk_stats(stats)?;
    let label = if chosen == "/" {
        "Disk (/)".to_string()
    } else {
        format!("Disk ({chosen})")
    };
    Some((label, value))
}

pub fn read_disk() -> Option<String> {
    read_disk_with_label().map(|(_, value)| value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(total_bytes: u64, used_bytes: u64, avail_bytes: u64) -> DiskStats {
        DiskStats {
            total_bytes,
            used_bytes,
            avail_bytes,
        }
    }

    #[test]
    fn test_format_disk_usage_prompt_example() {
        let total_bytes = 255_443_180_052;
        let used_bytes = 48_533_130_445;
        let free_bytes = total_bytes - used_bytes;
        assert_eq!(
            format_disk_usage(total_bytes, free_bytes),
            Some("45.2 GiB / 237.9 GiB (19%)".to_string())
        );
    }

    #[test]
    fn test_format_disk_usage_fedora_workstation() {
        let gib = 1024 * 1024 * 1024;
        let total_bytes = (476.8 * gib as f64).round() as u64;
        let used_bytes = (105.4 * gib as f64).round() as u64;
        let free_bytes = total_bytes - used_bytes;
        assert_eq!(
            format_disk_usage(total_bytes, free_bytes),
            Some("105.4 GiB / 476.8 GiB (22%)".to_string())
        );
    }

    #[test]
    fn test_format_disk_usage_edge_cases() {
        assert_eq!(format_disk_usage(0, 0), None);
        assert_eq!(format_disk_usage(100, 200), None);
        let gib = 1024 * 1024 * 1024;
        assert_eq!(
            format_disk_usage(100 * gib, 100 * gib),
            Some("0.0 GiB / 100.0 GiB (0%)".to_string())
        );
        assert_eq!(
            format_disk_usage(100 * gib, 0),
            Some("100.0 GiB / 100.0 GiB (100%)".to_string())
        );
    }

    #[test]
    fn test_format_disk_stats_uses_df_denominator() {
        let gib = 1024 * 1024 * 1024;
        // 90 GiB used, 5 GiB reserved but available to root only -> df says 90/95.
        let df = format_disk_stats(stats(100 * gib, 90 * gib, 5 * gib)).unwrap();
        assert!(
            df.ends_with("(95%)"),
            "unexpected df-style percentage: {df}"
        );
    }

    #[test]
    fn test_read_disk_from_labels_sysroot_fallback() {
        let gib = 1024 * 1024 * 1024;
        let provider = |path: &str| match path {
            "/" => Some(stats(gib / 4, gib / 8, gib / 8)),
            "/sysroot" => Some(stats(100 * gib, 60 * gib, 30 * gib)),
            _ => None,
        };
        let (label, value) = read_disk_from(provider).unwrap();
        assert_eq!(label, "Disk (/sysroot)");
        assert!(value.contains("100.0 GiB"));
    }

    #[test]
    fn test_read_disk_from_keeps_root_label_for_normal_systems() {
        let gib = 1024 * 1024 * 1024;
        let (label, _) = read_disk_from(|path: &str| match path {
            "/" => Some(stats(500 * gib, 100 * gib, 400 * gib)),
            _ => None,
        })
        .unwrap();
        assert_eq!(label, "Disk (/)");
    }

    #[test]
    fn test_read_disk_from_var_fallback() {
        let gib = 1024 * 1024 * 1024;
        let (label, _) = read_disk_from(|path: &str| match path {
            "/" => Some(stats(gib / 2, 0, gib / 2)),
            "/var" => Some(stats(10 * gib, gib, 9 * gib)),
            _ => None,
        })
        .unwrap();
        assert_eq!(label, "Disk (/var)");
    }

    #[test]
    fn test_statvfs_and_detect_live() {
        let usage = get_disk_usage("/");
        assert!(usage.is_some(), "get_disk_usage('/') should succeed");
        let (total, free) = usage.unwrap();
        assert!(total > 0);
        assert!(total >= free);

        let stats = get_disk_stats("/").unwrap();
        assert!(stats.avail_bytes <= stats.total_bytes);
        assert!(stats.used_bytes + stats.avail_bytes <= stats.total_bytes);

        let module = Disk::new();
        assert_eq!(module.name(), "disk");
        let detected = module.detect();
        assert!(detected.is_some());
        let info = detected.unwrap();
        assert_eq!(info.label, "Disk (/)");
        assert!(info.value.contains("GiB"));
        assert!(info.value.contains('%'));

        let custom_mount = Disk::with_mount("/tmp");
        if let Some(tmp_info) = custom_mount.detect() {
            assert_eq!(tmp_info.label, "Disk (/tmp)");
        }
    }

    #[test]
    fn test_statvfs_nonexistent_path() {
        assert_eq!(
            get_disk_usage("/nonexistent_path_for_rustfetch_test_12345"),
            None
        );
        assert_eq!(
            get_disk_stats("/nonexistent_path_for_rustfetch_test_12345"),
            None
        );
    }
}
