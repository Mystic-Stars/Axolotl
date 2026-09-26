//! Links between saved multiplayer entries and managed local servers.
//!
//! A saved multiplayer entry can point back at this machine (localhost,
//! loopback, or one of the machine's own interface addresses). When the user
//! links such an entry to a managed server, launching that entry from the home
//! pinned-servers widget starts the server first and waits until it accepts
//! connections before the instance is launched.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

use crate::Result;

use super::lifecycle::is_running;
use super::manifest::{read_server_ip, read_server_port, server_path};

/// Port used when `server.properties` has no `server-port` entry, matching the
/// vanilla default.
pub(super) const DEFAULT_SERVER_PORT: u16 = 25565;

/// How long a launch waits for a linked server to accept connections.
pub const DEFAULT_READY_TIMEOUT_MS: u64 = 180_000;

/// Upper bound for the caller supplied timeout, so a bad value cannot make the
/// launcher wait forever.
const MAX_READY_TIMEOUT_MS: u64 = 600_000;

const READY_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Splits the host out of a server address: `host`, `host:port`, or `[v6]:port`.
pub(super) fn address_host(address: &str) -> &str {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return trimmed;
    }
    if trimmed.parse::<IpAddr>().is_ok() {
        return trimmed;
    }
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return &rest[..end];
        }
    }
    match trimmed.rsplit_once(':') {
        Some((host, port))
            if !host.is_empty() && port.chars().all(|c| c.is_ascii_digit()) =>
        {
            host
        }
        _ => trimmed,
    }
}

/// Whether `address` points at this machine: `localhost`, a loopback address,
/// the unspecified address, or any address assigned to a local interface
/// (IPv4 or IPv6).
pub fn is_local_address(address: &str) -> bool {
    let host = address_host(address)
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    let Ok(ip) = host.parse::<IpAddr>() else {
        return false;
    };
    if ip.is_loopback() || ip.is_unspecified() {
        return true;
    }
    local_interface_addresses()
        .into_iter()
        .any(|local| local == ip)
}

fn local_interface_addresses() -> Vec<IpAddr> {
    let mut addresses: Vec<IpAddr> = local_ip_address::list_afinet_netifas()
        .unwrap_or_default()
        .into_iter()
        .map(|(_, address)| address)
        .filter(|address| !address.is_unspecified())
        .collect();
    addresses.sort_unstable();
    addresses.dedup();
    addresses
}

/// Waits until the managed server accepts TCP connections on its port.
///
/// Returns `false` when the server is not running, exits while waiting, or the
/// timeout elapses.
pub async fn wait_until_ready(
    server_id: &str,
    timeout_ms: u64,
) -> Result<bool> {
    if !is_running(server_id) {
        return Ok(false);
    }

    let path = server_path(server_id).await?;
    let port = read_server_port(&path).await.unwrap_or(DEFAULT_SERVER_PORT);
    let bind = read_server_ip(&path).await;
    let targets = readiness_targets(port, bind.as_deref());
    let timeout =
        Duration::from_millis(timeout_ms.clamp(1_000, MAX_READY_TIMEOUT_MS));
    let deadline = Instant::now() + timeout;

    loop {
        if !is_running(server_id) {
            return Ok(false);
        }
        for target in &targets {
            if tokio::net::TcpStream::connect(target).await.is_ok() {
                return Ok(true);
            }
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        tokio::time::sleep(READY_POLL_INTERVAL).await;
    }
}

/// Addresses to probe for readiness: the configured `server-ip` when it points
/// at a specific address, plus both loopback addresses. Servers that bind to
/// every interface (`server-ip` unset, empty, `0.0.0.0`, or `::`) stay
/// reachable through loopback, and IPv6-only binds are covered by `::1`.
fn readiness_targets(port: u16, bind: Option<&str>) -> Vec<SocketAddr> {
    let mut targets = Vec::new();
    if let Some(ip) = bind.and_then(|value| value.trim().parse::<IpAddr>().ok())
    {
        if !ip.is_unspecified() {
            targets.push(SocketAddr::new(ip, port));
        }
    }
    for loopback in [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
    ] {
        let candidate = SocketAddr::new(loopback, port);
        if !targets.contains(&candidate) {
            targets.push(candidate);
        }
    }
    targets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_hosts_from_server_addresses() {
        assert_eq!(address_host("localhost"), "localhost");
        assert_eq!(address_host("localhost:25565"), "localhost");
        assert_eq!(address_host("192.168.1.2:25565"), "192.168.1.2");
        assert_eq!(address_host("mc.example.com"), "mc.example.com");
        assert_eq!(address_host("[::1]:25565"), "::1");
        assert_eq!(address_host("::1"), "::1");
        assert_eq!(address_host("  example.com:25565  "), "example.com");
    }

    #[test]
    fn detects_local_addresses() {
        assert!(is_local_address("localhost"));
        assert!(is_local_address("LOCALHOST:25565"));
        assert!(is_local_address("127.0.0.1:25565"));
        assert!(is_local_address("127.42.0.9"));
        assert!(is_local_address("[::1]:25565"));
        assert!(is_local_address("0.0.0.0:25565"));
    }

    #[test]
    fn ignores_remote_addresses() {
        assert!(!is_local_address("mc.example.com:25565"));
        assert!(!is_local_address("192.0.2.10:25565"));
        assert!(!is_local_address("203.0.113.7"));
        assert!(!is_local_address("[2001:db8::1]:25565"));
        assert!(!is_local_address(""));
    }

    #[test]
    fn readiness_targets_cover_the_configured_bind_address() {
        let bound = readiness_targets(25565, Some("192.168.1.10"));
        assert!(bound.contains(&"192.168.1.10:25565".parse().unwrap()));
        assert!(bound.contains(&"127.0.0.1:25565".parse().unwrap()));

        let ipv6_only = readiness_targets(25565, Some("2001:db8::5"));
        assert!(ipv6_only.contains(&"[2001:db8::5]:25565".parse().unwrap()));
        assert!(ipv6_only.contains(&"[::1]:25565".parse().unwrap()));

        // Unset and "every interface" binds are covered by both loopbacks.
        for bind in [None, Some(""), Some("0.0.0.0"), Some("::")] {
            let targets = readiness_targets(25565, bind);
            assert_eq!(targets.len(), 2, "unexpected targets for {bind:?}");
            assert!(targets.contains(&"127.0.0.1:25565".parse().unwrap()));
            assert!(targets.contains(&"[::1]:25565".parse().unwrap()));
        }
    }
}
