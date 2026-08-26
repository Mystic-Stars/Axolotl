//! HTTP/2 multiplexed file downloads over shared per-authority connections.
//!
//! Every download to the same authority reuses one long-lived HTTP/2
//! connection. General file downloads use one stream so larger files can
//! switch to independent HTTP/1.1 range connections when that is faster;
//! Minecraft assets use the dedicated batch multiplexer below.

use super::h2_pool::{H2ConnectFailureKind, SharedH2Connection};
use crate::util::fetch;
use crate::util::fetch::{
    DownloadRequest, DownloadResult, DownloadRoute, DownloadRouteSource,
    Integrity,
};
use futures::StreamExt;
use http::header::{ACCEPT_ENCODING, RANGE, USER_AGENT};
use http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use url::Url;

const STREAM_RECV_TIMEOUT: Duration = Duration::from_secs(30);

/// Client-side concurrency target for the batch asset downloader. All
/// concurrent streams are multiplexed over one shared HTTP/2 connection per
/// authority, so this is the number of streams, not connections.
pub(crate) const ASSET_BATCH_CONCURRENCY: usize = 512;
/// Internal retry passes for failed batch items before they are handed back
/// to the caller for the regular per-file download path.
const ASSET_BATCH_RETRY_PASSES: usize = 2;

/// Outcome of attempting a multiplexed download.
pub(crate) enum H2DownloadOutcome {
    /// The download completed through the multiplexed path.
    Completed(DownloadResult),
    /// The multiplexed path cannot be used; the caller should fall back to
    /// the legacy path.
    Fallback {
        failure: H2DownloadFailure,
        preserve_partial: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum H2DownloadFailure {
    Ineligible(&'static str),
    Connect,
    Tls,
    Protocol,
    Http,
    Integrity,
    Content,
    Io,
    Slow,
}

impl H2DownloadFailure {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ineligible(reason) => reason,
            Self::Connect => "HTTP/2 TCP connection failed",
            Self::Tls => "HTTP/2 TLS connection failed",
            Self::Protocol => "HTTP/2 protocol failed",
            Self::Http => "HTTP/2 response was unsuccessful",
            Self::Integrity => "HTTP/2 integrity validation failed",
            Self::Content => "HTTP/2 content validation failed",
            Self::Io => "HTTP/2 local I/O failed",
            Self::Slow => "HTTP/2 single stream stayed below expectation",
        }
    }

    pub(crate) const fn should_cooldown_authority(self) -> bool {
        matches!(self, Self::Protocol)
    }

    pub(crate) const fn is_transfer_failure(self) -> bool {
        matches!(self, Self::Connect | Self::Tls | Self::Protocol)
    }

    pub(crate) const fn integrity_failure(self) -> bool {
        matches!(self, Self::Integrity | Self::Content)
    }
}

/// Attempts to download `request` as one stream on a shared HTTP/2 connection.
pub(crate) async fn try_download_via_h2(
    request: &DownloadRequest,
    route: &DownloadRoute,
    destination: &Path,
    part_path: &Path,
    policy: super::native::NativeH2Policy,
) -> H2DownloadOutcome {
    if let Some(reason) = super::native::h2_ineligible_reason(route) {
        return H2DownloadOutcome::Fallback {
            failure: H2DownloadFailure::Ineligible(reason.as_str()),
            preserve_partial: false,
        };
    }
    let connection = match connect_authority(&route.url).await {
        Ok(connection) => connection,
        Err(failure) => {
            return H2DownloadOutcome::Fallback {
                failure,
                preserve_partial: false,
            };
        }
    };
    let Ok(uri) = route.url.parse::<Uri>() else {
        return H2DownloadOutcome::Fallback {
            failure: H2DownloadFailure::Http,
            preserve_partial: false,
        };
    };

    let integrity = request.integrity.clone();
    let expected_size = integrity.size;

    fetch::record_install_download_started(request, route, 0, 1).await;

    // When the size is known (Modrinth metadata provides it) skip the probe
    // entirely: small files fetch the body directly, large files split into
    // range streams right away. The probe is only used when the size must be
    // discovered from the server.
    let total_size = if let Some(size) = expected_size {
        size
    } else {
        let mut probe_headers = request_headers(request, route);
        probe_headers.insert(RANGE, HeaderValue::from_static("bytes=0-0"));
        probe_headers
            .insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));

        let (response, mut probe_body) =
            match open_stream(&connection, &uri, probe_headers).await {
                Ok(pair) => pair,
                Err(error) => {
                    tracing::debug!(
                        url = %fetch::sanitize_url_for_log(&request.url),
                        error = %error,
                        "HTTP/2 probe failed; falling back to legacy download"
                    );
                    return H2DownloadOutcome::Fallback {
                        failure: classify_download_error(&error),
                        preserve_partial: false,
                    };
                }
            };

        let status = response.status();
        let headers = response.headers();
        let total_size = parse_content_range_total(headers)
            .or_else(|| parse_content_length(headers));
        // Drain the probe body so the stream slot is released.
        drain_body(&mut probe_body).await;
        drop(probe_body);

        let Some(total_size) = total_size else {
            return H2DownloadOutcome::Fallback {
                failure: H2DownloadFailure::Http,
                preserve_partial: false,
            };
        };
        if total_size == 0 {
            return H2DownloadOutcome::Fallback {
                failure: H2DownloadFailure::Content,
                preserve_partial: false,
            };
        }
        if status != StatusCode::PARTIAL_CONTENT {
            return H2DownloadOutcome::Fallback {
                failure: H2DownloadFailure::Http,
                preserve_partial: false,
            };
        }
        total_size
    };

    let result = single_stream(
        &connection,
        &uri,
        request,
        route,
        destination,
        part_path,
        &integrity,
        total_size,
        policy,
    )
    .await;
    match result {
        Ok(result) => H2DownloadOutcome::Completed(result),
        Err(error) => {
            let failure = classify_download_error(&error);
            tracing::debug!(
                url = %fetch::sanitize_url_for_log(&request.url),
                error = %error,
                "Multiplexed download failed; falling back to legacy download"
            );
            H2DownloadOutcome::Fallback {
                failure,
                preserve_partial: integrity.supports_resume()
                    && matches!(failure, H2DownloadFailure::Protocol),
            }
        }
    }
}

async fn connect_authority(
    url: &str,
) -> Result<Arc<SharedH2Connection>, H2DownloadFailure> {
    let authority = fetch::url_authority(url).ok_or(H2DownloadFailure::Http)?;
    match super::h2_pool::shared_connection(&authority).await {
        Ok(connection) => Ok(connection),
        Err(error) => {
            tracing::debug!(
                authority,
                error = %error,
                "Failed to establish shared HTTP/2 connection"
            );
            Err(match error.kind {
                H2ConnectFailureKind::Tcp => H2DownloadFailure::Connect,
                H2ConnectFailureKind::Tls => H2DownloadFailure::Tls,
                H2ConnectFailureKind::Protocol => H2DownloadFailure::Protocol,
            })
        }
    }
}

fn classify_download_error(error: &crate::Error) -> H2DownloadFailure {
    if fetch::is_integrity_error(error) {
        return H2DownloadFailure::Integrity;
    }
    match error.raw.as_ref() {
        crate::ErrorKind::HttpError { .. }
        | crate::ErrorKind::LabrinthError(_) => H2DownloadFailure::Http,
        crate::ErrorKind::IOError(_) | crate::ErrorKind::StdIOError(_) => {
            H2DownloadFailure::Io
        }
        crate::ErrorKind::JSONError(_) => H2DownloadFailure::Content,
        crate::ErrorKind::NetworkError(message)
            if message.contains("below expectation") =>
        {
            H2DownloadFailure::Slow
        }
        crate::ErrorKind::NetworkError(message)
            if message.contains("HTTP/2")
                || message.contains("range stream") =>
        {
            H2DownloadFailure::Protocol
        }
        crate::ErrorKind::OtherError(message)
            if message.contains("HTTP/2")
                || message.contains("Content-Range")
                || message.contains("segment") =>
        {
            H2DownloadFailure::Protocol
        }
        crate::ErrorKind::OtherError(message)
            if message.contains("empty") || message.contains("Invalid JAR") =>
        {
            H2DownloadFailure::Content
        }
        _ => H2DownloadFailure::Http,
    }
}

fn request_headers(
    request: &DownloadRequest,
    route: &DownloadRoute,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&crate::launcher_user_agent())
            .unwrap_or_else(|_| HeaderValue::from_static("Axolotl Launcher")),
    );
    let route_host = Url::parse(&route.url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string));
    if let Some((name, value)) = &request.header
        && (route.allow_sensitive_headers || !fetch::is_sensitive_header(name))
        && (!name.eq_ignore_ascii_case("x-api-key")
            || route_host.as_deref() == Some("api.curseforge.com"))
    {
        if let Ok(name) = http::header::HeaderName::from_str(name) {
            if let Ok(value) = HeaderValue::from_str(value) {
                headers.insert(name, value);
            }
        }
    }
    if route.source == DownloadRouteSource::Official
        && fetch::is_official_modrinth_download_url(&request.url)
        && let Some(download_meta) = &request.download_meta
    {
        if let Ok(value) =
            HeaderValue::from_str(&download_meta.to_header_value())
        {
            headers.insert("modrinth-download-meta", value);
        }
    }
    headers
}

fn parse_content_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

fn parse_content_range_total(headers: &HeaderMap) -> Option<u64> {
    let value = headers.get(http::header::CONTENT_RANGE)?.to_str().ok()?;
    let (_, total) = value.split_once('/')?;
    if total == "*" {
        return None;
    }
    total.parse().ok()
}

type StreamPair = (http::Response<()>, h2::RecvStream);

async fn open_stream(
    connection: &SharedH2Connection,
    uri: &Uri,
    headers: HeaderMap,
) -> crate::Result<StreamPair> {
    let mut request = http::Request::builder()
        .method(Method::GET)
        .uri(uri.clone())
        .version(http::Version::HTTP_2)
        .body(())
        .unwrap();
    *request.headers_mut() = headers;
    let response = connection.open(request).await.map_err(|error| {
        crate::ErrorKind::NetworkError(format!("HTTP/2 stream error: {error}"))
    })?;
    let (parts, body) = response.into_parts();
    let response = http::Response::from_parts(parts, ());
    Ok((response, body))
}

async fn drain_body(stream: &mut h2::RecvStream) {
    while let Ok(Some(Ok(_))) =
        tokio::time::timeout(STREAM_RECV_TIMEOUT, stream.data()).await
    {}
}

/// Downloads a single-stream body to `part_path`, hashing as it streams,
/// then verifies and finalises.
async fn single_stream(
    connection: &SharedH2Connection,
    uri: &Uri,
    request: &DownloadRequest,
    route: &DownloadRoute,
    destination: &Path,
    part_path: &Path,
    integrity: &Integrity,
    total_size: u64,
    policy: super::native::NativeH2Policy,
) -> crate::Result<DownloadResult> {
    let mut headers = request_headers(request, route);
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));

    let (response, mut stream) = open_stream(connection, uri, headers).await?;
    if !response.status().is_success() {
        return Err(crate::ErrorKind::HttpError {
            status: response.status().as_u16(),
            method: "GET".to_string(),
            url: fetch::sanitize_url_for_log(uri.to_string().as_str()),
        }
        .into());
    }

    let mut hashers = fetch::IntegrityHashers::new_integrity_hashers(integrity);
    let mut file = tokio::fs::File::create(part_path).await?;
    let mut downloaded = 0_u64;
    let mut slow_policy =
        super::native_slow::NativeSlowPolicy::new(0, policy.expected_speed);
    loop {
        let chunk = tokio::time::timeout(STREAM_RECV_TIMEOUT, stream.data())
            .await
            .map_err(|_| {
                crate::ErrorKind::NetworkError(
                    "HTTP/2 stream receive timed out".into(),
                )
            })?
            .transpose()
            .map_err(|error| {
                crate::ErrorKind::NetworkError(format!(
                    "HTTP/2 stream error: {error}"
                ))
            })?;
        let Some(chunk) = chunk else {
            break;
        };
        file.write_all(&chunk).await?;
        hashers.update(&chunk);
        downloaded += chunk.len() as u64;
        record_install_progress(request, downloaded, total_size).await;
        if policy.abort_if_slow
            && matches!(
				slow_policy.observe(
					downloaded,
					total_size.saturating_sub(downloaded),
				),
				super::native_slow::SlowDecision::Probe { .. }
			) {
            return Err(crate::ErrorKind::NetworkError(
                "HTTP/2 single stream stayed below expectation".to_string(),
            )
            .into());
        }
    }
    file.flush().await?;
    drop(file);
    let computed = hashers.finish(downloaded);
    record_install_stage(request).await;

    verify_and_finalize(
        part_path,
        destination,
        integrity,
        computed,
        downloaded,
        total_size,
    )
    .await?;

    Ok(DownloadResult {
        path: destination.to_path_buf(),
        url: uri.to_string(),
        source: route.source,
        size: downloaded,
        attempts: 0,
        fallback_count: 0,
    })
}

async fn record_install_stage(request: &DownloadRequest) {
    if let Some(tracking) = &request.install_tracking {
        let reporter = tracking.reporter.clone();
        let item_id = tracking.item_id.clone();
        let _ = reporter
            .record_download_stage(
                item_id,
                crate::install::DownloadItemStatus::Verifying,
            )
            .await;
    }
}

async fn record_install_progress(
    request: &DownloadRequest,
    downloaded: u64,
    total_size: u64,
) {
    if let Some(tracking) = &request.install_tracking {
        let reporter = tracking.reporter.clone();
        let item_id = tracking.item_id.clone();
        let _ = reporter
            .record_download_progress(item_id, downloaded, total_size)
            .await;
    }
}

async fn verify_and_finalize(
    part_path: &Path,
    destination: &Path,
    integrity: &Integrity,
    hashers: fetch::ComputedIntegrity,
    downloaded: u64,
    _expected_size: u64,
) -> crate::Result<()> {
    // The size check lives inside `verify_computed_integrity`: the hash is
    // authoritative whenever one is available, mirroring the legacy path.
    if let Err(error) = fetch::verify_computed_integrity(integrity, &hashers) {
        return Err(error);
    }
    if let Err(error) =
        fetch::validate_file_content(part_path, integrity.content).await
    {
        return Err(error);
    }
    if downloaded == 0 {
        return Err(crate::ErrorKind::OtherError(
            "downloaded file is empty".to_string(),
        )
        .into());
    }
    fetch::finalize_download(part_path, destination).await?;
    Ok(())
}

/// A single small file (a Minecraft asset object) to download over a shared
/// HTTP/2 connection. All items in one batch must share the same authority.
pub(crate) struct H2BatchAsset {
    /// Route-resolved URL for the asset.
    pub url: String,
    /// Destination for the object (`assets/objects/<hh>/<hash>`).
    pub destination: std::path::PathBuf,
    /// Optional legacy `resources/` copy destination.
    pub legacy_destination: Option<std::path::PathBuf>,
    /// Expected SHA-1 hash of the asset (also its file name).
    pub sha1: String,
    /// Expected size in bytes.
    pub size: u64,
}

impl Clone for H2BatchAsset {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            destination: self.destination.clone(),
            legacy_destination: self.legacy_destination.clone(),
            sha1: self.sha1.clone(),
            size: self.size,
        }
    }
}

/// Downloads a batch of small files over one shared HTTP/2 connection,
/// multiplexing up to `concurrency` streams. This is deliberately NOT one
/// connection per file: the caller groups items by authority and every item
/// opens an independent stream on that single connection. Items that cannot be
/// downloaded after internal retries are returned so the caller can retry them
/// through the regular per-file path (which performs route fallback).
/// Returned items have exhausted every batch pass, so downstream can treat
/// them as persistently failing against the chosen route.
pub(crate) async fn download_asset_batch_via_h2<F>(
    route: &DownloadRoute,
    items: Vec<H2BatchAsset>,
    concurrency: usize,
    apply_native_policy: bool,
    native_semaphore: Option<&fetch::FetchSemaphore>,
    on_completed: F,
) -> Vec<H2BatchAsset>
where
    F: FnMut(H2BatchAsset) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + 'static,
{
    if apply_native_policy
        && super::native::h2_ineligible_reason(route).is_some()
    {
        return items;
    }
    let _authority_permit = if apply_native_policy {
        match super::native_budget::acquire(route).await {
            Ok(permit) => Some(permit),
            Err(_) => return items,
        }
    } else {
        None
    };
    let _global_permit = if let Some(semaphore) = native_semaphore {
        match semaphore.0.acquire().await {
            Ok(permit) => Some(permit),
            Err(_) => return items,
        }
    } else {
        None
    };
    let connection = match connect_authority(&route.url).await {
        Ok(connection) => connection,
        Err(failure) => {
            if apply_native_policy
                && failure.should_cooldown_authority()
                && let Some(authority) = fetch::url_authority(&route.url)
            {
                fetch::record_authority_h2_failure(&authority);
            }
            if apply_native_policy && failure.is_transfer_failure() {
                super::native_breaker::record_failure(route);
            }
            return items;
        }
    };
    if apply_native_policy {
        super::native_breaker::record_success(route);
    }
    let route_authority = fetch::url_authority(&route.url);

    // Items whose URL targets a different authority cannot be multiplexed on
    // this connection; hand them straight back without wasting batch passes.
    let (mut items, mut failed): (Vec<_>, Vec<_>) = items
        .into_iter()
        .partition(|item| fetch::url_authority(&item.url) == route_authority);
    if !failed.is_empty() {
        tracing::warn!(
            items = failed.len(),
            route = %fetch::sanitize_url_for_log(&route.url),
            "Skipping {} assets whose resolved URL does not match the batch authority",
            failed.len(),
        );
    }

    let callback = Arc::new(tokio::sync::Mutex::new(on_completed));
    for pass in 0..ASSET_BATCH_RETRY_PASSES {
        if items.is_empty() {
            break;
        }
        let results = futures::stream::iter(items)
            .map(|item| {
                let connection = connection.clone();
                let callback = callback.clone();
                async move {
                    let Ok(uri) = item.url.parse::<Uri>() else {
                        let error =
                            crate::Error::from(crate::ErrorKind::InputError(
                                format!("invalid asset URL: {}", item.url),
                            ));
                        return (item, Err(error));
                    };
                    let result =
                        download_asset_item(&connection, &uri, &item).await;
                    if result.is_ok() {
                        let mut callback = callback.lock().await;
                        callback(item.clone()).await;
                    }
                    (item, result)
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;
        items = Vec::new();
        for (item, result) in results {
            match result {
                Ok(()) => {}
                Err(error) => {
                    tracing::debug!(
                        url = %fetch::sanitize_url_for_log(&item.url),
                        pass = pass + 1,
                        error = %error,
                        "Batch asset download failed"
                    );
                    if pass + 1 < ASSET_BATCH_RETRY_PASSES {
                        items.push(item);
                    } else {
                        failed.push(item);
                    }
                }
            }
        }
    }
    if !failed.is_empty() {
        tracing::warn!(
            items = failed.len(),
            retry_passes = ASSET_BATCH_RETRY_PASSES,
            source = route.source.as_str(),
            "{} assets exhausted all {ASSET_BATCH_RETRY_PASSES} batch retry passes on route {}",
            failed.len(),
            route.source.as_str(),
        );
    }
    failed
}

async fn download_asset_item(
    connection: &SharedH2Connection,
    uri: &Uri,
    item: &H2BatchAsset,
) -> crate::Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&crate::launcher_user_agent())
            .unwrap_or_else(|_| HeaderValue::from_static("Axolotl Launcher")),
    );
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));

    let (response, mut stream) = open_stream(connection, uri, headers).await?;
    if !response.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP/2 GET failed with status {}",
            response.status()
        ))
        .into());
    }

    let integrity = Integrity {
        size: Some(item.size),
        sha1: Some(item.sha1.clone()),
        ..Integrity::default()
    };
    let mut hashers =
        fetch::IntegrityHashers::new_integrity_hashers(&integrity);
    let part_path = fetch::suffixed_path(&item.destination, ".part");
    if let Some(parent) = part_path.parent() {
        crate::util::io::create_dir_all(parent).await?;
    }
    // Any failure below leaves a partial file behind; clean it up so retries
    // start from a clean slate and no orphaned `.part` files accumulate.
    let result: crate::Result<()> = async {
        let mut file = tokio::fs::File::create(&part_path).await?;
        let mut downloaded = 0_u64;
        loop {
            let chunk =
                tokio::time::timeout(STREAM_RECV_TIMEOUT, stream.data())
                    .await
                    .map_err(|_| {
                        crate::ErrorKind::NetworkError(
                            "HTTP/2 asset stream receive timed out".into(),
                        )
                    })?
                    .transpose()
                    .map_err(|error| {
                        crate::ErrorKind::NetworkError(format!(
                            "HTTP/2 asset stream error: {error}"
                        ))
                    })?;
            let Some(chunk) = chunk else {
                break;
            };
            file.write_all(&chunk).await?;
            hashers.update(&chunk);
            downloaded += chunk.len() as u64;
        }
        file.flush().await?;
        drop(file);
        if downloaded == 0 {
            return Err(crate::ErrorKind::OtherError(
                "downloaded asset is empty".to_string(),
            )
            .into());
        }
        let computed = hashers.finish(downloaded);
        fetch::verify_computed_integrity(&integrity, &computed)?;
        fetch::finalize_download(&part_path, &item.destination).await?;

        if let Some(legacy) = &item.legacy_destination {
            if let Some(parent) = legacy.parent() {
                crate::util::io::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&item.destination, legacy).await?;
        }
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&part_path).await;
    }
    result
}
