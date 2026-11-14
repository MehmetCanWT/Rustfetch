use crate::{Info, Module};
use std::fs;
use std::path::Path;

pub struct Memory;

impl Module for Memory {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn detect(&self) -> Option<Info> {
        let content = read_meminfo()?;
        let value = parse_meminfo_with_arc(&content, read_zfs_arc_kb())?;
        Some(Info::new("Memory", value))
    }
}

fn read_meminfo() -> Option<String> {
    std::fs::read_to_string("/proc/meminfo").ok()
}

fn read_zfs_arc_kb() -> u64 {
    read_zfs_arc_kb_from(Path::new("/proc/spl/kstat/zfs/arcstats"))
}

/// Reads the ZFS ARC size in KiB from an arcstats fixture.
pub fn read_zfs_arc_kb_from(path: &Path) -> u64 {
    let Ok(arc_content) = fs::read_to_string(path) else {
        return 0;
    };
    for line in arc_content.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some("size")
            && let Some(arc_kb) = parts
                .nth(1)
                .and_then(|v| v.parse::<u64>().ok())
                .map(|b| b / 1024)
        {
            return arc_kb;
        }
    }
    0
}

pub fn format_kb(kb: u64) -> String {
    const KIB_PER_MIB: f64 = 1024.0;
    const KIB_PER_GIB: f64 = 1024.0 * 1024.0;

    let kb_f = kb as f64;
    if kb_f >= KIB_PER_GIB {
        format!("{:.1} GiB", kb_f / KIB_PER_GIB)
    } else {
        format!("{:.1} MiB", kb_f / KIB_PER_MIB)
    }
}

fn parse_kb_line(rest: &str) -> Option<u64> {
    rest.split_whitespace().next()?.parse().ok()
}

pub fn parse_meminfo(content: &str) -> Option<String> {
    parse_meminfo_with_arc(content, 0)
}

/// Parses /proc/meminfo, subtracting reclaimable ZFS ARC memory from "used".
///
/// A corrupt or swapped meminfo where MemAvailable exceeds MemTotal is treated
/// as zero usage instead of dropping the whole row; a ZFS ARC larger than the
/// used amount is clamped so the value never underflows.
pub fn parse_meminfo_with_arc(content: &str, zfs_arc_kb: u64) -> Option<String> {
    let mut mem_total = None;
    let mut mem_available = None;
    let mut mem_free = None;
    let mut buffers = None;
    let mut cached = None;

    for line in content.lines() {
        if let Some((key, rest)) = line.split_once(':') {
            match key.trim() {
                "MemTotal" => mem_total = parse_kb_line(rest),
                "MemAvailable" => mem_available = parse_kb_line(rest),
                "MemFree" => mem_free = parse_kb_line(rest),
                "Buffers" => buffers = parse_kb_line(rest),
                "Cached" => cached = parse_kb_line(rest),
                _ => {}
            }
            if mem_total.is_some() && mem_available.is_some() {
                break;
            }
        }
    }

    let total = mem_total?;
    let available = mem_available.or_else(|| {
        let free = mem_free?;
        Some(free + buffers.unwrap_or(0) + cached.unwrap_or(0))
    })?;

    let used = total.saturating_sub(available);
    let arc = zfs_arc_kb.min(used);
    let used = used.saturating_sub(arc);

    Some(format!("{} / {}", format_kb(used), format_kb(total)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Realistic Fedora Linux /proc/meminfo fixture.
    const FEDORA_MEMINFO: &str = r#"MemTotal:       16252928 kB
MemFree:         2092500 kB
MemAvailable:   12897485 kB
Buffers:            4184 kB
Cached:          6128348 kB
SwapCached:            0 kB
Active:          6605884 kB
Inactive:        5555404 kB
Active(anon):    6605884 kB
Inactive(anon):        0 kB
Active(file):          0 kB
Inactive(file):  5555404 kB
Unevictable:      485868 kB
Mlocked:             136 kB
SwapTotal:       8388604 kB
SwapFree:        8388604 kB
Zswap:                 0 kB
Zswapped:              0 kB
Dirty:             13244 kB
Writeback:             0 kB
AnonPages:       6515228 kB
Mapped:          1167996 kB
Shmem:            577396 kB
KReclaimable:     477664 kB
Slab:             486212 kB
SReclaimable:     229556 kB
SUnreclaim:       256656 kB
KernelStack:       18080 kB
PageTables:        60304 kB
SecPageTables:      2056 kB
"#;

    #[test]
    fn test_parse_meminfo_fedora() {
        let result = parse_meminfo(FEDORA_MEMINFO);
        assert_eq!(result.as_deref(), Some("3.2 GiB / 15.5 GiB"));
    }

    #[test]
    fn test_format_kb() {
        assert_eq!(format_kb(1048576), "1.0 GiB");
        assert_eq!(format_kb(16252928), "15.5 GiB");
        assert_eq!(format_kb(3355443), "3.2 GiB");
        assert_eq!(format_kb(524288), "512.0 MiB");
        assert_eq!(format_kb(1024), "1.0 MiB");
        assert_eq!(format_kb(0), "0.0 MiB");
    }

    #[test]
    fn test_parse_meminfo_without_memavailable() {
        let sample =
            "MemTotal: 4194304 kB\nMemFree: 1048576 kB\nBuffers: 524288 kB\nCached: 524288 kB\n";
        assert_eq!(parse_meminfo(sample), Some("2.0 GiB / 4.0 GiB".to_string()));
    }

    #[test]
    fn test_parse_meminfo_empty_and_invalid() {
        assert_eq!(parse_meminfo(""), None);
        assert_eq!(parse_meminfo("MemTotal: 16252928 kB"), None);
        assert_eq!(parse_meminfo("MemAvailable: 12897485 kB"), None);
    }

    #[test]
    fn test_available_greater_than_total_does_not_drop_row() {
        let invalid = "MemTotal: 1000 kB\nMemAvailable: 2000 kB\n";
        assert_eq!(
            parse_meminfo(invalid),
            Some("0.0 MiB / 1.0 MiB".to_string())
        );
    }

    #[test]
    fn test_zfs_arc_is_subtracted_and_clamped() {
        let sample = "MemTotal: 16000000 kB\nMemAvailable: 12000000 kB\n";
        // 4 GiB used, 1 GiB ARC -> 3 GiB used
        assert_eq!(
            parse_meminfo_with_arc(sample, 1024 * 1024),
            Some("2.8 GiB / 15.3 GiB".to_string())
        );
        // ARC larger than used must clamp to zero instead of underflowing
        assert_eq!(
            parse_meminfo_with_arc(sample, 8 * 1024 * 1024),
            Some("0.0 MiB / 15.3 GiB".to_string())
        );
    }

    #[test]
    fn test_read_zfs_arc_kb_from_fixture() {
        let path =
            std::env::temp_dir().join(format!("rustfetch_meminfo_arc_{}", std::process::id()));
        fs::write(
            &path,
            "name                            type data\nsize                            4    1073741824\n",
        )
        .unwrap();
        assert_eq!(read_zfs_arc_kb_from(&path), 1024 * 1024);
        let _ = fs::remove_file(&path);
        assert_eq!(read_zfs_arc_kb_from(&path), 0);
    }

    #[test]
    fn test_memory_module_name() {
        assert_eq!(Memory.name(), "memory");
    }

    #[test]
    fn test_memory_detect() {
        let mem = Memory;
        if let Some(info) = mem.detect() {
            assert_eq!(info.label, "Memory");
            assert!(info.value.contains(" / "));
        }
    }
}
