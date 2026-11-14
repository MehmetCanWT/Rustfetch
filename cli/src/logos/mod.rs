//! ASCII distro logos for terminal display.

mod arch_family;
mod debian_redhat;
mod independent;

use arch_family::*;
use debian_redhat::*;
use independent::*;
/// ANSI reset code.
pub const RESET: &str = "\x1b[0m";

// Authentic 24-bit TrueColor RGB escape codes that completely bypass Kitty themes and terminal palettes
pub const FEDORA_BLUE: &str = "\x1b[38;2;60;110;180m"; // #3c6eb4
pub const ARCH_CYAN: &str = "\x1b[38;2;23;147;209m"; // #1793d1
pub const ARCH_LIGHT: &str = "\x1b[38;2;81;190;240m"; // #51bef0
pub const GENTOO_PURPLE: &str = "\x1b[38;2;186;150;226m"; // #ba96e2
pub const UBUNTU_ORANGE: &str = "\x1b[38;2;233;84;32m"; // #e95420
pub const DEBIAN_RED: &str = "\x1b[38;2;215;10;83m"; // #d70a53
pub const MINT_GREEN: &str = "\x1b[38;2;135;207;128m"; // #87cf80
pub const MANJARO_GREEN: &str = "\x1b[38;2;53;191;92m"; // #35bf5c
pub const SUSE_GREEN: &str = "\x1b[38;2;115;186;37m"; // #73ba25
pub const NIXOS_BLUE: &str = "\x1b[38;2;82;119;195m"; // #5277c3
pub const NIXOS_CYAN: &str = "\x1b[38;2;126;186;228m"; // #7ebae4
pub const VOID_GREEN: &str = "\x1b[38;2;71;128;97m"; // #478061
pub const POP_TEAL: &str = "\x1b[38;2;72;185;199m"; // #48b9c7
pub const POP_ORANGE: &str = "\x1b[38;2;255;160;0m"; // #ffa000
pub const ALPINE_BLUE: &str = "\x1b[38;2;13;89;127m"; // #0d597f
pub const KALI_BLUE: &str = "\x1b[38;2;85;124;148m"; // #557c94
pub const STEAM_BLUE: &str = "\x1b[38;2;26;159;255m"; // #1a9fff
pub const ARTIX_CYAN: &str = "\x1b[38;2;23;147;209m";
pub const ZORIN_BLUE: &str = "\x1b[38;2;0;163;255m";
pub const RASPBIAN_RED: &str = "\x1b[38;2;197;26;74m";
pub const RASPBIAN_GREEN: &str = "\x1b[38;2;117;169;40m";
pub const REDHAT_RED: &str = "\x1b[38;2;238;0;0m";
pub const TUX_YELLOW: &str = "\x1b[38;2;255;193;7m";
pub const TUX_DARK: &str = "\x1b[38;2;45;45;45m";
pub const WHITE: &str = "\x1b[38;2;255;255;255m";
pub const CACHYOS_CYAN: &str = "\x1b[38;2;0;180;216m";
pub const CACHYOS_GREEN: &str = "\x1b[38;2;0;204;153m";
pub const CACHYOS_DARK: &str = "\x1b[38;2;40;40;40m";
pub const FREEBSD_RED: &str = "\x1b[38;2;218;31;40m";
pub const FREEBSD_WHITE: &str = "\x1b[38;2;255;255;255m";
pub const OPENBSD_YELLOW: &str = "\x1b[38;2;242;194;0m";
pub const OPENBSD_BLUE: &str = "\x1b[38;2;0;122;204m";
pub const NETBSD_ORANGE: &str = "\x1b[38;2;255;102;0m";
pub const ENDEAVOUR_PURPLE: &str = "\x1b[38;2;127;63;191m";
pub const ENDEAVOUR_RED: &str = "\x1b[38;2;255;75;100m";
pub const ENDEAVOUR_BLUE: &str = "\x1b[38;2;0;164;239m";
pub const MACOS_GREEN: &str = "\x1b[38;2;97;187;70m";
pub const MACOS_YELLOW: &str = "\x1b[38;2;244;180;0m";
pub const MACOS_ORANGE: &str = "\x1b[38;2;230;113;34m";
pub const MACOS_RED: &str = "\x1b[38;2;227;61;64m";
pub const MACOS_PURPLE: &str = "\x1b[38;2;150;61;151m";
pub const MACOS_BLUE: &str = "\x1b[38;2;0;157;220m";
pub const WINDOWS_BLUE: &str = "\x1b[38;2;0;164;239m";
pub const WINDOWS_RED: &str = "\x1b[38;2;242;80;34m";
pub const WINDOWS_GREEN: &str = "\x1b[38;2;127;186;0m";
pub const WINDOWS_YELLOW: &str = "\x1b[38;2;255;185;0m";
pub const GARUDA_ORANGE: &str = "\x1b[38;2;240;80;35m";
pub const GARUDA_YELLOW: &str = "\x1b[38;2;255;170;0m";
pub const NOBARA_WHITE: &str = "\x1b[38;2;255;255;255m";
pub const ARCHCRAFT_TEAL: &str = "\x1b[38;2;94;178;167m";
pub const BAZZITE_PURPLE: &str = "\x1b[38;2;138;79;255m"; // #8a4fff
pub const CAT_AMBER: &str = "\x1b[38;2;249;175;79m"; // #f9af4f
pub const CAT_EMERALD: &str = "\x1b[38;2;102;217;160m"; // #66d9a0
pub const CAT_CREAM: &str = "\x1b[38;2;245;224;220m"; // #f5e0dc

pub fn list_available_logos() -> &'static [(&'static str, &'static str)] {
    &[
        ("arch", "Arch Linux (also: arch_small)"),
        ("fedora", "Fedora Linux (also: fedora_small)"),
        ("bazzite", "Bazzite (Immutable Gaming OS)"),
        ("nobara", "Nobara Linux"),
        ("cachyos", "CachyOS (also: cachyos_small)"),
        ("nixos", "NixOS (also: nixos_small)"),
        ("ubuntu", "Ubuntu (also: ubuntu_small)"),
        ("debian", "Debian GNU/Linux (also: debian_small)"),
        ("mint", "Linux Mint"),
        ("manjaro", "Manjaro Linux"),
        ("endeavouros", "EndeavourOS"),
        ("garuda", "Garuda Linux"),
        ("archcraft", "Archcraft"),
        ("pop", "Pop!_OS"),
        ("gentoo", "Gentoo Linux"),
        ("alpine", "Alpine Linux"),
        ("void", "Void Linux"),
        ("opensuse", "openSUSE"),
        ("kali", "Kali Linux"),
        ("steamos", "SteamOS"),
        ("artix", "Artix Linux"),
        ("zorin", "Zorin OS"),
        ("raspbian", "Raspberry Pi OS"),
        ("rocky", "Rocky Linux"),
        ("almalinux", "AlmaLinux"),
        ("centos", "CentOS Stream"),
        ("redhat", "Red Hat Enterprise Linux"),
        ("freebsd", "FreeBSD (also: freebsd_small)"),
        ("openbsd", "OpenBSD"),
        ("netbsd", "NetBSD"),
        ("macos", "macOS / Darwin"),
        ("windows", "Windows"),
        ("cat", "RustFetch Cat Mascot (also: cat_small)"),
        ("bengal", "Bengal Cat (v0.1.2 Codename Mascot)"),
        ("maine_coon", "Maine Coon Cat (v0.1.3 Codename Mascot)"),
        ("siamese", "Siamese Cat (v0.1.4 Codename Mascot)"),
        ("ragdoll", "Ragdoll Cat (v0.1.5 Codename Mascot)"),
        (
            "turkish_van",
            "Turkish Van Cat — Heterochromia Eyes (v0.1.0 Mascot)",
        ),
        ("turkish_angora", "Turkish Angora Cat (v0.1.1 Mascot)"),
    ]
}

/// A terminal-printable ASCII logo.
#[derive(Debug, Clone)]
pub struct Logo {
    /// Raw ASCII lines (may contain $1..$9 color placeholders).
    pub lines: Vec<&'static str>,
    /// Primary ANSI color escape code for this logo.
    pub color: &'static str,
    /// ANSI colors corresponding to $1, $2, $3, etc.
    pub colors: Vec<&'static str>,
    /// Visual width of the widest line.
    pub width: usize,
}

impl Logo {
    /// Return the logo lines pre-colored with ANSI codes.
    /// Carries the active color across lines for multi-colored logos.
    pub fn colored_lines(&self) -> Vec<String> {
        self.colored_lines_with_custom(None, None)
    }

    /// Return the logo lines colored with optional custom override color or custom palette.
    pub fn colored_lines_with_custom(
        &self,
        override_color: Option<&str>,
        custom_palette: Option<&[&str]>,
    ) -> Vec<String> {
        let base_color = override_color.unwrap_or(self.color);
        let mut active_color = if let Some(pal) = custom_palette {
            pal.first().copied().unwrap_or(base_color)
        } else {
            override_color.unwrap_or_else(|| self.colors.first().copied().unwrap_or(self.color))
        };
        self.lines
            .iter()
            .map(|line| {
                let mut out = String::with_capacity(line.len() + 32);
                out.push_str(active_color);
                let mut chars = line.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '$'
                        && let Some(&next) = chars.peek()
                        && let Some(digit) = next.to_digit(10)
                    {
                        chars.next();
                        let idx = (digit as usize).saturating_sub(1);
                        if let Some(pal) = custom_palette {
                            active_color = pal.get(idx).copied().unwrap_or(base_color);
                        } else if let Some(ov) = override_color {
                            active_color = ov;
                        } else {
                            active_color = self.colors.get(idx).copied().unwrap_or(self.color);
                        }
                        out.push_str(active_color);
                        continue;
                    }
                    out.push(c);
                }
                out.push_str(RESET);
                out
            })
            .collect()
    }
}

/// Compute visual width ignoring color placeholders ($1..$9).
pub fn compute_logo_width(lines: &[&str]) -> usize {
    lines
        .iter()
        .map(|line| {
            let mut w = 0;
            let mut chars = line.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '$'
                    && let Some(&next) = chars.peek()
                    && next.is_ascii_digit()
                {
                    chars.next();
                    continue;
                }
                w += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            }
            w
        })
        .max()
        .unwrap_or(0)
}

/// Get the logo for a distro name (case-insensitive, partial match).
/// Returns a generic Linux/Tux logo for unknown distros.
pub fn get_logo(distro: &str) -> Logo {
    let lower = distro.to_lowercase();
    // 1. Compact / small variants first
    if lower.contains("cachyos_small") || lower.contains("cachy_small") {
        cachyos_small()
    } else if lower.contains("freebsd_small") || lower.contains("bsd_small") {
        freebsd_small()
    } else if lower.contains("nixos_small") || lower.contains("nix_small") {
        nixos_small()
    } else if lower.contains("arch_small") {
        arch_small()
    } else if lower.contains("fedora_small") {
        fedora_small()
    } else if lower.contains("ubuntu_small") {
        ubuntu_small()
    } else if lower.contains("debian_small") {
        debian_small()
    } else if lower.contains("cat_small")
        || lower.contains("kitty_cat_small")
        || lower.contains("kedi_small")
        || lower.contains("anatolian_small")
        || lower.contains("turkish_van_small")
        || lower.contains("van_cat_small")
        || lower.contains("van_small")
        || lower.contains("angora_small")
        || lower.contains("turkish_angora_small")
        || lower.contains("bengal_small")
        || lower.contains("bengal_cat_small")
        || lower.contains("maine_coon_small")
        || lower.contains("mainecoon_small")
        || lower.contains("siamese_small")
        || lower.contains("siamese_cat_small")
        || lower.contains("ragdoll_small")
        || lower.contains("ragdoll_cat_small")
    {
        cat_small()
    // 2. Cat Breed Mascot Logos
    } else if lower.contains("turkish_van") || lower.contains("turkish van") || lower == "van" {
        turkish_van()
    } else if lower.contains("turkish_angora")
        || lower.contains("turkish angora")
        || lower == "angora"
    {
        turkish_angora()
    } else if lower.contains("bengal") {
        bengal_cat()
    } else if lower.contains("maine_coon") || lower.contains("maine coon") || lower == "mainecoon" {
        maine_coon_cat()
    } else if lower.contains("siamese") || lower == "siamese_cat" {
        siamese_cat()
    } else if lower.contains("ragdoll") || lower == "ragdoll_cat" {
        ragdoll_cat()
    } else if lower == "cat" || lower == "kitty_cat" || lower == "kedi" || lower == "anatolian" {
        cat_mascot()
    // 3. Standard distribution logos
    } else if lower.contains("cachyos") || lower.contains("cachy") {
        cachyos()
    } else if lower.contains("freebsd") {
        freebsd()
    } else if lower.contains("openbsd") {
        openbsd()
    } else if lower.contains("netbsd") {
        netbsd()
    } else if lower.contains("archcraft") {
        archcraft()
    } else if lower.contains("garuda") {
        garuda()
    } else if lower.contains("nobara") {
        nobara()
    } else if lower.contains("bazzite") {
        let mut logo = fedora();
        logo.color = BAZZITE_PURPLE;
        logo.colors = vec![BAZZITE_PURPLE, WHITE];
        logo
    } else if lower.contains("darwin") || lower.contains("macos") || lower.contains("apple") {
        macos()
    } else if lower.contains("windows") {
        windows()
    } else if lower.contains("fedora") {
        fedora()
    } else if lower.contains("arch") {
        arch()
    } else if lower.contains("debian") {
        debian()
    } else if lower.contains("ubuntu") {
        ubuntu()
    } else if lower.contains("mint") {
        mint()
    } else if lower.contains("manjaro") {
        manjaro()
    } else if lower.contains("pop") {
        pop()
    } else if lower.contains("alpine") {
        alpine()
    } else if lower.contains("void") {
        void()
    } else if lower.contains("gentoo") {
        gentoo()
    } else if lower.contains("endeavour") {
        endeavour()
    } else if lower.contains("kali") {
        kali()
    } else if lower.contains("steam") {
        steamos()
    } else if lower.contains("artix") {
        artix()
    } else if lower.contains("zorin") {
        zorin()
    } else if lower.contains("raspbian") || lower.contains("raspberry") {
        raspbian()
    } else if lower.contains("rocky") {
        rocky()
    } else if lower.contains("alma") {
        almalinux()
    } else if lower.contains("centos") {
        centos()
    } else if lower.contains("red hat") || lower.contains("redhat") || lower.contains("rhel") {
        redhat()
    } else if lower.contains("opensuse") || lower.contains("suse") {
        opensuse()
    } else if lower.contains("nixos") || lower.contains("nix") {
        nixos()
    } else {
        linux()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_logo_fedora() {
        let logo = get_logo("fedora");
        assert!(!logo.lines.is_empty());
        assert_eq!(logo.color, FEDORA_BLUE);
    }

    #[test]
    fn test_get_logo_case_insensitive() {
        let logo1 = get_logo("Fedora");
        let logo2 = get_logo("FEDORA");
        let logo3 = get_logo("Fedora Linux 44");
        assert_eq!(logo1.color, logo2.color);
        assert_eq!(logo2.color, logo3.color);
    }

    #[test]
    fn test_get_logo_arch() {
        let logo = get_logo("arch");
        assert!(!logo.lines.is_empty());
        assert_eq!(logo.color, ARCH_CYAN);
    }

    #[test]
    fn test_get_logo_unknown_returns_linux() {
        let logo = get_logo("totally_unknown_distro");
        assert!(!logo.lines.is_empty());
        assert_eq!(logo.color, WHITE);
    }

    #[test]
    fn test_colored_lines() {
        let logo = get_logo("fedora");
        let colored = logo.colored_lines();
        assert_eq!(colored.len(), logo.lines.len());
        assert!(colored[0].starts_with("\x1b[38;2;"));
    }

    #[test]
    fn test_all_expanded_distros_return_correct_logos() {
        assert_eq!(get_logo("Linux Mint").color, MINT_GREEN);
        assert_eq!(get_logo("Manjaro Linux").color, MANJARO_GREEN);
        assert_eq!(get_logo("Pop!_OS").color, POP_TEAL);
        assert_eq!(get_logo("Alpine Linux").color, ALPINE_BLUE);
        assert_eq!(get_logo("Void Linux").color, VOID_GREEN);
        assert_eq!(get_logo("Gentoo").color, GENTOO_PURPLE);
        assert_eq!(get_logo("EndeavourOS").color, ENDEAVOUR_PURPLE);
        assert_eq!(get_logo("Kali GNU/Linux").color, KALI_BLUE);
        assert_eq!(get_logo("SteamOS").color, STEAM_BLUE);
        assert_eq!(get_logo("Artix Linux").color, ARTIX_CYAN);
        assert_eq!(get_logo("Red Hat Enterprise Linux").color, REDHAT_RED);
        assert_eq!(get_logo("Rocky Linux").color, SUSE_GREEN);
        assert_eq!(get_logo("CentOS Stream").color, REDHAT_RED);
        assert_eq!(get_logo("AlmaLinux").color, DEBIAN_RED);
        assert_eq!(get_logo("Zorin OS").color, ZORIN_BLUE);
        assert_eq!(get_logo("Raspbian").color, RASPBIAN_RED);
        assert_eq!(get_logo("CachyOS").color, CACHYOS_CYAN);
        assert_eq!(get_logo("cachyos_small").color, CACHYOS_CYAN);
        assert_eq!(get_logo("FreeBSD").color, FREEBSD_RED);
        assert_eq!(get_logo("freebsd_small").color, FREEBSD_RED);
        assert_eq!(get_logo("OpenBSD").color, OPENBSD_YELLOW);
        assert_eq!(get_logo("NetBSD").color, NETBSD_ORANGE);
        assert_eq!(get_logo("Garuda Linux").color, GARUDA_ORANGE);
        assert_eq!(get_logo("Nobara Linux").color, NOBARA_WHITE);
        assert_eq!(get_logo("Archcraft").color, ARCHCRAFT_TEAL);
        assert_eq!(get_logo("macOS").color, MACOS_GREEN);
        assert_eq!(get_logo("Windows").color, WINDOWS_BLUE);
        assert_eq!(get_logo("nixos_small").color, NIXOS_CYAN);
        assert_eq!(get_logo("arch_small").color, ARCH_CYAN);
        assert_eq!(get_logo("fedora_small").color, FEDORA_BLUE);
        assert_eq!(get_logo("ubuntu_small").color, UBUNTU_ORANGE);
        assert_eq!(get_logo("debian_small").color, DEBIAN_RED);
    }
}
