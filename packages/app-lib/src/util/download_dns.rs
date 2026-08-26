use parking_lot::Mutex;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, LazyLock};

#[derive(Clone, Default)]
pub struct DownloadDnsResolver {
    reliability: Arc<Mutex<HashMap<IpAddr, f64>>>,
    last_resolved: Arc<Mutex<HashMap<String, Vec<IpAddr>>>>,
    #[cfg(test)]
    test_addresses: Arc<Mutex<HashMap<String, Vec<SocketAddr>>>>,
}

static PRE_RESOLVE_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

impl DownloadDnsResolver {
    pub fn record_result(&self, address: IpAddr, result: f64) {
        let mut reliability = self.reliability.lock();
        reliability
            .entry(address)
            .and_modify(|value| *value = *value * 0.5 + result * 0.5)
            .or_insert(result * 0.5);
    }

    pub fn record_host_success(&self, host: &str, address: IpAddr) {
        if self
            .last_resolved
            .lock()
            .get(host)
            .is_some_and(|addresses| addresses.contains(&address))
        {
            self.record_result(address, 0.5);
        }
    }

    pub fn resolved_addresses(&self, host: &str) -> Vec<IpAddr> {
        self.last_resolved
            .lock()
            .get(host)
            .cloned()
            .unwrap_or_default()
    }

    /// Resolves a host ahead of the first request so batch downloads can
    /// share a single ordered address list. Idempotent and non-fatal: a
    /// failed lookup leaves the resolver untouched and requests will resolve
    /// on demand later.
    pub async fn pre_resolve(&self, host: &str) {
        if !self.resolved_addresses(host).is_empty() {
            return;
        }
        let _guard = PRE_RESOLVE_LOCK.lock().await;
        if !self.resolved_addresses(host).is_empty() {
            return;
        }
        let Ok(addresses) = tokio::net::lookup_host((host, 0)).await else {
            return;
        };
        let mut addresses = addresses.collect::<Vec<_>>();
        if addresses.is_empty() {
            return;
        }
        addresses.sort_unstable_by_key(|address| address.ip());
        addresses.dedup_by_key(|address| address.ip());
        let addresses = self.order_addresses(host, addresses);
        self.last_resolved.lock().insert(
            host.to_string(),
            addresses.iter().map(|address| address.ip()).collect(),
        );
    }

    #[cfg(test)]
    fn set_test_addresses(&self, host: &str, addresses: Vec<SocketAddr>) {
        self.test_addresses
            .lock()
            .insert(host.to_string(), addresses);
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

impl Resolve for DownloadDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let resolver = self.clone();
        Box::pin(async move {
            let cached_addresses = resolver
                .resolved_addresses(&host)
                .into_iter()
                .map(|address| SocketAddr::new(address, 0))
                .collect::<Vec<_>>();
            #[cfg(test)]
            let test_addresses =
                resolver.test_addresses.lock().get(&host).cloned();
            #[cfg(test)]
            let addresses = if let Some(addresses) = test_addresses {
                addresses
            } else if !cached_addresses.is_empty() {
                cached_addresses
            } else {
                tokio::net::lookup_host((host.as_str(), 0))
                    .await?
                    .collect::<Vec<_>>()
            };
            #[cfg(not(test))]
            let addresses = if !cached_addresses.is_empty() {
                cached_addresses
            } else {
                tokio::net::lookup_host((host.as_str(), 0))
                    .await?
                    .collect::<Vec<_>>()
            };
            let addresses = resolver.order_addresses(&host, addresses);
            resolver.last_resolved.lock().insert(
                host,
                addresses.iter().map(|address| address.ip()).collect(),
            );
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
        resolver
            .last_resolved
            .lock()
            .insert("cdn.example.com".to_string(), vec![failed, succeeded]);

        resolver.record_host_success("cdn.example.com", succeeded);

        assert_eq!(resolver.score(failed), 0.0);
        assert!(resolver.score(succeeded) > 0.0);
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
}
