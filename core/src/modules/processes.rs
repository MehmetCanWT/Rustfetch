use crate::{Info, Module};
use std::fs;
use std::path::Path;

pub struct Processes;

impl Module for Processes {
    fn name(&self) -> &'static str {
        "processes"
    }

    fn detect(&self) -> Option<Info> {
        let pids = count_pid_directories(Path::new("/proc"))?;
        let load = fs::read_to_string("/proc/loadavg")
            .ok()
            .and_then(|content| parse_loadavg_load1(&content));
        let value = match load {
            Some(load1) => format!("{pids} (load {load1:.2})"),
            None => pids.to_string(),
        };
        Some(Info::new("Processes", value))
    }
}

/// Counts the numeric directories directly under `root` — one per process.
///
/// `/proc/loadavg` reports kernel scheduling entities (threads included),
/// which is not the process count users expect; the `/proc` directory listing
/// matches `ps -e | wc -l` semantics instead.
pub fn count_pid_directories(root: &Path) -> Option<usize> {
    let entries = fs::read_dir(root).ok()?;
    let count = entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit())
        })
        .count();
    (count > 0).then_some(count)
}

/// First field of `/proc/loadavg` (1-minute load average).
pub fn parse_loadavg_load1(content: &str) -> Option<f64> {
    let load: f64 = content.split_whitespace().next()?.parse().ok()?;
    load.is_finite().then_some(load)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_processes_{label}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_processes_module_name() {
        assert_eq!(Processes.name(), "processes");
    }

    #[test]
    fn test_count_pid_directories_counts_only_numeric_entries() {
        let root = temp_dir("pids");
        fs::create_dir_all(root.join("1")).unwrap();
        fs::create_dir_all(root.join("4242")).unwrap();
        fs::write(root.join("123"), b"").unwrap();
        fs::create_dir_all(root.join("self")).unwrap();
        fs::create_dir_all(root.join("sys")).unwrap();
        fs::create_dir_all(root.join("12abc")).unwrap();
        assert_eq!(count_pid_directories(&root), Some(3));
        let empty = temp_dir("empty");
        assert_eq!(count_pid_directories(&empty), None);
        assert_eq!(
            count_pid_directories(std::path::Path::new("/definitely/missing")),
            None
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&empty);
    }

    #[test]
    fn test_parse_loadavg_load1() {
        assert_eq!(
            parse_loadavg_load1("6.72 5.83 5.61 3/1545 234197"),
            Some(6.72)
        );
        assert_eq!(parse_loadavg_load1("0.00 0.01 0.05 1/100 42"), Some(0.0));
        assert_eq!(parse_loadavg_load1(""), None);
        assert_eq!(parse_loadavg_load1("invalid"), None);
        assert_eq!(parse_loadavg_load1("NaN 1 1 1/1 1"), None);
    }
}
