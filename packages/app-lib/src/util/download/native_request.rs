//! HTTP request construction and redirect handling for downloads.

use super::modrinth_cdn;
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

/// CDN byte fetches use the same redirect normalization as streamed downloads.
pub(crate) async fn send_cdn_request(
    builder: reqwest::RequestBuilder,
) -> Result<reqwest::Response, reqwest::Error> {
    let (client, request) = builder.build_split();
    let mut request = request?;
    *request.url_mut() = modrinth_cdn::normalize_parsed(request.url().clone());
    let original = request.url().clone();
    for hop in 0..=5 {
        let next_request = request.try_clone();
        let response = client.execute(request).await?;
        if !response.status().is_redirection() || hop == 5 {
            return Ok(response);
        }
        let Some(location) =
            response.headers().get(header::LOCATION).map(|value| {
                String::from_utf8_lossy(value.as_bytes()).into_owned()
            })
        else {
            return Ok(response);
        };
        if location.len() > MAX_REDIRECT_LOCATION_BYTES
            || location.chars().any(char::is_control)
        {
            return Ok(response);
        }
        let Ok(next) = response.url().join(&location) else {
            return Ok(response);
        };
        if !is_allowed_download_redirect(&next) {
            return Ok(response);
        }
        let Some(mut cloned) = next_request else {
            return Ok(response);
        };
        let next = repair_official_cdn_redirect(&original, &next, &location)
            .unwrap_or(next);
        if !same_origin(cloned.url(), &next) {
            let names = cloned
                .headers()
                .keys()
                .filter(|name| {
                    is_sensitive_header(name.as_str())
                        || name.as_str() == DOWNLOAD_META_HEADER
                })
                .cloned()
                .collect::<Vec<_>>();
            for name in names {
                cloned.headers_mut().remove(name);
            }
        }
        *cloned.url_mut() = next;
        request = cloned;
    }
    unreachable!()
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
    let original = Url::parse(&modrinth_cdn::normalize(&route.url))?;
    let mut current = match redirect_target {
        Some(target) => target
            .lock()
            .await
            .as_ref()
            .cloned()
            .unwrap_or_else(|| original.clone()),
        None => original.clone(),
    };
    current = modrinth_cdn::normalize_parsed(current);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fetch::{DownloadRouteSource, ResourceClass};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn read_headers(stream: &mut tokio::net::TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let n = stream.read(&mut buffer).await.unwrap();
            assert_ne!(n, 0);
            request.extend_from_slice(&buffer[..n]);
        }
        String::from_utf8(request).unwrap()
    }

    async fn proxy_client(
        redirect: bool,
    ) -> (reqwest::Client, tokio::task::JoinHandle<String>) {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            if redirect {
                let (mut first, _) = listener.accept().await.unwrap();
                let headers = read_headers(&mut first).await;
                assert!(headers.starts_with("GET http://origin.invalid/"));
                first.write_all(b"HTTP/1.1 302 Found\r\nLocation: https://cdn-alt.modrinth.com/data/file.jar?x=%2b\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
            }
            let (mut stream, _) = listener.accept().await.unwrap();
            let headers = read_headers(&mut stream).await;
            stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
            headers
        });
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(format!("http://{address}")).unwrap())
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        (client, task)
    }

    #[tokio::test]
    async fn retired_modrinth_streamed_requests_never_connect_to_the_retired_host()
     {
        let (client, proxy) = proxy_client(false).await;
        let route = DownloadRoute {
            url: "https://cdn-alt.modrinth.com/data/file.jar".into(),
            source: DownloadRouteSource::Official,
            is_mirror: false,
            allow_sensitive_headers: true,
            supports_range: true,
            proxy: ProxyPolicy::System,
        };
        let clients = DownloadClients::for_request(&client, &client);
        assert!(
            send_path_request_with_clients(
                &route, None, None, None, None, None, &clients, None
            )
            .await
            .is_err()
        );
        let headers = tokio::time::timeout(Duration::from_secs(5), proxy)
            .await
            .unwrap()
            .unwrap();
        assert!(
            headers.starts_with("CONNECT cdn.modrinth.com:443 "),
            "{headers}"
        );
        assert!(!headers.contains("cdn-alt"));
    }

    #[tokio::test]
    async fn retired_modrinth_redirects_never_connect_to_the_retired_host() {
        for streamed in [true, false] {
            let (client, proxy) = proxy_client(true).await;
            if streamed {
                let route = DownloadRoute {
                    url: "http://origin.invalid/".into(),
                    source: DownloadRouteSource::Official,
                    is_mirror: false,
                    allow_sensitive_headers: false,
                    supports_range: true,
                    proxy: ProxyPolicy::System,
                };
                let clients = DownloadClients::for_request(&client, &client);
                assert!(
                    send_path_request_with_clients(
                        &route, None, None, None, None, None, &clients, None
                    )
                    .await
                    .is_err()
                );
            } else {
                assert!(
                    send_cdn_request(client.get("http://origin.invalid/"))
                        .await
                        .is_err()
                );
            }
            let headers = tokio::time::timeout(Duration::from_secs(5), proxy)
                .await
                .unwrap()
                .unwrap();
            assert!(
                headers.starts_with("CONNECT cdn.modrinth.com:443 "),
                "{headers}"
            );
            assert!(!headers.contains("cdn-alt"));
        }
    }

    #[tokio::test]
    async fn retired_modrinth_byte_requests_never_connect_to_the_retired_host()
    {
        let (client, proxy) = proxy_client(false).await;
        assert!(
            send_cdn_request(
                client.get("https://cdn-alt.modrinth.com/data/file.jar")
            )
            .await
            .is_err()
        );
        let headers = tokio::time::timeout(Duration::from_secs(5), proxy)
            .await
            .unwrap()
            .unwrap();
        assert!(
            headers.starts_with("CONNECT cdn.modrinth.com:443 "),
            "{headers}"
        );
        assert!(!headers.contains("cdn-alt"));
    }

    #[tokio::test]
    async fn modrinth_byte_redirects_strip_protected_headers_across_origins() {
        let first = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let second =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}/", first.local_addr().unwrap());
        let target = format!(
            "http://{}/file.jar?download=1",
            second.local_addr().unwrap()
        );
        let first_task = tokio::spawn(async move {
            let (mut stream, _) = first.accept().await.unwrap();
            let headers = read_headers(&mut stream).await.to_ascii_lowercase();
            assert!(headers.contains("authorization: secret"));
            stream.write_all(format!("HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        });
        let second_task = tokio::spawn(async move {
            let (mut stream, _) = second.accept().await.unwrap();
            let headers = read_headers(&mut stream).await.to_ascii_lowercase();
            assert!(!headers.contains("authorization:"));
            assert!(!headers.contains("cookie:"));
            assert!(!headers.contains("modrinth-download-meta:"));
            assert!(headers.contains("range: bytes=0-3"));
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata").await.unwrap();
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let response = send_cdn_request(
            client
                .get(origin)
                .header("Authorization", "secret")
                .header("Cookie", "secret=1")
                .header(DOWNLOAD_META_HEADER, "statistics")
                .header(header::RANGE, "bytes=0-3"),
        )
        .await
        .unwrap();
        assert_eq!(response.bytes().await.unwrap(), "data");
        first_task.await.unwrap();
        second_task.await.unwrap();
    }

    #[test]
    fn retired_modrinth_routes_keep_download_metadata_for_both_transports() {
        let routes = crate::util::fetch::resolve_download_routes_for(
            "https://cdn-alt.modrinth.com/data/file.jar",
            ResourceClass::Modrinth,
            crate::state::DownloadSourceMode::OfficialOnly,
        );
        let request = crate::util::fetch::DownloadRequest::new(
            &routes[0].url,
            ResourceClass::Modrinth,
        )
        .with_download_meta(DownloadMeta {
            reason: crate::util::fetch::DownloadReason::Standalone,
            game_version: "1.21.1".into(),
            loader: "fabric".into(),
            dependent_on: None,
        });
        let headers =
            super::super::h2_download::request_headers(&request, &routes[0]);
        assert!(headers.contains_key(DOWNLOAD_META_HEADER));
        assert!(is_official_modrinth_download_url(&routes[0].url));
    }
}
