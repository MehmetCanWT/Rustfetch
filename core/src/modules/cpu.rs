use crate::{Info, Module};
use std::fs;
use std::path::Path;

pub struct Cpu;

/// Parsed CPU identity from /proc/cpuinfo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuInfo {
    pub model: String,
    pub threads: usize,
}

impl Module for Cpu {
    fn name(&self) -> &'static str {
        "cpu"
    }

    fn detect(&self) -> Option<Info> {
        let content = fs::read_to_string("/proc/cpuinfo").ok()?;
        let info = parse_cpuinfo_fields(&content)
            .or_else(|| cpuinfo_from_device_tree(Path::new("/proc/device-tree/model"), &content))?;
        let boost_mhz = read_boost_mhz(Path::new("/sys/devices/system/cpu"));
        Some(Info::new("CPU", format_cpu(&info, boost_mhz)))
    }
}

/// Parses the model name and logical processor count from /proc/cpuinfo.
pub fn parse_cpuinfo_fields(content: &str) -> Option<CpuInfo> {
    let mut model_name: Option<String> = None;
    let mut fallback_model: Option<String> = None;
    let mut core_count: usize = 0;

    for line in content.lines() {
        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim();
            if key.eq_ignore_ascii_case("model name") {
                if model_name.is_none() {
                    let cleaned = val.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !cleaned.is_empty() {
                        model_name = Some(cleaned);
                    }
                }
            } else if key.eq_ignore_ascii_case("Hardware") || key.eq_ignore_ascii_case("Model") {
                if fallback_model.is_none() {
                    let cleaned = val.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !cleaned.is_empty() {
                        fallback_model = Some(cleaned);
                    }
                }
            } else if key.eq_ignore_ascii_case("processor") {
                if val.trim().parse::<usize>().is_ok() {
                    core_count += 1;
                } else if fallback_model.is_none() {
                    let cleaned = val.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !cleaned.is_empty() {
                        fallback_model = Some(cleaned);
                    }
                }
            }
        }
    }

    let model = model_name.or(fallback_model)?;
    if core_count == 0 {
        return None;
    }
    Some(CpuInfo {
        model,
        threads: core_count,
    })
}

fn count_threads(content: &str) -> usize {
    content
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(key, val)| {
            key.trim().eq_ignore_ascii_case("processor") && val.trim().parse::<usize>().is_ok()
        })
        .count()
}

/// Legacy formatter without the boost clock suffix.
pub fn parse_cpuinfo(content: &str) -> Option<String> {
    parse_cpuinfo_fields(content).map(|info| format!("{} ({})", info.model, info.threads))
}

/// Strips a trailing base clock like ` @ 2.70GHz` from a CPU model string when
/// a measured boost clock will be appended, avoiding duplicate `@ ... GHz` suffixes.
fn strip_trailing_base_clock(model: &str) -> &str {
    if let Some((prefix, suffix)) = model.rsplit_once(" @ ") {
        let lower = suffix.trim().to_ascii_lowercase();
        if (lower.ends_with("ghz") || lower.ends_with("mhz"))
            && lower
                .trim_end_matches("ghz")
                .trim_end_matches("mhz")
                .trim()
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.')
        {
            return prefix.trim_end();
        }
    }
    model
}

/// Formats the CPU row, appending the maximum (boost) clock when available.
pub fn format_cpu(info: &CpuInfo, boost_mhz: Option<u64>) -> String {
    let has_boost = boost_mhz.is_some_and(|mhz| mhz > 0);
    let model = if has_boost {
        strip_trailing_base_clock(&info.model)
    } else {
        info.model.as_str()
    };
    let mut value = format!("{model} ({})", info.threads);
    if let Some(mhz) = boost_mhz.filter(|mhz| *mhz > 0) {
        value.push_str(&format!(" @ {:.2} GHz", mhz as f64 / 1000.0));
    }
    value
}

/// Reads the maximum frequency (in MHz) from cpufreq, if the driver exposes it.
pub fn read_boost_mhz(cpu_root: &Path) -> Option<u64> {
    for relative in [
        "cpu0/cpufreq/cpuinfo_max_freq",
        "cpu0/cpufreq/scaling_max_freq",
    ] {
        if let Ok(text) = fs::read_to_string(cpu_root.join(relative))
            && let Ok(khz) = text.trim().parse::<u64>()
            && khz > 0
        {
            return Some(khz / 1000);
        }
    }
    None
}

/// aarch64 boards often expose no model name in /proc/cpuinfo; fall back to the
/// device-tree model string.
pub fn cpuinfo_from_device_tree(model_path: &Path, cpuinfo: &str) -> Option<CpuInfo> {
    let model = fs::read_to_string(model_path).ok()?;
    let model = model.trim_matches('\0').trim().to_string();
    if model.is_empty() {
        return None;
    }
    let threads = count_threads(cpuinfo);
    if threads == 0 {
        return None;
    }
    Some(CpuInfo { model, threads })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Realistic Fedora Linux /proc/cpuinfo fixture with 2 processor blocks (AMD Ryzen 7 5800X).
    const FEDORA_RYZEN_CPUINFO: &str = "processor\t: 0
vendor_id\t: AuthenticAMD
model\t\t: 33
model name\t: AMD Ryzen 7 5800X
cpu MHz\t\t: 3800.000
cache size\t: 512 KB
physical id\t: 0
core id\t\t: 0
cpu cores\t: 8
\nprocessor\t: 1
vendor_id\t: AuthenticAMD
model\t\t: 33
model name\t: AMD Ryzen 7 5800X
cpu MHz\t\t: 3800.000
cache size\t: 512 KB
physical id\t: 0
core id\t\t: 1
cpu cores\t: 8
";

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rustfetch_cpu_test_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_parse_cpuinfo_fedora_2_cores() {
        let result = parse_cpuinfo(FEDORA_RYZEN_CPUINFO);
        assert_eq!(result.as_deref(), Some("AMD Ryzen 7 5800X (2)"));
    }

    #[test]
    fn test_parse_cpuinfo_16_cores() {
        let mut content = String::new();
        for i in 0..16 {
            content.push_str(&format!(
                "processor\t: {i}\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 7 5800X\n\n"
            ));
        }
        let result = parse_cpuinfo(&content);
        assert_eq!(result.as_deref(), Some("AMD Ryzen 7 5800X (16)"));
    }

    #[test]
    fn test_parse_cpuinfo_empty_and_invalid() {
        assert_eq!(parse_cpuinfo(""), None);
        assert_eq!(parse_cpuinfo("unknown: field"), None);
        assert_eq!(parse_cpuinfo("processor: 0"), None);
        assert_eq!(parse_cpuinfo("model name: Intel Core i5"), None);
    }

    #[test]
    fn test_parse_cpuinfo_arm() {
        let arm_sample =
            "processor : 0\nBogoMIPS : 108.0\nprocessor : 1\nModel : Raspberry Pi 4 Model B\n";
        assert_eq!(
            parse_cpuinfo(arm_sample),
            Some("Raspberry Pi 4 Model B (2)".to_string())
        );
    }

    #[test]
    fn test_parse_fields_thread_count() {
        let info = parse_cpuinfo_fields(FEDORA_RYZEN_CPUINFO).unwrap();
        assert_eq!(info.threads, 2);
        assert_eq!(info.model, "AMD Ryzen 7 5800X");
    }

    #[test]
    fn test_format_cpu_with_boost() {
        let info = CpuInfo {
            model: "AMD Ryzen 7 5800X".to_string(),
            threads: 16,
        };
        assert_eq!(
            format_cpu(&info, Some(4700)),
            "AMD Ryzen 7 5800X (16) @ 4.70 GHz"
        );
        assert_eq!(format_cpu(&info, None), "AMD Ryzen 7 5800X (16)");
        assert_eq!(format_cpu(&info, Some(0)), "AMD Ryzen 7 5800X (16)");

        let intel_info = CpuInfo {
            model: "11th Gen Intel(R) Core(TM) i5-11400H @ 2.70GHz".to_string(),
            threads: 12,
        };
        assert_eq!(
            format_cpu(&intel_info, Some(4500)),
            "11th Gen Intel(R) Core(TM) i5-11400H (12) @ 4.50 GHz"
        );
        assert_eq!(
            format_cpu(&intel_info, None),
            "11th Gen Intel(R) Core(TM) i5-11400H @ 2.70GHz (12)"
        );
    }

    #[test]
    fn test_read_boost_mhz_from_fixture() {
        let root = unique_dir("boost");
        let cpufreq = root.join("cpu0/cpufreq");
        fs::create_dir_all(&cpufreq).unwrap();
        fs::write(cpufreq.join("cpuinfo_max_freq"), "4700000\n").unwrap();
        fs::write(cpufreq.join("scaling_max_freq"), "3200000\n").unwrap();
        assert_eq!(read_boost_mhz(&root), Some(4700));
        fs::remove_file(cpufreq.join("cpuinfo_max_freq")).unwrap();
        assert_eq!(read_boost_mhz(&root), Some(3200));
        fs::remove_file(cpufreq.join("scaling_max_freq")).unwrap();
        assert_eq!(read_boost_mhz(&root), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_device_tree_fallback_trimming_null() {
        let root = unique_dir("dt");
        fs::create_dir_all(&root).unwrap();
        let model_path = root.join("model");
        fs::write(&model_path, "Raspberry Pi 5\0").unwrap();
        let cpuinfo = "processor\t: 0\nprocessor\t: 1\nprocessor\t: 2\nprocessor\t: 3\n";
        let info = cpuinfo_from_device_tree(&model_path, cpuinfo).unwrap();
        assert_eq!(info.model, "Raspberry Pi 5");
        assert_eq!(info.threads, 4);
        fs::write(&model_path, "\0").unwrap();
        assert!(cpuinfo_from_device_tree(&model_path, cpuinfo).is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_cpu_module_name() {
        let cpu = Cpu;
        assert_eq!(cpu.name(), "cpu");
    }

    #[test]
    fn test_cpu_detect() {
        let cpu = Cpu;
        if let Some(info) = cpu.detect() {
            assert_eq!(info.label, "CPU");
            assert!(info.value.contains('('));
            // Either ends with the thread count or with an appended boost clock.
            assert!(info.value.ends_with(')') || info.value.contains(" GHz"));
        }
    }
}
