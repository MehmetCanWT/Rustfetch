pub fn default_icon_for_module(name: &str) -> Option<&'static str> {
    if name.starts_with("disk:") {
        return Some("󰋊");
    }
    match name {
        "os" => Some("󰌽"),
        "os_age" | "age" => Some("󰃭"),
        "host" => Some("󰌢"),
        "board" => Some("󰘚"),
        "kernel" => Some("󰒋"),
        "uptime" => Some("󱑂"),
        "packages" => Some("󰏖"),
        "shell" => Some("󰞷"),
        "desktop" => Some("󰨇"),
        "font" => Some("󰛖"),
        "terminal" => Some("󰆍"),
        "cpu" => Some("󰍛"),
        "gpu" | "dgpu" | "igpu" => Some("󰢮"),
        "gpu_driver" | "driver" => Some("󰢮"),
        "memory" => Some("󰘚"),
        "swap" => Some("󰓡"),
        "disk" => Some("󰋊"),
        "battery" => Some("󰂁"),
        "power_profile" | "power" => Some("󰓅"),
        "brightness" => Some("󰃠"),
        "cpu_usage" => Some("󰓅"),
        "bluetooth" => Some("󰂯"),
        "local_ip" => Some("󰩟"),
        "net_io" | "network" | "net" => Some("󰛳"),
        "locale" => Some("󰗊"),
        "temp" => Some("󰔏"),
        "sound" => Some("󰕾"),
        "media" => Some("󰝚"),
        "display" => Some("󰍹"),
        "wifi" => Some("󰖩"),
        "processes" => Some("󰒋"),
        "init" | "initsystem" => Some(""),
        "container" | "vm" => Some("󰡨"),
        "update" => Some("󰚰"),
        "custom" => Some("󰅐"),
        _ => None,
    }
}

pub fn distro_icon(distro_name: &str) -> &'static str {
    let lower = distro_name.to_lowercase();
    if lower.contains("arch") {
        "󰣇"
    } else if lower.contains("ubuntu") {
        "󰕈"
    } else if lower.contains("fedora") || lower.contains("bazzite") || lower.contains("nobara") {
        "󰣛"
    } else if lower.contains("debian") {
        "󰣚"
    } else if lower.contains("nixos") {
        "󱄅"
    } else if lower.contains("gentoo") {
        "󰣨"
    } else if lower.contains("manjaro") {
        "󱘊"
    } else if lower.contains("mint") {
        "󰣭"
    } else if lower.contains("alpine") {
        "󰣠"
    } else if lower.contains("suse") {
        "󰣡"
    } else if lower.contains("kali") {
        "󰣳"
    } else if lower.contains("pop") {
        "󰣩"
    } else if lower.contains("endeavour") {
        "󰣫"
    } else if lower.contains("artix") {
        "󰣢"
    } else if lower.contains("void") {
        "󰣲"
    } else if lower.contains("cachy") {
        "󰣇"
    } else if lower.contains("darwin") || lower.contains("macos") || lower.contains("apple") {
        "󰀵"
    } else if lower.contains("windows") {
        "󰍲"
    } else if lower.contains("android") {
        "󰀲"
    } else if lower.contains("freebsd") || lower.contains("bsd") {
        "󰣶"
    } else {
        "󰌽"
    }
}
