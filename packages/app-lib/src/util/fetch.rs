//! Functions for fetching information from the Internet
use super::io::{self, IOError};
use crate::event::LoadingBarId;
use crate::event::emit::emit_loading;
use crate::{ErrorKind, LabrinthError};
use bytes::Bytes;
use chrono::{DateTime, TimeDelta, Utc};
use eyre::{Context, eyre};
use parking_lot::Mutex;
use rand::Rng;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::ffi::OsStr;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::LazyLock;
use std::time::{self};
use tokio::sync::Semaphore;
use tokio::{fs::File, io::AsyncReadExt, io::AsyncWriteExt};

pub const DOWNLOAD_META_HEADER: &str = "modrinth-download-meta";

const BMCLAPI_BASE_URL: &str = "https://bmclapi2.bangbang93.com";
const MCIM_BASE_URL: &str = "https://mod.mcimirror.top";

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DownloadMirrorSettings {
    pub minecraft: bool,
    pub modrinth: bool,
    pub curseforge: bool,
}

impl DownloadMirrorSettings {
    pub fn current() -> Self {
        crate::State::get_if_initialized()
            .map(|state| Self {
                minecraft: state.use_minecraft_mirror(),
                modrinth: state.use_modrinth_mirror(),
                curseforge: state.use_curseforge_mirror(),
            })
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedDownloadUrl {
    pub url: String,
    pub is_mirror: bool,
}

fn replace_url_base(url: &str, source: &str, target: &str) -> Option<String> {
    if url == source {
        return Some(target.to_string());
    }

    url.strip_prefix(source)
        .filter(|suffix| suffix.starts_with('/'))
        .map(|suffix| format!("{target}{suffix}"))
}

pub(crate) fn resolve_download_url(
    url: &str,
    mirrors: DownloadMirrorSettings,
) -> ResolvedDownloadUrl {
    let mappings = [
        (
            mirrors.minecraft,
            "https://resources.download.minecraft.net",
            concat!("https://bmclapi2.bangbang93.com", "/assets"),
        ),
        (
            mirrors.minecraft,
            "https://libraries.minecraft.net",
            concat!("https://bmclapi2.bangbang93.com", "/maven"),
        ),
        (
            mirrors.minecraft,
            "https://maven.minecraftforge.net",
            concat!("https://bmclapi2.bangbang93.com", "/maven"),
        ),
        (
            mirrors.minecraft,
            "https://files.minecraftforge.net/maven",
            concat!("https://bmclapi2.bangbang93.com", "/maven"),
        ),
        (
            mirrors.minecraft,
            "https://maven.fabricmc.net",
            concat!("https://bmclapi2.bangbang93.com", "/maven"),
        ),
        (
            mirrors.minecraft,
            "https://maven.neoforged.net/releases",
            concat!("https://bmclapi2.bangbang93.com", "/maven"),
        ),
        (
            mirrors.minecraft,
            "https://launcher-meta.modrinth.com/maven",
            concat!("https://bmclapi2.bangbang93.com", "/maven"),
        ),
        (
            mirrors.minecraft,
            "https://meta.fabricmc.net",
            concat!("https://bmclapi2.bangbang93.com", "/fabric-meta"),
        ),
        (
            mirrors.minecraft,
            "https://piston-meta.mojang.com",
            BMCLAPI_BASE_URL,
        ),
        (
            mirrors.minecraft,
            "https://launchermeta.mojang.com",
            BMCLAPI_BASE_URL,
        ),
        (
            mirrors.minecraft,
            "https://launcher.mojang.com",
            BMCLAPI_BASE_URL,
        ),
        (
            mirrors.minecraft,
            "https://piston-data.mojang.com",
            BMCLAPI_BASE_URL,
        ),
        (
            mirrors.modrinth,
            "https://api.modrinth.com",
            concat!("https://mod.mcimirror.top", "/modrinth"),
        ),
        (mirrors.modrinth, "https://cdn.modrinth.com", MCIM_BASE_URL),
        (
            mirrors.curseforge,
            "https://api.curseforge.com",
            concat!("https://mod.mcimirror.top", "/curseforge"),
        ),
        (
            mirrors.curseforge,
            "https://edge.forgecdn.net",
            MCIM_BASE_URL,
        ),
    ];

    for (enabled, source, target) in mappings {
        if enabled && let Some(url) = replace_url_base(url, source, target) {
            return ResolvedDownloadUrl {
                url,
                is_mirror: true,
            };
        }
    }

    ResolvedDownloadUrl {
        url: url.to_string(),
        is_mirror: false,
    }
}

fn resolve_download_routes(
    url: &str,
    mirrors: DownloadMirrorSettings,
) -> Vec<ResolvedDownloadUrl> {
    let resolved = resolve_download_url(url, mirrors);
    if resolved.is_mirror {
        vec![
            resolved,
            ResolvedDownloadUrl {
                url: url.to_string(),
                is_mirror: false,
            },
        ]
    } else {
        vec![resolved]
    }
}

#[derive(Debug, derive_more::Display, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[display(rename_all = "snake_case")]
pub enum DownloadReason {
    Standalone,
    Dependency,
    Modpack,
    Update,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadMeta {
    pub reason: DownloadReason,
    pub game_version: String,
    pub loader: String,
    pub dependent_on: Option<String>,
}

impl DownloadMeta {
    pub fn to_header_value(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct IoSemaphore(pub Semaphore);
#[derive(Debug)]
pub struct FetchSemaphore(pub Semaphore);

struct FetchFence {
    inner: Mutex<HashMap<&'static str, FenceInner>>,
}

impl FetchFence {
    pub fn is_blocked(&self, key: &'static str) -> bool {
        self.inner
            .lock()
            .entry(key)
            .or_insert_with(FenceInner::new)
            .is_blocked()
    }

    pub fn record_ok(&self, key: &'static str) {
        self.inner
            .lock()
            .entry(key)
            .or_insert_with(FenceInner::new)
            .record_ok()
    }

    pub fn record_fail(&self, key: &'static str) {
        self.inner
            .lock()
            .entry(key)
            .or_insert_with(FenceInner::new)
            .record_fail()
    }

    pub fn latest_block_minutes(&self) -> u32 {
        let now = Utc::now();

        self.inner
            .lock()
            .values()
            .filter_map(|fence| fence.block_until)
            .filter(|until| *until > now)
            .max()
            .map(|until| {
                let seconds = until.signed_duration_since(now).num_seconds();
                (seconds.max(0) as u32).div_ceil(60).max(1)
            })
            .unwrap_or(1)
    }
}

struct FenceInner {
    failures: VecDeque<DateTime<Utc>>,
    block_until: Option<DateTime<Utc>>,
    block_factor: i32,
}

impl FenceInner {
    const FAILURE_WINDOW: TimeDelta = TimeDelta::minutes(3);
    const FAILURE_THRESHOLD: usize = 4;
    const BLOCK_DURATION_MIN_BASE: TimeDelta = TimeDelta::minutes(2);
    const BLOCK_DURATION_MAX_BASE: TimeDelta = TimeDelta::minutes(5);
    const BLOCK_DURATION_MAX_FACTOR: i32 = 3;

    pub fn new() -> Self {
        Self {
            failures: VecDeque::new(),
            block_until: None,
            block_factor: 0,
        }
    }

    pub fn is_blocked(&mut self) -> bool {
        if let Some(until) = self.block_until {
            if until > Utc::now() {
                return true;
            } else {
                self.block_until = None;
            }
        }

        false
    }

    pub fn record_ok(&mut self) {
        self.prune(Utc::now());
    }

    pub fn record_fail(&mut self) {
        self.prune(Utc::now());
        self.failures.push_back(Utc::now());

        if self.failures.len() >= Self::FAILURE_THRESHOLD {
            self.trigger_block();
        }
    }

    /// Blocks further requests for a random duration between the min and max base durations, scaled by a factor
    /// of how many blocks have been triggered in this session.
    ///
    /// As such, for the first block, the duration will be between 2 and 5 minutes.
    /// - For the second block, between 4 and 10 minutes.
    /// - For the third block and any further blocks, between 6 and 15 minutes.
    fn trigger_block(&mut self) {
        self.block_factor =
            i32::min(self.block_factor + 1, Self::BLOCK_DURATION_MAX_FACTOR);

        let min = Self::BLOCK_DURATION_MIN_BASE
            .checked_mul(self.block_factor)
            .unwrap_or(Self::BLOCK_DURATION_MIN_BASE);
        let max = Self::BLOCK_DURATION_MAX_BASE
            .checked_mul(self.block_factor)
            .unwrap_or(Self::BLOCK_DURATION_MAX_BASE);

        let delta_seconds = (max - min).as_seconds_f64()
            * rand::thread_rng().gen_range(0.0..=1.0);
        let duration =
            min + TimeDelta::milliseconds((delta_seconds * 1000.0) as i64);

        self.block_until = Some(Utc::now() + duration);
    }

    /// Removes all failure points older than the failure window
    fn prune(&mut self, now: DateTime<Utc>) {
        let cutoff = now - Self::FAILURE_WINDOW;

        while let Some(&front) = self.failures.front() {
            if front < cutoff {
                self.failures.pop_front();
            } else {
                break;
            }
        }
    }
}

static GLOBAL_FETCH_FENCE: LazyLock<FetchFence> =
    LazyLock::new(|| FetchFence {
        inner: Mutex::new(HashMap::new()),
    });

fn reqwest_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(time::Duration::from_secs(15))
        .read_timeout(time::Duration::from_secs(30))
        .tcp_keepalive(Some(time::Duration::from_secs(10)))
        .user_agent(crate::launcher_user_agent())
}

pub static INSECURE_REQWEST_CLIENT: LazyLock<reqwest::Client> =
    LazyLock::new(|| {
        reqwest_client_builder()
            .build()
            .expect("client configuration should be valid")
    });

pub static REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest_client_builder()
        .https_only(true)
        .build()
        .expect("client configuration should be valid")
});

const FETCH_ATTEMPTS: usize = 4;
const FETCH_RETRY_DELAY: time::Duration = time::Duration::from_secs(1);

fn fetch_retry_delay(attempt: usize) -> time::Duration {
    let multiplier = 1_u32 << (attempt - 1);
    FETCH_RETRY_DELAY.saturating_mul(multiplier)
}

pub type FetchProgressFn<'a> = dyn FnMut(
        u64,
        u64,
    ) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + 'a>>
    + Send
    + 'a;

#[tracing::instrument(skip(semaphore))]
pub async fn fetch(
    url: &str,
    sha1: Option<&str>,
    download_meta: Option<&DownloadMeta>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
) -> crate::Result<Bytes> {
    fetch_advanced(
        Method::GET,
        url,
        sha1,
        None,
        None,
        download_meta,
        None,
        uri_path,
        semaphore,
        exec,
    )
    .await
}

#[tracing::instrument(skip(semaphore))]
pub async fn fetch_with_client(
    url: &str,
    sha1: Option<&str>,
    download_meta: Option<&DownloadMeta>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    client: &reqwest::Client,
) -> crate::Result<Bytes> {
    fetch_advanced_with_client(
        Method::GET,
        url,
        sha1,
        None,
        None,
        download_meta,
        None,
        uri_path,
        semaphore,
        exec,
        client,
    )
    .await
}

#[tracing::instrument(skip(semaphore, progress))]
pub async fn fetch_with_client_progress(
    url: &str,
    sha1: Option<&str>,
    download_meta: Option<&DownloadMeta>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    client: &reqwest::Client,
    progress: Option<&mut FetchProgressFn<'_>>,
) -> crate::Result<Bytes> {
    fetch_advanced_with_client_and_progress(
        Method::GET,
        url,
        sha1,
        None,
        None,
        download_meta,
        None,
        uri_path,
        semaphore,
        exec,
        client,
        progress,
        None,
    )
    .await
}

#[tracing::instrument(skip(json_body, semaphore))]
pub async fn fetch_json<T>(
    method: Method,
    url: &str,
    sha1: Option<&str>,
    json_body: Option<serde_json::Value>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
) -> crate::Result<T>
where
    T: DeserializeOwned,
{
    let validate_json = |bytes: &Bytes| -> crate::Result<()> {
        serde_json::from_slice::<T>(bytes)
            .map(|_| ())
            .map_err(Into::into)
    };
    let result = fetch_advanced_with_client_and_progress(
        method,
        url,
        sha1,
        json_body,
        None,
        None,
        None,
        uri_path,
        semaphore,
        exec,
        &INSECURE_REQWEST_CLIENT,
        None,
        Some(&validate_json),
    )
    .await?;
    Ok(serde_json::from_slice(&result)?)
}

/// Downloads a file with retry and checksum functionality, and a specific
/// [`reqwest::Client`].
#[tracing::instrument(skip(json_body, semaphore))]
#[allow(clippy::too_many_arguments)]
pub async fn fetch_advanced(
    method: Method,
    url: &str,
    sha1: Option<&str>,
    json_body: Option<serde_json::Value>,
    header: Option<(&str, &str)>,
    download_meta: Option<&DownloadMeta>,
    loading_bar: Option<(&LoadingBarId, f64)>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
) -> crate::Result<Bytes> {
    fetch_advanced_with_client(
        method,
        url,
        sha1,
        json_body,
        header,
        download_meta,
        loading_bar,
        uri_path,
        semaphore,
        exec,
        &INSECURE_REQWEST_CLIENT,
    )
    .await
}

#[tracing::instrument(skip(json_body, semaphore, progress))]
#[allow(clippy::too_many_arguments)]
pub async fn fetch_advanced_with_progress(
    method: Method,
    url: &str,
    sha1: Option<&str>,
    json_body: Option<serde_json::Value>,
    header: Option<(&str, &str)>,
    download_meta: Option<&DownloadMeta>,
    loading_bar: Option<(&LoadingBarId, f64)>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    progress: Option<&mut FetchProgressFn<'_>>,
) -> crate::Result<Bytes> {
    fetch_advanced_with_client_and_progress(
        method,
        url,
        sha1,
        json_body,
        header,
        download_meta,
        loading_bar,
        uri_path,
        semaphore,
        exec,
        &INSECURE_REQWEST_CLIENT,
        progress,
        None,
    )
    .await
}

/// Downloads a file with retry and checksum functionality
#[tracing::instrument(skip(json_body, semaphore))]
#[allow(clippy::too_many_arguments)]
pub async fn fetch_advanced_with_client(
    method: Method,
    url: &str,
    sha1: Option<&str>,
    json_body: Option<serde_json::Value>,
    header: Option<(&str, &str)>,
    download_meta: Option<&DownloadMeta>,
    loading_bar: Option<(&LoadingBarId, f64)>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    client: &reqwest::Client,
) -> crate::Result<Bytes> {
    fetch_advanced_with_client_and_progress(
        method,
        url,
        sha1,
        json_body,
        header,
        download_meta,
        loading_bar,
        uri_path,
        semaphore,
        exec,
        client,
        None,
        None,
    )
    .await
}

#[tracing::instrument(skip(
    json_body,
    semaphore,
    client,
    progress,
    response_validator
))]
#[allow(clippy::too_many_arguments)]
async fn fetch_advanced_with_client_and_progress(
    method: Method,
    url: &str,
    sha1: Option<&str>,
    json_body: Option<serde_json::Value>,
    header: Option<(&str, &str)>,
    download_meta: Option<&DownloadMeta>,
    loading_bar: Option<(&LoadingBarId, f64)>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    client: &reqwest::Client,
    mut progress: Option<&mut FetchProgressFn<'_>>,
    response_validator: Option<
        &(dyn Fn(&Bytes) -> crate::Result<()> + Send + Sync),
    >,
) -> crate::Result<Bytes> {
    let _permit = semaphore.0.acquire().await?;

    let request_routes =
        resolve_download_routes(url, DownloadMirrorSettings::current());
    let is_api_url = url.starts_with(env!("MODRINTH_API_URL"))
        || url.starts_with(env!("MODRINTH_API_URL_V3"));
    let creds = if header
        .as_ref()
        .is_none_or(|x| !x.0.eq_ignore_ascii_case("authorization"))
        && (url.starts_with("https://cdn.modrinth.com") || is_api_url)
    {
        crate::state::ModrinthCredentials::get_active(exec).await?
    } else {
        None
    };

    for (route_index, route) in request_routes.iter().enumerate() {
        let request_url = &route.url;
        let is_mirror = route.is_mirror;
        let has_next_route = route_index + 1 < request_routes.len();
        let fence_key = if is_api_url && !is_mirror {
            uri_path
        } else {
            None
        };
        let download_meta_header = (!is_mirror)
            .then(|| {
                download_meta.map(|m| {
                    (DOWNLOAD_META_HEADER.to_string(), m.to_header_value())
                })
            })
            .flatten();

        for attempt in 1..=(FETCH_ATTEMPTS + 1) {
            if let Some(fence_key) = fence_key
                && GLOBAL_FETCH_FENCE.is_blocked(fence_key)
            {
                return Err(ErrorKind::ApiIsDownError(
                    GLOBAL_FETCH_FENCE.latest_block_minutes(),
                )
                .into());
            }

            let mut req = client.request(method.clone(), request_url);

            if let Some(body) = json_body.clone() {
                req = req.json(&body);
            }

            if let Some(header) = header
                && (!is_mirror
                    || !header.0.eq_ignore_ascii_case("authorization"))
            {
                req = req.header(header.0, header.1);
            }

            if !is_mirror && let Some(ref creds) = creds {
                req = req.header("Authorization", &creds.session);
            }

            if let Some((name, value)) = &download_meta_header {
                tracing::debug!("Sending download analytics: {value}");
                req = req.header(name.as_str(), value.as_str());
            }

            let result = req.send().await;
            match result {
                Ok(resp) => {
                    if resp.status().is_server_error() {
                        if let Some(fence_key) = fence_key {
                            GLOBAL_FETCH_FENCE.record_fail(fence_key);
                        }

                        if attempt <= FETCH_ATTEMPTS {
                            tokio::time::sleep(fetch_retry_delay(attempt))
                                .await;
                            continue;
                        }
                    }

                    if resp.status().is_client_error()
                        || resp.status().is_server_error()
                    {
                        let status = resp.status();
                        let backup_error =
                            resp.error_for_status_ref().unwrap_err();
                        let route_error: crate::Error = if let Ok(mut error) =
                            resp.json::<LabrinthError>().await
                        {
                            error.status = Some(status.as_u16());
                            error.method = Some(method.as_str().to_string());
                            error.url = Some(request_url.to_string());
                            error.route = uri_path.map(str::to_string);
                            ErrorKind::LabrinthError(error).into()
                        } else {
                            backup_error.into()
                        };
                        if has_next_route {
                            tracing::warn!(
                                url = request_url,
                                status = status.as_u16(),
                                error = %route_error,
                                "Mirror request failed; falling back to official source"
                            );
                            break;
                        }
                        return Err(route_error);
                    }

                    let bytes: eyre::Result<Bytes> = if loading_bar.is_some()
                        || progress.is_some()
                    {
                        let length = resp.content_length();
                        if let Some(total_size) = length {
                            use futures::StreamExt;
                            let mut stream = resp.bytes_stream();

                            async {
                                let mut bytes = Vec::new();
                                let mut downloaded = 0_u64;

                                while let Some(item) = stream.next().await {
                                    let chunk = item.wrap_err_with(|| {
                                        eyre!(
                                            "failed to read response body from {request_url}"
                                        )
                                    })?;

                                    downloaded += chunk.len() as u64;
                                    bytes.extend_from_slice(&chunk);

                                    if let Some((bar, total)) = &loading_bar {
                                        emit_loading(
                                            bar,
                                            (chunk.len() as f64
                                                / total_size as f64)
                                                * total,
                                            None,
                                        )?;
                                    }

                                    if let Some(progress) = progress.as_mut() {
                                        progress(downloaded, total_size).await?;
                                    }
                                }

                                Ok(Bytes::from(bytes))
                            }
                            .await
                        } else {
                            resp.bytes().await.wrap_err_with(|| {
                                eyre!(
                                    "failed to read response body from {request_url}"
                                )
                            })
                        }
                    } else {
                        resp.bytes().await.wrap_err_with(|| {
                            eyre!(
                                "failed to read response body from {request_url}"
                            )
                        })
                    };

                    if let Ok(bytes) = bytes {
                        if let Some(sha1) = sha1 {
                            let hash = sha1_async(bytes.clone()).await?;
                            if &*hash != sha1 {
                                if attempt <= FETCH_ATTEMPTS {
                                    tokio::time::sleep(fetch_retry_delay(
                                        attempt,
                                    ))
                                    .await;
                                    continue;
                                } else {
                                    let route_error: crate::Error =
                                        ErrorKind::HashError(
                                            sha1.to_string(),
                                            hash,
                                        )
                                        .into();
                                    if has_next_route {
                                        tracing::warn!(
                                            url = request_url,
                                            error = %route_error,
                                            "Mirror checksum validation failed; falling back to official source"
                                        );
                                        break;
                                    }
                                    return Err(route_error);
                                }
                            }
                        }

                        if let Some(validate_response) = response_validator
                            && let Err(error) = validate_response(&bytes)
                        {
                            if is_mirror && has_next_route {
                                tracing::warn!(
                                    url = request_url,
                                    error = %error,
                                    "Mirror returned incompatible response data; falling back to official source"
                                );
                                break;
                            }
                            return Err(error);
                        }

                        tracing::trace!("Done downloading URL {request_url}");

                        if let Some(fence_key) = fence_key {
                            GLOBAL_FETCH_FENCE.record_ok(fence_key);
                        }

                        return Ok(bytes);
                    } else if attempt <= FETCH_ATTEMPTS {
                        tokio::time::sleep(fetch_retry_delay(attempt)).await;
                        continue;
                    } else if let Err(err) = bytes {
                        if has_next_route {
                            tracing::warn!(
                                url = request_url,
                                error = %err,
                                "Mirror response failed; falling back to official source"
                            );
                            break;
                        }
                        return Err(err.into());
                    }
                }
                Err(error) if attempt <= FETCH_ATTEMPTS => {
                    tracing::debug!(
                        attempt,
                        url = request_url,
                        error = %error,
                        "Fetch failed; retrying"
                    );
                    tokio::time::sleep(fetch_retry_delay(attempt)).await;
                    continue;
                }
                Err(err) => {
                    if has_next_route {
                        tracing::warn!(
                            url = request_url,
                            error = %err,
                            "Mirror connection failed; falling back to official source"
                        );
                        break;
                    }
                    return Err(err.into());
                }
            }
        }
    }

    unreachable!()
}

/// Downloads a file from specified mirrors
#[tracing::instrument(skip(semaphore))]
pub async fn fetch_mirrors(
    mirrors: &[&str],
    sha1: Option<&str>,
    download_meta: Option<&DownloadMeta>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<Bytes> {
    if mirrors.is_empty() {
        return Err(
            ErrorKind::InputError("No mirrors provided!".to_string()).into()
        );
    }

    for (index, mirror) in mirrors.iter().enumerate() {
        let result = fetch_with_client(
            mirror,
            sha1,
            download_meta,
            uri_path,
            semaphore,
            exec,
            &REQWEST_CLIENT,
        )
        .await;

        if result.is_ok() || (result.is_err() && index == (mirrors.len() - 1)) {
            return result;
        }
    }

    unreachable!()
}

#[tracing::instrument(skip(semaphore, progress))]
pub async fn fetch_mirrors_with_progress(
    mirrors: &[&str],
    sha1: Option<&str>,
    download_meta: Option<&DownloadMeta>,
    uri_path: Option<&'static str>,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
    mut progress: Option<&mut FetchProgressFn<'_>>,
) -> crate::Result<Bytes> {
    if mirrors.is_empty() {
        return Err(
            ErrorKind::InputError("No mirrors provided!".to_string()).into()
        );
    }

    for (index, mirror) in mirrors.iter().enumerate() {
        let result = fetch_with_client_progress(
            mirror,
            sha1,
            download_meta,
            uri_path,
            semaphore,
            exec,
            &REQWEST_CLIENT,
            progress.as_deref_mut(),
        )
        .await;

        if result.is_ok() || (result.is_err() && index == (mirrors.len() - 1)) {
            return result;
        }
    }

    unreachable!()
}

/// Posts a JSON to a URL
#[tracing::instrument(skip(json_body, semaphore))]
pub async fn post_json(
    url: &str,
    json_body: serde_json::Value,
    semaphore: &FetchSemaphore,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
) -> crate::Result<()> {
    let _permit = semaphore.0.acquire().await?;

    let mut req = INSECURE_REQWEST_CLIENT.post(url).json(&json_body);

    if let Some(creds) =
        crate::state::ModrinthCredentials::get_active(exec).await?
    {
        req = req.header("Authorization", &creds.session);
    }

    req.send().await?.error_for_status()?;
    Ok(())
}

pub async fn read_json<T>(
    path: &Path,
    semaphore: &IoSemaphore,
) -> crate::Result<T>
where
    T: DeserializeOwned,
{
    let _permit = semaphore.0.acquire().await?;

    let json = io::read(path).await?;
    let json = serde_json::from_slice::<T>(&json)?;

    Ok(json)
}

#[tracing::instrument(skip(bytes, semaphore))]
pub async fn write(
    path: &Path,
    bytes: &[u8],
    semaphore: &IoSemaphore,
) -> crate::Result<()> {
    let _permit = semaphore.0.acquire().await?;

    if let Some(parent) = path.parent() {
        io::create_dir_all(parent).await?;
    }

    let mut file = File::create(path)
        .await
        .map_err(|e| IOError::with_path(e, path))?;
    file.write_all(bytes)
        .await
        .map_err(|e| IOError::with_path(e, path))?;
    tracing::trace!("Done writing file {}", path.display());
    Ok(())
}

pub async fn copy(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    semaphore: &IoSemaphore,
) -> crate::Result<()> {
    let src: &Path = src.as_ref();
    let dest = dest.as_ref();

    let _permit = semaphore.0.acquire().await?;

    if let Some(parent) = dest.parent() {
        io::create_dir_all(parent).await?;
    }

    io::copy(src, dest).await?;
    tracing::trace!(
        "Done copying file {} to {}",
        src.display(),
        dest.display()
    );
    Ok(())
}

// Writes a icon to the cache and returns the absolute path of the icon within the cache directory
#[tracing::instrument(skip(bytes, semaphore))]
pub async fn write_cached_icon(
    icon_path: &str,
    cache_dir: &Path,
    bytes: Bytes,
    semaphore: &IoSemaphore,
) -> crate::Result<PathBuf> {
    let extension = Path::new(&icon_path).extension().and_then(OsStr::to_str);
    let hash = sha1_async(bytes.clone()).await?;
    let path = cache_dir.join("icons").join(if let Some(ext) = extension {
        format!("{hash}.{ext}")
    } else {
        hash
    });

    write(&path, &bytes, semaphore).await?;

    let path = io::canonicalize(path)?;
    Ok(path)
}

pub async fn sha1_async(bytes: Bytes) -> crate::Result<String> {
    let hash = tokio::task::spawn_blocking(move || {
        sha1_smol::Sha1::from(bytes).hexdigest()
    })
    .await?;

    Ok(hash)
}

pub async fn sha1_file_async(
    path: impl AsRef<Path>,
) -> crate::Result<(u64, String)> {
    let path = path.as_ref();
    // Local files can be multi-gigabyte .mrpacks, so hash them without materializing bytes.
    let mut file = File::open(path)
        .await
        .map_err(|e| IOError::with_path(e, path))?;
    let mut hasher = sha1_smol::Sha1::new();
    let mut size = 0;
    let mut buffer = vec![0; 262144];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .await
            .map_err(|e| IOError::with_path(e, path))?;
        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
        size += bytes_read as u64;
    }

    Ok((size, hasher.digest().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeDelta, Utc};
    use std::time::Duration;

    fn mirrors(
        minecraft: bool,
        modrinth: bool,
        curseforge: bool,
    ) -> DownloadMirrorSettings {
        DownloadMirrorSettings {
            minecraft,
            modrinth,
            curseforge,
        }
    }

    #[test]
    fn minecraft_mirror_rewrites_supported_urls() {
        let mirrors = mirrors(true, false, false);
        let cases = [
            (
                "https://piston-meta.mojang.com/v1/packages/version.json",
                "https://bmclapi2.bangbang93.com/v1/packages/version.json",
            ),
            (
                "https://resources.download.minecraft.net/ab/abcdef",
                "https://bmclapi2.bangbang93.com/assets/ab/abcdef",
            ),
            (
                "https://libraries.minecraft.net/com/example/library.jar",
                "https://bmclapi2.bangbang93.com/maven/com/example/library.jar",
            ),
            (
                "https://maven.minecraftforge.net/net/minecraftforge/forge.jar",
                "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge.jar",
            ),
            (
                "https://maven.fabricmc.net/net/fabricmc/loader.jar",
                "https://bmclapi2.bangbang93.com/maven/net/fabricmc/loader.jar",
            ),
            (
                "https://maven.neoforged.net/releases/net/neoforged/neoforge.jar",
                "https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge.jar",
            ),
            (
                "https://launcher-meta.modrinth.com/maven/net/fabricmc/loader.jar",
                "https://bmclapi2.bangbang93.com/maven/net/fabricmc/loader.jar",
            ),
            (
                "https://meta.fabricmc.net/v2/versions/loader?limit=1",
                "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader?limit=1",
            ),
        ];

        for (source, expected) in cases {
            assert_eq!(resolve_download_url(source, mirrors).url, expected);
        }
    }

    #[test]
    fn provider_mirrors_are_independent() {
        let modrinth = mirrors(false, true, false);
        assert_eq!(
            resolve_download_url(
                "https://api.modrinth.com/v2/project",
                modrinth
            ),
            ResolvedDownloadUrl {
                url: "https://mod.mcimirror.top/modrinth/v2/project"
                    .to_string(),
                is_mirror: true,
            }
        );
        assert_eq!(
            resolve_download_url(
                "https://cdn.modrinth.com/data/project/file.jar",
                modrinth,
            )
            .url,
            "https://mod.mcimirror.top/data/project/file.jar"
        );
        assert_eq!(
            resolve_download_url(
                "https://api.curseforge.com/v1/mods/search",
                modrinth,
            )
            .url,
            "https://api.curseforge.com/v1/mods/search"
        );

        let curseforge = mirrors(false, false, true);
        assert_eq!(
            resolve_download_url(
                "https://api.curseforge.com/v1/mods/search",
                curseforge,
            )
            .url,
            "https://mod.mcimirror.top/curseforge/v1/mods/search"
        );
        assert_eq!(
            resolve_download_url(
                "https://edge.forgecdn.net/files/1/2/file.jar",
                curseforge,
            )
            .url,
            "https://mod.mcimirror.top/files/1/2/file.jar"
        );
        assert_eq!(
            resolve_download_url(
                "https://api.modrinth.com/v2/project",
                curseforge
            )
            .url,
            "https://api.modrinth.com/v2/project"
        );
    }

    #[test]
    fn mirror_resolution_preserves_unmatched_urls() {
        let all = mirrors(true, true, true);
        for url in [
            "https://example.com/file.jar?source=official",
            "https://mediafilez.forgecdn.net/files/1/2/file.jar",
            "https://api.modrinth.com.evil.example/v2/project",
        ] {
            assert_eq!(
                resolve_download_url(url, all),
                ResolvedDownloadUrl {
                    url: url.to_string(),
                    is_mirror: false,
                }
            );
        }

        let disabled = mirrors(false, false, false);
        assert_eq!(
            resolve_download_url(
                "https://resources.download.minecraft.net/ab/abcdef",
                disabled,
            )
            .url,
            "https://resources.download.minecraft.net/ab/abcdef"
        );
    }

    #[test]
    fn mirror_routes_fall_back_to_official_source() {
        let source = "https://api.modrinth.com/v2/project";
        assert_eq!(
            resolve_download_routes(source, mirrors(false, true, false)),
            vec![
                ResolvedDownloadUrl {
                    url: "https://mod.mcimirror.top/modrinth/v2/project"
                        .to_string(),
                    is_mirror: true,
                },
                ResolvedDownloadUrl {
                    url: source.to_string(),
                    is_mirror: false,
                },
            ]
        );
        assert_eq!(
            resolve_download_routes(source, mirrors(false, false, false)),
            vec![ResolvedDownloadUrl {
                url: source.to_string(),
                is_mirror: false,
            }]
        );
    }

    #[test]
    fn fetch_retries_use_exponential_backoff() {
        assert_eq!(fetch_retry_delay(1), Duration::from_secs(1));
        assert_eq!(fetch_retry_delay(2), Duration::from_secs(2));
        assert_eq!(fetch_retry_delay(3), Duration::from_secs(4));
        assert_eq!(fetch_retry_delay(4), Duration::from_secs(8));
    }

    #[test]
    fn test_fence_block_after_4_fails() {
        // Update tests if the FenceInner constants change

        let mut fence = FenceInner::new();

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(fence.is_blocked());
    }

    #[test]
    fn test_fetch_fence_keys_are_independent() {
        let fence = FetchFence {
            inner: Mutex::new(HashMap::new()),
        };

        for _ in 0..FenceInner::FAILURE_THRESHOLD {
            fence.record_fail("/v3/version_file/:sha1/update");
        }

        assert!(fence.is_blocked("/v3/version_file/:sha1/update"));
        assert!(!fence.is_blocked("/v3/project/:id"));
    }

    #[test]
    fn test_fetch_fence_latest_block_minutes() {
        let fence = FetchFence {
            inner: Mutex::new(HashMap::new()),
        };

        {
            let mut inner = fence.inner.lock();
            inner.insert("/expired", FenceInner::new());
            inner.get_mut("/expired").unwrap().block_until =
                Some(Utc::now() - TimeDelta::minutes(1));
            inner.insert("/short", FenceInner::new());
            inner.get_mut("/short").unwrap().block_until =
                Some(Utc::now() + TimeDelta::seconds(61));
            inner.insert("/long", FenceInner::new());
            inner.get_mut("/long").unwrap().block_until =
                Some(Utc::now() + TimeDelta::seconds(140));
        }

        assert_eq!(fence.latest_block_minutes(), 3);
    }

    #[test]
    fn test_fence_block_after_4_fails_with_oks() {
        // Update tests if the FenceInner constants change

        let mut fence = FenceInner::new();

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_ok();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(fence.is_blocked());
    }

    #[test]
    fn test_fence_not_blocked_after_fails_expire() {
        // Update tests if the FenceInner constants change

        let mut fence = FenceInner::new();

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.prune(Utc::now() + TimeDelta::seconds(60 * 3 + 55)); // Should prune all failures

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(!fence.is_blocked());

        fence.record_fail();
        assert!(fence.is_blocked());
    }

    #[test]
    fn test_fence_trigger_block_windows() {
        // brute force flukes
        for i in 0..128 {
            let mut fence = FenceInner::new();

            fence.trigger_block();
            assert!(fence.is_blocked(), "Should be blocked (attempt {i})");

            let block_until = fence.block_until.unwrap();
            assert!(
                block_until > Utc::now() + TimeDelta::seconds(60 + 55),
                "Should be more than 2 minutes (with some leeway) (attempt {i})"
            ); // more than 2 minutes (with some leeway)
            assert!(
                block_until < Utc::now() + TimeDelta::seconds(60 * 5),
                "Should be less than 5 minutes (attempt {i})"
            ); // less than 5 minutes

            fence.block_until = None;

            fence.trigger_block();
            let block_until = fence.block_until.unwrap();
            assert!(
                block_until > Utc::now() + TimeDelta::seconds(60 * 3 + 55),
                "Should be more than 4 minutes (with some leeway) (attempt {i})"
            ); // more than 4 minutes (with some leeway)
            assert!(
                block_until < Utc::now() + TimeDelta::seconds(60 * 10),
                "Should be less than 10 minutes (attempt {i})"
            ); // less than 10 minutes

            fence.block_until = None;

            fence.trigger_block();
            let block_until = fence.block_until.unwrap();
            assert!(
                block_until > Utc::now() + TimeDelta::seconds(60 * 5 + 55),
                "Should be more than 6 minutes (with some leeway) (attempt {i})"
            ); // more than 6 minutes (with some leeway)
            assert!(
                block_until < Utc::now() + TimeDelta::seconds(60 * 15),
                "Should be less than 15 minutes (attempt {i})"
            ); // less than 15 minutes
        }
    }
}
