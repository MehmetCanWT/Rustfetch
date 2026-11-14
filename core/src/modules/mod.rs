pub mod battery;
pub mod bluetooth;
pub mod board;
pub mod brightness;
pub mod container;
pub mod cpu;
pub mod cpu_usage;
pub mod custom;
pub mod desktop;
pub mod disk;
pub mod display;
pub mod font;
pub mod gpu;
pub mod gpu_driver;
pub mod host;
pub mod init;
pub mod kernel;
pub mod local_ip;
pub mod locale;
pub mod media;
pub mod memory;
pub mod net_io;
pub mod os;
pub mod os_age;
pub mod packages;
pub mod power_profile;
pub mod processes;
pub mod shell;
pub mod sound;
pub mod swap;
pub mod temp;
pub mod terminal;
pub mod update;
pub mod uptime;
pub mod wifi;

/// Canonical module identifiers understood by `module_by_name`, in display order.
///
/// This is the single source of truth used by `--list-modules` and by the
/// documentation consistency tests.
pub const MODULE_NAMES: &[&str] = &[
    "os",
    "os_age",
    "host",
    "board",
    "kernel",
    "uptime",
    "packages",
    "shell",
    "desktop",
    "font",
    "terminal",
    "cpu",
    "cpu_usage",
    "temp",
    "gpu",
    "dgpu",
    "igpu",
    "gpu_driver",
    "memory",
    "swap",
    "disk",
    "battery",
    "power_profile",
    "brightness",
    "display",
    "sound",
    "media",
    "bluetooth",
    "wifi",
    "net_io",
    "local_ip",
    "locale",
    "processes",
    "init",
    "container",
    "update",
    "custom",
    "break",
    "divider",
];

/// Returns the canonical module identifier list.
pub fn module_names() -> &'static [&'static str] {
    MODULE_NAMES
}

use crate::Module;

pub fn all_modules() -> Vec<Box<dyn Module>> {
    [
        "os",
        "os_age",
        "host",
        "board",
        "kernel",
        "uptime",
        "packages",
        "shell",
        "desktop",
        "font",
        "terminal",
        "cpu",
        "temp",
        "gpu",
        "gpu_driver",
        "memory",
        "swap",
        "disk",
        "battery",
        "power_profile",
        "brightness",
        "display",
        "sound",
        "wifi",
        "net_io",
        "local_ip",
        "locale",
        "processes",
        "init",
        "container",
    ]
    .into_iter()
    .filter_map(module_by_name)
    .collect()
}

pub fn module_from_config(mc: &crate::config::ModuleConfig) -> Option<Box<dyn Module>> {
    let name = mc.name.trim();
    if name == "disk"
        && let Some(p) = &mc.path
        && !p.trim().is_empty()
    {
        return Some(Box::new(disk::Disk::with_mount(p.trim())));
    }
    if name == "gpu" {
        let filter = mc.gpu_type.as_deref().unwrap_or("dgpu");
        return Some(Box::new(gpu::GpuModule::with_type(filter)));
    }
    if name == "dgpu" {
        return Some(Box::new(gpu::DgpuModule));
    }
    if name == "igpu" {
        return Some(Box::new(gpu::IgpuModule));
    }
    module_by_name(name)
}

pub fn module_by_name(name: &str) -> Option<Box<dyn Module>> {
    let name = name.trim();
    if let Some(mount) = name.strip_prefix("disk:") {
        let m = mount.trim();
        if !m.is_empty() {
            return Some(Box::new(disk::Disk::with_mount(m)));
        }
    }
    match name {
        "os" => Some(Box::new(os::Os)),
        "os_age" | "age" => Some(Box::new(os_age::OsAge)),
        "host" => Some(Box::new(host::Host)),
        "board" => Some(Box::new(board::Board)),
        "kernel" => Some(Box::new(kernel::Kernel)),
        "uptime" => Some(Box::new(uptime::Uptime)),
        "packages" => Some(Box::new(packages::PackagesModule)),
        "shell" => Some(Box::new(shell::Shell)),
        "desktop" => Some(Box::new(desktop::DesktopModule)),
        "font" => Some(Box::new(font::FontModule)),
        "terminal" => Some(Box::new(terminal::Terminal)),
        "cpu" => Some(Box::new(cpu::Cpu)),
        "cpu_usage" => Some(Box::new(cpu_usage::CpuUsage)),
        "temp" => Some(Box::new(temp::Temp)),
        "gpu" => Some(Box::new(gpu::GpuModule::default())),
        "dgpu" => Some(Box::new(gpu::DgpuModule)),
        "igpu" => Some(Box::new(gpu::IgpuModule)),
        "gpu_driver" | "driver" => Some(Box::new(gpu_driver::GpuDriver)),
        "memory" => Some(Box::new(memory::Memory)),
        "swap" => Some(Box::new(swap::Swap)),
        "disk" => Some(Box::new(disk::Disk::new())),
        "battery" => Some(Box::new(battery::BatteryModule)),
        "power_profile" | "power" => Some(Box::new(power_profile::PowerProfile)),
        "brightness" => Some(Box::new(brightness::BrightnessModule)),
        "display" => Some(Box::new(display::Display)),
        "sound" => Some(Box::new(sound::Sound)),
        "media" => Some(Box::new(media::Media)),
        "bluetooth" => Some(Box::new(bluetooth::BluetoothModule)),
        "wifi" => Some(Box::new(wifi::Wifi)),
        "net_io" | "network" | "net" => Some(Box::new(net_io::NetIo)),
        "local_ip" => Some(Box::new(local_ip::LocalIpModule)),
        "locale" => Some(Box::new(locale::Locale)),
        "processes" => Some(Box::new(processes::Processes)),
        "init" | "initsystem" => Some(Box::new(init::Init)),
        "container" | "vm" => Some(Box::new(container::Container)),
        "update" => Some(Box::new(update::Update)),
        "custom" => Some(Box::new(custom::Custom)),
        "break" | "divider" | "separator" => Some(Box::new(BreakLine)),
        _ => None,
    }
}

pub struct BreakLine;

impl Module for BreakLine {
    fn name(&self) -> &'static str {
        "break"
    }

    fn detect(&self) -> Option<crate::Info> {
        Some(crate::Info::new("", ""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modules_contains_standard_fast_modules_without_slow_or_forking_ones() {
        let all = all_modules();
        let names: Vec<&str> = all.iter().map(|m| m.name()).collect();
        for expected in [
            "os",
            "os_age",
            "host",
            "board",
            "kernel",
            "uptime",
            "packages",
            "shell",
            "desktop",
            "font",
            "terminal",
            "cpu",
            "temp",
            "gpu",
            "gpu_driver",
            "memory",
            "swap",
            "disk",
            "battery",
            "power_profile",
            "brightness",
            "display",
            "sound",
            "wifi",
            "net_io",
            "local_ip",
            "locale",
            "processes",
            "init",
            "container",
        ] {
            assert!(
                names.contains(&expected),
                "all_modules() is missing '{expected}'"
            );
        }
        // Slow / process-spawning modules are opt-in only.
        assert!(!names.contains(&"media"));
        assert!(!names.contains(&"cpu_usage"));
    }

    #[test]
    fn test_module_names_are_all_resolvable() {
        for name in module_names() {
            assert!(
                module_by_name(name).is_some(),
                "module_names() lists '{name}' but module_by_name() cannot build it"
            );
            assert!(
                crate::config::is_known_module_name(name),
                "config validation must accept the canonical module name '{name}'"
            );
        }
    }

    #[test]
    fn test_module_by_name_resolution() {
        assert!(module_by_name("temp").is_some());
        assert!(module_by_name("sound").is_some());
        assert!(module_by_name("media").is_some());
        assert!(module_by_name("brightness").is_some());
        assert!(module_by_name("cpu_usage").is_some());
        assert!(module_by_name("bluetooth").is_some());
        assert!(module_by_name("init").is_some());
        assert!(module_by_name("container").is_some());
        assert!(module_by_name("os_age").is_some());
        assert!(module_by_name("power_profile").is_some());
        assert!(module_by_name("gpu_driver").is_some());
        assert!(module_by_name("net_io").is_some());
        assert!(module_by_name("disk:/home").is_some());
        assert!(module_by_name("update").is_some());
        assert!(module_by_name("custom").is_some());
        assert!(module_by_name("gpu").is_some());
        assert!(module_by_name("dgpu").is_some());
        assert!(module_by_name("igpu").is_some());
        assert!(module_by_name("break").is_some());
        assert!(module_by_name("nonexistent").is_none());

        assert!(module_from_config(&crate::config::ModuleConfig::new("dgpu")).is_some());
        assert!(module_from_config(&crate::config::ModuleConfig::new("igpu")).is_some());
        let mut gpu_cfg = crate::config::ModuleConfig::new("gpu");
        gpu_cfg.gpu_type = Some("igpu".to_string());
        assert!(module_from_config(&gpu_cfg).is_some());
    }
}
