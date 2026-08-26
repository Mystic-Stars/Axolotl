//! Policy boundary for the native download engine.

use crate::util::fetch::{DownloadRoute, ProxyPolicy};

const SMALL_FILE_LIMIT: u64 = 4 * 1024 * 1024;
const MEDIUM_FILE_LIMIT: u64 = 16 * 1024 * 1024;
const LARGE_FILE_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeH2Policy {
    pub(crate) abort_if_slow: bool,
    pub(crate) expected_speed: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeH2IneligibleReason {
    Http1Fallback,
    SystemProxy,
}

impl NativeH2IneligibleReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Http1Fallback => "authority is temporarily using HTTP/1.1",
            Self::SystemProxy => "system proxy requires the reqwest transport",
        }
    }
}

pub(crate) fn h2_ineligible_reason(
    route: &DownloadRoute,
) -> Option<NativeH2IneligibleReason> {
    let authority = crate::util::fetch::url_authority(&route.url)?;
    if crate::util::fetch::authority_uses_http1_fallback(&authority) {
        return Some(NativeH2IneligibleReason::Http1Fallback);
    }
    if route.proxy == ProxyPolicy::System && system_proxy_configured() {
        return Some(NativeH2IneligibleReason::SystemProxy);
    }
    None
}

pub(crate) async fn h2_policy(
    route: &DownloadRoute,
    size: Option<u64>,
) -> Option<NativeH2Policy> {
    if h2_ineligible_reason(route).is_some() {
        return None;
    }
    let Some(size) = size else {
        return Some(NativeH2Policy {
            abort_if_slow: false,
            expected_speed: None,
        });
    };
    let authority = crate::util::fetch::url_authority(&route.url)?;
    let h2 = super::native_reputation::get_transport(
        &authority,
        route.proxy,
        super::native_reputation::NativeTransport::H2Single,
    );
    let multi_range = super::native_reputation::get_transport(
        &authority,
        route.proxy,
        super::native_reputation::NativeTransport::Http1MultiRange,
    );
    let expected_speed = h2
        .and_then(|health| health.throughput_bps)
        .map(|speed| speed.min(u64::MAX as f64) as u64);
    if size < SMALL_FILE_LIMIT {
        return Some(NativeH2Policy {
            abort_if_slow: false,
            expected_speed,
        });
    }
    if size < MEDIUM_FILE_LIMIT {
        return super::h2_pool::has_live_connection(&authority)
            .await
            .then_some(NativeH2Policy {
                abort_if_slow: true,
                expected_speed,
            });
    }
    if !super::h2_pool::has_live_connection(&authority).await {
        return None;
    }
    let h2 = h2.filter(|health| health.success_samples >= 2)?;
    let h2_speed = h2.throughput_bps?;
    let range_speed = multi_range.and_then(|health| health.throughput_bps);
    if size < LARGE_FILE_LIMIT {
        let h2_preferred = range_speed
            .map(|range_speed| h2_speed >= range_speed * 1.1)
            .unwrap_or(true);
        return h2_preferred.then_some(NativeH2Policy {
            abort_if_slow: true,
            expected_speed: Some(h2_speed.min(u64::MAX as f64) as u64),
        });
    }
    let range = multi_range.filter(|health| health.success_samples >= 2)?;
    let range_speed = range.throughput_bps?;
    (h2.success_samples >= 3 && h2_speed >= range_speed * 1.25).then_some(
        NativeH2Policy {
            abort_if_slow: true,
            expected_speed: Some(h2_speed.min(u64::MAX as f64) as u64),
        },
    )
}

fn system_proxy_configured() -> bool {
    let environment_proxy = [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ]
    .into_iter()
    .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()));
    environment_proxy || platform_proxy_configured()
}

#[cfg(windows)]
fn platform_proxy_configured() -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let Ok(settings) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
    ) else {
        return false;
    };
    let enabled = settings
        .get_value::<u32, _>("ProxyEnable")
        .is_ok_and(|value| value != 0);
    let auto_configured = settings
        .get_value::<String, _>("AutoConfigURL")
        .is_ok_and(|value| !value.trim().is_empty());
    enabled || auto_configured
}

#[cfg(not(windows))]
fn platform_proxy_configured() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fetch::{DownloadRouteSource, ProxyPolicy};

    fn route(proxy: ProxyPolicy) -> DownloadRoute {
        DownloadRoute {
            url: "https://native-policy.example/file".to_string(),
            source: DownloadRouteSource::Official,
            is_mirror: false,
            allow_sensitive_headers: true,
            supports_range: true,
            proxy,
        }
    }

    #[test]
    fn direct_routes_ignore_system_proxy_environment() {
        assert_ne!(
            h2_ineligible_reason(&route(ProxyPolicy::Direct)),
            Some(NativeH2IneligibleReason::SystemProxy)
        );
    }

    #[tokio::test]
    async fn medium_file_requires_a_live_h2_connection() {
        assert!(
            h2_policy(&route(ProxyPolicy::Direct), Some(8 * 1024 * 1024),)
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn small_file_can_establish_h2_without_history() {
        assert_eq!(
            h2_policy(&route(ProxyPolicy::Direct), Some(1024 * 1024),).await,
            Some(NativeH2Policy {
                abort_if_slow: false,
                expected_speed: None,
            })
        );
    }
}
