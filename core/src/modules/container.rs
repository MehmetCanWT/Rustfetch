use crate::{Info, Module};
use std::fs;
use std::path::Path;

pub struct Container;

impl Module for Container {
    fn name(&self) -> &'static str {
        "container"
    }

    fn detect(&self) -> Option<Info> {
        let name = detect_container()?;
        Some(Info::new("Container", name))
    }
}

pub fn detect_container() -> Option<String> {
    detect_container_impl(
        |p| Path::new(p).exists(),
        |p| fs::symlink_metadata(p).map(|m| m.is_dir()).unwrap_or(false),
        |var| std::env::var(var).ok(),
        |p| fs::read_to_string(p).ok(),
        |p| fs::read(p).ok(),
    )
}

pub fn detect_container_impl<FPath, FDir, FEnv, FStr, FBytes>(
    path_exists: FPath,
    is_real_dir: FDir,
    env_lookup: FEnv,
    read_str: FStr,
    read_bytes: FBytes,
) -> Option<String>
where
    FPath: Fn(&str) -> bool,
    FDir: Fn(&str) -> bool,
    FEnv: Fn(&str) -> Option<String>,
    FStr: Fn(&str) -> Option<String>,
    FBytes: Fn(&str) -> Option<Vec<u8>>,
{
    // 1. Toolbx container
    if path_exists("/run/.toolboxenv") {
        return Some("Toolbx".to_string());
    }

    // 2. Distrobox container
    let is_host_dir = is_real_dir("/run/host");
    let has_container_id = env_lookup("CONTAINER_ID")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if has_container_id || is_host_dir || env_lookup("DISTROBOX_ENTERED").is_some() {
        return Some("Distrobox".to_string());
    }

    // 3. Podman rootless / container marker
    if path_exists("/run/.containerenv") {
        return Some("Podman".to_string());
    }

    // 4. Docker container marker
    if path_exists("/.dockerenv") {
        return Some("Docker".to_string());
    }

    // 5. OrbStack container / VM
    if path_exists("/run/orbstack") || path_exists("/.orbstack") {
        return Some("OrbStack".to_string());
    }

    // 6. Apple Silicon Rosetta binary translation
    if path_exists("/proc/sys/fs/binfmt_misc/rosetta") {
        return Some("Rosetta".to_string());
    }

    // 7. Systemd container marker
    if let Some(content) = read_str("/run/systemd/container") {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return Some(format_runtime(trimmed));
        }
    }

    // 8. LXC / LXD cgroup marker (works for non-systemd containers too)
    for cgroup_path in ["/proc/1/cgroup", "/proc/self/cgroup"] {
        if let Some(cgroup) = read_str(cgroup_path)
            && cgroup_marker(&cgroup)
        {
            return Some("LXC".to_string());
        }
    }

    // 9. Flatpak / Snap sandboxes
    if path_exists("/.flatpak-info") || env_lookup("FLATPAK_ID").is_some() {
        return Some("Flatpak".to_string());
    }
    if env_lookup("SNAP").is_some() {
        return Some("Snap".to_string());
    }

    // 10. WSL1 / WSL2 detection via the kernel release string
    if let Some(ver) = read_str("/proc/version") {
        let lower = ver.to_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            let release = read_str("/proc/sys/kernel/osrelease")
                .unwrap_or_default()
                .to_lowercase();
            let combined = format!("{lower} {release}");
            if is_wsl2_marker(&combined) {
                return Some("WSL2".to_string());
            }
            return Some("WSL1".to_string());
        }
    }

    // 11. PID 1 environment container variable
    if let Some(environ) = read_bytes("/proc/1/environ") {
        for entry in environ.split(|&b| b == 0) {
            if let Ok(s) = std::str::from_utf8(entry)
                && let Some(val) = s.strip_prefix("container=")
            {
                let val = val.trim();
                if !val.is_empty() {
                    return Some(format_runtime(val));
                }
            }
        }
    }

    // 12. Virtual machine inspection via DMI
    let product = read_str("/sys/class/dmi/id/product_name").unwrap_or_default();
    let vendor = read_str("/sys/class/dmi/id/sys_vendor").unwrap_or_default();
    let combined = format!("{product} {vendor}").to_lowercase();

    if combined.contains("kvm") {
        Some("KVM".to_string())
    } else if combined.contains("qemu") {
        Some("QEMU".to_string())
    } else if combined.contains("virtualbox") || combined.contains("innotek") {
        Some("VirtualBox".to_string())
    } else if combined.contains("vmware") {
        Some("VMware".to_string())
    } else if combined.contains("bochs") {
        Some("Bochs".to_string())
    } else if combined.contains("hyper-v")
        || (combined.contains("microsoft") && combined.contains("virtual"))
    {
        Some("Hyper-V".to_string())
    } else if combined.contains("xen") {
        Some("Xen".to_string())
    } else if combined.contains("parallels") {
        Some("Parallels".to_string())
    } else {
        None
    }
}

/// True when a `/proc/*/cgroup` content indicates an LXC/LXD payload.
fn cgroup_marker(cgroup: &str) -> bool {
    cgroup.contains("lxc") || cgroup.contains("lxd")
}

/// Distinguishes WSL2 from WSL1 using the kernel release string.
///
/// WSL1 always reports a 4.4.y Microsoft kernel;
/// WSL2 reports "standard-WSL2" (or a newer 5.x/6.x Microsoft kernel).
fn is_wsl2_marker(combined_lowercase: &str) -> bool {
    if combined_lowercase.contains("wsl2") {
        return true;
    }
    if combined_lowercase.contains("4.4.0") {
        return false;
    }
    true
}

pub fn format_runtime(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "docker" => "Docker".to_string(),
        "podman" => "Podman".to_string(),
        "lxc" => "LXC".to_string(),
        "systemd-nspawn" | "nspawn" => "systemd-nspawn".to_string(),
        "kubevirt" => "KubeVirt".to_string(),
        "distrobox" => "Distrobox".to_string(),
        "toolbx" | "toolbox" => "Toolbx".to_string(),
        "orbstack" => "OrbStack".to_string(),
        "rosetta" => "Rosetta".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_module_name() {
        assert_eq!(Container.name(), "container");
    }

    #[test]
    fn test_format_runtime() {
        assert_eq!(format_runtime("docker"), "Docker");
        assert_eq!(format_runtime("podman"), "Podman");
        assert_eq!(format_runtime("lxc"), "LXC");
        assert_eq!(format_runtime("nspawn"), "systemd-nspawn");
        assert_eq!(format_runtime("distrobox"), "Distrobox");
        assert_eq!(format_runtime("toolbx"), "Toolbx");
        assert_eq!(format_runtime("toolbox"), "Toolbx");
        assert_eq!(format_runtime("orbstack"), "OrbStack");
        assert_eq!(format_runtime("rosetta"), "Rosetta");
        assert_eq!(format_runtime("custom"), "Custom");
    }

    #[test]
    fn test_detect_container_toolbx_precedence() {
        let res = detect_container_impl(
            |p| p == "/run/.toolboxenv" || p == "/run/.containerenv",
            |_| false,
            |_| None,
            |_| None,
            |_| None,
        );
        assert_eq!(res, Some("Toolbx".to_string()));
    }

    #[test]
    fn test_detect_container_distrobox_via_host_dir() {
        let res = detect_container_impl(
            |p| p == "/run/.containerenv",
            |p| p == "/run/host",
            |_| None,
            |_| None,
            |_| None,
        );
        assert_eq!(res, Some("Distrobox".to_string()));
    }

    #[test]
    fn test_detect_container_distrobox_via_container_id() {
        let res = detect_container_impl(
            |_| false,
            |_| false,
            |k| {
                if k == "CONTAINER_ID" {
                    Some("arch-box".to_string())
                } else {
                    None
                }
            },
            |_| None,
            |_| None,
        );
        assert_eq!(res, Some("Distrobox".to_string()));
    }

    #[test]
    fn test_detect_container_ignores_empty_container_id() {
        let res = detect_container_impl(
            |_| false,
            |_| false,
            |k| {
                if k == "CONTAINER_ID" {
                    Some("  ".to_string())
                } else {
                    None
                }
            },
            |_| None,
            |_| None,
        );
        assert_eq!(res, None);
    }

    #[test]
    fn test_detect_container_podman() {
        let res = detect_container_impl(
            |p| p == "/run/.containerenv",
            |_| false,
            |_| None,
            |_| None,
            |_| None,
        );
        assert_eq!(res, Some("Podman".to_string()));
    }

    #[test]
    fn test_detect_container_orbstack() {
        let res = detect_container_impl(
            |p| p == "/run/orbstack",
            |_| false,
            |_| None,
            |_| None,
            |_| None,
        );
        assert_eq!(res, Some("OrbStack".to_string()));
    }

    #[test]
    fn test_detect_container_rosetta() {
        let res = detect_container_impl(
            |p| p == "/proc/sys/fs/binfmt_misc/rosetta",
            |_| false,
            |_| None,
            |_| None,
            |_| None,
        );
        assert_eq!(res, Some("Rosetta".to_string()));
    }

    #[test]
    fn test_detect_container_wsl_variants() {
        let wsl1 = detect_container_impl(
            |_| false,
            |_| false,
            |_| None,
            |p| match p {
                "/proc/version" => Some(
                    "Linux version 4.4.0-19041-Microsoft (Microsoft@Microsoft.com) #1 SMP".into(),
                ),
                "/proc/sys/kernel/osrelease" => Some("4.4.0-19041-Microsoft".into()),
                _ => None,
            },
            |_| None,
        );
        assert_eq!(wsl1, Some("WSL1".to_string()));

        let wsl2 = detect_container_impl(
            |_| false,
            |_| false,
            |_| None,
            |p| match p {
                "/proc/version" => Some(
                    "Linux version 5.15.153.1-microsoft-standard-WSL2 (root@container) #1 SMP"
                        .into(),
                ),
                "/proc/sys/kernel/osrelease" => Some("5.15.153.1-microsoft-standard-WSL2".into()),
                _ => None,
            },
            |_| None,
        );
        assert_eq!(wsl2, Some("WSL2".to_string()));
    }

    #[test]
    fn test_detect_container_lxc_cgroup() {
        let lxc = detect_container_impl(
            |_| false,
            |_| false,
            |_| None,
            |p| match p {
                "/proc/1/cgroup" => Some("12:pids:/lxc.payload.mycontainer\n".into()),
                _ => None,
            },
            |_| None,
        );
        assert_eq!(lxc, Some("LXC".to_string()));
    }

    #[test]
    fn test_detect_container_vm_types() {
        for (vendor, product, expected) in [
            ("Microsoft Corporation", "Virtual Machine", "Hyper-V"),
            ("Xen", "HVM domU", "Xen"),
            (
                "Parallels Software International Inc.",
                "Parallels Virtual Platform",
                "Parallels",
            ),
        ] {
            let res = detect_container_impl(
                |_| false,
                |_| false,
                |_| None,
                |p| match p {
                    "/sys/class/dmi/id/product_name" => Some(product.to_string()),
                    "/sys/class/dmi/id/sys_vendor" => Some(vendor.to_string()),
                    _ => None,
                },
                |_| None,
            );
            assert_eq!(
                res.as_deref(),
                Some(expected),
                "vendor={vendor} product={product}"
            );
        }
    }
}
