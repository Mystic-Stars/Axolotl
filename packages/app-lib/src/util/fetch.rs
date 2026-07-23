//! Functions for fetching information from the Internet
use super::download_dns::DownloadDnsResolver;
use super::download_manager::{DownloadSpeedTracker, SpeedSnapshot};
use super::io::{self, IOError};
use crate::event::LoadingBarId;
use crate::event::emit::emit_loading;
use crate::{ErrorKind, LabrinthError};
use bytes::Bytes;
use chrono::{DateTime, TimeDelta, Utc};
use eyre::{Context, eyre};
use futures::StreamExt;
use parking_lot::Mutex;
use rand::Rng;
use reqwest::{Method, StatusCode, header};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::collections::{HashMap, VecDeque};
use std::ffi::OsStr;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Weak};
use std::time::{self, Instant};
use tokio::sync::{Mutex as AsyncMutex, Semaphore, SemaphorePermit};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use url::Url;

pub const DOWNLOAD_META_HEADER: &str = "modrinth-download-meta";

const BMCLAPI_BASE_URL: &str = "https://bmclapi2.bangbang93.com";
const MCIM_BASE_URL: &str = "https://mod.mcimirror.top";
const METADATA_ATTEMPT_BUDGET: usize = 4;
const SEGMENTED_DOWNLOAD_THRESHOLD: u64 = 1024 * 1024;
const INITIAL_SEGMENT_CONCURRENCY: usize = 4;
const MAX_SEGMENT_CONCURRENCY: usize = 8;
const MIN_SEGMENT_SIZE: u64 = 256 * 1024;
const SEGMENT_RETRY_ATTEMPTS: usize = 3;
const SEGMENT_EXPANSION_SAMPLE_COUNT: usize = 5;
const SEGMENT_EXPANSION_INTERVAL: time::Duration = time::Duration::from_secs(2);
const MAX_REDIRECT_LOCATION_BYTES: usize = 8 * 1024;
const FILE_TRANSFER_CONNECT_TIMEOUT: time::Duration =
    time::Duration::from_secs(15);
const FILE_TRANSFER_READ_TIMEOUT: time::Duration =
    time::Duration::from_secs(15);
const FILE_TRANSFER_SLOW_INTERVAL: time::Duration =
    time::Duration::from_secs(5);
#[cfg(not(test))]
const FILE_TRANSFER_FIRST_BYTE_TIMEOUT: time::Duration =
    time::Duration::from_secs(30);
#[cfg(test)]
const FILE_TRANSFER_FIRST_BYTE_TIMEOUT: time::Duration =
    time::Duration::from_millis(250);
const MIRROR_REQUEST_START_INTERVAL: time::Duration =
    time::Duration::from_millis(100);

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceClass {
    Metadata,
    MinecraftAsset,
    MinecraftLibrary,
    Loader,
    Java,
    Modrinth,
    CurseForge,
    Modpack,
    #[default]
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadRouteSource {
    Official,
    Bmclapi,
    Mcim,
    Alternate,
}

impl DownloadRouteSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Bmclapi => "bmclapi",
            Self::Mcim => "mcim",
            Self::Alternate => "alternate",
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ProxyPolicy {
    #[default]
    System,
    Direct,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DownloadRoute {
    pub url: String,
    pub source: DownloadRouteSource,
    pub is_mirror: bool,
    pub allow_sensitive_headers: bool,
    pub supports_range: bool,
    pub proxy: ProxyPolicy,
}

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContentValidation {
    #[default]
    None,
    Json,
    Jar,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Integrity {
    pub size: Option<u64>,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub sha256: Option<String>,
    pub md5: Option<String>,
    pub content: ContentValidation,
}

impl Integrity {
    pub fn sha1(hash: impl Into<String>) -> Self {
        Self {
            sha1: Some(hash.into()),
            ..Self::default()
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_content_validation(
        mut self,
        content: ContentValidation,
    ) -> Self {
        self.content = content;
        self
    }

    fn is_empty(&self) -> bool {
        self.size.is_none()
            && self.sha1.is_none()
            && self.sha512.is_none()
            && self.sha256.is_none()
            && self.md5.is_none()
            && self.content == ContentValidation::None
    }
}

#[derive(Clone, Debug)]
pub struct DownloadRequest {
    pub url: String,
    pub resource: ResourceClass,
    pub integrity: Integrity,
    pub download_meta: Option<DownloadMeta>,
    pub header: Option<(String, String)>,
    pub candidate_urls: Vec<String>,
}

impl DownloadRequest {
    pub fn new(url: impl Into<String>, resource: ResourceClass) -> Self {
        Self {
            url: url.into(),
            resource,
            integrity: Integrity::default(),
            download_meta: None,
            header: None,
            candidate_urls: Vec::new(),
        }
    }

    pub fn with_integrity(mut self, integrity: Integrity) -> Self {
        self.integrity = integrity;
        self
    }

    pub fn with_download_meta(mut self, download_meta: DownloadMeta) -> Self {
        self.download_meta = Some(download_meta);
        self
    }

    pub fn with_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.header = Some((name.into(), value.into()));
        self
    }

    pub fn with_candidate_urls<I, S>(mut self, urls: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.candidate_urls.extend(urls.into_iter().map(Into::into));
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DownloadResult {
    pub path: PathBuf,
    pub url: String,
    pub source: DownloadRouteSource,
    pub size: u64,
    pub attempts: usize,
    pub fallback_count: usize,
}

static AUTO_PREFERS_OFFICIAL: AtomicBool = AtomicBool::new(false);
static AUTO_SOURCE_PROBED: AtomicBool = AtomicBool::new(false);
static IN_FLIGHT_DOWNLOADS: LazyLock<
    dashmap::DashMap<String, Weak<AsyncMutex<()>>>,
> = LazyLock::new(dashmap::DashMap::new);

fn modrinth_request_kind(url: &str) -> Option<&'static str> {
    if url.starts_with(env!("MODRINTH_API_URL"))
        || url.starts_with(env!("MODRINTH_API_URL_V3"))
    {
        Some("API")
    } else if url.starts_with("https://cdn.modrinth.com") {
        Some("CDN")
    } else {
        None
    }
}

fn sanitize_url_for_log(url: &str) -> String {
    if let Ok(mut url) = Url::parse(url) {
        url.set_query(None);
        url.set_fragment(None);
        return url.into();
    }

    url.split(['?', '#']).next().unwrap_or(url).to_string()
}

fn is_safe_redirect_location(location: &str) -> bool {
    location.len() <= MAX_REDIRECT_LOCATION_BYTES && location.is_ascii()
}

fn is_official_modrinth_cdn_redirect(location: Option<&str>) -> bool {
    let Some(location) = location.filter(|location| {
        is_safe_redirect_location(location)
            && location
                .get(..8)
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
    }) else {
        return false;
    };
    let authority = location[8..]
        .split(|character| matches!(character, '/' | '?' | '#'))
        .next()
        .unwrap_or_default();
    authority.eq_ignore_ascii_case("cdn.modrinth.com")
        || authority.eq_ignore_ascii_case("cdn.modrinth.com:443")
}

fn is_mrpack_url(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .is_some_and(|url| url.path().to_ascii_lowercase().ends_with(".mrpack"))
}

fn route(
    url: String,
    source: DownloadRouteSource,
    is_mirror: bool,
    supports_range: bool,
) -> DownloadRoute {
    DownloadRoute {
        url,
        source,
        is_mirror,
        allow_sensitive_headers: !is_mirror,
        supports_range,
        proxy: ProxyPolicy::System,
    }
}

fn official_route(url: &str, resource: ResourceClass) -> DownloadRoute {
    let source = Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .map_or(DownloadRouteSource::Official, |host| match host.as_str() {
            "bmclapi2.bangbang93.com" => DownloadRouteSource::Bmclapi,
            "mod.mcimirror.top" => DownloadRouteSource::Mcim,
            _ => DownloadRouteSource::Official,
        });
    let is_mirror = matches!(
        source,
        DownloadRouteSource::Bmclapi | DownloadRouteSource::Mcim
    );
    route(
        url.to_string(),
        source,
        is_mirror,
        !matches!(resource, ResourceClass::Metadata),
    )
}

fn url_with_base(original: &Url, base: &str, path: &str) -> Option<String> {
    let mut target = Url::parse(base).ok()?;
    target.set_path(path);
    target.set_query(original.query());
    target.set_fragment(None);
    Some(target.into())
}

fn explicit_mirror_routes(
    url: &str,
    resource: ResourceClass,
) -> Vec<DownloadRoute> {
    let Ok(parsed) = Url::parse(url) else {
        return Vec::new();
    };
    if parsed.scheme() != "https" {
        return Vec::new();
    }

    let host = parsed.host_str().unwrap_or_default();
    let path = parsed.path();
    let supports_range = !matches!(resource, ResourceClass::Metadata);
    let mut routes = Vec::new();
    let push_mirror = |routes: &mut Vec<DownloadRoute>,
                       base: &str,
                       path: String,
                       source: DownloadRouteSource| {
        if let Some(url) = url_with_base(&parsed, base, &path) {
            routes.push(route(url, source, true, supports_range));
        }
    };

    match host {
        "resources.download.minecraft.net" => {
            push_mirror(
                &mut routes,
                BMCLAPI_BASE_URL,
                format!("/assets{path}"),
                DownloadRouteSource::Bmclapi,
            );
        }
        "libraries.minecraft.net" => {
            push_mirror(
                &mut routes,
                BMCLAPI_BASE_URL,
                format!("/maven{path}"),
                DownloadRouteSource::Bmclapi,
            );
            push_mirror(
                &mut routes,
                BMCLAPI_BASE_URL,
                format!("/libraries{path}"),
                DownloadRouteSource::Bmclapi,
            );
        }
        "maven.minecraftforge.net" | "maven.fabricmc.net" => {
            push_mirror(
                &mut routes,
                BMCLAPI_BASE_URL,
                format!("/maven{path}"),
                DownloadRouteSource::Bmclapi,
            );
        }
        "files.minecraftforge.net" if path.starts_with("/maven/") => {
            push_mirror(
                &mut routes,
                BMCLAPI_BASE_URL,
                path.to_string(),
                DownloadRouteSource::Bmclapi,
            );
        }
        "maven.neoforged.net" if path.starts_with("/releases/") => {
            push_mirror(
                &mut routes,
                BMCLAPI_BASE_URL,
                format!("/maven/{}", path.trim_start_matches("/releases/")),
                DownloadRouteSource::Bmclapi,
            );
        }
        "meta.fabricmc.net" => {
            push_mirror(
                &mut routes,
                BMCLAPI_BASE_URL,
                format!("/fabric-meta{path}"),
                DownloadRouteSource::Bmclapi,
            );
        }
        "piston-meta.mojang.com"
        | "launchermeta.mojang.com"
        | "launcher.mojang.com"
        | "piston-data.mojang.com" => push_mirror(
            &mut routes,
            BMCLAPI_BASE_URL,
            path.to_string(),
            DownloadRouteSource::Bmclapi,
        ),
        "api.modrinth.com" => push_mirror(
            &mut routes,
            MCIM_BASE_URL,
            format!("/modrinth{path}"),
            DownloadRouteSource::Mcim,
        ),
        "cdn.modrinth.com" => push_mirror(
            &mut routes,
            MCIM_BASE_URL,
            path.to_string(),
            DownloadRouteSource::Mcim,
        ),
        "api.curseforge.com" => push_mirror(
            &mut routes,
            MCIM_BASE_URL,
            format!("/curseforge{path}"),
            DownloadRouteSource::Mcim,
        ),
        "edge.forgecdn.net"
        | "media.forgecdn.net"
        | "mediafilez.forgecdn.net" => push_mirror(
            &mut routes,
            MCIM_BASE_URL,
            path.to_string(),
            DownloadRouteSource::Mcim,
        ),
        _ => {}
    }

    routes
}

fn route_host(route: &DownloadRoute) -> Option<String> {
    Url::parse(&route.url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
}

fn is_official_modrinth_download_url(url: &str) -> bool {
    Url::parse(url).is_ok_and(|url| {
        matches!(
            url.host_str(),
            Some("api.modrinth.com" | "cdn.modrinth.com")
        )
    })
}

fn is_official_version_manifest_url(url: &str) -> bool {
    Url::parse(url).is_ok_and(|url| {
        matches!(
            url.host_str(),
            Some("piston-meta.mojang.com" | "launchermeta.mojang.com")
        ) && url.path().contains("version_manifest")
    })
}

fn order_auto_routes(routes: &mut [DownloadRoute]) {
    let prefer_official = AUTO_PREFERS_OFFICIAL.load(Ordering::Relaxed);
    routes.sort_by_key(|route| route.is_mirror == prefer_official);
}

fn uses_mirror_only_loader_routes(url: &str, resource: ResourceClass) -> bool {
    if !matches!(
        resource,
        ResourceClass::MinecraftLibrary | ResourceClass::Loader
    ) {
        return false;
    }

    let Ok(url) = Url::parse(url) else {
        return false;
    };
    if matches!(
        url.host_str(),
        Some(
            "maven.minecraftforge.net"
                | "maven.fabricmc.net"
                | "maven.neoforged.net"
        )
    ) {
        return true;
    }

    let path = url.path().to_ascii_lowercase();
    matches!(
        url.host_str(),
        Some(
            "libraries.minecraft.net"
                | "piston-data.mojang.com"
                | "piston-meta.mojang.com"
        )
    ) && ["minecraftforge", "fabricmc", "neoforged"]
        .iter()
        .any(|loader| path.contains(loader))
}

pub fn resolve_download_routes_for(
    url: &str,
    resource: ResourceClass,
    mode: crate::state::DownloadSourceMode,
) -> Vec<DownloadRoute> {
    let official = official_route(url, resource);
    let mut routes = explicit_mirror_routes(url, resource);
    if !uses_mirror_only_loader_routes(url, resource) {
        routes.push(official);
    }
    match mode {
        crate::state::DownloadSourceMode::Auto
            if is_official_version_manifest_url(url)
                && !AUTO_SOURCE_PROBED.load(Ordering::Relaxed) =>
        {
            routes.sort_by_key(|route| route.is_mirror)
        }
        crate::state::DownloadSourceMode::Auto => {
            order_auto_routes(&mut routes)
        }
        crate::state::DownloadSourceMode::OfficialOnly => {
            routes.retain(|route| !route.is_mirror);
        }
        crate::state::DownloadSourceMode::MirrorPreferred => {
            routes.sort_by_key(|route| !route.is_mirror);
        }
    }
    routes
}

fn source_mode_for_resource(
    resource: ResourceClass,
) -> crate::state::DownloadSourceMode {
    let Some(state) = crate::State::get_if_initialized() else {
        return crate::state::DownloadSourceMode::OfficialOnly;
    };

    match resource {
        ResourceClass::Metadata => state.minecraft_metadata_source(),
        ResourceClass::MinecraftAsset
        | ResourceClass::MinecraftLibrary
        | ResourceClass::Loader
        | ResourceClass::Java => state.minecraft_file_source(),
        ResourceClass::Modrinth | ResourceClass::Modpack => {
            state.modrinth_source()
        }
        ResourceClass::CurseForge => state.curseforge_source(),
        ResourceClass::Other => crate::state::DownloadSourceMode::OfficialOnly,
    }
}

fn infer_resource_class(url: &str) -> ResourceClass {
    let Ok(parsed) = Url::parse(url) else {
        return ResourceClass::Other;
    };
    let host = parsed.host_str().unwrap_or_default();
    match host {
        "resources.download.minecraft.net" => ResourceClass::MinecraftAsset,
        "libraries.minecraft.net" => ResourceClass::MinecraftLibrary,
        "maven.minecraftforge.net"
        | "files.minecraftforge.net"
        | "maven.fabricmc.net"
        | "maven.neoforged.net"
        | "meta.fabricmc.net" => ResourceClass::Loader,
        "repo1.maven.org" | "repo.maven.apache.org" => {
            ResourceClass::MinecraftLibrary
        }
        "piston-meta.mojang.com" if parsed.path().contains("java-runtime") => {
            ResourceClass::Java
        }
        "piston-data.mojang.com" if parsed.path().contains("java-runtime") => {
            ResourceClass::Java
        }
        "piston-meta.mojang.com" | "launchermeta.mojang.com" => {
            ResourceClass::Metadata
        }
        "launcher.mojang.com" | "piston-data.mojang.com" => {
            ResourceClass::MinecraftLibrary
        }
        "api.modrinth.com" | "cdn.modrinth.com" => ResourceClass::Modrinth,
        "api.curseforge.com"
        | "edge.forgecdn.net"
        | "media.forgecdn.net"
        | "mediafilez.forgecdn.net" => ResourceClass::CurseForge,
        _ => ResourceClass::Other,
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

static DOWNLOAD_DNS_RESOLVER: LazyLock<Arc<DownloadDnsResolver>> =
    LazyLock::new(|| Arc::new(DownloadDnsResolver::default()));
static MIRROR_REQUEST_SLOTS: LazyLock<AsyncMutex<[Instant; 2]>> =
    LazyLock::new(|| AsyncMutex::new([Instant::now(); 2]));

fn reqwest_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(FILE_TRANSFER_CONNECT_TIMEOUT)
        .read_timeout(FILE_TRANSFER_READ_TIMEOUT)
        .tcp_keepalive(Some(time::Duration::from_secs(10)))
        .tcp_nodelay(true)
        .pool_max_idle_per_host(64)
        .http1_only()
        .dns_resolver(Arc::clone(&DOWNLOAD_DNS_RESOLVER))
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

const DOWNLOAD_PROGRESS_LOG_INTERVAL: u64 = 8 * 1024 * 1024;
const MODRINTH_CDN_ATTEMPTS: usize = 3;
const MODRINTH_CDN_ATTEMPT_TIMEOUT: time::Duration =
    time::Duration::from_secs(120);

static NO_REDIRECT_REQWEST_CLIENT: LazyLock<reqwest::Client> =
    LazyLock::new(|| {
        let builder = reqwest_client_builder()
            .redirect(reqwest::redirect::Policy::none());
        #[cfg(not(test))]
        let builder = builder.https_only(true);
        builder
            .build()
            .expect("client configuration should be valid")
    });

static DIRECT_REQWEST_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let builder = reqwest_client_builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none());
    #[cfg(not(test))]
    let builder = builder.https_only(true);
    builder
        .build()
        .expect("client configuration should be valid")
});

static DIRECT_FETCH_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest_client_builder()
        .https_only(true)
        .no_proxy()
        .build()
        .expect("client configuration should be valid")
});

const FETCH_RETRY_DELAYS: [time::Duration; 3] = [
    time::Duration::from_millis(250),
    time::Duration::from_millis(750),
    time::Duration::from_secs(2),
];

fn fetch_retry_delay(attempt: usize) -> time::Duration {
    let base = FETCH_RETRY_DELAYS
        .get(attempt.saturating_sub(1))
        .copied()
        .unwrap_or(*FETCH_RETRY_DELAYS.last().unwrap());
    let jitter = rand::thread_rng().gen_range(0.85..=1.15);
    time::Duration::from_secs_f64(base.as_secs_f64() * jitter)
}

fn retry_after(response: &reqwest::Response) -> Option<time::Duration> {
    let value = response.headers().get(header::RETRY_AFTER)?.to_str().ok()?;
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(time::Duration::from_secs(seconds.min(60)));
    }

    let retry_at = DateTime::parse_from_rfc2822(value)
        .ok()?
        .with_timezone(&Utc);
    let seconds = retry_at.signed_duration_since(Utc::now()).num_seconds();
    Some(time::Duration::from_secs(seconds.clamp(0, 60) as u64))
}

fn is_sensitive_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("cookie")
        || name.eq_ignore_ascii_case("x-api-key")
}

fn header_requires_official_only(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("cookie")
}

fn requires_modrinth_auth(
    method: &Method,
    header: Option<(&str, &str)>,
    uri_path: Option<&str>,
) -> bool {
    if method != Method::GET
        || header.is_some_and(|(name, _)| is_sensitive_header(name))
    {
        return true;
    }

    uri_path.is_some_and(|path| {
        matches!(path, "/v2/user" | "/v3/friends")
            || path.starts_with("/v2/session")
            || path.starts_with("/v3/friend/")
            || path.starts_with("/v3/notification")
    })
}

fn record_route_success(
    route: &DownloadRoute,
    ttfb: time::Duration,
    _bytes: u64,
    transfer_elapsed: time::Duration,
) {
    if let Some(host) = route_host(route) {
        DOWNLOAD_DNS_RESOLVER.record_host_result(&host, 0.5);
    }
    if route.source == DownloadRouteSource::Official
        && is_official_version_manifest_url(&route.url)
    {
        AUTO_PREFERS_OFFICIAL.store(
            ttfb.saturating_add(transfer_elapsed)
                <= time::Duration::from_secs(4),
            Ordering::Relaxed,
        );
        AUTO_SOURCE_PROBED.store(true, Ordering::Relaxed);
    }
}

fn record_route_failure(route: &DownloadRoute) {
    if let Some(host) = route_host(route) {
        DOWNLOAD_DNS_RESOLVER.record_host_result(&host, -0.7);
    }
    if route.source == DownloadRouteSource::Official
        && is_official_version_manifest_url(&route.url)
    {
        AUTO_PREFERS_OFFICIAL.store(false, Ordering::Relaxed);
        AUTO_SOURCE_PROBED.store(true, Ordering::Relaxed);
    }
}

fn range_splitting_allowed(route: &DownloadRoute) -> bool {
    let host = route_host(route).unwrap_or_default();
    ![
        "bmclapi",
        "github.com",
        "optifine.net",
        "momot.rs",
        "meloong.com",
    ]
    .iter()
    .any(|blocked| host.contains(blocked))
}

fn disable_range_splitting(route: &DownloadRoute) {
    let _ = route;
}

pub type FetchProgressFn<'a> = dyn FnMut(
        u64,
        u64,
    ) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + 'a>>
    + Send
    + 'a;

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip_all)]
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
        METADATA_ATTEMPT_BUDGET,
    )
    .await?;
    Ok(serde_json::from_slice(&result)?)
}

/// Downloads a file with retry and checksum functionality, and a specific
/// [`reqwest::Client`].
#[tracing::instrument(skip_all)]
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

/// Downloads a file with retry and checksum functionality
#[tracing::instrument(skip_all)]
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
        METADATA_ATTEMPT_BUDGET,
    )
    .await
}

#[tracing::instrument(skip_all)]
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
    attempt_budget: usize,
) -> crate::Result<Bytes> {
    let resource = infer_resource_class(url);
    let mode = source_mode_for_resource(resource);
    let mut request_routes = resolve_download_routes_for(url, resource, mode);
    let modrinth_request_kind = modrinth_request_kind(url);
    let is_mrpack_download =
        modrinth_request_kind == Some("CDN") && is_mrpack_url(url);
    let is_api_url = url.starts_with(env!("MODRINTH_API_URL"))
        || url.starts_with(env!("MODRINTH_API_URL_V3"));
    let requires_auth =
        is_api_url && requires_modrinth_auth(&method, header, uri_path);
    let creds = if requires_auth
        && header.as_ref().is_none_or(|x| !is_sensitive_header(x.0))
    {
        crate::state::ModrinthCredentials::get_active(exec).await?
    } else {
        None
    };
    if method != Method::GET
        || header
            .as_ref()
            .is_some_and(|header| header_requires_official_only(header.0))
        || requires_auth
    {
        request_routes.retain(|route| !route.is_mirror);
    }
    if request_routes.is_empty() {
        request_routes.push(official_route(url, resource));
    }

    let mut total_attempts = 0;
    let mut last_error = None;

    for (route_index, route) in request_routes.iter().enumerate() {
        let request_url = &route.url;
        let log_request_url = sanitize_url_for_log(request_url);
        let is_mirror = route.is_mirror;
        let route_source = route.source;
        let request_target = if is_mirror { "mirror" } else { "official" };
        let has_next_route = route_index + 1 < request_routes.len();
        let fence_key = if is_api_url && !is_mirror {
            uri_path
        } else {
            None
        };
        let download_meta_header = (!is_mirror
            && is_official_modrinth_download_url(request_url))
        .then(|| {
            download_meta.map(|m| {
                (DOWNLOAD_META_HEADER.to_string(), m.to_header_value())
            })
        })
        .flatten();

        let max_attempts = if modrinth_request_kind == Some("CDN") {
            if is_mirror { 1 } else { MODRINTH_CDN_ATTEMPTS }
        } else {
            attempt_budget
        };
        let mut retried_server_error = false;
        let mut route_attempts = 0;
        while total_attempts < attempt_budget {
            let remaining_routes = request_routes.len() - route_index - 1;
            route_attempts += 1;
            let attempt = route_attempts;
            let has_more_attempts = attempt < max_attempts
                && total_attempts + remaining_routes < attempt_budget;
            if let Some(fence_key) = fence_key
                && GLOBAL_FETCH_FENCE.is_blocked(fence_key)
            {
                return Err(ErrorKind::ApiIsDownError(
                    GLOBAL_FETCH_FENCE.latest_block_minutes(),
                )
                .into());
            }
            total_attempts += 1;

            let started = time::Instant::now();
            if let Some(request_kind) = modrinth_request_kind {
                tracing::info!(
                    request_target,
                    source = route_source.as_str(),
                    request_kind,
                    method = %method,
                    url = %log_request_url,
                    route = route_index + 1,
                    attempt,
                    max_attempts,
                    "Attempting Modrinth request"
                );
            }

            let protected_headers = creds.is_some()
                || download_meta_header.is_some()
                || header.is_some_and(|header| is_sensitive_header(header.0));
            let route_client = match (route.proxy, protected_headers) {
                (ProxyPolicy::System, false)
                    if is_mirror && modrinth_request_kind.is_some() =>
                {
                    &*NO_REDIRECT_REQWEST_CLIENT
                }
                (ProxyPolicy::System, false) => client,
                (ProxyPolicy::System, true) => &*NO_REDIRECT_REQWEST_CLIENT,
                (ProxyPolicy::Direct, false) => &*DIRECT_FETCH_CLIENT,
                (ProxyPolicy::Direct, true) => &*DIRECT_REQWEST_CLIENT,
            };
            let mut req = route_client.request(method.clone(), request_url);
            if modrinth_request_kind == Some("CDN") && !is_mrpack_download {
                req = req.timeout(MODRINTH_CDN_ATTEMPT_TIMEOUT);
            }

            if let Some(body) = json_body.clone() {
                req = req.json(&body);
            }

            if let Some(header) = header
                && (route.allow_sensitive_headers
                    || !is_sensitive_header(header.0))
            {
                req = req.header(header.0, header.1);
            }

            if route.allow_sensitive_headers
                && let Some(ref creds) = creds
            {
                req = req.header("Authorization", &creds.session);
            }

            if let Some((name, value)) = &download_meta_header {
                tracing::debug!("Sending download analytics: {value}");
                req = req.header(name.as_str(), value.as_str());
            }

            let permit = semaphore.0.acquire().await?;
            let request_started = Instant::now();
            let result = req.send().await;
            let ttfb = request_started.elapsed();
            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let retry_after = retry_after(&resp);
                    if status.is_redirection() {
                        if is_mirror
                            && has_next_route
                            && modrinth_request_kind.is_some()
                        {
                            let status = resp.status();
                            let redirect_url = resp
                                .headers()
                                .get(reqwest::header::LOCATION)
                                .and_then(|value| value.to_str().ok())
                                .map(str::to_string);
                            let cache_status = resp
                                .headers()
                                .get("eo-cache-status")
                                .and_then(|value| value.to_str().ok())
                                .unwrap_or("unknown");
                            let redirects_to_official =
                                is_official_modrinth_cdn_redirect(
                                    redirect_url.as_deref(),
                                );
                            let log_redirect_url = redirect_url
                                .as_deref()
                                .map(sanitize_url_for_log)
                                .unwrap_or_else(|| "<missing>".to_string());
                            if redirects_to_official {
                                tracing::warn!(
                                    mirror_status = "cache_miss",
                                    request_target,
                                    source = route_source.as_str(),
                                    mirror_url = %log_request_url,
                                    redirect_url = %log_redirect_url,
                                    cache_status,
                                    attempt,
                                    max_attempts,
                                    status = status.as_u16(),
                                    elapsed_ms = started.elapsed().as_millis(),
                                    "Modrinth mirror redirected to official CDN; falling back to official source"
                                );
                            } else {
                                tracing::warn!(
                                    mirror_status = "redirect_unresolved",
                                    request_target,
                                    source = route_source.as_str(),
                                    mirror_url = %log_request_url,
                                    redirect_url = %log_redirect_url,
                                    cache_status,
                                    attempt,
                                    max_attempts,
                                    status = status.as_u16(),
                                    elapsed_ms = started.elapsed().as_millis(),
                                    "Modrinth mirror returned an unresolved redirect; falling back to official source"
                                );
                            }
                        }
                        drop(permit);
                        record_route_failure(route);
                        last_error = Some(
							ErrorKind::OtherError(format!(
								"Refusing to automatically forward protected headers while redirecting {log_request_url}"
							))
							.into(),
						);
                        break;
                    }
                    if status.is_client_error() || status.is_server_error() {
                        if let Some(fence_key) = fence_key {
                            if status.is_server_error() {
                                GLOBAL_FETCH_FENCE.record_fail(fence_key);
                            }
                        }
                        record_route_failure(route);
                        let backup_error =
                            resp.error_for_status_ref().unwrap_err();
                        let route_error: crate::Error = if let Ok(mut error) =
                            resp.json::<LabrinthError>().await
                        {
                            error.status = Some(status.as_u16());
                            error.method = Some(method.as_str().to_string());
                            error.url = Some(log_request_url.clone());
                            error.route = uri_path.map(str::to_string);
                            ErrorKind::LabrinthError(error).into()
                        } else {
                            backup_error.into()
                        };
                        let route_error_message = route_error.to_string();
                        drop(permit);
                        last_error = Some(route_error);

                        if status == StatusCode::TOO_MANY_REQUESTS
                            && !has_next_route
                            && has_more_attempts
                        {
                            tokio::time::sleep(retry_after.unwrap_or_else(
                                || fetch_retry_delay(total_attempts),
                            ))
                            .await;
                            continue;
                        }

                        if status.is_server_error()
                            && !retried_server_error
                            && has_more_attempts
                            && total_attempts + remaining_routes
                                < attempt_budget
                        {
                            retried_server_error = true;
                            tokio::time::sleep(fetch_retry_delay(
                                total_attempts,
                            ))
                            .await;
                            continue;
                        }

                        if has_next_route {
                            if modrinth_request_kind.is_some() {
                                tracing::warn!(
                                    request_target,
                                    source = route_source.as_str(),
                                    url = %log_request_url,
                                    attempt,
                                    max_attempts,
                                    status = status.as_u16(),
                                    elapsed_ms = started.elapsed().as_millis(),
                                    error = %route_error_message,
                                    "Modrinth mirror failed; falling back to official source"
                                );
                            } else {
                                tracing::warn!(
                                    url = %log_request_url,
                                    status = status.as_u16(),
                                    error = %route_error_message,
                                    "Mirror request failed; falling back to official source"
                                );
                            }
                            break;
                        }
                        if modrinth_request_kind.is_some() {
                            tracing::warn!(
                                request_target,
                                source = route_source.as_str(),
                                url = %log_request_url,
                                attempt,
                                max_attempts,
                                status = status.as_u16(),
                                elapsed_ms = started.elapsed().as_millis(),
                                error = %route_error_message,
                                "Modrinth official request failed"
                            );
                        }
                        break;
                    }

                    let response_url = resp.url().to_string();
                    let log_response_url = sanitize_url_for_log(&response_url);
                    if is_mirror && modrinth_request_kind == Some("CDN") {
                        let cache_status = resp
                            .headers()
                            .get("eo-cache-status")
                            .and_then(|value| value.to_str().ok())
                            .unwrap_or("unknown");
                        tracing::info!(
                            mirror_status = "cache_hit",
                            request_target,
                            source = route_source.as_str(),
                            mirror_url = %log_request_url,
                            final_url = %log_response_url,
                            cache_status,
                            attempt,
                            max_attempts,
                            status = resp.status().as_u16(),
                            elapsed_ms = started.elapsed().as_millis(),
                            "Modrinth mirror resolved cached file"
                        );
                    }
                    let transfer_started = Instant::now();
                    let bytes: eyre::Result<Bytes> = if loading_bar.is_some()
                        || progress.is_some()
                    {
                        let total_size = resp.content_length().unwrap_or(0);
                        let mut stream = resp.bytes_stream();

                        async {
                            let mut bytes = Vec::new();
                            let mut downloaded = 0_u64;
                            let mut next_progress_log =
                                DOWNLOAD_PROGRESS_LOG_INTERVAL;

                            while let Some(item) = stream.next().await {
                                let chunk = item.wrap_err_with(|| {
									eyre!(
										"failed to read response body from {log_request_url}"
									)
								})?;

                                downloaded += chunk.len() as u64;
                                bytes.extend_from_slice(&chunk);

                                if modrinth_request_kind == Some("CDN")
                                    && downloaded >= next_progress_log
                                {
                                    tracing::info!(
                                        request_target,
                                        source = route_source.as_str(),
                                        attempt,
                                        max_attempts,
                                        url = %log_request_url,
                                        final_url = %log_response_url,
                                        downloaded_bytes = downloaded,
                                        expected_bytes = total_size,
                                        "Modrinth CDN download progress"
                                    );
                                    while next_progress_log <= downloaded {
                                        next_progress_log = next_progress_log
                                            .saturating_add(
                                                DOWNLOAD_PROGRESS_LOG_INTERVAL,
                                            );
                                    }
                                }

                                if total_size > 0
                                    && let Some((bar, total)) = &loading_bar
                                {
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
                                "failed to read response body from {log_request_url}"
                            )
						})
                    };
                    drop(permit);

                    if let Ok(bytes) = bytes {
                        if let Some(sha1) = sha1 {
                            let hash = sha1_async(bytes.clone()).await?;
                            if &*hash != sha1 {
                                record_route_failure(route);
                                let route_error: crate::Error =
                                    ErrorKind::HashError(
                                        sha1.to_string(),
                                        hash,
                                    )
                                    .into();
                                last_error = Some(route_error);
                                if !has_next_route && has_more_attempts {
                                    if modrinth_request_kind.is_some() {
                                        tracing::warn!(
                                            request_target,
                                            source = route_source.as_str(),
                                            url = %log_request_url,
                                            attempt,
                                            max_attempts,
                                            elapsed_ms = started.elapsed().as_millis(),
                                            "Modrinth checksum validation failed; retrying"
                                        );
                                    }
                                    tokio::time::sleep(fetch_retry_delay(
                                        total_attempts,
                                    ))
                                    .await;
                                    continue;
                                }
                                break;
                            }
                        }

                        if let Some(validate_response) = response_validator
                            && let Err(error) = validate_response(&bytes)
                        {
                            record_route_failure(route);
                            if has_next_route {
                                tracing::warn!(
                                    url = %log_request_url,
                                    error = %error,
                                    "Download route returned incompatible data; trying the next source"
                                );
                                last_error = Some(error);
                                break;
                            }
                            return Err(error);
                        }

                        tracing::trace!(
                            "Done downloading URL {log_request_url}"
                        );
                        if let Some(request_kind) = modrinth_request_kind {
                            tracing::info!(
                                request_target,
                                source = route_source.as_str(),
                                request_kind,
                                url = %log_request_url,
                                final_url = %log_response_url,
                                attempt,
                                max_attempts,
                                bytes = bytes.len(),
                                elapsed_ms = started.elapsed().as_millis(),
                                "Completed Modrinth request"
                            );
                        }

                        if let Some(fence_key) = fence_key {
                            GLOBAL_FETCH_FENCE.record_ok(fence_key);
                        }
                        record_route_success(
                            route,
                            ttfb,
                            bytes.len() as u64,
                            transfer_started.elapsed(),
                        );

                        return Ok(bytes);
                    } else if let Err(err) = bytes {
                        record_route_failure(route);
                        let error_message = err.to_string();
                        last_error = Some(err.into());
                        if has_next_route {
                            if modrinth_request_kind.is_some() {
                                tracing::warn!(
                                    request_target,
                                    source = route_source.as_str(),
                                    url = %log_request_url,
                                    attempt,
                                    max_attempts,
                                    elapsed_ms = started.elapsed().as_millis(),
                                    error = %error_message,
                                    "Modrinth mirror response failed; falling back to official source"
                                );
                            } else {
                                tracing::warn!(
                                    url = %log_request_url,
                                    error = %error_message,
                                    "Mirror response failed; falling back to official source"
                                );
                            }
                            break;
                        }
                        if has_more_attempts {
                            if modrinth_request_kind.is_some() {
                                tracing::warn!(
                                    request_target,
                                    source = route_source.as_str(),
                                    url = %log_request_url,
                                    attempt,
                                    max_attempts,
                                    elapsed_ms = started.elapsed().as_millis(),
                                    error = %error_message,
                                    "Modrinth response body failed; retrying"
                                );
                            }
                            tokio::time::sleep(fetch_retry_delay(
                                total_attempts,
                            ))
                            .await;
                            continue;
                        }
                        break;
                    }
                }
                Err(err) => {
                    drop(permit);
                    record_route_failure(route);
                    let error_message = err.to_string();
                    last_error = Some(err.into());
                    if has_next_route {
                        if modrinth_request_kind.is_some() {
                            tracing::warn!(
                                request_target,
                                source = route_source.as_str(),
                                url = %log_request_url,
                                attempt,
                                max_attempts,
                                elapsed_ms = started.elapsed().as_millis(),
                                error = %error_message,
                                "Modrinth mirror connection failed; falling back to official source"
                            );
                        } else {
                            tracing::warn!(
                                url = %log_request_url,
                                error = %error_message,
                                "Mirror connection failed; falling back to official source"
                            );
                        }
                        break;
                    }
                    if has_more_attempts {
                        if modrinth_request_kind.is_some() {
                            tracing::warn!(
                                request_target,
                                source = route_source.as_str(),
                                url = %log_request_url,
                                attempt,
                                max_attempts,
                                elapsed_ms = started.elapsed().as_millis(),
                                error = %error_message,
                                "Modrinth connection failed; retrying"
                            );
                        } else {
                            tracing::debug!(
                                attempt,
                                url = %log_request_url,
                                error = %error_message,
                                "Fetch failed; retrying"
                            );
                        }
                        tokio::time::sleep(fetch_retry_delay(total_attempts))
                            .await;
                        continue;
                    }
                    break;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        ErrorKind::OtherError(format!(
            "Unable to download {url} from any source"
        ))
        .into()
    }))
}

#[derive(Default)]
struct IntegrityHashers {
    sha1: Option<sha1_smol::Sha1>,
    sha512: Option<Sha512>,
    sha256: Option<Sha256>,
    md5: Option<md5::Context>,
}

#[derive(Default)]
struct ComputedIntegrity {
    size: u64,
    sha1: Option<String>,
    sha512: Option<String>,
    sha256: Option<String>,
    md5: Option<String>,
}

impl IntegrityHashers {
    fn new(integrity: &Integrity) -> Self {
        Self {
            sha1: integrity.sha1.as_ref().map(|_| sha1_smol::Sha1::new()),
            sha512: integrity.sha512.as_ref().map(|_| Sha512::new()),
            sha256: integrity.sha256.as_ref().map(|_| Sha256::new()),
            md5: integrity.md5.as_ref().map(|_| md5::Context::new()),
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        if let Some(hasher) = &mut self.sha1 {
            hasher.update(bytes);
        }
        if let Some(hasher) = &mut self.sha512 {
            hasher.update(bytes);
        }
        if let Some(hasher) = &mut self.sha256 {
            hasher.update(bytes);
        }
        if let Some(hasher) = &mut self.md5 {
            hasher.consume(bytes);
        }
    }

    fn finish(self, size: u64) -> ComputedIntegrity {
        ComputedIntegrity {
            size,
            sha1: self.sha1.map(|hasher| hasher.digest().to_string()),
            sha512: self
                .sha512
                .map(|hasher| format!("{:x}", hasher.finalize())),
            sha256: self
                .sha256
                .map(|hasher| format!("{:x}", hasher.finalize())),
            md5: self.md5.map(|hasher| format!("{:x}", hasher.finalize())),
        }
    }
}

fn suffixed_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

async fn remove_if_exists(path: &Path) -> crate::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(IOError::with_path(error, path).into()),
    }
}

async fn compute_file_integrity(
    path: &Path,
    integrity: &Integrity,
) -> crate::Result<ComputedIntegrity> {
    let mut file = File::open(path)
        .await
        .map_err(|error| IOError::with_path(error, path))?;
    let mut hashers = IntegrityHashers::new(integrity);
    let mut size = 0;
    let mut buffer = vec![0_u8; 256 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|error| IOError::with_path(error, path))?;
        if read == 0 {
            break;
        }
        hashers.update(&buffer[..read]);
        size += read as u64;
    }
    Ok(hashers.finish(size))
}

fn verify_computed_integrity(
    expected: &Integrity,
    actual: &ComputedIntegrity,
) -> crate::Result<()> {
    if let Some(size) = expected.size
        && actual.size != size
    {
        return Err(ErrorKind::OtherError(format!(
            "Incorrect size for download: {size} != {}",
            actual.size
        ))
        .into());
    }

    let checks = [
        ("sha1", expected.sha1.as_ref(), actual.sha1.as_ref()),
        ("sha512", expected.sha512.as_ref(), actual.sha512.as_ref()),
        ("sha256", expected.sha256.as_ref(), actual.sha256.as_ref()),
        ("md5", expected.md5.as_ref(), actual.md5.as_ref()),
    ];
    for (algorithm, expected, actual) in checks {
        if let Some(expected) = expected
            && actual
                .is_none_or(|actual| !actual.eq_ignore_ascii_case(expected))
        {
            return Err(ErrorKind::OtherError(format!(
                "Incorrect {algorithm} hash for download: {expected} != {}",
                actual.map(String::as_str).unwrap_or("not computed")
            ))
            .into());
        }
    }
    Ok(())
}

async fn validate_file_content(
    path: &Path,
    validation: ContentValidation,
) -> crate::Result<()> {
    if validation == ContentValidation::None {
        return Ok(());
    }
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || -> crate::Result<()> {
        let file = std::fs::File::open(&path)
            .map_err(|error| IOError::with_path(error, &path))?;
        match validation {
            ContentValidation::None => {}
            ContentValidation::Json => {
                serde_json::from_reader::<_, serde_json::Value>(file)?;
            }
            ContentValidation::Jar => {
                zip::ZipArchive::new(file).map_err(|error| {
                    ErrorKind::OtherError(format!(
                        "Invalid JAR archive {}: {error}",
                        path.display()
                    ))
                })?;
            }
        }
        Ok(())
    })
    .await??;
    Ok(())
}

async fn verify_file(path: &Path, integrity: &Integrity) -> crate::Result<u64> {
    let computed = compute_file_integrity(path, integrity).await?;
    verify_computed_integrity(integrity, &computed)?;
    validate_file_content(path, integrity.content).await?;
    Ok(computed.size)
}

fn integrity_cache_key(integrity: &Integrity) -> Option<(&'static str, &str)> {
    if let Some(hash) = &integrity.sha512 {
        Some(("sha512", hash))
    } else if let Some(hash) = &integrity.sha256 {
        Some(("sha256", hash))
    } else if let Some(hash) = &integrity.sha1 {
        Some(("sha1", hash))
    } else if let Some(hash) = &integrity.md5 {
        Some(("md5", hash))
    } else {
        None
    }
}

fn download_lock_key(destination: &Path, integrity: &Integrity) -> String {
    integrity_cache_key(integrity).map_or_else(
        || format!("path:{}", destination.display()),
        |(algorithm, hash)| {
            format!("{algorithm}:{}", hash.to_ascii_lowercase())
        },
    )
}

fn in_flight_download_lock(key: String) -> Arc<AsyncMutex<()>> {
    use dashmap::mapref::entry::Entry;

    if IN_FLIGHT_DOWNLOADS.len() > 4_096 {
        IN_FLIGHT_DOWNLOADS.retain(|_, lock| lock.strong_count() > 0);
    }
    match IN_FLIGHT_DOWNLOADS.entry(key) {
        Entry::Occupied(mut entry) => {
            if let Some(lock) = entry.get().upgrade() {
                lock
            } else {
                let lock = Arc::new(AsyncMutex::new(()));
                entry.insert(Arc::downgrade(&lock));
                lock
            }
        }
        Entry::Vacant(entry) => {
            let lock = Arc::new(AsyncMutex::new(()));
            entry.insert(Arc::downgrade(&lock));
            lock
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParsedContentRange {
    start: u64,
    end: u64,
    total: u64,
}

fn parse_content_range(
    response: &reqwest::Response,
) -> Option<ParsedContentRange> {
    let value = response
        .headers()
        .get(header::CONTENT_RANGE)?
        .to_str()
        .ok()?
        .strip_prefix("bytes ")?;
    let (range, total) = value.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    Some(ParsedContentRange {
        start: start.parse().ok()?,
        end: end.parse().ok()?,
        total: total.parse().ok()?,
    })
}

async fn response_status_error(
    response: reqwest::Response,
    method: &Method,
    request_url: &str,
) -> crate::Error {
    let status = response.status();
    let backup_error = response.error_for_status_ref().unwrap_err();
    if let Ok(mut error) = response.json::<LabrinthError>().await {
        error.status = Some(status.as_u16());
        error.method = Some(method.as_str().to_string());
        error.url = Some(sanitize_url_for_log(request_url));
        ErrorKind::LabrinthError(error).into()
    } else {
        backup_error.into()
    }
}

async fn finalize_download(
    part_path: &Path,
    destination: &Path,
) -> crate::Result<()> {
    if tokio::fs::try_exists(destination)
        .await
        .map_err(|error| IOError::with_path(error, destination))?
    {
        remove_if_exists(destination).await?;
    }
    tokio::fs::rename(part_path, destination)
        .await
        .map_err(|error| IOError::with_path(error, destination))?;
    Ok(())
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn is_allowed_download_redirect(url: &Url) -> bool {
    if url.scheme() == "https" {
        return true;
    }
    #[cfg(test)]
    if url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|host| host == "localhost" || host == "127.0.0.1")
    {
        return true;
    }
    false
}

fn byte_range_header_value(
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

async fn wait_for_mirror_request_slot(route: &DownloadRoute) {
    if route.source != DownloadRouteSource::Bmclapi {
        return;
    }

    let mut slots = MIRROR_REQUEST_SLOTS.lock().await;
    let index = slots
        .iter()
        .enumerate()
        .min_by_key(|(_, available_at)| **available_at)
        .map(|(index, _)| index)
        .expect("mirror request slots are never empty");
    if let Some(delay) = slots[index].checked_duration_since(Instant::now()) {
        tokio::time::sleep(delay).await;
    }
    slots[index] = Instant::now() + MIRROR_REQUEST_START_INTERVAL;
}

#[allow(clippy::too_many_arguments)]
async fn send_path_request_with_clients(
    route: &DownloadRoute,
    custom_header: Option<&(String, String)>,
    credentials: Option<&crate::state::ModrinthCredentials>,
    download_meta: Option<&DownloadMeta>,
    range_start: Option<u64>,
    range_end: Option<u64>,
    system_client: &reqwest::Client,
    direct_client: &reqwest::Client,
    redirect_target: Option<&AsyncMutex<Option<Url>>>,
) -> crate::Result<(reqwest::Response, String)> {
    wait_for_mirror_request_slot(route).await;
    let client = match route.proxy {
        ProxyPolicy::System => system_client,
        ProxyPolicy::Direct => direct_client,
    };
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

        let response = request.send().await?;
        if !response.status().is_redirection() {
            if reused_redirect_target
                && (response.status().is_client_error()
                    || response.status().is_server_error())
            {
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
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                ErrorKind::OtherError(format!(
                    "Redirect from {} did not include a valid Location header",
                    current
                ))
            })?;
        if !is_safe_redirect_location(location) {
            return Err(ErrorKind::OtherError(format!(
                "Redirect from {current} included an unsafe Location header"
            ))
            .into());
        }
        let next = current.join(location)?;
        if !is_allowed_download_redirect(&next) {
            return Err(ErrorKind::OtherError(format!(
                "Refusing insecure redirect from {current} to {next}"
            ))
            .into());
        }
        current = next;
    }
    unreachable!()
}

#[allow(clippy::too_many_arguments)]
async fn send_path_request(
    route: &DownloadRoute,
    custom_header: Option<&(String, String)>,
    credentials: Option<&crate::state::ModrinthCredentials>,
    download_meta: Option<&DownloadMeta>,
    range_start: Option<u64>,
    range_end: Option<u64>,
) -> crate::Result<(reqwest::Response, String)> {
    send_path_request_with_clients(
        route,
        custom_header,
        credentials,
        download_meta,
        range_start,
        range_end,
        &NO_REDIRECT_REQWEST_CLIENT,
        &DIRECT_REQWEST_CLIENT,
        None,
    )
    .await
}

#[derive(Clone)]
struct DownloadRange {
    index: usize,
    start: u64,
    state: Arc<Mutex<DownloadRangeState>>,
}

struct DownloadRangeState {
    end: u64,
    downloaded: u64,
    active: bool,
}

impl DownloadRange {
    fn new(index: usize, start: u64, end: u64) -> Self {
        Self {
            index,
            start,
            state: Arc::new(Mutex::new(DownloadRangeState {
                end,
                downloaded: 0,
                active: true,
            })),
        }
    }

    fn end(&self) -> u64 {
        self.state.lock().end
    }

    fn remaining(&self) -> u64 {
        let state = self.state.lock();
        state
            .end
            .saturating_add(1)
            .saturating_sub(self.start.saturating_add(state.downloaded))
    }

    fn is_active(&self) -> bool {
        self.state.lock().active
    }

    fn split_tail(&self, index: usize) -> Option<Self> {
        let mut state = self.state.lock();
        let remaining = state
            .end
            .saturating_add(1)
            .saturating_sub(self.start.saturating_add(state.downloaded));
        if remaining < 256 * 1024 {
            return None;
        }
        let split_size = remaining.saturating_mul(40) / 100;
        let split_start =
            state.end.saturating_add(1).saturating_sub(split_size);
        if split_start <= self.start.saturating_add(state.downloaded) {
            return None;
        }
        let split_end = state.end;
        state.end = split_start - 1;
        drop(state);
        Some(Self::new(index, split_start, split_end))
    }

    fn accept_chunk(&self, chunk_size: usize) -> (usize, bool) {
        let mut state = self.state.lock();
        let remaining = state
            .end
            .saturating_add(1)
            .saturating_sub(self.start.saturating_add(state.downloaded));
        let accepted = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(chunk_size);
        state.downloaded += accepted as u64;
        (accepted, state.downloaded == state.end - self.start + 1)
    }

    fn finish(&self) -> bool {
        let mut state = self.state.lock();
        state.active = false;
        state.downloaded == state.end - self.start + 1
    }
}

struct DownloadRangeGuard(Arc<Mutex<DownloadRangeState>>);

impl Drop for DownloadRangeGuard {
    fn drop(&mut self) {
        self.0.lock().active = false;
    }
}

struct SegmentedDownloadSuccess {
    size: u64,
    final_url: String,
    ttfb: time::Duration,
    transfer_elapsed: time::Duration,
}

enum SegmentedDownloadOutcome {
    Success(SegmentedDownloadSuccess),
    FallbackSingle {
        disable_range: bool,
        reason: &'static str,
    },
    SourceFailed,
    Fatal(crate::Error),
}

enum SegmentDownloadError {
    Protocol(&'static str),
    Transport,
    Fatal(crate::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResourceValidator {
    etag: Option<String>,
    last_modified: Option<String>,
}

struct SegmentDownloadCompletion {
    final_url: String,
    is_first_range: bool,
    ttfb: time::Duration,
}

struct SegmentCleanupGuard {
    part_path: PathBuf,
    armed: bool,
}

impl SegmentCleanupGuard {
    fn new(part_path: &Path) -> Self {
        Self {
            part_path: part_path.to_path_buf(),
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for SegmentCleanupGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let _ = std::fs::remove_file(&self.part_path);
        for index in 0..MAX_SEGMENT_CONCURRENCY {
            let _ = std::fs::remove_file(segment_path(&self.part_path, index));
        }
    }
}

fn response_validator(response: &reqwest::Response) -> ResourceValidator {
    ResourceValidator {
        etag: response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        last_modified: response
            .headers()
            .get(header::LAST_MODIFIED)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    }
}

fn validate_resource_version(
    expected: &Mutex<Option<ResourceValidator>>,
    response: &reqwest::Response,
) -> bool {
    let candidate = response_validator(response);
    let mut expected = expected.lock();
    match expected.as_ref() {
        Some(expected) => {
            (expected.etag.is_none() || expected.etag == candidate.etag)
                && (expected.last_modified.is_none()
                    || expected.last_modified == candidate.last_modified)
        }
        None => {
            *expected = Some(candidate);
            true
        }
    }
}

fn segmented_concurrency_cap(available_permits: usize) -> usize {
    let fair_share = if available_permits > 1 {
        available_permits - 1
    } else {
        available_permits
    };
    fair_share.min(MAX_SEGMENT_CONCURRENCY)
}

fn initial_segment_count(size: u64, available_permits: usize) -> usize {
    let size_limit = usize::try_from(size / MIN_SEGMENT_SIZE)
        .unwrap_or(usize::MAX)
        .max(1);
    INITIAL_SEGMENT_CONCURRENCY
        .min(segmented_concurrency_cap(available_permits))
        .min(size_limit)
}

fn create_initial_ranges(size: u64, count: usize) -> Vec<DownloadRange> {
    let base_size = size / count as u64;
    let remainder = size % count as u64;
    let mut start = 0_u64;
    (0..count)
        .map(|index| {
            let range_size = base_size + u64::from(index < remainder as usize);
            let end = start + range_size - 1;
            let range = DownloadRange::new(index, start, end);
            start = end + 1;
            range
        })
        .collect()
}

fn expansion_block_reason(
    snapshot: SpeedSnapshot,
    active_ranges: usize,
    concurrency_cap: usize,
    available_permits: usize,
    remaining_bytes: u64,
    elapsed_since_expansion: time::Duration,
) -> Option<&'static str> {
    if active_ranges >= concurrency_cap {
        return Some("effective concurrency cap reached");
    }
    if available_permits == 0 {
        return Some("no global permit available");
    }
    if elapsed_since_expansion < SEGMENT_EXPANSION_INTERVAL {
        return Some("expansion cooldown active");
    }
    if snapshot.sample_count < SEGMENT_EXPANSION_SAMPLE_COUNT {
        return Some("insufficient aggregate speed samples");
    }
    if snapshot.aggregate_speed < snapshot.speed_floor
        || snapshot.recent_average < snapshot.speed_floor
    {
        return Some("aggregate throughput below speed floor");
    }
    if remaining_bytes
        < MIN_SEGMENT_SIZE.saturating_mul(active_ranges as u64 + 1)
    {
        return Some("too little data remains");
    }
    None
}

fn segment_path(part_path: &Path, index: usize) -> PathBuf {
    suffixed_path(part_path, &format!(".segment-{index}"))
}

async fn cleanup_segment_files(
    part_path: &Path,
    segment_count: usize,
) -> crate::Result<()> {
    for index in 0..segment_count {
        remove_if_exists(&segment_path(part_path, index)).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn download_segment(
    route: &DownloadRoute,
    range: DownloadRange,
    total_size: u64,
    custom_header: Option<&(String, String)>,
    credentials: Option<&crate::state::ModrinthCredentials>,
    download_meta: Option<&DownloadMeta>,
    part_path: &Path,
    _permit: SemaphorePermit<'_>,
    system_client: &reqwest::Client,
    direct_client: &reqwest::Client,
    progress: tokio::sync::mpsc::UnboundedSender<u64>,
    speed: &DownloadSpeedTracker,
    validator: &Mutex<Option<ResourceValidator>>,
    redirect_target: Option<&AsyncMutex<Option<Url>>>,
) -> Result<SegmentDownloadCompletion, SegmentDownloadError> {
    let _range_guard = DownloadRangeGuard(Arc::clone(&range.state));
    let request_started = Instant::now();
    let path = segment_path(part_path, range.index);
    let mut file = File::create(&path).await.map_err(|error| {
        SegmentDownloadError::Fatal(IOError::with_path(error, &path).into())
    })?;
    let mut pending_progress = 0_u64;
    let mut final_url = route.url.clone();
    for attempt in 1..=SEGMENT_RETRY_ATTEMPTS {
        let requested_start = range.start + {
            let state = range.state.lock();
            state.downloaded
        };
        let downloaded_before_attempt = requested_start - range.start;
        let requested_end = range.end();
        let response = tokio::time::timeout(
            FILE_TRANSFER_FIRST_BYTE_TIMEOUT,
            send_path_request_with_clients(
                route,
                custom_header,
                credentials,
                download_meta,
                Some(requested_start),
                Some(requested_end),
                system_client,
                direct_client,
                redirect_target,
            ),
        )
        .await;
        let (response, response_url) = match response {
            Ok(Ok(response)) => response,
            Ok(Err(_)) | Err(_) if attempt < SEGMENT_RETRY_ATTEMPTS => {
                tracing::debug!(
                    url = %sanitize_url_for_log(&route.url),
                    range_start = requested_start,
                    range_end = requested_end,
                    attempt,
                    "Range request failed temporarily; retrying"
                );
                tokio::time::sleep(fetch_retry_delay(attempt)).await;
                continue;
            }
            Ok(Err(_)) | Err(_) => return Err(SegmentDownloadError::Transport),
        };
        final_url = response_url;
        let parsed_content_range = parse_content_range(&response);
        tracing::debug!(
            path = %part_path.display(),
            original_url = %sanitize_url_for_log(&route.url),
            final_host = Url::parse(&final_url)
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned))
                .unwrap_or_default(),
            source = route.source.as_str(),
            status = response.status().as_u16(),
            content_range = ?parsed_content_range,
            range_start = requested_start,
            range_end = requested_end,
            "Received download range response"
        );
        if response.status() == StatusCode::OK {
            return Err(SegmentDownloadError::Protocol(
                "server ignored Range and returned 200",
            ));
        }
        if response.status() != StatusCode::PARTIAL_CONTENT {
            if attempt < SEGMENT_RETRY_ATTEMPTS {
                tokio::time::sleep(fetch_retry_delay(attempt)).await;
                continue;
            }
            return Err(SegmentDownloadError::Transport);
        }
        if parsed_content_range
            != Some(ParsedContentRange {
                start: requested_start,
                end: requested_end,
                total: total_size,
            })
        {
            return Err(SegmentDownloadError::Protocol(
                "invalid Content-Range",
            ));
        }
        if !validate_resource_version(validator, &response) {
            return Err(SegmentDownloadError::Protocol(
                "resource validator changed between ranges",
            ));
        }
        if range.index == 0 && requested_start == range.start {
            tracing::debug!(
                original_url = %sanitize_url_for_log(&route.url),
                final_host = Url::parse(&final_url)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_owned))
                    .unwrap_or_default(),
                file_size = total_size,
                supports_range = true,
                "Confirmed byte-range download support"
            );
        }
        let mut stream = response.bytes_stream();
        let mut last_chunk_at = Instant::now();
        let mut stream_end_reason = None;
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    stream_end_reason = Some(error.to_string());
                    break;
                }
            };
            let (accepted, completed) = range.accept_chunk(chunk.len());
            file.write_all(&chunk[..accepted]).await.map_err(|error| {
                SegmentDownloadError::Fatal(
                    IOError::with_path(error, &path).into(),
                )
            })?;
            let elapsed = last_chunk_at.elapsed();
            pending_progress += accepted as u64;
            speed.record_bytes(accepted as u64);
            if range.remaining() > 0
                && elapsed > FILE_TRANSFER_SLOW_INTERVAL
                && (accepted as u128) < elapsed.as_millis()
            {
                stream_end_reason =
                    Some("range stopped making progress".to_string());
                break;
            }
            last_chunk_at = Instant::now();
            if pending_progress >= MIN_SEGMENT_SIZE {
                let _ = progress.send(pending_progress);
                pending_progress = 0;
            }
            if completed {
                break;
            }
        }
        if range.remaining() == 0 {
            break;
        }
        if attempt < SEGMENT_RETRY_ATTEMPTS {
            let downloaded_after_attempt = {
                let state = range.state.lock();
                state.downloaded
            };
            tracing::warn!(
                url = %sanitize_url_for_log(&route.url),
                range_start = requested_start,
                range_end = range.end(),
                attempt,
                received_bytes = downloaded_after_attempt
                    .saturating_sub(downloaded_before_attempt),
                remaining_bytes = range.remaining(),
                reason = stream_end_reason
                    .as_deref()
                    .unwrap_or("response ended before Content-Range boundary"),
                "Range stream ended early; resuming remaining bytes"
            );
            tokio::time::sleep(fetch_retry_delay(attempt)).await;
            continue;
        }
        return Err(SegmentDownloadError::Transport);
    }
    if pending_progress > 0 {
        let _ = progress.send(pending_progress);
    }
    file.flush().await.map_err(|error| {
        SegmentDownloadError::Fatal(IOError::with_path(error, &path).into())
    })?;
    drop(file);
    if !range.finish() {
        return Err(SegmentDownloadError::Protocol(
            "range response ended before expected boundary",
        ));
    }
    Ok(SegmentDownloadCompletion {
        final_url,
        is_first_range: range.index == 0,
        ttfb: request_started.elapsed(),
    })
}

#[allow(clippy::too_many_arguments)]
async fn try_segmented_download(
    request: &DownloadRequest,
    route: &DownloadRoute,
    size: u64,
    part_path: &Path,
    semaphore: &FetchSemaphore,
    credentials: Option<&crate::state::ModrinthCredentials>,
    mut progress: Option<&mut FetchProgressFn<'_>>,
    system_client: &reqwest::Client,
    direct_client: &reqwest::Client,
) -> SegmentedDownloadOutcome {
    if let Err(error) = cleanup_segment_files(part_path, 256).await {
        return SegmentedDownloadOutcome::Fatal(error);
    }
    let mut cleanup_guard = SegmentCleanupGuard::new(part_path);
    let available_at_start = semaphore.0.available_permits();
    let concurrency_cap = segmented_concurrency_cap(available_at_start);
    let requested_initial_count =
        initial_segment_count(size, available_at_start);
    let mut permits = Vec::with_capacity(requested_initial_count);
    for _ in 0..requested_initial_count {
        match semaphore.0.try_acquire() {
            Ok(permit) => permits.push(permit),
            Err(_) => break,
        }
    }
    if permits.len() < 2 {
        tracing::debug!(
            original_url = %sanitize_url_for_log(&route.url),
            file_size = size,
            supports_range = true,
            available_permits = semaphore.0.available_permits(),
            "Falling back to a single connection because initial segmented permits are unavailable"
        );
        return SegmentedDownloadOutcome::FallbackSingle {
            disable_range: false,
            reason: "fewer than two fair global permits available",
        };
    }
    let transfer_started = Instant::now();
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let speed = DownloadSpeedTracker::default();
    let validator = Mutex::new(None);
    let redirect_target = AsyncMutex::new(None);
    let mut downloads = futures::stream::FuturesUnordered::new();
    let mut ranges = create_initial_ranges(size, permits.len());
    for (range, permit) in ranges.iter().cloned().zip(permits) {
        downloads.push(download_segment(
            route,
            range,
            size,
            request.header.as_ref(),
            credentials,
            request.download_meta.as_ref(),
            part_path,
            permit,
            system_client,
            direct_client,
            progress_tx.clone(),
            &speed,
            &validator,
            (route.source == DownloadRouteSource::Mcim)
                .then_some(&redirect_target),
        ));
    }
    tracing::debug!(
        original_url = %sanitize_url_for_log(&route.url),
        file_size = size,
        supports_range = true,
        active_ranges = downloads.len(),
        concurrency_cap,
        available_permits = semaphore.0.available_permits(),
        reason = "initial parallel ranges",
        "Started segmented download"
    );
    let mut next_range_index = ranges.len();
    let mut scheduler = tokio::time::interval(time::Duration::from_millis(250));
    scheduler.tick().await;
    let mut last_expansion = Instant::now();
    let mut last_block_reason = None;
    let mut downloaded = 0_u64;
    let mut segment_error = None;
    let mut final_url = None;
    let mut initial_ttfb = None;
    while !downloads.is_empty() {
        tokio::select! {
            Some(delta) = progress_rx.recv() => {
                downloaded = downloaded.saturating_add(delta);
                if let Some(progress) = progress.as_mut()
                    && let Err(error) = progress(downloaded, size).await
                {
                    segment_error = Some(SegmentDownloadError::Fatal(error));
                    break;
                }
            }
            result = downloads.next() => {
                if let Some(result) = result {
                    match result {
                        Ok(completion) if completion.is_first_range => {
                            final_url = Some(completion.final_url);
                            initial_ttfb = Some(completion.ttfb);
                        }
                        Ok(_) => {}
                        Err(error) => {
                            segment_error = Some(error);
                            break;
                        }
                    }
                }
            }
            _ = scheduler.tick() => {
                let snapshot = speed.speed_snapshot();
                let active_ranges = downloads.len();
                let remaining_bytes = ranges
                    .iter()
                    .filter(|range| range.is_active())
                    .map(DownloadRange::remaining)
                    .sum();
                if let Some(reason) = expansion_block_reason(
                    snapshot,
                    active_ranges,
                    concurrency_cap,
                    semaphore.0.available_permits(),
                    remaining_bytes,
                    last_expansion.elapsed(),
                ) {
                    if last_block_reason != Some(reason) {
                        tracing::debug!(
                            original_url = %sanitize_url_for_log(&route.url),
                            active_ranges,
                            aggregate_speed = snapshot.aggregate_speed,
                            recent_average = snapshot.recent_average,
                            floor = snapshot.speed_floor,
                            remaining_bytes,
                            available_permits = semaphore.0.available_permits(),
                            reason,
                            "Segmented download did not increase concurrency"
                        );
                        last_block_reason = Some(reason);
                    }
                    continue;
                }
                let range = ranges
                    .iter()
                    .filter(|range| range.is_active())
                    .max_by_key(|range| range.remaining())
                    .cloned();
                if let Some(range) = range
                    && let Ok(permit) = semaphore.0.try_acquire()
                {
                    if let Some(new_range) =
                        range.split_tail(next_range_index)
                    {
                        tracing::debug!(
                            original_url = %sanitize_url_for_log(&route.url),
                            source = route.source.as_str(),
                            active_ranges = active_ranges + 1,
                            aggregate_speed = snapshot.aggregate_speed,
                            recent_average = snapshot.recent_average,
                            floor = snapshot.speed_floor,
                            range_start = new_range.start,
                            range_end = new_range.end(),
                            reason = "stable aggregate throughput above floor",
                            "Starting an additional download range"
                        );
                        next_range_index += 1;
                        downloads.push(download_segment(
                            route,
                            new_range.clone(),
                            size,
                            request.header.as_ref(),
                            credentials,
                            None,
                            part_path,
                            permit,
                            system_client,
                            direct_client,
                            progress_tx.clone(),
                            &speed,
                            &validator,
                            (route.source == DownloadRouteSource::Mcim)
                                .then_some(&redirect_target),
                        ));
                        ranges.push(new_range);
                        last_expansion = Instant::now();
                        last_block_reason = None;
                    }
                }
            }
        }
    }
    drop(progress_tx);
    drop(downloads);
    while let Ok(delta) = progress_rx.try_recv() {
        downloaded = downloaded.saturating_add(delta);
    }
    if let Some(error) = segment_error {
        let _ = cleanup_segment_files(part_path, 256).await;
        return match error {
            SegmentDownloadError::Protocol(reason) => {
                SegmentedDownloadOutcome::FallbackSingle {
                    disable_range: true,
                    reason,
                }
            }
            SegmentDownloadError::Transport => {
                SegmentedDownloadOutcome::SourceFailed
            }
            SegmentDownloadError::Fatal(error) => {
                SegmentedDownloadOutcome::Fatal(error)
            }
        };
    }

    ranges.sort_unstable_by_key(|range| range.start);
    let mut output = match File::create(part_path).await {
        Ok(file) => file,
        Err(error) => {
            return SegmentedDownloadOutcome::Fatal(
                IOError::with_path(error, part_path).into(),
            );
        }
    };
    let mut hashers = IntegrityHashers::new(&request.integrity);
    let mut merged_size = 0_u64;
    let mut buffer = vec![0_u8; 256 * 1024];
    for range in &ranges {
        let path = segment_path(part_path, range.index);
        let mut segment = match File::open(&path).await {
            Ok(file) => file,
            Err(error) => {
                return SegmentedDownloadOutcome::Fatal(
                    IOError::with_path(error, &path).into(),
                );
            }
        };
        loop {
            let read = match segment.read(&mut buffer).await {
                Ok(read) => read,
                Err(error) => {
                    return SegmentedDownloadOutcome::Fatal(
                        IOError::with_path(error, &path).into(),
                    );
                }
            };
            if read == 0 {
                break;
            }
            if let Err(error) = output.write_all(&buffer[..read]).await {
                return SegmentedDownloadOutcome::Fatal(
                    IOError::with_path(error, part_path).into(),
                );
            }
            hashers.update(&buffer[..read]);
            merged_size += read as u64;
        }
        if let Err(error) = remove_if_exists(&path).await {
            return SegmentedDownloadOutcome::Fatal(error);
        }
    }
    if let Err(error) = output.flush().await {
        return SegmentedDownloadOutcome::Fatal(
            IOError::with_path(error, part_path).into(),
        );
    }
    drop(output);
    if merged_size != size {
        let _ = remove_if_exists(part_path).await;
        return SegmentedDownloadOutcome::FallbackSingle {
            disable_range: true,
            reason: "merged segment size mismatch",
        };
    }
    let computed = hashers.finish(merged_size);
    if verify_computed_integrity(&request.integrity, &computed).is_err() {
        let _ = remove_if_exists(part_path).await;
        return SegmentedDownloadOutcome::FallbackSingle {
            disable_range: true,
            reason: "segmented integrity validation failed",
        };
    }
    if validate_file_content(part_path, request.integrity.content)
        .await
        .is_err()
    {
        let _ = remove_if_exists(part_path).await;
        return SegmentedDownloadOutcome::FallbackSingle {
            disable_range: true,
            reason: "segmented content validation failed",
        };
    }
    if downloaded < size
        && let Some(progress) = progress.as_mut()
        && let Err(error) = progress(size, size).await
    {
        return SegmentedDownloadOutcome::Fatal(error);
    }
    cleanup_guard.disarm();
    SegmentedDownloadOutcome::Success(SegmentedDownloadSuccess {
        size: merged_size,
        final_url: final_url.unwrap_or_else(|| route.url.clone()),
        ttfb: initial_ttfb.unwrap_or_default(),
        transfer_elapsed: transfer_started.elapsed(),
    })
}

/// Streams a download to a sibling `.part` file, verifies it, then atomically
/// moves it into place.
#[tracing::instrument(skip(semaphore, _exec, progress, request, destination))]
pub async fn download_to_path(
    request: DownloadRequest,
    destination: impl AsRef<Path>,
    semaphore: &FetchSemaphore,
    _exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    mut progress: Option<&mut FetchProgressFn<'_>>,
) -> crate::Result<DownloadResult> {
    let destination = destination.as_ref();
    if let Some(parent) = destination.parent() {
        io::create_dir_all(parent).await?;
    }
    let lock_key = download_lock_key(destination, &request.integrity);
    let download_lock = in_flight_download_lock(lock_key);
    let _download_guard = download_lock.lock().await;
    let mode = source_mode_for_resource(request.resource);
    let mut routes = {
        let mut urls = Vec::with_capacity(request.candidate_urls.len() + 1);
        urls.push(request.url.clone());
        urls.extend(request.candidate_urls.iter().cloned());
        let mut routes = Vec::new();
        for (index, url) in urls.into_iter().enumerate() {
            let mut candidates =
                resolve_download_routes_for(&url, request.resource, mode);
            if index > 0 {
                for candidate in &mut candidates {
                    if !candidate.is_mirror {
                        candidate.source = DownloadRouteSource::Alternate;
                        candidate.allow_sensitive_headers = false;
                    }
                }
            }
            for candidate in candidates {
                if !routes.iter().any(|existing: &DownloadRoute| {
                    existing.url == candidate.url
                        && existing.proxy == candidate.proxy
                }) {
                    routes.push(candidate);
                }
            }
        }
        routes
    };
    let credentials: Option<crate::state::ModrinthCredentials> = None;
    if request
        .header
        .as_ref()
        .is_some_and(|(name, _)| header_requires_official_only(name))
    {
        routes.retain(|route| !route.is_mirror);
    }
    if routes.is_empty() {
        routes.push(official_route(&request.url, request.resource));
    }
    let part_path = suffixed_path(destination, ".part");

    if !request.integrity.is_empty()
        && tokio::fs::try_exists(destination)
            .await
            .map_err(|error| IOError::with_path(error, destination))?
        && let Ok(size) = verify_file(destination, &request.integrity).await
    {
        let route = routes
            .first()
            .cloned()
            .unwrap_or_else(|| official_route(&request.url, request.resource));
        remove_if_exists(&part_path).await?;
        return Ok(DownloadResult {
            path: destination.to_path_buf(),
            url: route.url,
            source: route.source,
            size,
            attempts: 0,
            fallback_count: 0,
        });
    }
    remove_if_exists(&part_path).await?;

    let mut attempts = 0;
    let mut last_error = None;
    let mut fallback_count = 0;
    let file_attempt_budget = routes.len().saturating_mul(2).max(1);
    for retry_with_single_thread in [false, true] {
        for (route_index, route) in routes.iter().enumerate() {
            let log_url = sanitize_url_for_log(&route.url);
            if route_index > 0 {
                fallback_count += 1;
                remove_if_exists(&part_path).await?;
            }
            while attempts < file_attempt_budget {
                attempts += 1;
                tracing::debug!(
                    path = %destination.display(),
                    temporary_path = %part_path.display(),
                    url = %log_url,
                    source = route.source.as_str(),
                    expected_bytes = request.integrity.size,
                    attempt = attempts,
                    max_attempts = file_attempt_budget,
                    "Starting file download attempt"
                );
                if !retry_with_single_thread
                    && route.supports_range
                    && range_splitting_allowed(route)
                    && request.integrity.size.is_some_and(|size| {
                        size >= SEGMENTED_DOWNLOAD_THRESHOLD
                    })
                {
                    let size = request.integrity.size.unwrap();
                    match try_segmented_download(
                        &request,
                        route,
                        size,
                        &part_path,
                        semaphore,
                        credentials.as_ref(),
                        progress.as_deref_mut(),
                        &NO_REDIRECT_REQWEST_CLIENT,
                        &DIRECT_REQWEST_CLIENT,
                    )
                    .await
                    {
                        SegmentedDownloadOutcome::Success(result) => {
                            finalize_download(&part_path, destination).await?;
                            record_route_success(
                                route,
                                result.ttfb,
                                result.size,
                                result.transfer_elapsed,
                            );
                            tracing::debug!(
                                path = %destination.display(),
                                url = %sanitize_url_for_log(&result.final_url),
                                source = route.source.as_str(),
                                bytes = result.size,
                                elapsed_ms = result.transfer_elapsed.as_millis(),
                                attempt = attempts,
                                max_attempts = file_attempt_budget,
                                "Completed file download"
                            );
                            return Ok(DownloadResult {
                                path: destination.to_path_buf(),
                                url: result.final_url,
                                source: route.source,
                                size: result.size,
                                attempts,
                                fallback_count,
                            });
                        }
                        SegmentedDownloadOutcome::FallbackSingle {
                            disable_range,
                            reason,
                        } => {
                            tracing::debug!(
                                original_url = %log_url,
                                file_size = size,
                                supports_range = route.supports_range,
                                reason,
                                "Falling back to a single connection"
                            );
                            if disable_range {
                                disable_range_splitting(route);
                            }
                            remove_if_exists(&part_path).await?;
                        }
                        SegmentedDownloadOutcome::SourceFailed => {
                            record_route_failure(route);
                            last_error = Some(
                                ErrorKind::OtherError(format!(
                                    "File transfer failed from {}",
                                    log_url
                                ))
                                .into(),
                            );
                            tracing::warn!(
                                path = %destination.display(),
                                url = %log_url,
                                source = route.source.as_str(),
                                attempt = attempts,
                                max_attempts = file_attempt_budget,
                                "Segmented file download failed; retrying or switching source"
                            );
                            remove_if_exists(&part_path).await?;
                            break;
                        }
                        SegmentedDownloadOutcome::Fatal(error) => {
                            return Err(error);
                        }
                    }
                }

                let permit = semaphore.0.acquire().await?;
                let request_started = Instant::now();
                let (response, final_url) = match tokio::time::timeout(
                    FILE_TRANSFER_FIRST_BYTE_TIMEOUT,
                    send_path_request(
                        route,
                        request.header.as_ref(),
                        credentials.as_ref(),
                        request.download_meta.as_ref(),
                        None,
                        None,
                    ),
                )
                .await
                {
                    Ok(Ok(response)) => response,
                    Ok(Err(error)) => {
                        drop(permit);
                        record_route_failure(route);
                        tracing::warn!(
                            path = %destination.display(),
                            url = %log_url,
                            source = route.source.as_str(),
                            attempt = attempts,
                            max_attempts = file_attempt_budget,
                            error = %error,
                            "File download request failed; trying the next source or retry"
                        );
                        last_error = Some(error);
                        break;
                    }
                    Err(_) => {
                        drop(permit);
                        record_route_failure(route);
                        let error = ErrorKind::NetworkError(format!(
                            "no response received for {:.0} seconds while downloading {log_url} to {}",
                            FILE_TRANSFER_FIRST_BYTE_TIMEOUT.as_secs_f64(),
                            destination.display(),
                        ))
                        .into();
                        tracing::warn!(
                            path = %destination.display(),
                            url = %log_url,
                            source = route.source.as_str(),
                            no_data_seconds = FILE_TRANSFER_FIRST_BYTE_TIMEOUT.as_secs_f64(),
                            downloaded_bytes = 0,
                            attempt = attempts,
                            max_attempts = file_attempt_budget,
                            "File download stalled before receiving a response"
                        );
                        last_error = Some(error);
                        break;
                    }
                };
                let ttfb = request_started.elapsed();
                let status = response.status();
                tracing::debug!(
                    path = %destination.display(),
                    url = %log_url,
                    source = route.source.as_str(),
                    status = status.as_u16(),
                    content_length = response.content_length(),
                    ttfb_ms = ttfb.as_millis(),
                    "Received file download response"
                );
                if status.is_client_error() || status.is_server_error() {
                    record_route_failure(route);
                    let error = response_status_error(
                        response,
                        &Method::GET,
                        &route.url,
                    )
                    .await;
                    drop(permit);
                    last_error = Some(error);
                    break;
                }

                let starting_size = 0_u64;
                let mut hashers = IntegrityHashers::new(&request.integrity);
                let mut file = File::create(&part_path)
                    .await
                    .map_err(|error| IOError::with_path(error, &part_path))?;
                let response_length = response.content_length().unwrap_or(0);
                let total_size = request
                    .integrity
                    .size
                    .unwrap_or(starting_size.saturating_add(response_length));
                let transfer_started = Instant::now();
                let mut downloaded = starting_size;
                let mut stream = response.bytes_stream();
                let mut transfer_error: Option<crate::Error> = None;
                let mut last_chunk_at = Instant::now();
                while let Some(item) = stream.next().await {
                    let chunk = match item {
                        Ok(chunk) => chunk,
                        Err(error) => {
                            transfer_error = Some(error.into());
                            break;
                        }
                    };
                    file.write_all(&chunk).await.map_err(|error| {
                        IOError::with_path(error, &part_path)
                    })?;
                    hashers.update(&chunk);
                    let elapsed = last_chunk_at.elapsed();
                    downloaded += chunk.len() as u64;
                    if !retry_with_single_thread
                        && downloaded > starting_size + chunk.len() as u64
                        && elapsed > FILE_TRANSFER_SLOW_INTERVAL
                        && (chunk.len() as u128) < elapsed.as_millis()
                    {
                        tracing::warn!(
                            path = %destination.display(),
                            url = %log_url,
                            bytes = chunk.len(),
                            elapsed_ms = elapsed.as_millis(),
                            "Ending a stalled file download"
                        );
                        transfer_error = Some(
                            ErrorKind::OtherError(
                                "file transfer stalled".to_string(),
                            )
                            .into(),
                        );
                        break;
                    }
                    last_chunk_at = Instant::now();
                    if let Some(progress) = progress.as_mut() {
                        progress(downloaded, total_size).await?;
                    }
                }
                file.flush()
                    .await
                    .map_err(|error| IOError::with_path(error, &part_path))?;
                drop(file);
                drop(permit);

                if let Some(error) = transfer_error {
                    record_route_failure(route);
                    remove_if_exists(&part_path).await?;
                    tracing::warn!(
                        path = %destination.display(),
                        url = %log_url,
                        source = route.source.as_str(),
                        attempt = attempts,
                        max_attempts = file_attempt_budget,
                        error = %error,
                        "File download attempt failed; trying the next source or retry"
                    );
                    last_error = Some(error);
                    break;
                }

                let computed = hashers.finish(downloaded);
                if let Err(error) =
                    verify_computed_integrity(&request.integrity, &computed)
                {
                    record_route_failure(route);
                    remove_if_exists(&part_path).await?;
                    last_error = Some(error);
                    break;
                }
                if let Err(error) =
                    validate_file_content(&part_path, request.integrity.content)
                        .await
                {
                    record_route_failure(route);
                    remove_if_exists(&part_path).await?;
                    last_error = Some(error);
                    break;
                }

                finalize_download(&part_path, destination).await?;
                record_route_success(
                    route,
                    ttfb,
                    downloaded.saturating_sub(starting_size),
                    transfer_started.elapsed(),
                );
                let log_final_url = sanitize_url_for_log(&final_url);
                tracing::debug!(
                    path = %destination.display(),
                    url = %log_final_url,
                    source = route.source.as_str(),
                    bytes = downloaded.saturating_sub(starting_size),
                    elapsed_ms = transfer_started.elapsed().as_millis(),
                    "Completed file download"
                );
                return Ok(DownloadResult {
                    path: destination.to_path_buf(),
                    url: final_url,
                    source: route.source,
                    size: downloaded,
                    attempts,
                    fallback_count,
                });
            }
        }
    }

    remove_if_exists(&part_path).await?;
    Err(last_error.unwrap_or_else(|| {
        ErrorKind::OtherError(format!(
            "Unable to download {} from any source",
            sanitize_url_for_log(&request.url)
        ))
        .into()
    }))
}

/// Downloads a file from specified mirrors
#[tracing::instrument(skip_all)]
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

    let route_count = mirrors.len().min(METADATA_ATTEMPT_BUDGET);
    let mut last_error = None;
    for (index, mirror) in mirrors.iter().take(route_count).enumerate() {
        let attempt_budget = if index + 1 == route_count {
            METADATA_ATTEMPT_BUDGET - index
        } else {
            1
        };
        match fetch_advanced_with_client_and_progress(
            Method::GET,
            mirror,
            sha1,
            None,
            None,
            download_meta,
            None,
            uri_path,
            semaphore,
            exec,
            &REQWEST_CLIENT,
            None,
            None,
            attempt_budget,
        )
        .await
        {
            Ok(bytes) => return Ok(bytes),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        ErrorKind::OtherError("Unable to download from any mirror".to_string())
            .into()
    }))
}

/// Posts a JSON to a URL
#[tracing::instrument(skip_all)]
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
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    static RANGE_SPLITTING_TEST_LOCK: LazyLock<AsyncMutex<()>> =
        LazyLock::new(|| AsyncMutex::new(()));
    static MIRROR_REQUEST_SLOT_TEST_LOCK: LazyLock<AsyncMutex<()>> =
        LazyLock::new(|| AsyncMutex::new(()));
    static AUTO_SOURCE_TEST_LOCK: LazyLock<std::sync::Mutex<()>> =
        LazyLock::new(|| std::sync::Mutex::new(()));

    async fn spawn_range_server(
        data: Arc<Vec<u8>>,
        wrong_content_range: bool,
        ignore_range: bool,
        slow_body: bool,
        fail_first_range: bool,
    ) -> (
        String,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let request_count = requests.clone();
        let normal_requests = Arc::new(AtomicUsize::new(0));
        let normal_request_count = normal_requests.clone();
        let failed_range = Arc::new(AtomicBool::new(false));
        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let data = data.clone();
                let requests = request_count.clone();
                let normal_requests = normal_request_count.clone();
                let failed_range = Arc::clone(&failed_range);
                tokio::spawn(async move {
                    requests.fetch_add(1, Ordering::Relaxed);
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let Ok(read) = stream.read(&mut buffer).await else {
                            return;
                        };
                        if read == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..read]);
                        if request
                            .windows(4)
                            .any(|window| window == b"\r\n\r\n")
                        {
                            break;
                        }
                    }
                    let request =
                        String::from_utf8_lossy(&request).to_ascii_lowercase();
                    let requested_range = request
                        .lines()
                        .find_map(|line| line.strip_prefix("range: bytes="));
                    let (headers, body) = if let Some(range) =
                        requested_range.filter(|_| !ignore_range)
                    {
                        let Some((start, end)) = range.split_once('-') else {
                            return;
                        };
                        let Ok(start) = start.parse::<u64>() else {
                            return;
                        };
                        let end = if end.is_empty() {
                            data.len() as u64 - 1
                        } else {
                            let Ok(end) = end.parse::<u64>() else {
                                return;
                            };
                            end
                        };
                        let body = &data[start as usize..=end as usize];
                        let reported_start = if wrong_content_range {
                            start.saturating_add(1)
                        } else {
                            start
                        };
                        (
                            format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {reported_start}-{end}/{}\r\nETag: \"fixture\"\r\nConnection: close\r\n\r\n",
                                body.len(),
                                data.len(),
                            ),
                            body,
                        )
                    } else {
                        if requested_range.is_none() {
                            normal_requests.fetch_add(1, Ordering::Relaxed);
                        }
                        (
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"fixture\"\r\nConnection: close\r\n\r\n",
                                data.len(),
                            ),
                            &data[..],
                        )
                    };
                    if stream.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    if requested_range.is_some()
                        && fail_first_range
                        && !failed_range.swap(true, Ordering::Relaxed)
                    {
                        let midpoint = body.len() / 2;
                        let _ = stream.write_all(&body[..midpoint]).await;
                        return;
                    }
                    for chunk in body.chunks(64 * 1024) {
                        if stream.write_all(chunk).await.is_err() {
                            return;
                        }
                        if slow_body {
                            tokio::time::sleep(time::Duration::from_millis(
                                100,
                            ))
                            .await;
                        }
                    }
                });
            }
        });
        (
            format!("http://{address}/file"),
            requests,
            normal_requests,
            handle,
        )
    }

    async fn spawn_stream_server(
        chunks: Vec<(Duration, Vec<u8>)>,
        content_length: usize,
        hold_open: bool,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let chunks = Arc::new(chunks);
        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let chunks = Arc::clone(&chunks);
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let Ok(read) = stream.read(&mut buffer).await else {
                            return;
                        };
                        if read == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..read]);
                        if request
                            .windows(4)
                            .any(|window| window == b"\r\n\r\n")
                        {
                            break;
                        }
                    }
                    let headers = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n"
                    );
                    if stream.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    for (delay, chunk) in chunks.iter() {
                        tokio::time::sleep(*delay).await;
                        if stream.write_all(chunk).await.is_err() {
                            return;
                        }
                    }
                    if hold_open {
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                });
            }
        });
        (format!("http://{address}/file"), handle)
    }

    async fn spawn_redirect_server(
        location: String,
        response_delay: Duration,
    ) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let request_count = Arc::clone(&requests);
        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let location = location.clone();
                let requests = Arc::clone(&request_count);
                tokio::spawn(async move {
                    requests.fetch_add(1, Ordering::Relaxed);
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let Ok(read) = stream.read(&mut buffer).await else {
                            return;
                        };
                        if read == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..read]);
                        if request
                            .windows(4)
                            .any(|window| window == b"\r\n\r\n")
                        {
                            break;
                        }
                    }
                    tokio::time::sleep(response_delay).await;
                    let response = format!(
                        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });
        (format!("http://{address}/redirect"), requests, handle)
    }

    async fn test_download(
        url: &str,
        destination: &Path,
        expected_size: u64,
    ) -> crate::Result<DownloadResult> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();
        download_to_path(
            DownloadRequest::new(url, ResourceClass::Other)
                .with_integrity(Integrity::default().with_size(expected_size)),
            destination,
            &FetchSemaphore(Semaphore::new(2)),
            &pool,
            None,
        )
        .await
    }
    #[test]
    fn modrinth_requests_are_classified_for_logging() {
        assert_eq!(
            modrinth_request_kind("https://api.modrinth.com/v2/project"),
            Some("API")
        );
        assert_eq!(
            modrinth_request_kind(
                "https://cdn.modrinth.com/data/project/version/file.jar"
            ),
            Some("CDN")
        );
        assert_eq!(modrinth_request_kind("https://example.com/file.jar"), None);
    }

    #[tokio::test]
    async fn file_download_times_out_when_a_successful_response_stalls() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let (url, server) = spawn_stream_server(Vec::new(), 4, true).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("stalled.bin");

        let error = test_download(&url, &destination, 4).await.unwrap_err();

        assert!(matches!(
            error.raw.as_ref(),
            ErrorKind::NetworkError(_) | ErrorKind::FetchError(_)
        ));
        assert!(!destination.exists());
        assert!(!suffixed_path(&destination, ".part").exists());
        server.abort();
    }

    #[tokio::test]
    async fn file_download_allows_data_that_keeps_arriving_slowly() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let chunks = (0..4)
            .map(|byte| (Duration::from_millis(100), vec![byte]))
            .collect();
        let (url, server) = spawn_stream_server(chunks, 4, false).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("slow.bin");

        let result = test_download(&url, &destination, 4).await.unwrap();

        assert_eq!(result.size, 4);
        assert_eq!(tokio::fs::read(&destination).await.unwrap(), [0, 1, 2, 3]);
        server.abort();
    }

    #[tokio::test]
    async fn file_download_completes_normally() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let (url, server) = spawn_stream_server(
            vec![(Duration::ZERO, b"done".to_vec())],
            4,
            false,
        )
        .await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("complete.bin");

        let result = test_download(&url, &destination, 4).await.unwrap();

        assert_eq!(result.size, 4);
        assert_eq!(tokio::fs::read(&destination).await.unwrap(), b"done");
        server.abort();
    }

    #[tokio::test]
    async fn canceling_a_file_download_drops_the_response_read_promptly() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let (url, server) = spawn_stream_server(Vec::new(), 4, true).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("canceled.bin");
        let task =
            tokio::spawn(
                async move { test_download(&url, &destination, 4).await },
            );

        tokio::time::sleep(Duration::from_millis(50)).await;
        task.abort();
        let join_error = tokio::time::timeout(Duration::from_millis(100), task)
            .await
            .expect("canceled download should stop promptly")
            .unwrap_err();

        assert!(join_error.is_cancelled());
        server.abort();
    }

    #[test]
    fn log_urls_keep_route_but_remove_credentials_and_fragments() {
        assert_eq!(
            sanitize_url_for_log(
                "https://mod.mcimirror.top/data/file.jar?X-Amz-Credential=secret&X-Amz-Signature=signature#fragment"
            ),
            "https://mod.mcimirror.top/data/file.jar"
        );
        assert_eq!(
            sanitize_url_for_log("not-a-url?token=secret#fragment"),
            "not-a-url"
        );
    }

    #[tokio::test]
    async fn mirror_request_start_slots_allow_two_immediate_requests() {
        let _guard = MIRROR_REQUEST_SLOT_TEST_LOCK.lock().await;
        *MIRROR_REQUEST_SLOTS.lock().await = [Instant::now(); 2];
        let route = DownloadRoute {
            url: "https://bmclapi2.bangbang93.com/maven/file.jar".to_string(),
            source: DownloadRouteSource::Bmclapi,
            is_mirror: true,
            allow_sensitive_headers: false,
            supports_range: true,
            proxy: ProxyPolicy::System,
        };

        wait_for_mirror_request_slot(&route).await;
        wait_for_mirror_request_slot(&route).await;
        let started = Instant::now();
        wait_for_mirror_request_slot(&route).await;
        assert!(
            started.elapsed() >= time::Duration::from_millis(80),
            "the third request should wait for a mirror request slot"
        );
    }

    #[test]
    fn auto_source_uses_the_first_official_manifest_probe() {
        let _guard = AUTO_SOURCE_TEST_LOCK.lock().unwrap();
        let was_probed = AUTO_SOURCE_PROBED.swap(false, Ordering::Relaxed);
        let preferred_official =
            AUTO_PREFERS_OFFICIAL.swap(false, Ordering::Relaxed);
        let manifest_url =
            "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
        let manifest_routes = resolve_download_routes_for(
            manifest_url,
            ResourceClass::Metadata,
            crate::state::DownloadSourceMode::Auto,
        );
        assert_eq!(manifest_routes[0].source, DownloadRouteSource::Official);

        record_route_success(
            &manifest_routes[0],
            time::Duration::from_secs(1),
            1,
            time::Duration::from_secs(2),
        );
        let asset_routes = resolve_download_routes_for(
            "https://resources.download.minecraft.net/ab/abcdef",
            ResourceClass::MinecraftAsset,
            crate::state::DownloadSourceMode::Auto,
        );
        assert_eq!(asset_routes[0].source, DownloadRouteSource::Official);

        record_route_success(
            &manifest_routes[0],
            time::Duration::from_secs(1),
            1,
            time::Duration::from_secs(4),
        );
        let asset_routes = resolve_download_routes_for(
            "https://resources.download.minecraft.net/ab/abcdef",
            ResourceClass::MinecraftAsset,
            crate::state::DownloadSourceMode::Auto,
        );
        assert!(asset_routes[0].is_mirror);

        AUTO_SOURCE_PROBED.store(was_probed, Ordering::Relaxed);
        AUTO_PREFERS_OFFICIAL.store(preferred_official, Ordering::Relaxed);
    }

    #[test]
    fn modrinth_cdn_redirects_only_fall_back_to_official_cdn() {
        assert!(is_official_modrinth_cdn_redirect(Some(
            "https://cdn.modrinth.com/data/project/versions/version/file.jar"
        )));
        assert!(is_official_modrinth_cdn_redirect(Some(
            "https://CDN.MODRINTH.COM/data/project/versions/version/file.jar"
        )));
        assert!(!is_official_modrinth_cdn_redirect(Some(
            "https://cache.mcimirror.top/data/project/versions/version/file.jar"
        )));
        assert!(!is_official_modrinth_cdn_redirect(Some(
            "https://cdn.modrinth.com.evil.example/file.jar"
        )));
        assert!(!is_official_modrinth_cdn_redirect(Some(
            "https://cdn.modrinth.com@evil.example/file.jar"
        )));
        assert!(!is_official_modrinth_cdn_redirect(Some(
            "https://cdn.modrinth.com/\u{4e0b}\u{8f7d}/file.jar"
        )));
        assert!(!is_official_modrinth_cdn_redirect(None));
    }

    #[test]
    fn redirect_locations_are_bounded_ascii_values() {
        assert!(is_safe_redirect_location("/download/file.jar"));
        assert!(!is_safe_redirect_location("/\u{4e0b}\u{8f7d}/file.jar"));
        assert!(!is_safe_redirect_location(
            &"a".repeat(MAX_REDIRECT_LOCATION_BYTES + 1)
        ));
    }

    #[test]
    fn mrpack_urls_are_detected_without_query_string() {
        assert!(is_mrpack_url(
            "https://cdn.modrinth.com/data/project/version/pack.MRPACK?download=1"
        ));
        assert!(!is_mrpack_url(
            "https://cdn.modrinth.com/data/project/version/mod.jar"
        ));
    }

    #[test]
    fn fetch_retries_use_short_jittered_backoff() {
        let cases = [
            (1, Duration::from_millis(212), Duration::from_millis(288)),
            (2, Duration::from_millis(637), Duration::from_millis(863)),
            (3, Duration::from_millis(1700), Duration::from_millis(2300)),
            (4, Duration::from_millis(1700), Duration::from_millis(2300)),
        ];
        for (attempt, minimum, maximum) in cases {
            let delay = fetch_retry_delay(attempt);
            assert!(delay >= minimum, "attempt {attempt}: {delay:?}");
            assert!(delay <= maximum, "attempt {attempt}: {delay:?}");
        }
    }

    #[test]
    fn vanilla_libraries_have_both_bmcl_routes() {
        let source = "https://libraries.minecraft.net/com/example/library/1/library-1.jar";
        let routes = resolve_download_routes_for(
            source,
            ResourceClass::MinecraftLibrary,
            crate::state::DownloadSourceMode::MirrorPreferred,
        );
        assert_eq!(routes.len(), 3);
        assert_eq!(
            routes[0].url,
            "https://bmclapi2.bangbang93.com/maven/com/example/library/1/library-1.jar"
        );
        assert_eq!(
            routes[1].url,
            "https://bmclapi2.bangbang93.com/libraries/com/example/library/1/library-1.jar"
        );
        assert_eq!(routes[2].url, source);
    }

    #[test]
    fn loader_libraries_use_only_the_two_mirror_routes() {
        let source = "https://libraries.minecraft.net/net/minecraftforge/forge/1.20.1/forge-1.20.1.jar";
        let routes = resolve_download_routes_for(
            source,
            ResourceClass::MinecraftLibrary,
            crate::state::DownloadSourceMode::MirrorPreferred,
        );
        assert_eq!(routes.len(), 2);
        assert_eq!(
            routes[0].url,
            "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/1.20.1/forge-1.20.1.jar"
        );
        assert_eq!(
            routes[1].url,
            "https://bmclapi2.bangbang93.com/libraries/net/minecraftforge/forge/1.20.1/forge-1.20.1.jar"
        );
        assert!(routes.iter().all(|route| route.is_mirror));
    }

    #[test]
    fn maven_central_routes_remain_direct() {
        for source in [
            "https://repo1.maven.org/maven2/com/example/library/1/library-1.jar?download=1",
            "https://repo.maven.apache.org/maven2/com/example/library/1/library-1.jar?download=1",
        ] {
            let routes = resolve_download_routes_for(
                source,
                ResourceClass::MinecraftLibrary,
                crate::state::DownloadSourceMode::MirrorPreferred,
            );
            assert_eq!(routes.len(), 1);
            assert_eq!(routes[0].url, source);
            assert_eq!(routes[0].source, DownloadRouteSource::Official);
        }

        let unmatched = resolve_download_routes_for(
            "https://repo1.maven.org/repository/com/example/library.jar",
            ResourceClass::MinecraftLibrary,
            crate::state::DownloadSourceMode::MirrorPreferred,
        );
        assert_eq!(unmatched.len(), 1);
        assert_eq!(
            unmatched[0].url,
            "https://repo1.maven.org/repository/com/example/library.jar"
        );

        let prefer_official = resolve_download_routes_for(
            "https://repo1.maven.org/maven2/com/example/library/1/library-1.jar",
            ResourceClass::MinecraftLibrary,
            crate::state::DownloadSourceMode::OfficialOnly,
        );
        assert_eq!(prefer_official.len(), 1);
        assert_eq!(
            prefer_official[0].url,
            "https://repo1.maven.org/maven2/com/example/library/1/library-1.jar"
        );
    }

    #[test]
    fn curseforge_routes_include_mirror_and_official_fallback() {
        let routes = resolve_download_routes_for(
            "https://api.curseforge.com/v1/mods/search",
            ResourceClass::CurseForge,
            crate::state::DownloadSourceMode::MirrorPreferred,
        );
        assert_eq!(routes.len(), 2);
        assert!(routes[0].is_mirror);
        assert!(!routes[0].allow_sensitive_headers);
        assert_eq!(routes[1].proxy, ProxyPolicy::System);
        assert!(routes[1].allow_sensitive_headers);
    }

    #[test]
    fn source_matching_is_origin_safe() {
        assert!(same_origin(
            &Url::parse("https://api.curseforge.com/v1/mods").unwrap(),
            &Url::parse("https://api.curseforge.com/v1/files").unwrap(),
        ));
        assert!(!same_origin(
            &Url::parse("https://api.curseforge.com/v1/mods").unwrap(),
            &Url::parse("https://edge.forgecdn.net/files/1/2/a.jar").unwrap(),
        ));
        assert!(is_sensitive_header("x-api-key"));
        assert!(!header_requires_official_only("x-api-key"));
        assert!(is_sensitive_header("Authorization"));
        assert!(header_requires_official_only("Authorization"));
        assert!(!is_sensitive_header("accept"));
    }

    #[test]
    fn dynamic_ranges_split_the_largest_remaining_tail() {
        let range = DownloadRange::new(0, 0, 10 * 1024 * 1024 - 1);
        let tail = range.split_tail(1).unwrap();
        assert_eq!(range.end(), 6 * 1024 * 1024 - 1);
        assert_eq!(tail.start, 6 * 1024 * 1024);
        assert_eq!(tail.end(), 10 * 1024 * 1024 - 1);
        assert!(tail.remaining() >= 256 * 1024);

        let small = DownloadRange::new(2, 0, 256 * 1024 - 2);
        assert!(small.split_tail(3).is_none());
    }

    #[test]
    fn large_files_start_parallel_ranges_without_consulting_speed_floor() {
        let size = 16 * 1024 * 1024;
        assert_eq!(initial_segment_count(size, 64), 4);
        let ranges = create_initial_ranges(size, 4);
        assert_eq!(ranges.len(), 4);
        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end(), size - 1);
        for pair in ranges.windows(2) {
            assert_eq!(pair[0].end() + 1, pair[1].start);
        }
    }

    #[test]
    fn segmented_concurrency_respects_effective_and_global_limits() {
        assert_eq!(segmented_concurrency_cap(64), 8);
        assert_eq!(segmented_concurrency_cap(8), 7);
        assert_eq!(segmented_concurrency_cap(4), 3);
        assert_eq!(segmented_concurrency_cap(1), 1);
        assert_eq!(initial_segment_count(16 * 1024 * 1024, 3), 2);
    }

    #[test]
    fn stable_aggregate_throughput_allows_gradual_expansion() {
        let snapshot = SpeedSnapshot {
            aggregate_speed: 2 * 1024 * 1024,
            recent_average: 1900 * 1024,
            speed_floor: 1024 * 1024,
            sample_count: SEGMENT_EXPANSION_SAMPLE_COUNT,
        };
        assert_eq!(
            expansion_block_reason(
                snapshot,
                4,
                8,
                4,
                16 * 1024 * 1024,
                SEGMENT_EXPANSION_INTERVAL,
            ),
            None
        );
        assert_eq!(
            expansion_block_reason(
                snapshot,
                4,
                8,
                0,
                16 * 1024 * 1024,
                SEGMENT_EXPANSION_INTERVAL,
            ),
            Some("no global permit available")
        );
    }

    #[test]
    fn redirect_hops_rebuild_the_same_range_header() {
        let original = byte_range_header_value(Some(1024), Some(2047));
        let redirected = byte_range_header_value(Some(1024), Some(2047));
        assert_eq!(original.as_deref(), Some("bytes=1024-2047"));
        assert_eq!(redirected, original);
    }

    #[tokio::test]
    async fn verifies_streaming_integrity_algorithms() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"axolotl download").unwrap();
        let integrity = Integrity {
			size: Some(16),
			sha1: Some("90e438ead880c77ea2d7e726b5aa74e6d21a805f".to_string()),
			sha512: Some("2dcd3e0a9f198e9ef892a28ed6534dd154be2bd13531c961c0852aa6f1e24f633d2cd8288d3cf13c6a482a87c822b74a4901a2aa64292f4a371c5ebfea392c1b".to_string()),
			sha256: Some("120561bc60d59ebe2a08fc229ff2b1eb06b20c4211d21a17c15dd80790f48672".to_string()),
			md5: Some("30018bb52add8c6dbc5d4149c1325df0".to_string()),
			content: ContentValidation::None,
		};
        assert_eq!(verify_file(file.path(), &integrity).await.unwrap(), 16);
    }

    #[tokio::test]
    async fn segmented_download_uses_parallel_validated_ranges() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let size = (SEGMENTED_DOWNLOAD_THRESHOLD + 1024 * 1024) as usize;
        let data = Arc::new(
            (0..size)
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>(),
        );
        let hash = sha1_smol::Sha1::from(&data[..]).hexdigest();
        let (url, requests, normal_requests, server) =
            spawn_range_server(data.clone(), false, false, true, false).await;
        let route = DownloadRoute {
            url: url.clone(),
            source: DownloadRouteSource::Alternate,
            is_mirror: false,
            allow_sensitive_headers: false,
            supports_range: true,
            proxy: ProxyPolicy::System,
        };
        let request = DownloadRequest::new(&url, ResourceClass::Other)
            .with_integrity(Integrity::sha1(hash).with_size(size as u64));
        let directory = tempfile::tempdir().unwrap();
        let part_path = directory.path().join("fixture.part");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let semaphore = FetchSemaphore(Semaphore::new(8));
        let outcome = try_segmented_download(
            &request,
            &route,
            size as u64,
            &part_path,
            &semaphore,
            None,
            None,
            &client,
            &client,
        )
        .await;
        match outcome {
            SegmentedDownloadOutcome::Success(result) => {
                assert_eq!(result.size, size as u64);
            }
            _ => panic!("segmented fixture download did not succeed"),
        }
        assert!(requests.load(Ordering::Relaxed) >= 4);
        assert_eq!(normal_requests.load(Ordering::Relaxed), 0);
        assert_eq!(
            verify_file(&part_path, &request.integrity).await.unwrap(),
            size as u64
        );
        server.abort();
    }

    #[tokio::test]
    async fn mirror_ranges_start_concurrently_and_retries_reuse_redirect() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let size = (SEGMENTED_DOWNLOAD_THRESHOLD * 2) as usize;
        let data = Arc::new(
            (0..size)
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>(),
        );
        let hash = sha1_smol::Sha1::from(&data[..]).hexdigest();
        let (target_url, range_requests, normal_requests, range_server) =
            spawn_range_server(data, false, false, false, true).await;
        let (redirect_url, redirect_requests, redirect_server) =
            spawn_redirect_server(target_url, Duration::from_millis(50)).await;
        let route = DownloadRoute {
            url: redirect_url.clone(),
            source: DownloadRouteSource::Mcim,
            is_mirror: true,
            allow_sensitive_headers: false,
            supports_range: true,
            proxy: ProxyPolicy::System,
        };
        let request =
            DownloadRequest::new(&redirect_url, ResourceClass::Modrinth)
                .with_integrity(Integrity::sha1(hash).with_size(size as u64));
        let directory = tempfile::tempdir().unwrap();
        let part_path = directory.path().join("redirect.part");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let outcome = try_segmented_download(
            &request,
            &route,
            size as u64,
            &part_path,
            &FetchSemaphore(Semaphore::new(8)),
            None,
            None,
            &client,
            &client,
        )
        .await;

        assert!(matches!(outcome, SegmentedDownloadOutcome::Success(_)));
        assert_eq!(redirect_requests.load(Ordering::Relaxed), 4);
        assert!(range_requests.load(Ordering::Relaxed) >= 5);
        assert_eq!(normal_requests.load(Ordering::Relaxed), 0);
        redirect_server.abort();
        range_server.abort();
    }

    #[tokio::test]
    async fn temporary_range_failure_resumes_without_disabling_segments() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let size = (SEGMENTED_DOWNLOAD_THRESHOLD * 2) as usize;
        let data = Arc::new(
            (0..size)
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>(),
        );
        let hash = sha1_smol::Sha1::from(&data[..]).hexdigest();
        let (url, requests, _, server) =
            spawn_range_server(data, false, false, false, true).await;
        let route = DownloadRoute {
            url: url.clone(),
            source: DownloadRouteSource::Alternate,
            is_mirror: false,
            allow_sensitive_headers: false,
            supports_range: true,
            proxy: ProxyPolicy::System,
        };
        let request = DownloadRequest::new(&url, ResourceClass::Other)
            .with_integrity(Integrity::sha1(hash).with_size(size as u64));
        let directory = tempfile::tempdir().unwrap();
        let part_path = directory.path().join("retry.part");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let outcome = try_segmented_download(
            &request,
            &route,
            size as u64,
            &part_path,
            &FetchSemaphore(Semaphore::new(8)),
            None,
            None,
            &client,
            &client,
        )
        .await;

        assert!(matches!(outcome, SegmentedDownloadOutcome::Success(_)));
        assert!(requests.load(Ordering::Relaxed) >= 5);
        assert_eq!(
            verify_file(&part_path, &request.integrity).await.unwrap(),
            size as u64
        );
        server.abort();
    }

    #[tokio::test]
    async fn invalid_content_range_falls_back_without_persisting_host_state() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let data = Arc::new(vec![7_u8; 1024 * 1024]);
        let (url, _, _, server) =
            spawn_range_server(data.clone(), true, false, false, false).await;
        let route = DownloadRoute {
            url: url.clone(),
            source: DownloadRouteSource::Alternate,
            is_mirror: false,
            allow_sensitive_headers: false,
            supports_range: true,
            proxy: ProxyPolicy::System,
        };
        let directory = tempfile::tempdir().unwrap();
        let part_path = directory.path().join("fixture.part");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let semaphore = FetchSemaphore(Semaphore::new(4));
        let (progress, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let permit = semaphore.0.acquire().await.unwrap();
        let speed = DownloadSpeedTracker::default();
        let validator = Mutex::new(None);
        let result = download_segment(
            &route,
            DownloadRange::new(0, 0, data.len() as u64 - 1),
            data.len() as u64,
            None,
            None,
            None,
            &part_path,
            permit,
            &client,
            &client,
            progress,
            &speed,
            &validator,
            None,
        )
        .await;
        assert!(matches!(
            result,
            Err(SegmentDownloadError::Protocol("invalid Content-Range"))
        ));
        disable_range_splitting(&route);
        assert!(range_splitting_allowed(&route));
        server.abort();
    }

    #[tokio::test]
    async fn ignored_ranges_fall_back_to_one_full_file_write() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let size = (SEGMENTED_DOWNLOAD_THRESHOLD * 2) as usize;
        let data = Arc::new(
            (0..size)
                .map(|index| (index % 251) as u8)
                .collect::<Vec<_>>(),
        );
        let (url, requests, normal_requests, server) =
            spawn_range_server(data.clone(), false, true, false, false).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("ignored-range.bin");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();
        let result = download_to_path(
            DownloadRequest::new(&url, ResourceClass::Other)
                .with_integrity(Integrity::default().with_size(size as u64)),
            &destination,
            &FetchSemaphore(Semaphore::new(8)),
            &pool,
            None,
        )
        .await
        .unwrap();

        assert_eq!(result.size, size as u64);
        assert_eq!(tokio::fs::read(&destination).await.unwrap(), *data);
        assert!(requests.load(Ordering::Relaxed) >= 2);
        assert_eq!(normal_requests.load(Ordering::Relaxed), 1);
        for index in 0..MAX_SEGMENT_CONCURRENCY {
            assert!(
                !segment_path(&suffixed_path(&destination, ".part"), index)
                    .exists()
            );
        }
        server.abort();
    }

    #[tokio::test]
    async fn canceling_segmented_download_releases_permits_and_temp_files() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        let size = (SEGMENTED_DOWNLOAD_THRESHOLD * 4) as usize;
        let data = Arc::new(vec![13_u8; size]);
        let (url, _, _, server) =
            spawn_range_server(data, false, false, true, false).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("canceled-segments.bin");
        let destination_for_task = destination.clone();
        let semaphore = Arc::new(FetchSemaphore(Semaphore::new(8)));
        let semaphore_for_task = Arc::clone(&semaphore);
        let task = tokio::spawn(async move {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .connect_lazy("sqlite::memory:")
                .unwrap();
            download_to_path(
                DownloadRequest::new(url, ResourceClass::Other).with_integrity(
                    Integrity::default().with_size(size as u64),
                ),
                destination_for_task,
                &semaphore_for_task,
                &pool,
                None,
            )
            .await
        });

        tokio::time::sleep(Duration::from_millis(150)).await;
        task.abort();
        let _ = task.await;
        tokio::time::sleep(Duration::from_millis(25)).await;

        assert_eq!(semaphore.0.available_permits(), 8);
        let part_path = suffixed_path(&destination, ".part");
        assert!(!part_path.exists());
        for index in 0..MAX_SEGMENT_CONCURRENCY {
            assert!(!segment_path(&part_path, index).exists());
        }
        server.abort();
    }

    #[tokio::test]
    async fn unknown_size_and_small_files_use_one_connection() {
        let _guard = RANGE_SPLITTING_TEST_LOCK.lock().await;
        for (expected_size, integrity) in [
            (None, Integrity::default()),
            (Some(1024_u64), Integrity::default().with_size(1024)),
        ] {
            let data = Arc::new(vec![11_u8; 1024]);
            let (url, requests, normal_requests, server) =
                spawn_range_server(data.clone(), false, false, false, false)
                    .await;
            let directory = tempfile::tempdir().unwrap();
            let destination = directory.path().join("single.bin");
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .connect_lazy("sqlite::memory:")
                .unwrap();
            let result = download_to_path(
                DownloadRequest::new(&url, ResourceClass::Other)
                    .with_integrity(integrity),
                &destination,
                &FetchSemaphore(Semaphore::new(8)),
                &pool,
                None,
            )
            .await
            .unwrap();
            assert_eq!(result.size, expected_size.unwrap_or(1024));
            assert_eq!(requests.load(Ordering::Relaxed), 1);
            assert_eq!(normal_requests.load(Ordering::Relaxed), 1);
            server.abort();
        }
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
