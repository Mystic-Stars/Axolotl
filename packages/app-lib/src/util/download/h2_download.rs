//! HTTP/2 multiplexed file downloads over shared per-authority connections.
//!
//! General downloads to the same authority reuse one long-lived HTTP/2
//! connection. Minecraft assets use the dedicated batch multiplexer below,
//! which can add one sibling connection only after sustained saturation.

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
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex as AsyncMutex;
use url::Url;

/// Client-side concurrency target for the batch asset downloader. All
/// concurrent streams are multiplexed over one shared HTTP/2 connection per
/// authority, so this is the number of streams, not connections.
pub(crate) const ASSET_BATCH_CONCURRENCY: usize = 512;
/// Internal retry passes for failed batch items before they are handed back
/// to the caller for the regular per-file download path.
const ASSET_BATCH_RETRY_PASSES: usize = 2;
/// Only expand a busy batch after the first connection has had time to warm
/// up. This avoids extra handshakes for the common small/low-latency batch.
const ASSET_BATCH_EXPANSION_DELAY: Duration = Duration::from_millis(500);
/// Expansion is useful only when the primary is close to the authority-wide
/// stream budget (currently 32). The remaining streams can then be assigned
/// to a separate TCP congestion domain.
const ASSET_BATCH_EXPANSION_STREAMS: usize = 24;

fn should_expand_asset_batch_connection(
    elapsed: Duration,
    primary_active_streams: usize,
) -> bool {
    elapsed >= ASSET_BATCH_EXPANSION_DELAY
        && primary_active_streams >= ASSET_BATCH_EXPANSION_STREAMS
}

/// Outcome of attempting a multiplexed download.
pub(crate) enum H2DownloadOutcome {
    /// The download completed through the multiplexed path.
    Completed(DownloadResult),
    /// The install job canceled this transfer; do not enter fallback.
    Canceled,
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
    TianpaoRedirect,
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
            Self::TianpaoRedirect => {
                "Tianpao redirected Modrinth content to the official CDN"
            }
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
    if request
        .cancellation
        .as_ref()
        .is_some_and(tokio_util::sync::CancellationToken::is_cancelled)
    {
        return H2DownloadOutcome::Canceled;
    }
    if let Some(reason) = super::native::h2_ineligible_reason(route) {
        return H2DownloadOutcome::Fallback {
            failure: H2DownloadFailure::Ineligible(reason.as_str()),
            preserve_partial: false,
        };
    }
    let connection = match connect_authority(
        route,
        true,
        policy.allow_cold_connection,
    )
    .await
    {
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

    fetch::record_install_download_started(request, route, 1, 1).await;

    // When the size is known (Modrinth metadata provides it) skip the probe
    // entirely: small files fetch the body directly, large files split into
    // range streams right away. The probe is only used when the size must be
    // discovered from the server.
    let total_size = if let Some(size) = expected_size {
        size
    } else {
        let _probe_stream_permit =
            match super::h2_stream_budget::acquire(route).await {
                Ok(permit) => permit,
                Err(_) => {
                    return H2DownloadOutcome::Fallback {
                        failure: H2DownloadFailure::Connect,
                        preserve_partial: false,
                    };
                }
            };
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

    if let Some(concurrency) = request.h2_range_concurrency {
        return super::h2_range::download(
            &connection,
            &uri,
            request,
            route,
            destination,
            part_path,
            total_size,
            concurrency,
        )
        .await;
    }
    let _stream_permit = match super::h2_stream_budget::acquire(route).await {
        Ok(permit) => permit,
        Err(_) => {
            return H2DownloadOutcome::Fallback {
                failure: H2DownloadFailure::Connect,
                preserve_partial: false,
            };
        }
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
    route: &DownloadRoute,
    reserve_native_budget: bool,
    allow_cold_connection: bool,
) -> Result<Arc<SharedH2Connection>, H2DownloadFailure> {
    let authority =
        fetch::url_authority(&route.url).ok_or(H2DownloadFailure::Http)?;
    match super::h2_pool::shared_connection(
        route,
        reserve_native_budget,
        allow_cold_connection,
    )
    .await
    {
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
        crate::ErrorKind::OtherError(message)
            if message.contains("Tianpao redirected Modrinth content") =>
        {
            H2DownloadFailure::TianpaoRedirect
        }
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

pub(crate) fn request_headers(
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

pub(crate) async fn open_stream(
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
    loop {
        let chunk =
            match super::h2_receive::receive_chunk(stream, "probe").await {
                Ok(Some(chunk)) => chunk,
                Ok(None) | Err(_) => break,
            };
        if super::h2_receive::release_capacity(stream, chunk.len()).is_err() {
            break;
        }
    }
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
        let current_url = Url::parse(uri.to_string().as_str()).ok();
        if response.status().is_redirection()
            && current_url.as_ref().is_some_and(|current_url| {
                crate::util::download::modrinth_redirect::is_tianpao_official_redirect(
                    current_url,
                    response
                        .headers()
                        .get(http::header::LOCATION)
                        .and_then(|location| location.to_str().ok()),
                )
            })
        {
            return Err(crate::ErrorKind::OtherError(
                "Tianpao redirected Modrinth content to the official CDN".to_string(),
            )
            .into());
        }
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
    let activity = super::h2_receive::H2TransferActivity::begin();
    let mut progress_gate = super::h2_receive::H2ProgressGate::new(total_size);
    let mut slow_policy =
        super::native_slow::NativeSlowPolicy::new(0, policy.expected_speed);
    loop {
        let chunk =
            super::h2_receive::receive_chunk(&mut stream, "file").await?;
        let Some(chunk) = chunk else {
            break;
        };
        file.write_all(&chunk).await?;
        hashers.update(&chunk);
        downloaded += chunk.len() as u64;
        activity.record_bytes(chunk.len());
        super::h2_receive::release_capacity(&mut stream, chunk.len())?;
        if progress_gate.should_report(downloaded, total_size) {
            record_install_progress(request, downloaded, total_size).await;
        }
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
        attempts: 1,
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

pub(crate) async fn record_install_progress(
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
    /// Canonical asset URL used to rebuild routes after batch failure.
    pub original_url: String,
    /// Route-resolved URL for the asset.
    pub url: String,
    /// Destination for the object (`assets/objects/<hh>/<hash>`).
    pub destination: std::path::PathBuf,
    /// Legacy `resources/` copies to create after the object is committed.
    /// Several logical asset names may point to this one physical object.
    pub legacy_destinations: Vec<std::path::PathBuf>,
    /// Expected SHA-1 hash of the asset (also its file name).
    pub sha1: String,
    /// Expected size in bytes.
    pub size: u64,
    /// Number of logical index entries represented by this physical object.
    /// Progress remains index-based even when duplicate objects are coalesced.
    pub logical_items: u32,
}

/// Completion state for one physical asset object. A legacy-resource copy is
/// deliberately separate from fetching and committing the content-addressed
/// object: a local path error must not cause another GET for an object that is
/// already valid on disk.
enum AssetBatchItemOutcome {
    Completed {
        downloaded: bool,
    },
    LocalCopyFailed {
        downloaded: bool,
        error: crate::Error,
    },
    LocalObjectFailed {
        error: crate::Error,
    },
}

impl Clone for H2BatchAsset {
    fn clone(&self) -> Self {
        Self {
            original_url: self.original_url.clone(),
            url: self.url.clone(),
            destination: self.destination.clone(),
            legacy_destinations: self.legacy_destinations.clone(),
            sha1: self.sha1.clone(),
            size: self.size,
            logical_items: self.logical_items,
        }
    }
}

/// Selects the least busy connection in an asset batch. A sibling connection
/// is created once, at most, when the initial connection remains saturated
/// beyond the warm-up period; this keeps the normal case at one TCP/TLS
/// connection while giving a degraded long batch an independent recovery and
/// congestion domain.
struct AssetBatchConnectionGroup {
    primary: Arc<SharedH2Connection>,
    sibling: AsyncMutex<Option<Arc<SharedH2Connection>>>,
    expansion_attempted: AtomicBool,
    route: DownloadRoute,
    reserve_native_budget: bool,
    started: Instant,
}

impl AssetBatchConnectionGroup {
    fn new(
        primary: Arc<SharedH2Connection>,
        route: &DownloadRoute,
        reserve_native_budget: bool,
    ) -> Self {
        Self {
            primary,
            sibling: AsyncMutex::new(None),
            expansion_attempted: AtomicBool::new(false),
            route: route.clone(),
            reserve_native_budget,
            started: Instant::now(),
        }
    }

    fn should_expand(&self) -> bool {
        should_expand_asset_batch_connection(
            self.started.elapsed(),
            self.primary.active_streams(),
        )
    }

    async fn connection(&self, rescue: bool) -> Arc<SharedH2Connection> {
        if (rescue || self.should_expand())
            && self
                .expansion_attempted
                .compare_exchange(
                    false,
                    true,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
        {
            match super::h2_pool::shared_batch_connection(
                &self.route,
                self.reserve_native_budget,
            )
            .await
            {
                Ok(connection) => {
                    tracing::info!(
                        authority = %fetch::url_authority(&self.route.url).unwrap_or_default(),
                        primary_active_streams = self.primary.active_streams(),
                        "Expanded saturated HTTP/2 asset batch with a sibling connection"
                    );
                    *self.sibling.lock().await = Some(connection);
                }
                Err(error) => {
                    tracing::debug!(
                        authority = %fetch::url_authority(&self.route.url).unwrap_or_default(),
                        error = %error,
                        "Could not expand HTTP/2 asset batch; retaining primary connection"
                    );
                }
            }
        }

        let sibling = self.sibling.lock().await.clone();
        match sibling {
            Some(sibling) if rescue && !sibling.is_dead() => sibling,
            Some(sibling)
                if !sibling.is_dead()
                    && sibling.active_streams()
                        < self.primary.active_streams() =>
            {
                sibling
            }
            _ => Arc::clone(&self.primary),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_batch_expansion_requires_sustained_saturation() {
        assert!(!should_expand_asset_batch_connection(
            ASSET_BATCH_EXPANSION_DELAY,
            ASSET_BATCH_EXPANSION_STREAMS - 1,
        ));
        assert!(!should_expand_asset_batch_connection(
            ASSET_BATCH_EXPANSION_DELAY - Duration::from_millis(1),
            ASSET_BATCH_EXPANSION_STREAMS,
        ));
        assert!(should_expand_asset_batch_connection(
            ASSET_BATCH_EXPANSION_DELAY,
            ASSET_BATCH_EXPANSION_STREAMS,
        ));
    }

    #[tokio::test]
    async fn committed_asset_recovers_a_legacy_copy_without_redownloading() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("object");
        tokio::fs::write(&destination, b"already committed")
            .await
            .unwrap();
        let blocked_parent = temp.path().join("legacy-parent");
        tokio::fs::write(&blocked_parent, b"not a directory")
            .await
            .unwrap();
        let legacy = blocked_parent.join("resource");
        let item = H2BatchAsset {
            original_url: "https://resources.download.minecraft.net/aa/object"
                .into(),
            url: "https://resources.download.minecraft.net/aa/object".into(),
            destination: destination.clone(),
            legacy_destinations: vec![legacy.clone()],
            sha1: "unused-by-copy-test".into(),
            size: 17,
            logical_items: 1,
        };

        assert!(copy_asset_legacy_destinations(&item).await.is_err());
        assert_eq!(
            tokio::fs::read(&destination).await.unwrap(),
            b"already committed"
        );

        tokio::fs::remove_file(&blocked_parent).await.unwrap();
        tokio::fs::create_dir(&blocked_parent).await.unwrap();
        copy_asset_legacy_destinations(&item).await.unwrap();

        assert_eq!(
            tokio::fs::read(&legacy).await.unwrap(),
            b"already committed"
        );
    }

    #[tokio::test]
    async fn blocked_object_parent_is_detected_before_asset_get() {
        let temp = tempfile::tempdir().unwrap();
        let blocked_parent = temp.path().join("object-parent");
        tokio::fs::write(&blocked_parent, b"not a directory")
            .await
            .unwrap();
        let destination = blocked_parent.join("object");
        let part_path = fetch::suffixed_path(&destination, ".part");

        assert!(prepare_asset_part_path(&destination).await.is_err());
        assert!(tokio::fs::metadata(&part_path).await.is_err());
    }
}

/// Downloads a batch of small files over a shared HTTP/2 connection group,
/// multiplexing up to `concurrency` streams. The group begins with one
/// connection and may add one sibling only for a sustained saturated batch;
/// it never creates one connection per file. Items that cannot be downloaded
/// after internal retries are returned so the caller can retry them through
/// the regular per-file path (which performs route fallback).
/// Returned items have exhausted every batch pass, so downstream can treat
/// them as persistently failing against the chosen route.
pub(crate) async fn download_asset_batch_via_h2<F>(
    route: &DownloadRoute,
    items: Vec<H2BatchAsset>,
    concurrency: usize,
    apply_native_policy: bool,
    native_semaphore: Option<&fetch::FetchSemaphore>,
    on_completed: F,
) -> crate::Result<Vec<H2BatchAsset>>
where
    F: Fn(H2BatchAsset) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync
        + 'static,
{
    if apply_native_policy
        && super::native::h2_ineligible_reason(route).is_some()
    {
        return Ok(items);
    }
    // Local object I/O failures are deterministic per destination; remember the
    // first one and keep draining the batch so siblings already in flight or
    // still queued are not abandoned, then surface the error to the caller.
    let mut local_object_error: Option<crate::Error> = None;
    let _global_permit = if let Some(semaphore) = native_semaphore {
        match semaphore.0.acquire().await {
            Ok(permit) => Some(permit),
            Err(_) => return Ok(items),
        }
    } else {
        None
    };
    let connection =
        match connect_authority(route, apply_native_policy, true).await {
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
                    fetch::record_route_health_failure(
                        route,
                        fetch::ResourceClass::MinecraftAsset,
                        None,
                    );
                }
                return Ok(items);
            }
        };
    let connections = Arc::new(AssetBatchConnectionGroup::new(
        connection,
        route,
        apply_native_policy,
    ));
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

    let batch_started = std::time::Instant::now();
    let mut completed_bytes = 0_u64;
    let mut network_failures = 0_u32;
    let callback = Arc::new(on_completed);
    for pass in 0..ASSET_BATCH_RETRY_PASSES {
        if items.is_empty() {
            break;
        }
        let results = futures::stream::iter(items)
            .map(|item| {
                let connections = Arc::clone(&connections);
                let callback = callback.clone();
                async move {
                    let Ok(uri) = item.url.parse::<Uri>() else {
                        let error =
                            crate::Error::from(crate::ErrorKind::InputError(
                                format!("invalid asset URL: {}", item.url),
                            ));
                        return (item, Err(error));
                    };
                    let result = download_asset_item(
                        &connections,
                        &uri,
                        &item,
                        route,
                        apply_native_policy,
                        pass > 0,
                    )
                    .await;
                    if matches!(
                        &result,
                        Ok(AssetBatchItemOutcome::Completed { .. })
                    ) {
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
                Ok(AssetBatchItemOutcome::Completed { downloaded }) => {
                    if downloaded {
                        completed_bytes =
                            completed_bytes.saturating_add(item.size);
                    }
                }
                Ok(AssetBatchItemOutcome::LocalCopyFailed {
                    downloaded,
                    error,
                }) => {
                    if downloaded {
                        completed_bytes =
                            completed_bytes.saturating_add(item.size);
                    }
                    tracing::warn!(
                        url = %fetch::sanitize_url_for_log(&item.url),
                        destination = %item.destination.display(),
                        error = %error,
                        "Asset object is committed, but copying its legacy resource failed; retrying locally without another download"
                    );
                    // The ordinary fallback path sees the valid object and
                    // performs only the outstanding local copy. Do not spend
                    // another network retry pass on a local filesystem error.
                    failed.push(item);
                }
                Ok(AssetBatchItemOutcome::LocalObjectFailed { error }) => {
                    tracing::warn!(
                        url = %fetch::sanitize_url_for_log(&item.url),
                        destination = %item.destination.display(),
                        error = %error,
                        "Asset object failed local I/O; continuing to drain the batch"
                    );
                    if local_object_error.is_none() {
                        local_object_error = Some(error);
                    }
                }
                Err(error) => {
                    network_failures = network_failures.saturating_add(1);
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
    if apply_native_policy {
        if completed_bytes > 0 {
            fetch::record_route_transfer_success(
                route,
                fetch::ResourceClass::MinecraftAsset,
                completed_bytes,
                batch_started.elapsed(),
            );
        } else if network_failures > 0 {
            super::native_breaker::record_failure(route);
            fetch::record_route_health_failure(
                route,
                fetch::ResourceClass::MinecraftAsset,
                None,
            );
        }
    }
    if let Some(error) = local_object_error {
        return Err(error);
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
    Ok(failed)
}

async fn prepare_asset_part_path(
    destination: &Path,
) -> crate::Result<std::path::PathBuf> {
    let part_path = fetch::suffixed_path(destination, ".part");
    if let Some(parent) = part_path.parent() {
        crate::util::io::create_dir_all(parent).await?;
    }
    Ok(part_path)
}

async fn download_asset_item(
    connections: &AssetBatchConnectionGroup,
    uri: &Uri,
    item: &H2BatchAsset,
    route: &DownloadRoute,
    apply_native_policy: bool,
    rescue: bool,
) -> crate::Result<AssetBatchItemOutcome> {
    let integrity = Integrity {
        size: Some(item.size),
        sha1: Some(item.sha1.clone()),
        ..Integrity::default()
    };
    let destination_lock = fetch::destination_download_lock(&item.destination);
    let _destination_guard = destination_lock.lock().await;
    // A different downloader may have committed the object while this item
    // waited for the destination lock. Reuse it instead of opening another
    // stream, which also prevents cross-engine `.part`/rename races.
    if fetch::verify_file(&item.destination, &integrity)
        .await
        .is_ok()
    {
        return Ok(match copy_asset_legacy_destinations(item).await {
            Ok(()) => AssetBatchItemOutcome::Completed { downloaded: false },
            Err(error) => AssetBatchItemOutcome::LocalCopyFailed {
                downloaded: false,
                error,
            },
        });
    }
    let part_path = match prepare_asset_part_path(&item.destination).await {
        Ok(part_path) => part_path,
        Err(error) => {
            return Ok(AssetBatchItemOutcome::LocalObjectFailed { error });
        }
    };
    let _stream_permit = if apply_native_policy {
        Some(super::h2_stream_budget::acquire(route).await?)
    } else {
        None
    };
    let connection = connections.connection(rescue).await;
    let _connection_stream = connection.track_stream();
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&crate::launcher_user_agent())
            .unwrap_or_else(|_| HeaderValue::from_static("Axolotl Launcher")),
    );
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));

    let (response, mut stream) = open_stream(&connection, uri, headers).await?;
    if !response.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP/2 GET failed with status {}",
            response.status()
        ))
        .into());
    }

    let mut hashers =
        fetch::IntegrityHashers::new_integrity_hashers(&integrity);
    // Any failure below leaves a partial file behind; clean it up so retries
    // start from a clean slate and no orphaned `.part` files accumulate.
    let result: crate::Result<AssetBatchItemOutcome> = async {
        let mut file = match tokio::fs::File::create(&part_path).await {
            Ok(file) => file,
            Err(error) => {
                return Ok(AssetBatchItemOutcome::LocalObjectFailed {
                    error: error.into(),
                });
            }
        };
        let mut downloaded = 0_u64;
        let activity = super::h2_receive::H2TransferActivity::begin();
        loop {
            let chunk =
                super::h2_receive::receive_chunk(&mut stream, "asset").await?;
            let Some(chunk) = chunk else {
                break;
            };
            if let Err(error) = file.write_all(&chunk).await {
                return Ok(AssetBatchItemOutcome::LocalObjectFailed {
                    error: error.into(),
                });
            }
            hashers.update(&chunk);
            downloaded += chunk.len() as u64;
            activity.record_bytes(chunk.len());
            super::h2_receive::release_capacity(&mut stream, chunk.len())?;
        }
        if let Err(error) = file.flush().await {
            return Ok(AssetBatchItemOutcome::LocalObjectFailed {
                error: error.into(),
            });
        }
        drop(file);
        if downloaded == 0 {
            return Err(crate::ErrorKind::OtherError(
                "downloaded asset is empty".to_string(),
            )
            .into());
        }
        let computed = hashers.finish(downloaded);
        fetch::verify_computed_integrity(&integrity, &computed)?;
        if let Err(error) =
            fetch::finalize_download(&part_path, &item.destination).await
        {
            return Ok(AssetBatchItemOutcome::LocalObjectFailed { error });
        }

        Ok(match copy_asset_legacy_destinations(item).await {
            Ok(()) => AssetBatchItemOutcome::Completed { downloaded: true },
            Err(error) => AssetBatchItemOutcome::LocalCopyFailed {
                downloaded: true,
                error,
            },
        })
    }
    .await;
    if result.is_err()
        || matches!(
            &result,
            Ok(AssetBatchItemOutcome::LocalObjectFailed { .. })
        )
    {
        let _ = tokio::fs::remove_file(&part_path).await;
    }
    result
}

async fn copy_asset_legacy_destinations(
    item: &H2BatchAsset,
) -> crate::Result<()> {
    for legacy in &item.legacy_destinations {
        if let Some(state) = crate::State::get_if_initialized() {
            fetch::copy(&item.destination, legacy, &state.io_semaphore).await?;
        } else {
            if let Some(parent) = legacy.parent() {
                crate::util::io::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&item.destination, legacy).await?;
        }
    }
    Ok(())
}
