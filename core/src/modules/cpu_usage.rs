use crate::{Info, Module};
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Sampling window between the two /proc/stat reads.
const SAMPLE_WINDOW: Duration = Duration::from_millis(50);

pub struct CpuUsage;

impl Module for CpuUsage {
    fn name(&self) -> &'static str {
        "cpu_usage"
    }

    fn is_slow(&self) -> bool {
        true
    }

    fn detect(&self) -> Option<Info> {
        let (total1, idle1) = read_proc_stat(Path::new("/proc/stat"))?;
        std::thread::sleep(SAMPLE_WINDOW);
        let (total2, idle2) = read_proc_stat(Path::new("/proc/stat"))?;
        let usage = usage_percent(total1, idle1, total2, idle2)?;
        Some(Info::new("CPU Usage", format!("{usage}%")))
    }
}

/// Computes the aggregate CPU utilisation percentage between two samples.
pub fn usage_percent(total1: u64, idle1: u64, total2: u64, idle2: u64) -> Option<u32> {
    let total_delta = total2.saturating_sub(total1);
    let idle_delta = idle2.saturating_sub(idle1);
    if total_delta == 0 {
        return None;
    }
    let used = total_delta.saturating_sub(idle_delta);
    let usage = ((used as f64 / total_delta as f64) * 100.0).round() as u32;
    Some(usage.min(100))
}

/// Parses the aggregate line of /proc/stat into (total, idle) jiffies.
///
/// The guest/guest_nice fields are already accounted for inside user/nice, so
/// they are excluded from the total to avoid double counting on virtualised
/// hosts.
pub fn parse_proc_stat_line(line: &str) -> Option<(u64, u64)> {
    if !line.starts_with("cpu ") {
        return None;
    }
    let vals: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    if vals.len() < 4 {
        return None;
    }
    let idle = vals[3].saturating_add(vals.get(4).copied().unwrap_or(0)); // idle + iowait
    let total: u64 = vals.iter().take(8).copied().fold(0, u64::saturating_add);
    Some((total, idle))
}

pub fn sample_proc_stat() -> Option<(u64, u64)> {
    read_proc_stat(Path::new("/proc/stat"))
}

pub fn read_proc_stat(path: &Path) -> Option<(u64, u64)> {
    let stat = fs::read_to_string(path).ok()?;
    let first_line = stat.lines().next()?;
    parse_proc_stat_line(first_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_file(tag: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "rustfetch_cpu_usage_test_{tag}_{}",
            std::process::id()
        ));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_cpu_usage_name() {
        assert_eq!(CpuUsage.name(), "cpu_usage");
    }

    #[test]
    fn test_parse_proc_stat_line() {
        let line = "cpu  2255 34 2290 22625563 6290 127 456 0 0 0";
        let (total, idle) = parse_proc_stat_line(line).unwrap();
        assert_eq!(idle, 22625563 + 6290);
        assert!(total > idle);
    }

    #[test]
    fn test_parse_proc_stat_invalid() {
        assert!(parse_proc_stat_line("cpu0 100 200").is_none());
        assert!(parse_proc_stat_line("intr 12345").is_none());
        assert!(parse_proc_stat_line("").is_none());
        assert!(parse_proc_stat_line("cpu 1 2 3").is_none());
    }

    #[test]
    fn test_guest_time_is_not_double_counted() {
        let base = "cpu  100 0 50 850 0 0 0 0 0 0";
        let with_guest = "cpu  100 0 50 850 0 0 0 0 50 25";
        assert_eq!(
            parse_proc_stat_line(base),
            parse_proc_stat_line(with_guest),
            "guest/guest_nice are already inside user/nice"
        );
    }

    #[test]
    fn test_usage_percent_math() {
        assert_eq!(usage_percent(1000, 500, 1100, 550), Some(50));
        assert_eq!(usage_percent(1000, 500, 1100, 1100), Some(0));
        assert_eq!(usage_percent(1000, 500, 1100, 500), Some(100));
        assert_eq!(usage_percent(1000, 500, 1000, 500), None);
        assert_eq!(usage_percent(1000, 500, 900, 400), None);
    }

    #[test]
    fn test_read_proc_stat_from_fixture() {
        let path = unique_file("stat", "cpu  10 0 5 85 0 0 0 0 0 0\ncpu0 1 2 3 4\n");
        let (total, idle) = read_proc_stat(&path).unwrap();
        assert_eq!(total, 100);
        assert_eq!(idle, 85);
        let _ = fs::remove_file(&path);
    }
}
