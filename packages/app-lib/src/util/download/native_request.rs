//! HTTP request construction and redirect handling for downloads.

use super::modrinth_redirect::repair_official_redirect as repair_official_cdn_redirect;
use crate::ErrorKind;
use crate::util::fetch::{
    DIRECT_REQWEST_CLIENT, DOWNLOAD_META_HEADER, DownloadClients, DownloadMeta,
    DownloadRoute, MAX_REDIRECT_LOCATION_BYTES, NO_REDIRECT_REQWEST_CLIENT,
    ProxyPolicy, authority_uses_http1_fallback_for,
    forget_effective_route_authority, is_allowed_download_redirect,
    is_h2_protocol_failure, is_official_modrinth_download_url,
    is_sensitive_header, record_authority_h2_failure_for,
    record_dns_connection_failure, remember_effective_route_authority,
    same_origin, sanitize_url_for_log, url_authority,
};
use reqwest::{Url, header};
use tokio::sync::Mutex as AsyncMutex;

pub(crate) fn byte_range_header_value(
    range_start: Option<u64>,
    range_end: Option<u64>,
) -> Option<String> {
    range_start.map(|start| {
        range_end.map_or_else(
            || format!("bytes={start}-"),
            |end| format!("bytes={start}-{end}"),
        )
    })
}

pub(crate) async fn send_path_request_with_clients(
    route: &DownloadRoute,
    custom_header: Option<&(String, String)>,
    credentials: Option<&crate::state::ModrinthCredentials>,
    download_meta: Option<&DownloadMeta>,
    range_start: Option<u64>,
    range_end: Option<u64>,
    clients: &DownloadClients,
    redirect_target: Option<&AsyncMutex<Option<Url>>>,
) -> crate::Result<(reqwest::Response, String)> {
    let original = Url::parse(&route.url)?;
    let mut current = match redirect_target {
        Some(target) => target
            .lock()
            .await
            .as_ref()
            .cloned()
            .unwrap_or_else(|| original.clone()),
        None => original.clone(),
    };
    let mut reused_redirect_target = current != original;
    for redirect_count in 0..=5 {
        let fallback_to_http1 =
            url_authority(current.as_str()).is_some_and(|authority| {
                authority_uses_http1_fallback_for(&authority, route.proxy)
            });
        let (system_client_for_hop, direct_client_for_hop): (
            &reqwest::Client,
            &reqwest::Client,
        ) = if fallback_to_http1 {
            (&clients.http1_system, &clients.http1_direct)
        } else {
            (&clients.system, &clients.direct)
        };
        let client = if route.proxy == ProxyPolicy::Direct {
            direct_client_for_hop
        } else {
            system_client_for_hop
        };
        let same_as_original = same_origin(&original, &current);
        let allow_sensitive = route.allow_sensitive_headers && same_as_original;
        let mut request = client.get(current.clone());
        if let Some((name, value)) = custom_header
            && (allow_sensitive || !is_sensitive_header(name))
            && (!name.eq_ignore_ascii_case("x-api-key")
                || original.host_str() == Some("api.curseforge.com"))
        {
            request = request.header(name, value);
        }
        if allow_sensitive && let Some(credentials) = credentials {
            request = request.header("Authorization", &credentials.session);
        }
        if !route.is_mirror
            && same_as_original
            && is_official_modrinth_download_url(original.as_str())
            && let Some(download_meta) = download_meta
        {
            request = request
                .header(DOWNLOAD_META_HEADER, download_meta.to_header_value());
        }
        if let Some(range) = byte_range_header_value(range_start, range_end) {
            request = request
                .header(header::RANGE, range)
                .header(header::ACCEPT_ENCODING, "identity");
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                if let Some(host) = record_dns_connection_failure(route, &error)
                {
                    crate::util::fetch::prewarm_download_dns_for(
                        route.proxy,
                        &[&host],
                    )
                    .await;
                }
                if !fallback_to_http1
                    && redirect_count < 5
                    && is_h2_protocol_failure(&error)
                    && let Some(authority) = url_authority(current.as_str())
                {
                    tracing::warn!(
                        authority,
                        error = %error.without_url(),
                        "HTTP/2 download request failed; retrying over HTTP/1.1"
                    );
                    record_authority_h2_failure_for(&authority, route.proxy);
                    continue;
                }
                return Err(error.into());
            }
        };
        if !response.status().is_redirection() {
            if reused_redirect_target
                && (response.status().is_client_error()
                    || response.status().is_server_error())
            {
                forget_effective_route_authority(route, &current);
                if let Some(target) = redirect_target {
                    let mut cached = target.lock().await;
                    if cached.as_ref() == Some(&current) {
                        *cached = None;
                    }
                }
                current = original.clone();
                reused_redirect_target = false;
                continue;
            }
            remember_effective_route_authority(route, current.as_str());
            if response.status().is_success()
                && current != original
                && let Some(target) = redirect_target
            {
                let mut cached = target.lock().await;
                if cached.is_none() {
                    *cached = Some(current.clone());
                }
            }
            tracing::debug!(
                original_url = %sanitize_url_for_log(&route.url),
                final_host = current.host_str().unwrap_or_default(),
                reused_redirect_target,
                http1_fallback = fallback_to_http1,
                "Resolved file download route"
            );
            return Ok((response, current.into()));
        }
        if redirect_count == 5 {
            return Err(ErrorKind::OtherError(format!(
                "Too many redirects while downloading {}",
                route.url
            ))
            .into());
        }
        let location = response
            .headers()
            .get(header::LOCATION)
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
            .ok_or_else(|| {
                ErrorKind::OtherError(format!(
                    "Redirect from {current} did not include a valid Location header"
                ))
            })?;
        if location.len() > MAX_REDIRECT_LOCATION_BYTES
            || location.chars().any(char::is_control)
        {
            return Err(ErrorKind::OtherError(format!(
                "Redirect from {current} included an unsafe Location header"
            ))
            .into());
        }
        let next = current.join(&location)?;
        if !is_allowed_download_redirect(&next) {
            return Err(ErrorKind::OtherError(format!(
                "Refusing insecure redirect from {current} to {next}"
            ))
            .into());
        }
        current = repair_official_cdn_redirect(&original, &next, &location)
            .unwrap_or(next);
    }
    unreachable!()
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn send_path_request(
    route: &DownloadRoute,
    custom_header: Option<&(String, String)>,
    credentials: Option<&crate::state::ModrinthCredentials>,
    download_meta: Option<&DownloadMeta>,
    range_start: Option<u64>,
    range_end: Option<u64>,
) -> crate::Result<(reqwest::Response, String)> {
    let clients = DownloadClients::for_request(
        &NO_REDIRECT_REQWEST_CLIENT,
        &DIRECT_REQWEST_CLIENT,
    );
    send_path_request_with_clients(
        route,
        custom_header,
        credentials,
        download_meta,
        range_start,
        range_end,
        &clients,
        None,
    )
    .await
}
