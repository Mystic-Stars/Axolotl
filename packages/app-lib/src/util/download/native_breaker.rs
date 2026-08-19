//! Short-lived route circuit breaker for native transfers.

use crate::util::fetch::{DownloadRoute, ProxyPolicy};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const FAILURE_THRESHOLD: u32 = 3;
const DEFAULT_COOLDOWN: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct BreakerKey {
    authority: String,
    proxy: ProxyPolicy,
}

#[derive(Default)]
struct BreakerState {
    consecutive_failures: u32,
    open_until: Option<Instant>,
}

static BREAKERS: LazyLock<Mutex<HashMap<BreakerKey, BreakerState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn key(route: &DownloadRoute) -> Option<BreakerKey> {
    Some(BreakerKey {
        authority: crate::util::fetch::url_authority(&route.url)?,
        proxy: route.proxy,
    })
}

pub(crate) fn is_open(route: &DownloadRoute) -> bool {
    let Some(key) = key(route) else {
        return false;
    };
    let mut breakers = BREAKERS.lock();
    let Some(state) = breakers.get_mut(&key) else {
        return false;
    };
    match state.open_until {
        Some(until) if until > Instant::now() => true,
        Some(_) => {
            state.open_until = None;
            state.consecutive_failures = 0;
            false
        }
        None => false,
    }
}

pub(crate) fn should_skip(
    route: &DownloadRoute,
    has_healthy_alternate: bool,
) -> bool {
    if !has_healthy_alternate {
        return false;
    }
    is_open(route)
}

pub(crate) fn record_success(route: &DownloadRoute) {
    let Some(key) = key(route) else {
        return;
    };
    if let Some(state) = BREAKERS.lock().get_mut(&key) {
        state.consecutive_failures = 0;
        state.open_until = None;
    }
}

pub(crate) fn record_failure(route: &DownloadRoute) {
    record_failure_with_cooldown(route, DEFAULT_COOLDOWN);
}

pub(crate) fn record_failure_with_cooldown(
    route: &DownloadRoute,
    cooldown: Duration,
) {
    let Some(key) = key(route) else {
        return;
    };
    let mut breakers = BREAKERS.lock();
    let state = breakers.entry(key).or_default();
    state.consecutive_failures = state.consecutive_failures.saturating_add(1);
    if state.consecutive_failures >= FAILURE_THRESHOLD {
        state.open_until = Some(Instant::now() + cooldown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fetch::DownloadRouteSource;

    fn route() -> DownloadRoute {
        DownloadRoute {
            url: "https://breaker.example/file".to_string(),
            source: DownloadRouteSource::Official,
            is_mirror: false,
            allow_sensitive_headers: true,
            supports_range: true,
            proxy: ProxyPolicy::Direct,
        }
    }

    #[test]
    fn unique_route_is_never_skipped() {
        let route = route();
        for _ in 0..FAILURE_THRESHOLD {
            record_failure(&route);
        }
        assert!(should_skip(&route, true));
        assert!(!should_skip(&route, false));
        record_success(&route);
    }
}
