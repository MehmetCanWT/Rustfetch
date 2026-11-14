use crate::modules::net_io::{is_virtual_interface, parse_default_route_interface};
use crate::{Info, Module};
use std::net::{Ipv4Addr, UdpSocket};
use std::path::Path;

pub struct LocalIpModule;

impl Module for LocalIpModule {
    fn name(&self) -> &'static str {
        "local_ip"
    }

    fn detect(&self) -> Option<Info> {
        let ip = detect_local_ip()?;
        Some(Info {
            label: "Local IP".to_string(),
            value: ip,
        })
    }
}

pub fn detect_local_ip() -> Option<String> {
    detect_local_ip_from(Path::new("/proc/net/route"))
}

/// Resolves the primary LAN IPv4 address.
///
/// The kernel FIB lookup (unconnected UDP socket) is tried first; no packets
/// are sent. The getifaddrs fallback skips virtual bridges/tunnels and prefers
/// the interface owning the default route.
pub fn detect_local_ip_from(route_path: &Path) -> Option<String> {
    let route_iface = std::fs::read_to_string(route_path)
        .ok()
        .and_then(|content| parse_default_route_interface(&content));

    if let Some(ip) = fib_lookup_ipv4() {
        return Some(ip);
    }
    interface_ipv4_fallback(route_iface.as_deref())
}

fn fib_lookup_ipv4() -> Option<String> {
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        for target in [
            "8.8.8.8:53",
            "1.1.1.1:53",
            "10.255.255.255:53",
            "192.168.255.255:53",
        ] {
            if socket.connect(target).is_ok()
                && let Ok(addr) = socket.local_addr()
                && let std::net::IpAddr::V4(ip) = addr.ip()
                && !ip.is_loopback()
                && !ip.is_unspecified()
            {
                return Some(ip.to_string());
            }
        }
    }
    None
}

fn interface_ipv4_fallback(preferred_iface: Option<&str>) -> Option<String> {
    unsafe {
        let mut ifaddrs: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifaddrs) != 0 || ifaddrs.is_null() {
            return None;
        }

        let mut found: Vec<(String, String)> = Vec::new();
        let mut cur = ifaddrs;

        while !cur.is_null() {
            let entry = &*cur;
            if !entry.ifa_addr.is_null() && !entry.ifa_name.is_null() {
                let flags = entry.ifa_flags;
                let is_up = (flags & libc::IFF_UP as u32) != 0;
                let is_loopback = (flags & libc::IFF_LOOPBACK as u32) != 0;

                if is_up && !is_loopback {
                    let family = (*entry.ifa_addr).sa_family as i32;
                    if family == libc::AF_INET {
                        let name = std::ffi::CStr::from_ptr(entry.ifa_name)
                            .to_string_lossy()
                            .into_owned();
                        if !is_virtual_interface(&name) {
                            let sockaddr_in = entry.ifa_addr as *const libc::sockaddr_in;
                            let ip_bytes = (*sockaddr_in).sin_addr.s_addr.to_ne_bytes();
                            let ip =
                                Ipv4Addr::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]);
                            if !ip.is_loopback() && !ip.is_unspecified() && !ip.is_link_local() {
                                found.push((name, ip.to_string()));
                            }
                        }
                    }
                }
            }
            cur = entry.ifa_next;
        }

        libc::freeifaddrs(ifaddrs);

        found.sort();
        if let Some(preferred) = preferred_iface
            && let Some((_, ip)) = found.iter().find(|(name, _)| name == preferred)
        {
            return Some(ip.clone());
        }
        found.into_iter().next().map(|(_, ip)| ip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_ip_module_name() {
        assert_eq!(LocalIpModule.name(), "local_ip");
    }

    #[test]
    fn test_detect_local_ip_returns_or_safely_skips() {
        if let Some(ip) = detect_local_ip() {
            assert!(!ip.is_empty());
            assert!(!ip.starts_with("127."));
            assert!(!ip.contains(':'), "only IPv4 is reported");
        }
    }

    #[test]
    fn test_detect_with_missing_route_file_still_works() {
        // Missing route file must not panic; either the FIB lookup or the
        // interface fallback provides a value.
        let missing =
            std::env::temp_dir().join(format!("rustfetch_local_ip_missing_{}", std::process::id()));
        let result = detect_local_ip_from(&missing);
        if let Some(ip) = result {
            assert!(!ip.contains(':'));
        }
    }
}
