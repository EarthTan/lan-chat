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
        Err(e) => {
            tracing::warn!("if_addrs::get_if_addrs() failed: {}", e);
            return vec![];
        }
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

        result.push(NetworkInterface {
            name,
            ip: ip.to_string(),
            enabled: true,
        });
    }

    // Sort before deduping so non-consecutive duplicates are also caught
    result.sort_by(|a, b| (&a.name, &a.ip).cmp(&(&b.name, &b.ip)));
    result.dedup_by(|a, b| a.name == b.name && a.ip == b.ip);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_loopback() {
        // The actual filter is applied in get_candidate_interfaces via if_addrs.
        // We test the name-prefix blacklist logic directly.
        let excluded = HARD_EXCLUDE_PREFIXES;
        assert!(excluded.iter().any(|p| "tailscale0".starts_with(p)));
        assert!(excluded.iter().any(|p| "docker0".starts_with(p)));
        assert!(excluded.iter().any(|p| "tun0".starts_with(p)));
        assert!(excluded.iter().any(|p| "wg0".starts_with(p)));
        assert!(excluded.iter().any(|p| "br-abc123".starts_with(p)));
        // Real interfaces should NOT be excluded
        assert!(!excluded.iter().any(|p| "wlp130s0".starts_with(p)));
        assert!(!excluded.iter().any(|p| "eth0".starts_with(p)));
        assert!(!excluded.iter().any(|p| "enp3s0".starts_with(p)));
        assert!(!excluded.iter().any(|p| "thunderbolt0".starts_with(p)));
        assert!(!excluded.iter().any(|p| "en0".starts_with(p)));
    }
}
