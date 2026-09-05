use parking_lot::Mutex;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_HOST_OVERRIDES: [(&str, &str); 2] = [
    ("mod.tianpao.top", "www.shopify.com"),
    ("cdn.modrinth.com", "www.shopify.com"),
];

/// `lookup_host` does not expose the authoritative record TTL. Keep entries
/// long enough to retain the connection-reuse benefit, but short enough that
/// a changed CDN, VPN, or network is not pinned until the application exits.
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const CONNECTION_FAILURES_BEFORE_REFRESH: u8 = 2;

#[derive(Clone)]
struct CachedAddresses {
    addresses: Vec<IpAddr>,
    resolved_at: Instant,
    consecutive_connection_failures: u8,
}

impl CachedAddresses {
    fn is_fresh(&self) -> bool {
        self.resolved_at.elapsed() < CACHE_TTL
    }
}

#[derive(Clone)]
pub struct DownloadDnsResolver {
    reliability: Arc<Mutex<HashMap<IpAddr, f64>>>,
    last_resolved: Arc<Mutex<HashMap<String, CachedAddresses>>>,
    /// Locks only a single hostname's lookup. The map is held just long
    /// enough to obtain the per-host lock, never while DNS is awaited.
    resolving_hosts: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    host_overrides: Arc<Mutex<HashMap<String, String>>>,
    #[cfg(test)]
    test_addresses: Arc<Mutex<HashMap<String, Vec<SocketAddr>>>>,
    #[cfg(test)]
    test_lookup_delays: Arc<Mutex<HashMap<String, Duration>>>,
}

impl Default for DownloadDnsResolver {
    fn default() -> Self {
        let host_overrides = DEFAULT_HOST_OVERRIDES
            .into_iter()
            .map(|(host, resolver_host)| {
                (host.to_string(), resolver_host.to_string())
            })
            .collect();
        Self {
            reliability: Arc::default(),
            last_resolved: Arc::default(),
            resolving_hosts: Arc::default(),
            host_overrides: Arc::new(Mutex::new(host_overrides)),
            #[cfg(test)]
            test_addresses: Arc::default(),
            #[cfg(test)]
            test_lookup_delays: Arc::default(),
        }
    }
}

impl DownloadDnsResolver {
    /// Resolves `host` through `resolver_host` while preserving the original
    /// URL host for HTTP Host headers and TLS SNI.
    #[allow(dead_code)]
    pub fn set_host_override(
        &self,
        host: &str,
        resolver_host: &str,
    ) -> Result<(), &'static str> {
        let host = normalize_host(host)?;
        let resolver_host = normalize_host(resolver_host)?;
        let mut overrides = self.host_overrides.lock();
        if host == resolver_host {
            overrides.remove(&host);
        } else {
            overrides.insert(host.clone(), resolver_host);
        }
        drop(overrides);
        self.last_resolved.lock().remove(&host);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn clear_host_override(&self, host: &str) -> Result<(), &'static str> {
        let host = normalize_host(host)?;
        self.host_overrides.lock().remove(&host);
        self.last_resolved.lock().remove(&host);
        Ok(())
    }

    pub fn host_override(&self, host: &str) -> Option<String> {
        let host = normalize_host(host).ok()?;
        self.host_overrides.lock().get(&host).cloned()
    }

    fn resolution_host(&self, host: &str) -> String {
        self.host_override(host).unwrap_or_else(|| host.to_string())
    }
    pub fn record_result(&self, address: IpAddr, result: f64) {
        let mut reliability = self.reliability.lock();
        reliability
            .entry(address)
            .and_modify(|value| *value = *value * 0.5 + result * 0.5)
            .or_insert(result * 0.5);
    }

    pub fn record_host_success(&self, host: &str, address: IpAddr) {
        let mut cached = self.last_resolved.lock();
        if let Some(entry) = cached
            .get_mut(host)
            .filter(|entry| entry.addresses.contains(&address))
        {
            entry.consecutive_connection_failures = 0;
            drop(cached);
            self.record_result(address, 0.5);
        }
    }

    pub fn resolved_addresses(&self, host: &str) -> Vec<IpAddr> {
        self.last_resolved
            .lock()
            .get(host)
            .filter(|entry| entry.is_fresh())
            .map(|entry| &entry.addresses)
            .cloned()
            .unwrap_or_default()
    }

    /// Marks a failed connection attempt for `host`. The second consecutive
    /// failure expires its cache entry, so the next request or prewarm does a
    /// fresh lookup. A single transient failure keeps the hot cache intact.
    /// Returns whether this call expired the entry.
    pub fn record_connection_failure(&self, host: &str) -> bool {
        let mut cached = self.last_resolved.lock();
        let Some(entry) = cached.get_mut(host) else {
            return false;
        };
        entry.consecutive_connection_failures =
            entry.consecutive_connection_failures.saturating_add(1);
        if entry.consecutive_connection_failures
            < CONNECTION_FAILURES_BEFORE_REFRESH
        {
            return false;
        }
        entry.resolved_at = Instant::now() - CACHE_TTL;
        true
    }

    fn cache_addresses(&self, host: String, addresses: Vec<SocketAddr>) {
        self.last_resolved.lock().insert(
            host,
            CachedAddresses {
                addresses: addresses
                    .iter()
                    .map(|address| address.ip())
                    .collect(),
                resolved_at: Instant::now(),
                consecutive_connection_failures: 0,
            },
        );
    }

    fn resolving_lock(&self, host: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.resolving_hosts
            .lock()
            .entry(host.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    async fn lookup_addresses(
        &self,
        resolution_host: &str,
    ) -> std::io::Result<Vec<SocketAddr>> {
        #[cfg(test)]
        {
            let delay =
                self.test_lookup_delays.lock().get(resolution_host).copied();
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            if let Some(addresses) =
                self.test_addresses.lock().get(resolution_host).cloned()
            {
                return Ok(addresses);
            }
        }
        tokio::net::lookup_host((resolution_host, 0))
            .await
            .map(|addresses| addresses.collect())
    }

    async fn refresh(&self, host: &str) -> std::io::Result<Vec<IpAddr>> {
        let host = normalize_host(host).map_err(std::io::Error::other)?;
        let cached = self.resolved_addresses(&host);
        if !cached.is_empty() {
            return Ok(cached);
        }
        let host_lock = self.resolving_lock(&host);
        let _guard = host_lock.lock().await;
        let cached = self.resolved_addresses(&host);
        if !cached.is_empty() {
            return Ok(cached);
        }
        let resolution_host = self.resolution_host(&host);
        let mut addresses = self.lookup_addresses(&resolution_host).await?;
        if addresses.is_empty() {
            return Ok(Vec::new());
        }
        addresses = self.order_addresses(&host, addresses);
        let resolved = addresses.iter().map(|address| address.ip()).collect();
        self.cache_addresses(host, addresses);
        Ok(resolved)
    }

    /// Resolves a host ahead of the first request so batch downloads can
    /// share a single ordered address list. Idempotent and non-fatal: a
    /// failed lookup leaves the resolver untouched and requests will resolve
    /// on demand later.
    pub async fn pre_resolve(&self, host: &str) {
        let _ = self.refresh(host).await;
    }

    #[cfg(test)]
    fn set_test_addresses(&self, host: &str, addresses: Vec<SocketAddr>) {
        self.test_addresses
            .lock()
            .insert(host.to_string(), addresses);
    }

    #[cfg(test)]
    fn set_test_lookup_delay(&self, host: &str, delay: Duration) {
        self.test_lookup_delays
            .lock()
            .insert(host.to_string(), delay);
    }

    #[cfg(test)]
    fn expire_cache(&self, host: &str) {
        if let Some(entry) = self.last_resolved.lock().get_mut(host) {
            entry.resolved_at = Instant::now() - CACHE_TTL;
        }
    }

    fn score(&self, address: IpAddr) -> f64 {
        self.reliability
            .lock()
            .get(&address)
            .copied()
            .unwrap_or_default()
    }

    fn order_addresses(
        &self,
        host: &str,
        mut addresses: Vec<SocketAddr>,
    ) -> Vec<SocketAddr> {
        addresses.sort_unstable_by_key(|address| address.ip());
        addresses.dedup_by_key(|address| address.ip());

        let best_v4 = addresses
            .iter()
            .filter(|address| address.is_ipv4())
            .map(|address| self.score(address.ip()))
            .max_by(f64::total_cmp);
        let mut best_v6 = addresses
            .iter()
            .filter(|address| address.is_ipv6())
            .map(|address| self.score(address.ip()))
            .max_by(f64::total_cmp);
        if host == "api.modrinth.com" {
            best_v6 = best_v6.map(|score| score - 0.1);
        }
        addresses.sort_unstable_by(|left, right| {
            let preferred_v4 =
                best_v4.unwrap_or_default() >= best_v6.unwrap_or_default();
            let left_family = left.is_ipv4() == preferred_v4;
            let right_family = right.is_ipv4() == preferred_v4;
            right_family.cmp(&left_family).then_with(|| {
                self.score(right.ip()).total_cmp(&self.score(left.ip()))
            })
        });
        addresses
    }
}

fn normalize_host(host: &str) -> Result<String, &'static str> {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty()
        || host.contains(['/', ':', '@', '[', ']'])
        || host.split('.').any(str::is_empty)
    {
        return Err("DNS host override must be a hostname without a port");
    }
    Ok(host)
}

impl Resolve for DownloadDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let resolver = self.clone();
        Box::pin(async move {
            let addresses = resolver
                .refresh(&host)
                .await?
                .into_iter()
                .map(|address| SocketAddr::new(address, 0))
                .collect::<Vec<_>>();
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn spawn_ipv4_server() -> (u16, tokio::task::JoinHandle<()>) {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await;
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                )
                .await
                .unwrap();
        });
        (port, handle)
    }

    async fn request_with_resolver(
        resolver: DownloadDnsResolver,
        host: &str,
        port: u16,
    ) -> String {
        reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(2))
            .dns_resolver(Arc::new(resolver))
            .build()
            .unwrap()
            .get(format!("http://{host}:{port}/"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    }

    #[test]
    fn returns_both_protocol_families_in_preferred_order() {
        let resolver = DownloadDnsResolver::default();
        let ipv4 = SocketAddr::from((Ipv4Addr::new(203, 0, 113, 10), 0));
        let ipv6 = SocketAddr::from((Ipv6Addr::LOCALHOST, 0));
        resolver.record_result(ipv6.ip(), -0.7);

        assert_eq!(
            resolver.order_addresses("api.modrinth.com", vec![ipv6, ipv4]),
            vec![ipv4, ipv6]
        );
    }

    #[test]
    fn selects_the_most_reliable_address_within_a_family() {
        let resolver = DownloadDnsResolver::default();
        let slower = SocketAddr::from((Ipv4Addr::new(203, 0, 113, 10), 0));
        let faster = SocketAddr::from((Ipv4Addr::new(203, 0, 113, 11), 0));
        resolver.record_result(faster.ip(), 0.5);

        assert_eq!(
            resolver.order_addresses("cdn.example.com", vec![slower, faster]),
            vec![faster, slower]
        );
    }

    #[test]
    fn only_records_the_address_that_completed_the_request() {
        let resolver = DownloadDnsResolver::default();
        let failed = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10));
        let succeeded = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 11));
        resolver.cache_addresses(
            "cdn.example.com".to_string(),
            vec![SocketAddr::new(failed, 0), SocketAddr::new(succeeded, 0)],
        );

        resolver.record_host_success("cdn.example.com", succeeded);

        assert_eq!(resolver.score(failed), 0.0);
        assert!(resolver.score(succeeded) > 0.0);
    }

    #[test]
    fn host_success_does_not_refresh_an_expired_dns_entry() {
        let resolver = DownloadDnsResolver::default();
        let address = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 11));
        resolver.cache_addresses(
            "expired-success.test".to_string(),
            vec![SocketAddr::new(address, 0)],
        );
        resolver.expire_cache("expired-success.test");

        resolver.record_host_success("expired-success.test", address);

        assert!(
            resolver
                .resolved_addresses("expired-success.test")
                .is_empty()
        );
    }

    #[test]
    fn late_success_does_not_undo_failure_expiration() {
        let resolver = DownloadDnsResolver::default();
        let address = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 12));
        resolver.cache_addresses(
            "late-success.test".to_string(),
            vec![SocketAddr::new(address, 0)],
        );
        assert!(!resolver.record_connection_failure("late-success.test"));
        assert!(resolver.record_connection_failure("late-success.test"));

        resolver.record_host_success("late-success.test", address);

        assert!(resolver.resolved_addresses("late-success.test").is_empty());
    }

    #[tokio::test]
    async fn one_request_falls_back_when_the_first_ip_refuses_connection() {
        let resolver = DownloadDnsResolver::default();
        let refused = SocketAddr::from((Ipv4Addr::new(127, 0, 0, 2), 0));
        let available = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        resolver.set_test_addresses("multi.test", vec![refused, available]);
        resolver.record_result(refused.ip(), 1.0);
        let (port, server) = spawn_ipv4_server().await;

        let body = request_with_resolver(resolver, "multi.test", port).await;

        assert_eq!(body, "ok");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn one_request_falls_back_from_ipv6_to_ipv4() {
        let resolver = DownloadDnsResolver::default();
        let unavailable_v6 = SocketAddr::from((Ipv6Addr::LOCALHOST, 0));
        let available_v4 = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        resolver.set_test_addresses(
            "dual-stack.test",
            vec![unavailable_v6, available_v4],
        );
        resolver.record_result(unavailable_v6.ip(), 1.0);
        let (port, server) = spawn_ipv4_server().await;

        let body =
            request_with_resolver(resolver, "dual-stack.test", port).await;

        assert_eq!(body, "ok");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn expired_cache_refreshes_to_the_current_addresses() {
        let resolver = DownloadDnsResolver::default();
        let old = SocketAddr::from((Ipv4Addr::new(127, 0, 0, 2), 0));
        let current = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        resolver.set_test_addresses("ttl-refresh.test", vec![old]);
        resolver.pre_resolve("ttl-refresh.test").await;
        resolver.set_test_addresses("ttl-refresh.test", vec![current]);

        assert_eq!(
            resolver.resolved_addresses("ttl-refresh.test"),
            vec![old.ip()]
        );
        resolver.expire_cache("ttl-refresh.test");
        resolver.pre_resolve("ttl-refresh.test").await;

        assert_eq!(
            resolver.resolved_addresses("ttl-refresh.test"),
            vec![current.ip()]
        );
    }

    #[tokio::test]
    async fn repeated_connection_failures_refresh_the_cached_address() {
        let resolver = DownloadDnsResolver::default();
        let old = SocketAddr::from((Ipv4Addr::new(127, 0, 0, 2), 0));
        let current = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        resolver.set_test_addresses("failure-refresh.test", vec![old]);
        resolver.pre_resolve("failure-refresh.test").await;
        resolver.set_test_addresses("failure-refresh.test", vec![current]);

        assert!(!resolver.record_connection_failure("failure-refresh.test"));
        assert!(resolver.record_connection_failure("failure-refresh.test"));
        assert!(
            resolver
                .resolved_addresses("failure-refresh.test")
                .is_empty()
        );
        resolver.pre_resolve("failure-refresh.test").await;

        assert_eq!(
            resolver.resolved_addresses("failure-refresh.test"),
            vec![current.ip()]
        );
        let (port, server) = spawn_ipv4_server().await;
        assert_eq!(
            request_with_resolver(resolver, "failure-refresh.test", port).await,
            "ok"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn pre_resolving_one_host_does_not_block_another_host() {
        let resolver = DownloadDnsResolver::default();
        resolver.set_test_addresses(
            "slow-resolution.test",
            vec![SocketAddr::from((Ipv4Addr::LOCALHOST, 0))],
        );
        resolver.set_test_lookup_delay(
            "slow-resolution.test",
            Duration::from_millis(100),
        );
        resolver.set_test_addresses(
            "fast-resolution.test",
            vec![SocketAddr::from((Ipv4Addr::LOCALHOST, 0))],
        );

        let slow_resolver = resolver.clone();
        let slow = tokio::spawn(async move {
            slow_resolver.pre_resolve("slow-resolution.test").await;
        });
        tokio::task::yield_now().await;
        tokio::time::timeout(
            Duration::from_millis(50),
            resolver.pre_resolve("fast-resolution.test"),
        )
        .await
        .expect("an unrelated DNS lookup must not wait for the slow host");

        assert!(
            !resolver
                .resolved_addresses("fast-resolution.test")
                .is_empty()
        );
        slow.await.unwrap();
    }

    #[tokio::test]
    async fn host_override_uses_the_target_hosts_addresses() {
        let resolver = DownloadDnsResolver::default();
        let (port, server) = spawn_ipv4_server().await;
        resolver.set_test_addresses(
            "resolver-target.test",
            vec![SocketAddr::from((Ipv4Addr::LOCALHOST, 0))],
        );
        resolver
            .set_host_override("REQUEST-HOST.TEST.", "resolver-target.test")
            .unwrap();

        let body =
            request_with_resolver(resolver.clone(), "request-host.test", port)
                .await;

        assert_eq!(body, "ok");
        assert_eq!(
            resolver.host_override("request-host.test").as_deref(),
            Some("resolver-target.test"),
        );
        assert!(!resolver.resolved_addresses("request-host.test").is_empty());
        server.await.unwrap();
    }

    #[test]
    fn clearing_override_removes_the_cached_request_host_addresses() {
        let resolver = DownloadDnsResolver::default();
        resolver
            .set_host_override("request-host.test", "resolver-target.test")
            .unwrap();
        resolver.cache_addresses(
            "request-host.test".to_string(),
            vec![SocketAddr::from((Ipv4Addr::LOCALHOST, 0))],
        );

        resolver.clear_host_override("request-host.test").unwrap();

        assert!(resolver.host_override("request-host.test").is_none());
        assert!(resolver.resolved_addresses("request-host.test").is_empty());
        assert!(normalize_host("https://resolver-target.test").is_err());
        assert!(normalize_host("resolver-target.test:443").is_err());
    }

    #[test]
    fn tianpao_default_uses_the_shopify_resolver_host() {
        assert_eq!(
            DownloadDnsResolver::default()
                .host_override("mod.tianpao.top")
                .as_deref(),
            Some("www.shopify.com"),
        );
    }

    #[test]
    fn legacy_modrinth_cdn_uses_the_shopify_resolver_host() {
        assert_eq!(
            DownloadDnsResolver::default()
                .host_override("cdn.modrinth.com")
                .as_deref(),
            Some("www.shopify.com"),
        );
    }
}
