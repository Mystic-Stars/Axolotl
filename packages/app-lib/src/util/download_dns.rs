use parking_lot::Mutex;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct DownloadDnsResolver {
    reliability: Arc<Mutex<HashMap<IpAddr, f64>>>,
    last_selected: Arc<Mutex<HashMap<String, IpAddr>>>,
}

impl DownloadDnsResolver {
    pub fn record_result(&self, address: IpAddr, result: f64) {
        let mut reliability = self.reliability.lock();
        reliability
            .entry(address)
            .and_modify(|value| *value = *value * 0.5 + result * 0.5)
            .or_insert(result * 0.5);
    }

    pub fn record_host_result(&self, host: &str, result: f64) {
        let address = self.last_selected.lock().get(host).copied();
        if let Some(address) = address {
            self.record_result(address, result);
        }
    }

    fn score(&self, address: IpAddr) -> f64 {
        self.reliability
            .lock()
            .get(&address)
            .copied()
            .unwrap_or_default()
    }

    fn select_address(
        &self,
        host: &str,
        mut addresses: Vec<SocketAddr>,
    ) -> Option<SocketAddr> {
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
        if let (Some(v4), Some(v6)) = (best_v4, best_v6) {
            addresses.retain(|address| address.is_ipv4() == (v4 >= v6));
        }
        addresses.sort_unstable_by(|left, right| {
            self.score(right.ip()).total_cmp(&self.score(left.ip()))
        });
        addresses.into_iter().next()
    }
}

impl Resolve for DownloadDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let resolver = self.clone();
        Box::pin(async move {
            let addresses = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .collect::<Vec<_>>();
            let selected = resolver.select_address(&host, addresses);
            if let Some(address) = selected {
                resolver.last_selected.lock().insert(host, address.ip());
                resolver.record_result(address.ip(), -0.01);
            }
            Ok(Box::new(selected.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn selects_one_address_from_the_preferred_protocol_family() {
        let resolver = DownloadDnsResolver::default();
        let ipv4 = SocketAddr::from((Ipv4Addr::new(203, 0, 113, 10), 0));
        let ipv6 = SocketAddr::from((Ipv6Addr::LOCALHOST, 0));
        resolver.record_result(ipv6.ip(), -0.7);

        assert_eq!(
            resolver.select_address("api.modrinth.com", vec![ipv4, ipv6]),
            Some(ipv4)
        );
    }

    #[test]
    fn selects_the_most_reliable_address_within_a_family() {
        let resolver = DownloadDnsResolver::default();
        let slower = SocketAddr::from((Ipv4Addr::new(203, 0, 113, 10), 0));
        let faster = SocketAddr::from((Ipv4Addr::new(203, 0, 113, 11), 0));
        resolver.record_result(faster.ip(), 0.5);

        assert_eq!(
            resolver.select_address("cdn.example.com", vec![slower, faster]),
            Some(faster)
        );
    }
}
