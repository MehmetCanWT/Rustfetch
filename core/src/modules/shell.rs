use crate::{Info, Module};
use std::path::Path;

pub struct Shell;

impl Module for Shell {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn detect(&self) -> Option<Info> {
        let shell = detect_shell_from_process_tree().or_else(|| {
            std::env::var("SHELL")
                .ok()
                .and_then(|path| parse_shell(&path))
        })?;
        Some(Info::new("Shell", shell))
    }
}

/// Known interactive shell binary names.
const KNOWN_SHELLS: &[&str] = &[
    "bash", "dash", "sh", "zsh", "fish", "nu", "nushell", "ksh", "ksh93", "mksh", "pdksh", "oksh",
    "tcsh", "csh", "yash", "elvish", "xonsh", "posh", "ash",
];

/// Walks up the process tree (bounded) looking for a known shell.
///
/// `$SHELL` is the *login* shell and can be wrong when another shell or a
/// wrapper starts rustfetch, so the process tree is authoritative and `$SHELL`
/// is only the last resort.
fn detect_shell_from_process_tree() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let start = std::os::unix::process::parent_id();
        walk_parent_shell(start, parent_pid, |pid| {
            std::fs::read_to_string(format!("/proc/{pid}/comm"))
                .ok()
                .map(|comm| comm.trim().to_string())
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn parent_pid(pid: u32) -> Option<u32> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("PPid:\t")
            .and_then(|rest| rest.trim().parse::<u32>().ok())
    })
}

/// Bounded (max 6 levels) parent walk with injectable process accessors, so the
/// detection logic is unit-testable without spawning processes.
pub fn walk_parent_shell(
    start_pid: u32,
    get_parent: impl Fn(u32) -> Option<u32>,
    get_comm: impl Fn(u32) -> Option<String>,
) -> Option<String> {
    let mut pid = start_pid;
    for _ in 0..6 {
        if pid <= 1 {
            break;
        }
        if let Some(comm) = get_comm(pid) {
            let trimmed = comm.trim();
            if let Some(shell) = known_shell_name(trimmed) {
                return Some(shell.to_string());
            }
            // Wrapper binaries can be a versioned shell, e.g. "bash-5.2" or "zsh".
            if let Some(stem) = trimmed.split('-').next() {
                if let Some(shell) = known_shell_name(stem) {
                    return Some(shell.to_string());
                }
            }
        }
        // Do not loop forever on a corrupted/racy parent chain.
        match get_parent(pid) {
            Some(parent) if parent != 0 && parent != pid => pid = parent,
            _ => break,
        }
    }
    None
}

fn known_shell_name(name: &str) -> Option<&'static str> {
    KNOWN_SHELLS.iter().copied().find(|known| *known == name)
}

pub fn parse_shell(path: &str) -> Option<String> {
    let trimmed = path.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let file_name = Path::new(trimmed).file_name()?.to_str()?;
    if file_name.is_empty() {
        return None;
    }
    Some(file_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shell_fixtures() {
        assert_eq!(parse_shell("/bin/bash").as_deref(), Some("bash"));
        assert_eq!(parse_shell("/usr/bin/bash").as_deref(), Some("bash"));
        assert_eq!(parse_shell("/usr/bin/zsh").as_deref(), Some("zsh"));
        assert_eq!(parse_shell("/usr/bin/fish").as_deref(), Some("fish"));
        assert_eq!(parse_shell("/bin/sh").as_deref(), Some("sh"));
        assert_eq!(parse_shell("/usr/local/bin/nu").as_deref(), Some("nu"));
    }

    #[test]
    fn test_parse_shell_trailing_slashes_and_whitespace() {
        assert_eq!(parse_shell("/bin/bash/").as_deref(), Some("bash"));
        assert_eq!(parse_shell("  /usr/bin/fish  ").as_deref(), Some("fish"));
    }

    #[test]
    fn test_parse_shell_just_name() {
        assert_eq!(parse_shell("bash").as_deref(), Some("bash"));
        assert_eq!(parse_shell("zsh").as_deref(), Some("zsh"));
    }

    #[test]
    fn test_parse_shell_invalid() {
        assert_eq!(parse_shell(""), None);
        assert_eq!(parse_shell("   "), None);
        assert_eq!(parse_shell("/"), None);
        assert_eq!(parse_shell("///"), None);
    }

    #[test]
    fn test_walk_parent_shell_finds_shell_behind_wrapper() {
        // rustfetch <- sh -c wrapper <- zsh
        let parents = |pid: u32| match pid {
            100 => Some(200),
            200 => Some(300),
            _ => Some(1),
        };
        let comms = |pid: u32| match pid {
            100 => Some("rustfetch".to_string()),
            200 => Some("sh".to_string()),
            _ => Some("zsh".to_string()),
        };
        assert_eq!(
            walk_parent_shell(100, parents, comms).as_deref(),
            Some("sh")
        );

        // A non-shell wrapper is skipped and the next ancestor is used.
        let parents2 = |pid: u32| match pid {
            100 => Some(200),
            200 => Some(300),
            _ => Some(1),
        };
        let comms2 = |pid: u32| match pid {
            100 => Some("rustfetch".to_string()),
            200 => Some("sudo".to_string()),
            _ => Some("bash".to_string()),
        };
        assert_eq!(
            walk_parent_shell(100, parents2, comms2).as_deref(),
            Some("bash")
        );
    }

    #[test]
    fn test_walk_parent_shell_bounded_and_robust() {
        // Self-parenting chain must terminate (no infinite loop).
        assert_eq!(
            walk_parent_shell(42, Some, |_| Some("rustfetch".to_string())),
            None
        );
        assert_eq!(walk_parent_shell(0, |_| None, |_| None), None);
        // Versioned shell wrapper name.
        assert_eq!(
            walk_parent_shell(
                7,
                |_| Some(8),
                |pid| {
                    if pid == 7 {
                        Some("bash-5.2".to_string())
                    } else {
                        None
                    }
                }
            )
            .as_deref(),
            Some("bash")
        );
    }

    #[test]
    fn test_known_shells() {
        assert_eq!(known_shell_name("fish"), Some("fish"));
        assert_eq!(known_shell_name("nushell"), Some("nushell"));
        assert_eq!(known_shell_name("python"), None);
    }
}
