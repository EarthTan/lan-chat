// lan-chat/src-tauri/src/network.rs
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub enabled: bool, // user-selected (default: true)
}

/// Hard-exclude prefixes: VPN tunnels, virtual bridges, Docker, etc.
const HARD_EXCLUDE_PREFIXES: &[&str] = &[
    "lo",        // loopback
    "tun",       // OpenVPN, WireGuard tun
    "utun",      // macOS VPN tunnels
    "wg",        // WireGuard
    "ppp",       // PPP
    "ipsec",     // IPSec
    "tailscale", // Tailscale
    "docker",    // Docker bridge
    "br-",       // Docker/LXC named bridges
    "virbr",     // libvirt
    "veth",      // Docker veth pair
    "vmnet",     // VMware
    "vboxnet",   // VirtualBox
    "pvnet",     // Parallels
    "llw",       // macOS low latency WLAN (internal)
    "awdl",      // Apple Wireless Direct Link (AirDrop)
];

/// Link-local prefix: 169.254.x.x
const LINK_LOCAL_PREFIX: [u8; 2] = [169, 254];

pub fn get_candidate_interfaces() -> Vec<NetworkInterface> {
    let ifaces = match if_addrs::get_if_addrs() {
        Ok(i) => i,
        Err(_) => return vec![],
    };

    let mut result = Vec::new();

    for iface in ifaces {
        // IPv4 only
        let ip = match iface.ip() {
            IpAddr::V4(v4) => v4,
            _ => continue,
        };

        let name = iface.name.clone();

        // Hard filter: loopback
        if ip.is_loopback() {
            continue;
        }

        // Hard filter: link-local (169.254.x.x)
        let octets = ip.octets();
        if octets[0] == LINK_LOCAL_PREFIX[0] && octets[1] == LINK_LOCAL_PREFIX[1] {
            continue;
        }

        // Hard filter: name prefix blacklist
        let name_lower = name.to_lowercase();
        if HARD_EXCLUDE_PREFIXES
            .iter()
            .any(|prefix| name_lower.starts_with(prefix))
        {
            continue;
        }

        // Only private RFC 1918 addresses
        if !is_private_ipv4(&ip) {
            continue;
        }

        result.push(NetworkInterface {
            name,
            ip: ip.to_string(),
            enabled: true,
        });
    }

    // Remove exact duplicates (same name + same IP)
    result.dedup_by(|a, b| a.name == b.name && a.ip == b.ip);
    result
}

fn is_private_ipv4(ip: &std::net::Ipv4Addr) -> bool {
    let o = ip.octets();
    // 10.0.0.0/8
    if o[0] == 10 {
        return true;
    }
    // 172.16.0.0/12
    if o[0] == 172 && (16..=31).contains(&o[1]) {
        return true;
    }
    // 192.168.0.0/16
    if o[0] == 192 && o[1] == 168 {
        return true;
    }
    false
}
