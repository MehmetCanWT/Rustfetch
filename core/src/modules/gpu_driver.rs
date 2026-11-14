use crate::{Info, Module};
use std::fs;
use std::path::Path;

pub struct GpuDriver;

impl Module for GpuDriver {
    fn name(&self) -> &'static str {
        "gpu_driver"
    }

    fn detect(&self) -> Option<Info> {
        let value = detect_gpu_driver()?;
        Some(Info::new("GPU Driver", value))
    }
}

pub fn detect_gpu_driver() -> Option<String> {
    detect_gpu_driver_from(
        Path::new("/sys/class/drm"),
        Path::new("/proc/driver/nvidia/version"),
        Path::new("/sys/module"),
    )
}

/// Detects bound GPU kernel modules and their versions.
///
/// The module version comes from /sys/module/<driver>/version first; the
/// /proc/driver/nvidia/version text is only a fallback for the nvidia module.
/// Drivers are sorted so the output is stable across runs.
pub fn detect_gpu_driver_from(
    drm_root: &Path,
    nvidia_proc: &Path,
    module_root: &Path,
) -> Option<String> {
    let nvidia_version = fs::read_to_string(nvidia_proc)
        .ok()
        .and_then(|text| parse_nvidia_version(&text));

    let mut drivers: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(drm_root) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if !name_str.starts_with("card") || name_str.contains('-') {
                continue;
            }
            let driver_link = entry.path().join("device/driver");
            if let Ok(target) = fs::read_link(&driver_link)
                && let Some(drv) = target.file_name().and_then(|s| s.to_str())
            {
                let drv_str = drv.trim().to_string();
                if !drv_str.is_empty() && !drivers.contains(&drv_str) {
                    drivers.push(drv_str);
                }
            }
        }
    }
    drivers.sort();

    if drivers.is_empty() {
        return nvidia_version.map(|version| format!("nvidia {version}"));
    }

    let rendered: Vec<String> = drivers
        .into_iter()
        .map(|drv| {
            let module_version = fs::read_to_string(module_root.join(&drv).join("version"))
                .ok()
                .map(|text| text.trim().to_string())
                .filter(|version| !version.is_empty());
            let version = module_version.or_else(|| {
                if drv == "nvidia" {
                    nvidia_version.clone()
                } else {
                    None
                }
            });
            match version {
                Some(version) => format!("{drv} {version}"),
                None => drv,
            }
        })
        .collect();

    Some(rendered.join(", "))
}

/// Returns the first dotted numeric version (e.g. 615.78.08) in a text line,
/// regardless of where it appears. This handles both the proprietary text
/// ("Kernel Module  565.77") and the open module text
/// ("Open Kernel Module for x86_64  615.78.08").
pub fn first_version_token(text: &str) -> Option<String> {
    let mut candidate = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            candidate.push(ch);
        } else {
            if let Some(version) = valid_version(&candidate) {
                return Some(version);
            }
            candidate.clear();
        }
    }
    valid_version(&candidate)
}

fn valid_version(candidate: &str) -> Option<String> {
    if !candidate.contains('.') {
        return None;
    }
    let parts: Vec<&str> = candidate.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    if parts
        .iter()
        .any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some(candidate.to_string())
}

/// Extracts "nvidia <version>" from /proc/driver/nvidia/version content.
pub fn parse_nvidia_version(content: &str) -> Option<String> {
    for line in content.lines() {
        if (line.contains("Kernel Module") || line.contains("NVRM version"))
            && let Some(version) = first_version_token(line)
        {
            return Some(version);
        }
    }
    None
}

/// Backwards compatible wrapper returning the full "nvidia <version>" string.
pub fn parse_nvidia_proc_version(content: &str) -> Option<String> {
    parse_nvidia_version(content).map(|version| format!("nvidia {version}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    const OPEN_MODULE: &str = "NVRM version: NVIDIA UNIX Open Kernel Module for x86_64  615.78.08  Release Build  (notroot@)  Sat Oct 31 20:20:36 UTC 2026\nGCC version:  Selected multilib: .;@m64\n";
    const PROPRIETARY: &str = "NVRM version: NVIDIA UNIX x86_64 Kernel Module  565.77  Wed Nov 27 23:33:08 UTC 2024\nGCC version:  gcc version 14.2.1\n";

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_gpu_driver_test_{tag}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_gpu_driver_module_name() {
        assert_eq!(GpuDriver.name(), "gpu_driver");
    }

    #[test]
    fn test_parse_nvidia_proc_version() {
        assert_eq!(
            parse_nvidia_proc_version(PROPRIETARY),
            Some("nvidia 565.77".to_string())
        );
    }

    #[test]
    fn test_parse_open_kernel_module_version() {
        // Regression: "Kernel Module for x86_64  615.78.08" must yield the version
        // even though the token right after "Module" is "for".
        assert_eq!(
            parse_nvidia_version(OPEN_MODULE),
            Some("615.78.08".to_string())
        );
        assert_eq!(
            parse_nvidia_proc_version(OPEN_MODULE),
            Some("nvidia 615.78.08".to_string())
        );
    }

    #[test]
    fn test_first_version_token() {
        assert_eq!(
            first_version_token("x86_64  615.78.08  Release"),
            Some("615.78.08".to_string())
        );
        assert_eq!(first_version_token("no version here"), None);
        assert_eq!(first_version_token("build 2026-10-31"), None);
        assert_eq!(first_version_token("v535.54"), Some("535.54".to_string()));
    }

    #[test]
    fn test_parse_nvidia_missing() {
        assert_eq!(parse_nvidia_version("GCC version: gcc 14.2.1"), None);
    }

    #[test]
    fn test_detect_lists_drivers_sorted_with_module_version() {
        let root = unique_dir("detect");
        let drm = root.join("drm");
        let modules = root.join("module");
        let nvidia_proc = root.join("nvidia_version");
        for (card, drv) in [("card0", "nvidia"), ("card1", "i915")] {
            let dev = drm.join(card).join("device");
            fs::create_dir_all(&dev).unwrap();
            fs::create_dir_all(root.join("drivers").join(drv)).unwrap();
            symlink(format!("../../../drivers/{drv}"), dev.join("driver")).unwrap();
        }
        fs::create_dir_all(modules.join("nvidia")).unwrap();
        fs::write(modules.join("nvidia/version"), "615.78.08\n").unwrap();
        fs::write(&nvidia_proc, OPEN_MODULE).unwrap();

        let value = detect_gpu_driver_from(&drm, &nvidia_proc, &modules).unwrap();
        assert_eq!(value, "i915, nvidia 615.78.08");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_detect_prefers_module_version_over_proc() {
        let root = unique_dir("prefer_module");
        let drm = root.join("drm");
        let modules = root.join("module");
        let nvidia_proc = root.join("nvidia_version");
        let dev = drm.join("card0").join("device");
        fs::create_dir_all(&dev).unwrap();
        fs::create_dir_all(root.join("drivers/nvidia")).unwrap();
        symlink("../../../drivers/nvidia", dev.join("driver")).unwrap();
        fs::create_dir_all(modules.join("nvidia")).unwrap();
        fs::write(modules.join("nvidia/version"), "999.1.2\n").unwrap();
        fs::write(&nvidia_proc, OPEN_MODULE).unwrap();

        assert_eq!(
            detect_gpu_driver_from(&drm, &nvidia_proc, &modules).unwrap(),
            "nvidia 999.1.2"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_detect_without_drm_falls_back_to_nvidia_proc() {
        let root = unique_dir("proc_only");
        fs::create_dir_all(&root).unwrap();
        let nvidia_proc = root.join("nvidia_version");
        fs::write(&nvidia_proc, OPEN_MODULE).unwrap();
        assert_eq!(
            detect_gpu_driver_from(
                &root.join("missing_drm"),
                &nvidia_proc,
                &root.join("missing_module")
            )
            .unwrap(),
            "nvidia 615.78.08"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
