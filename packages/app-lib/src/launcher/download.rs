//! Downloader for Minecraft data

use crate::data::ModLoader;
use crate::install::{
    InstallErrorContext, InstallPhaseDetails, InstallPhaseId, InstallProgress,
    InstallProgressReporter,
};
use crate::instance::QuickPlayType;
use crate::launcher::parse_rules;
use crate::{
    event::{
        LoadingBarId,
        emit::{emit_loading, loading_try_for_each_concurrent},
    },
    state::State,
    util::{fetch::*, io},
};
use daedalus::minecraft::{LibraryDownload, LoggingConfiguration, LoggingSide};
use daedalus::{
    self as d,
    minecraft::{
        Asset, AssetsIndex, Library, Version as GameVersion,
        VersionInfo as GameVersionInfo,
    },
    modded::LoaderVersion,
};
use futures::prelude::*;
use reqwest::Method;
use std::{
    collections::HashSet,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use url::Url;

const MINECRAFT_DOWNLOAD_PROGRESS_MIN_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalRuntimeSource {
    pub root: PathBuf,
}

impl LocalRuntimeSource {
    pub fn discover(instance_or_dotminecraft: &Path) -> Option<Self> {
        if is_local_runtime_root(instance_or_dotminecraft) {
            return Some(Self {
                root: instance_or_dotminecraft.to_path_buf(),
            });
        }

        let dotminecraft = instance_or_dotminecraft.join(".minecraft");
        if is_local_runtime_root(&dotminecraft) {
            return Some(Self { root: dotminecraft });
        }

        instance_or_dotminecraft
            .ancestors()
            .find(|path| path.join("versions").is_dir())
            .map(|root| Self {
                root: root.to_path_buf(),
            })
    }
}

fn is_local_runtime_root(path: &Path) -> bool {
    path.join("assets").is_dir() && path.join("libraries").is_dir()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactAvailability {
    Cached,
    LocalReusable,
    NetworkRequired,
}

pub async fn classify_local_artifact(
    local: Option<&LocalRuntimeSource>,
    destination: &Path,
    relative_path: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
) -> crate::Result<ArtifactAvailability> {
    if destination.exists() {
        return Ok(ArtifactAvailability::Cached);
    }

    let Some(local) = local else {
        return Ok(ArtifactAvailability::NetworkRequired);
    };
    let Some(expected_sha1) = expected_sha1 else {
        return Ok(ArtifactAvailability::NetworkRequired);
    };

    let candidate = local.root.join(relative_path);
    if !candidate.is_file() {
        return Ok(ArtifactAvailability::NetworkRequired);
    }

    let metadata = io::metadata(&candidate).await?;
    if let Some(expected_size) = expected_size
        && metadata.len() != expected_size
    {
        return Ok(ArtifactAvailability::NetworkRequired);
    }

    let (_, actual_sha1) = sha1_file_async(&candidate).await?;
    if actual_sha1.eq_ignore_ascii_case(expected_sha1) {
        Ok(ArtifactAvailability::LocalReusable)
    } else {
        Ok(ArtifactAvailability::NetworkRequired)
    }
}

pub async fn copy_verified_local_artifact(
    st: &State,
    local: &LocalRuntimeSource,
    relative_path: &Path,
    destination: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
    progress: Option<&MinecraftDownloadProgress>,
    context: InstallErrorContext,
) -> crate::Result<bool> {
    if !matches!(
        classify_local_artifact(
            Some(local),
            destination,
            relative_path,
            expected_sha1,
            expected_size,
        )
        .await?,
        ArtifactAvailability::LocalReusable
    ) {
        return Ok(false);
    }

    if let Some(progress) = progress {
        progress.set_context(context.clone()).await?;
    }

    let source = local.root.join(relative_path);
    let size = match expected_size {
        Some(size) => size,
        None => io::metadata(&source).await?.len(),
    };
    if let Err(error) =
        crate::util::fetch::copy(&source, destination, &st.io_semaphore).await
    {
        if let Some(progress) = progress {
            progress.persist_failure_context(context).await;
        }
        return Err(error);
    }
    if let Some(progress) = progress {
        progress.add_bytes(size).await?;
    }

    Ok(true)
}

#[derive(Clone, Debug)]
pub struct MinecraftDownloadProgress {
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
    current: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    last_reported: Arc<AtomicU64>,
    source: Arc<Mutex<Option<String>>>,
    fallback_count: Arc<AtomicU64>,
}

impl MinecraftDownloadProgress {
    async fn new(
        reporter: InstallProgressReporter,
        details: InstallPhaseDetails,
        total: u64,
    ) -> crate::Result<Self> {
        let progress = Self {
            reporter,
            details,
            current: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(total)),
            last_reported: Arc::new(AtomicU64::new(0)),
            source: Arc::new(Mutex::new(None)),
            fallback_count: Arc::new(AtomicU64::new(0)),
        };

        if total > 0 {
            progress.emit_progress(0, total).await?;
        }

        Ok(progress)
    }

    async fn add_total(&self, total: u64) -> crate::Result<()> {
        if total == 0 {
            return Ok(());
        }

        let total = self.total.fetch_add(total, Ordering::Relaxed) + total;
        self.emit_if_needed(self.current.load(Ordering::Relaxed), total, true)
            .await
    }

    async fn add_bytes(&self, bytes: u64) -> crate::Result<()> {
        if bytes == 0 {
            return Ok(());
        }

        let current = self.current.fetch_add(bytes, Ordering::Relaxed) + bytes;
        let total = self.total.load(Ordering::Relaxed);
        self.emit_if_needed(current, total, false).await
    }

    /// Rolls back bytes that were counted for an abandoned download attempt,
    /// so retried files do not inflate the progress bar to 100% early.
    async fn sub_bytes(&self, bytes: u64) -> crate::Result<()> {
        if bytes == 0 {
            return Ok(());
        }

        let mut current = self.current.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_sub(bytes);
            match self.current.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    current = next;
                    break;
                }
                Err(actual) => current = actual,
            }
        }
        let total = self.total.load(Ordering::Relaxed);
        self.emit_if_needed(current, total, true).await
    }

    async fn emit_if_needed(
        &self,
        current: u64,
        total: u64,
        force: bool,
    ) -> crate::Result<()> {
        if total == 0 {
            return Ok(());
        }

        let min_delta =
            (total / 200).max(MINECRAFT_DOWNLOAD_PROGRESS_MIN_BYTES);
        let last_reported = self.last_reported.load(Ordering::Relaxed);
        if !force
            && current < total
            && current.saturating_sub(last_reported) < min_delta
        {
            return Ok(());
        }

        self.last_reported.store(current, Ordering::Relaxed);
        self.emit_progress(current, total).await
    }

    async fn emit_progress(
        &self,
        current: u64,
        total: u64,
    ) -> crate::Result<()> {
        self.reporter
            .update(
                InstallPhaseId::DownloadingMinecraft,
                Some(InstallProgress {
                    current: current.min(total),
                    total,
                    secondary: None,
                }),
                self.details.clone(),
            )
            .await
    }

    async fn set_context(
        &self,
        context: InstallErrorContext,
    ) -> crate::Result<()> {
        self.reporter.set_transient_context(context).await
    }

    async fn persist_failure_context(&self, context: InstallErrorContext) {
        self.reporter.persist_failure_context(context).await;
    }

    fn record_download_result(&self, result: &DownloadResult) {
        if result.attempts > 0
            && let Ok(mut source) = self.source.lock()
        {
            *source = Some(result.source.as_str().to_string());
        }
        self.fallback_count
            .fetch_add(result.fallback_count as u64, Ordering::Relaxed);
    }

    async fn finish(&self) -> crate::Result<()> {
        let source = self.source.lock().ok().and_then(|source| source.clone());
        let fallback_count = self.fallback_count.load(Ordering::Relaxed);
        if let Some(source) = source {
            self.reporter
                .record_download_metrics(source, fallback_count)
                .await?;
        }
        Ok(())
    }
}

async fn download_minecraft_file(
    st: &State,
    url: &str,
    sha1: Option<&str>,
    expected_size: Option<u64>,
    destination: &std::path::Path,
    resource: ResourceClass,
    content_validation: ContentValidation,
    force: bool,
    progress: Option<MinecraftDownloadProgress>,
    context: InstallErrorContext,
) -> crate::Result<DownloadResult> {
    let urls = minecraft_library_mirrors(url);
    download_minecraft_file_with_candidates(
        st,
        &urls,
        sha1,
        expected_size,
        destination,
        resource,
        content_validation,
        force,
        progress,
        context,
    )
    .await
}

async fn download_minecraft_file_with_candidates(
    st: &State,
    urls: &[String],
    sha1: Option<&str>,
    expected_size: Option<u64>,
    destination: &std::path::Path,
    resource: ResourceClass,
    content_validation: ContentValidation,
    force: bool,
    progress: Option<MinecraftDownloadProgress>,
    context: InstallErrorContext,
) -> crate::Result<DownloadResult> {
    let Some(url) = urls.first() else {
        return Err(crate::ErrorKind::LauncherError(
            "No trusted download URL is available".to_string(),
        )
        .into());
    };
    let mut context = context;
    context.urls.extend(urls.iter().cloned());
    context.expected_hash = sha1.map(str::to_string);
    context.expected_size = expected_size;
    if let Some(progress) = &progress {
        progress.set_context(context.clone()).await?;
    }
    if force && destination.exists() {
        io::remove_file(destination).await?;
    }

    let integrity = Integrity {
        size: expected_size,
        sha1: sha1.map(str::to_string),
        content: content_validation,
        ..Integrity::default()
    };
    let mut request = DownloadRequest::new(url, resource)
        .with_candidate_urls(urls.iter().skip(1).cloned())
        .with_integrity(integrity);
    if let Some(progress) = &progress {
        request = request.with_install_tracking(
            progress.reporter.clone(),
            destination.display().to_string(),
            destination
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| url.to_string()),
        );
    }
    let Some(progress) = progress else {
        return download_to_path(
            request,
            destination,
            &st.download_semaphore,
            &st.pool,
            None,
        )
        .await;
    };

    let last_downloaded = Arc::new(AtomicU64::new(0));
    let mut progress_fn = {
        let progress = progress.clone();
        let last_downloaded = last_downloaded.clone();
        move |downloaded: u64,
              _total: u64|
              -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>> {
            let previous =
                last_downloaded.swap(downloaded, Ordering::Relaxed);
            let progress = progress.clone();
            Box::pin(async move {
                if downloaded >= previous {
                    progress.add_bytes(downloaded - previous).await
                } else {
                    // The downloaded count went backwards, meaning a failed
                    // attempt restarted; un-count the abandoned bytes.
                    progress.sub_bytes(previous - downloaded).await
                }
            })
        }
    };

    let result = match download_to_path(
        request,
        destination,
        &st.download_semaphore,
        &st.pool,
        Some(&mut progress_fn as &mut FetchProgressFn<'_>),
    )
    .await
    {
        Ok(bytes) => bytes,
        Err(error) => {
            progress.persist_failure_context(context).await;
            return Err(error);
        }
    };
    progress.record_download_result(&result);

    if let Some(expected_size) = expected_size {
        let downloaded = last_downloaded.load(Ordering::Relaxed);
        progress
            .add_bytes(expected_size.saturating_sub(downloaded))
            .await?;
    }

    Ok(result)
}

fn minecraft_library_mirrors(url: &str) -> Vec<String> {
    const MACHINA_LWJGL_RELEASE: &str = "https://github.com/MinecraftMachina/lwjgl/releases/download/2.9.4-20150209-mmachina.2/";
    const MOJANG_LWJGL_PATH: &str = "https://libraries.minecraft.net/org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/";

    let mut mirrors = vec![url.to_string()];
    if let Some(file_name) = url.strip_prefix(MACHINA_LWJGL_RELEASE) {
        mirrors.push(format!("{MOJANG_LWJGL_PATH}{file_name}"));
    }
    mirrors
}

const LAUNCHER_META_MAVEN: &str = "https://launcher-meta.modrinth.com/maven";
const LIBRARIES_MAVEN: &str = "https://libraries.minecraft.net";
const FABRIC_MAVEN: &str = "https://maven.fabricmc.net";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases";
const QUILT_MAVEN: &str = "https://maven.quiltmc.org/repository/release";
const SPONGE_MAVEN: &str = "https://repo.spongepowered.org/maven";
const MAVEN_CENTRAL: &str = "https://repo.maven.apache.org/maven2";

fn legacy_library_download_urls(
    repository: Option<&str>,
    artifact_path: &str,
) -> Option<Vec<String>> {
    if artifact_path.starts_with('/')
        || artifact_path.contains('\\')
        || artifact_path
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return None;
    }

    let repository = repository.unwrap_or(LIBRARIES_MAVEN);
    let mut repository_url = Url::parse(repository).ok()?;
    if repository_url.scheme() != "https"
        || repository_url.host_str().is_none()
        || !repository_url.username().is_empty()
        || repository_url.password().is_some()
        || repository_url.query().is_some()
        || repository_url.fragment().is_some()
    {
        return None;
    }
    let normalized_path =
        format!("{}/", repository_url.path().trim_end_matches('/'));
    repository_url.set_path(&normalized_path);
    let declared_url = repository_url.join(artifact_path).ok()?.to_string();
    let canonical_urls = legacy_library_canonical_urls(artifact_path);
    let mut candidates = Vec::new();

    if repository.trim_end_matches('/') == LAUNCHER_META_MAVEN {
        candidates.extend(canonical_urls);
        candidates.push(declared_url);
    } else {
        candidates.push(declared_url);
        candidates.extend(canonical_urls);
    }
    candidates.push(format!("{LIBRARIES_MAVEN}/{artifact_path}"));

    let mut urls = Vec::new();
    for candidate in candidates {
        if !urls.contains(&candidate) {
            urls.push(candidate);
        }
    }

    Some(urls)
}

fn legacy_library_canonical_urls(artifact_path: &str) -> Vec<String> {
    let in_repository =
        |repository: &str| format!("{repository}/{artifact_path}");

    if artifact_path.starts_with("org/scala-lang/scala-parser-combinators_")
        || artifact_path.starts_with("org/scala-lang/scala-swing_")
        || artifact_path.starts_with("org/scala-lang/scala-xml_")
    {
        vec![in_repository(FORGE_MAVEN)]
    } else if artifact_path.starts_with("net/fabricmc/") {
        vec![in_repository(FABRIC_MAVEN)]
    } else if artifact_path.starts_with("org/quiltmc/") {
        vec![in_repository(QUILT_MAVEN)]
    } else if artifact_path.starts_with("net/minecraftforge/")
        || artifact_path.starts_with("cpw/mods/")
    {
        vec![in_repository(FORGE_MAVEN)]
    } else if artifact_path.starts_with("net/neoforged/") {
        vec![in_repository(NEOFORGE_MAVEN)]
    } else if artifact_path.starts_with("org/spongepowered/") {
        vec![in_repository(SPONGE_MAVEN)]
    } else if artifact_path.starts_with("net/minecraft/launchwrapper/") {
        vec![in_repository(LIBRARIES_MAVEN)]
    } else if artifact_path.starts_with("org/ow2/")
        || artifact_path.starts_with("org/scala-lang/")
        || artifact_path.starts_with("org/jline/")
        || artifact_path.starts_with("jline/")
        || artifact_path.starts_with("net/java/dev/jna/")
        || artifact_path.starts_with("com/typesafe/")
    {
        vec![in_repository(MAVEN_CENTRAL)]
    } else if artifact_path.starts_with("com/modrinth/daedalus/") {
        vec![in_repository(LAUNCHER_META_MAVEN)]
    } else {
        Vec::new()
    }
}

fn legacy_library_content_validation(artifact_path: &str) -> ContentValidation {
    if artifact_path.ends_with(".jar") {
        ContentValidation::Jar
    } else {
        ContentValidation::None
    }
}

pub(crate) fn legacy_library_sha1(library: &Library) -> Option<&str> {
    library
        .checksums
        .as_deref()
        .and_then(|checksums| {
            checksums.iter().find(|checksum| {
                checksum.len() == 40
                    && checksum
                        .bytes()
                        .all(|character| character.is_ascii_hexdigit())
            })
        })
        .map(String::as_str)
}

pub(crate) fn local_asset_index_path(asset_index_id: &str) -> PathBuf {
    Path::new("assets")
        .join("indexes")
        .join(format!("{asset_index_id}.json"))
}

pub(crate) fn local_asset_object_path(hash: &str) -> PathBuf {
    Path::new("assets")
        .join("objects")
        .join(&hash[..2])
        .join(hash)
}

pub(crate) fn local_library_path(library_name: &str) -> crate::Result<PathBuf> {
    Ok(Path::new("libraries").join(d::get_path_from_artifact(library_name)?))
}

pub(crate) fn local_native_library_path(
    library: &Library,
    native: &LibraryDownload,
    classifier: &str,
) -> crate::Result<PathBuf> {
    let artifact_path = match native.path.as_deref() {
        Some(path) => path.to_string(),
        None => {
            let artifact_path = d::get_path_from_artifact(&library.name)?;
            if let Some((prefix, extension)) = artifact_path.rsplit_once('.') {
                format!("{prefix}-{classifier}.{extension}")
            } else {
                format!("{artifact_path}-{classifier}")
            }
        }
    };

    Ok(Path::new("libraries").join(artifact_path))
}

pub(crate) fn local_client_path(game_version: &str) -> PathBuf {
    Path::new("versions")
        .join(game_version)
        .join(format!("{game_version}.jar"))
}

pub(crate) fn local_log_config_path(log_config_id: &str) -> PathBuf {
    Path::new("assets").join("log_configs").join(log_config_id)
}

async fn try_reuse_local_artifact(
    st: &State,
    local: Option<&LocalRuntimeSource>,
    relative_path: &Path,
    destination: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
    progress: Option<&MinecraftDownloadProgress>,
    context: InstallErrorContext,
) -> crate::Result<bool> {
    let Some(local) = local else {
        return Ok(false);
    };

    copy_verified_local_artifact(
        st,
        local,
        relative_path,
        destination,
        expected_sha1,
        expected_size,
        progress,
        context,
    )
    .await
}

async fn download_or_reuse_local<F, Fut>(
    st: &State,
    local: Option<&LocalRuntimeSource>,
    relative_path: &Path,
    destination: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
    progress: Option<&MinecraftDownloadProgress>,
    context: InstallErrorContext,
    force: bool,
    download: F,
) -> crate::Result<bool>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = crate::Result<DownloadResult>>,
{
    if !force
        && try_reuse_local_artifact(
            st,
            local,
            relative_path,
            destination,
            expected_sha1,
            expected_size,
            progress,
            context,
        )
        .await?
    {
        return Ok(true);
    }
    download().await?;
    Ok(false)
}

fn should_download(path_exists: bool, force: bool) -> bool {
    !path_exists || force
}

fn missing_client_bytes(
    st: &State,
    version: &GameVersionInfo,
    force: bool,
) -> crate::Result<u64> {
    let client_download = version
        .downloads
        .get(&d::minecraft::DownloadType::Client)
        .ok_or(
            crate::ErrorKind::LauncherError(format!(
                "No client downloads exist for version {}",
                version.id
            ))
            .as_error(),
        )?;
    let path = st
        .directories
        .version_dir(&version.id)
        .join(format!("{}.jar", version.id));

    Ok(if should_download(path.exists(), force) {
        client_download.size as u64
    } else {
        0
    })
}

fn missing_assets_index_bytes(
    st: &State,
    version: &GameVersionInfo,
    force: bool,
) -> u64 {
    let path = st
        .directories
        .assets_index_dir()
        .join(format!("{}.json", &version.asset_index.id));

    if should_download(path.exists(), force) {
        version.asset_index.size as u64
    } else {
        0
    }
}

fn missing_log_config_bytes(
    st: &State,
    version: &GameVersionInfo,
    force: bool,
) -> u64 {
    let log_download = version
        .logging
        .as_ref()
        .and_then(|x| x.get(&LoggingSide::Client));
    let Some(LoggingConfiguration::Log4j2Xml {
        file: log_download, ..
    }) = log_download
    else {
        return 0;
    };

    let path = st.directories.log_configs_dir().join(&log_download.id);
    if should_download(path.exists(), force) {
        log_download.size as u64
    } else {
        0
    }
}

fn missing_asset_bytes(
    st: &State,
    with_legacy: bool,
    index: &AssetsIndex,
    force: bool,
) -> u64 {
    index
        .objects
        .iter()
        .filter_map(|(name, asset)| {
            let hash = &asset.hash;
            let object_path = st.directories.object_dir(hash);
            let legacy_path = st.directories.legacy_assets_dir().join(
                name.replace('/', &String::from(std::path::MAIN_SEPARATOR)),
            );
            let should_fetch_object =
                should_download(object_path.exists(), force);
            let should_fetch_legacy =
                (with_legacy && !legacy_path.exists()) || force;

            (should_fetch_object || should_fetch_legacy)
                .then_some(asset.size as u64)
        })
        .sum()
}

fn missing_library_bytes(
    st: &State,
    libraries: &[Library],
    java_arch: &str,
    force: bool,
    minecraft_updated: bool,
) -> crate::Result<u64> {
    let mut total = 0;

    for library in libraries {
        if let Some(rules) = &library.rules
            && !parse_rules(
                rules,
                java_arch,
                &QuickPlayType::None,
                minecraft_updated,
            )
        {
            continue;
        }

        if !library.downloadable {
            continue;
        }

        if let Some((os_key, classifiers)) =
            library.natives_os_key_and_classifiers(java_arch)
        {
            let parsed_key =
                os_key.replace("${arch}", crate::util::platform::ARCH_WIDTH);

            if let Some(native) = classifiers.get(&parsed_key) {
                total += native.size as u64;
            }
        } else {
            let artifact_path = d::get_path_from_artifact(&library.name)?;
            let path = st.directories.libraries_dir().join(&artifact_path);

            if path.exists() && !force {
                continue;
            }

            if let Some(artifact) = library
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
                && !artifact.url.is_empty()
            {
                total += artifact.size as u64;
            }
        }
    }

    Ok(total)
}

fn missing_initial_minecraft_bytes(
    st: &State,
    version: &GameVersionInfo,
    java_arch: &str,
    force: bool,
    minecraft_updated: bool,
) -> crate::Result<u64> {
    Ok(missing_client_bytes(st, version, force)?
        + missing_assets_index_bytes(st, version, force)
        + missing_log_config_bytes(st, version, force)
        + missing_library_bytes(
            st,
            version.libraries.as_slice(),
            java_arch,
            force,
            minecraft_updated,
        )?)
}

#[tracing::instrument(skip_all, fields(version = version.id.as_str()))]
#[allow(clippy::too_many_arguments)]
pub async fn download_minecraft(
    st: &State,
    local_source: Option<&LocalRuntimeSource>,
    game_version: &str,
    version: &GameVersionInfo,
    loading_bar: Option<&LoadingBarId>,
    java_arch: &str,
    force: bool,
    minecraft_updated: bool,
    reporter: Option<InstallProgressReporter>,
    phase_details: InstallPhaseDetails,
) -> crate::Result<()> {
    tracing::info!("Downloading Minecraft version {}", version.id);
    let progress = if let Some(reporter) = reporter {
        Some(
            MinecraftDownloadProgress::new(
                reporter,
                phase_details,
                missing_initial_minecraft_bytes(
                    st,
                    version,
                    java_arch,
                    force,
                    minecraft_updated,
                )?,
            )
            .await?,
        )
    } else {
        None
    };

    let amount = if version.processors.as_ref().is_some_and(|x| !x.is_empty()) {
        25.0
    } else {
        40.0
    };

    tokio::try_join! {
        async {
            let assets_index = download_assets_index(
                st,
                local_source,
                version,
                loading_bar,
                force,
                progress.clone(),
            )
            .await?;
            if let Some(progress) = &progress {
                progress
                    .add_total(missing_asset_bytes(
                        st,
                        version.assets == "legacy",
                        &assets_index,
                        force,
                    ))
                    .await?;
            }
            download_assets(
                st,
                local_source,
                version.assets == "legacy",
                &assets_index,
                loading_bar,
                amount,
                force,
                progress.clone(),
            )
            .await
        },
        async {
            tokio::try_join! {
                download_client(st, local_source, game_version, version, loading_bar, force, progress.clone()),
                download_log_config(st, local_source, version, loading_bar, force, progress.clone()),
                download_libraries(st, local_source, version.libraries.as_slice(), &version.id, loading_bar, amount, java_arch, force, minecraft_updated, progress.clone())
            }?;
            Ok::<_, crate::Error>(())
        }
    }?;
    if let Some(progress) = &progress {
        progress.finish().await?;
    }

    tracing::info!("Done downloading Minecraft!");
    Ok(())
}

#[tracing::instrument(skip_all, fields(version = version.id.as_str(), loader = ?loader))]

pub async fn download_version_info(
    st: &State,
    version: &GameVersion,
    mod_loader: ModLoader,
    loader: Option<&LoaderVersion>,
    force: Option<bool>,
    loading_bar: Option<&LoadingBarId>,
    reporter: Option<&InstallProgressReporter>,
) -> crate::Result<GameVersionInfo> {
    let version_id = loader
        .map_or(version.id.clone(), |it| format!("{}-{}", version.id, it.id));
    tracing::debug!("Loading version info for Minecraft {version_id}");
    let path = st
        .directories
        .version_dir(&version_id)
        .join(format!("{version_id}.json"));

    let res = if path.exists() && !force.unwrap_or(false) {
        let mut info: GameVersionInfo = io::read(&path)
            .err_into::<crate::Error>()
            .await
            .and_then(|ref it| Ok(serde_json::from_slice(it)?))?;
        if normalize_version_info_libraries(
            mod_loader,
            &version.id,
            &mut info,
            "cache",
        ) {
            write_version_info(&path, serde_json::to_vec(&info)?).await?;
        }
        info
    } else {
        tracing::info!(
            "Downloading version info for version {} from {}",
            &version.id,
            version.url
        );
        if let Some(reporter) = reporter {
            reporter
                .set_context(
                    InstallErrorContext::new(
                        "download Minecraft version metadata",
                    )
                    .minecraft_version(version.id.clone())
                    .urls(vec![version.url.clone()])
                    .target_path(path.display().to_string())
                    .build(),
                )
                .await?;
        }
        let mut info = match fetch_json(
            Method::GET,
            &version.url,
            Some(&version.sha1),
            None,
            None,
            &st.api_semaphore,
            &st.pool,
        )
        .await
        {
            Ok(info) => info,
            Err(primary_error) => {
                tracing::warn!(
                    minecraft_version = %version.id,
                    url = %version.url,
                    error = %primary_error,
                    "Version metadata failed; looking up the Mojang fallback"
                );
                let manifest: d::minecraft::VersionManifest = match fetch_json(
                    Method::GET,
                    d::minecraft::VERSION_MANIFEST_URL,
                    None,
                    None,
                    None,
                    &st.api_semaphore,
                    &st.pool,
                )
                .await
                {
                    Ok(manifest) => manifest,
                    Err(error) => {
                        tracing::warn!(
                            minecraft_version = %version.id,
                            error = %error,
                            "Mojang manifest fallback failed"
                        );
                        return Err(primary_error);
                    }
                };
                let Some(fallback_version) = manifest
                    .versions
                    .into_iter()
                    .find(|candidate| candidate.id == version.id)
                else {
                    return Err(primary_error);
                };
                if fallback_version.url == version.url {
                    return Err(primary_error);
                }
                if let Some(reporter) = reporter {
                    reporter
                        .set_context(
                            InstallErrorContext::new(
                                "download Minecraft version metadata",
                            )
                            .minecraft_version(version.id.clone())
                            .urls(vec![
                                version.url.clone(),
                                fallback_version.url.clone(),
                            ])
                            .target_path(path.display().to_string())
                            .build(),
                        )
                        .await?;
                }
                fetch_json(
                    Method::GET,
                    &fallback_version.url,
                    Some(&fallback_version.sha1),
                    None,
                    None,
                    &st.api_semaphore,
                    &st.pool,
                )
                .await?
            }
        };

        if let Some(loader) = loader {
            if let Some(reporter) = reporter {
                reporter
                    .set_context(
                        InstallErrorContext::new(
                            "download loader version metadata",
                        )
                        .minecraft_version(version.id.clone())
                        .urls(vec![loader.url.clone()])
                        .target_path(path.display().to_string())
                        .build(),
                    )
                    .await?;
            }
            let partial: d::modded::PartialVersionInfo =
                if mod_loader == ModLoader::OptiFine {
                    crate::launcher::optifine::build_partial_version_info(
                        st,
                        &info,
                        &version.id,
                        &loader.id,
                    )
                    .await?
                } else {
                    crate::api::loader_metadata::resolve_loader_profile(
                        st,
                        &version.id,
                        loader,
                    )
                    .await?
                };
            info = d::modded::merge_partial_version(partial, info);
        }

        normalize_version_info_libraries(
            mod_loader,
            &version.id,
            &mut info,
            "network",
        );

        info.id.clone_from(&version_id);

        write_version_info(&path, serde_json::to_vec(&info)?).await?;
        info
    };

    crate::api::loader_metadata::ensure_installer_artifacts(st, &res).await?;

    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 5.0, None)?;
    }

    tracing::debug!("Loaded version info for Minecraft {version_id}");
    Ok(res)
}

pub async fn load_local_version_info(
    st: &State,
    version: &GameVersion,
    mod_loader: ModLoader,
    loader: Option<&LoaderVersion>,
) -> crate::Result<GameVersionInfo> {
    let version_id = loader
        .map_or(version.id.clone(), |it| format!("{}-{}", version.id, it.id));
    let path = st
        .directories
        .version_dir(&version_id)
        .join(format!("{version_id}.json"));

    if !path.is_file() {
        return Err(crate::ErrorKind::LauncherError(format!(
            "Offline mode can only launch fully downloaded instances; missing {}",
            path.display()
        ))
        .as_error());
    }

    let bytes = io::read(&path).err_into::<crate::Error>().await?;
    let mut info: GameVersionInfo = serde_json::from_slice(&bytes)?;
    if normalize_version_info_libraries(
        mod_loader,
        &version.id,
        &mut info,
        "cache",
    ) {
        write_version_info(&path, serde_json::to_vec(&info)?).await?;
    }
    Ok(info)
}

async fn write_version_info(path: &Path, data: Vec<u8>) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        io::create_dir_all(parent).await?;
    }
    io::write(path, data).await?;
    Ok(())
}

fn normalize_version_info_libraries(
    loader: ModLoader,
    game_version: &str,
    version_info: &mut GameVersionInfo,
    version_info_source: &str,
) -> bool {
    let removed = d::modded::normalize_loader_libraries(
        loader.as_meta_str(),
        game_version,
        &mut version_info.libraries,
    );

    for removed_library in &removed {
        tracing::info!(
            loader = loader.as_meta_str(),
            game_version,
            removed_library,
            version_info_source,
            "Removed obsolete Fabric intermediary library for unobfuscated Minecraft version"
        );
    }

    !removed.is_empty()
}

pub fn ensure_local_log_config(
    st: &State,
    version_info: &GameVersionInfo,
) -> crate::Result<()> {
    let log_download = version_info
        .logging
        .as_ref()
        .and_then(|logging| logging.get(&LoggingSide::Client));
    let Some(LoggingConfiguration::Log4j2Xml { file, .. }) = log_download
    else {
        return Ok(());
    };

    let path = st.directories.log_configs_dir().join(&file.id);
    if !path.is_file() {
        return Err(crate::ErrorKind::LauncherError(format!(
            "Offline mode can only launch fully downloaded instances; missing {}",
            path.display()
        ))
        .as_error());
    }

    Ok(())
}

#[tracing::instrument(skip_all)]

pub async fn download_client(
    st: &State,
    local_source: Option<&LocalRuntimeSource>,
    game_version: &str,
    version_info: &GameVersionInfo,
    loading_bar: Option<&LoadingBarId>,
    force: bool,
    progress: Option<MinecraftDownloadProgress>,
) -> crate::Result<()> {
    let version = &version_info.id;
    tracing::debug!("Locating client for version {version}");
    let client_download = version_info
        .downloads
        .get(&d::minecraft::DownloadType::Client)
        .ok_or(
            crate::ErrorKind::LauncherError(format!(
                "No client downloads exist for version {version}"
            ))
            .as_error(),
        )?;
    let path = st
        .directories
        .version_dir(version)
        .join(format!("{version}.jar"));

    if !path.exists() || force {
        let context = InstallErrorContext::new("download Minecraft client")
            .minecraft_version(version.to_string())
            .file_path(format!("{version}.jar"))
            .target_path(path.display().to_string())
            .build();
        let reused = download_or_reuse_local(
            st,
            local_source,
            &local_client_path(game_version),
            &path,
            Some(&client_download.sha1),
            Some(client_download.size as u64),
            progress.as_ref(),
            context.clone(),
            force,
            || {
                download_minecraft_file(
                    st,
                    &client_download.url,
                    Some(&client_download.sha1),
                    Some(client_download.size as u64),
                    &path,
                    ResourceClass::MinecraftLibrary,
                    ContentValidation::Jar,
                    force,
                    progress.clone(),
                    context,
                )
            },
        )
        .await?;
        if reused {
            tracing::trace!("Reused local client version {version}");
        } else {
            tracing::trace!("Fetched client version {version}");
        }
    }
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 9.0, None)?;
    }

    tracing::debug!("Client loaded for version {version}!");
    Ok(())
}

#[tracing::instrument(skip_all)]

pub async fn download_assets_index(
    st: &State,
    local_source: Option<&LocalRuntimeSource>,
    version: &GameVersionInfo,
    loading_bar: Option<&LoadingBarId>,
    force: bool,
    progress: Option<MinecraftDownloadProgress>,
) -> crate::Result<AssetsIndex> {
    tracing::debug!("Loading assets index");
    let path = st
        .directories
        .assets_index_dir()
        .join(format!("{}.json", &version.asset_index.id));

    let res = if path.exists() && !force {
        io::read(path)
            .err_into::<crate::Error>()
            .await
            .and_then(|ref it| Ok(serde_json::from_slice(it)?))
    } else {
        let context =
            InstallErrorContext::new("download Minecraft assets index")
                .minecraft_version(version.id.clone())
                .file_path(format!("{}.json", version.asset_index.id))
                .target_path(path.display().to_string())
                .build();
        let reused = download_or_reuse_local(
            st,
            local_source,
            &local_asset_index_path(&version.asset_index.id),
            &path,
            Some(&version.asset_index.sha1),
            Some(version.asset_index.size as u64),
            progress.as_ref(),
            context.clone(),
            force,
            || {
                download_minecraft_file(
                    st,
                    &version.asset_index.url,
                    Some(&version.asset_index.sha1),
                    Some(version.asset_index.size as u64),
                    &path,
                    ResourceClass::Metadata,
                    ContentValidation::Json,
                    force,
                    progress.clone(),
                    context,
                )
            },
        )
        .await?;
        if reused {
            tracing::info!("Reused local assets index");
        } else {
            tracing::info!("Fetched assets index");
        }
        let index = serde_json::from_slice(&io::read(&path).await?)?;
        Ok(index)
    }?;

    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 5.0, None)?;
    }
    tracing::debug!("Assets index successfully loaded!");
    Ok(res)
}

#[tracing::instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn download_assets(
    st: &State,
    local_source: Option<&LocalRuntimeSource>,
    with_legacy: bool,
    index: &AssetsIndex,
    loading_bar: Option<&LoadingBarId>,
    loading_amount: f64,
    force: bool,
    progress: Option<MinecraftDownloadProgress>,
) -> crate::Result<()> {
    tracing::debug!("Loading assets");
    let num_futs = index.objects.len();
    let assets = stream::iter(index.objects.iter())
        .map(Ok::<(&String, &Asset), crate::Error>);

    loading_try_for_each_concurrent(assets,
			Some(st.download_concurrency().saturating_mul(2)),
            loading_bar,
            loading_amount,
            num_futs,
			None,
            |(name, asset)| {
                let progress = progress.clone();
                async move {
                let hash = &asset.hash;
                let resource_path = st.directories.object_dir(hash);
                let legacy_resource_path = st.directories.legacy_assets_dir().join(
                    name.replace('/', &String::from(std::path::MAIN_SEPARATOR))
                );
                let should_fetch_object = !resource_path.exists() || force;
                let should_fetch_legacy =
                    (with_legacy && !legacy_resource_path.exists()) || force;
                let fetch_progress = if should_fetch_object || should_fetch_legacy {
                    progress.clone()
                } else {
                    None
                };
                let object_progress = fetch_progress.clone();
                let legacy_progress = if should_fetch_object {
                    None
                } else {
                    fetch_progress
                };
                let url = format!(
                    "https://resources.download.minecraft.net/{sub_hash}/{hash}",
                    sub_hash = &hash[..2]
                );

                tokio::try_join! {
                    async {
                        if should_fetch_object {
                            let context =
                                InstallErrorContext::new("download Minecraft asset")
                                    .file_path(name.clone())
                                    .target_path(resource_path.display().to_string())
                                    .build();
                            let reused = download_or_reuse_local(
                                st,
                                local_source,
                                &local_asset_object_path(hash),
                                &resource_path,
                                Some(hash),
                                Some(asset.size as u64),
                                object_progress.as_ref(),
                                context.clone(),
                                force,
                                || {
                                    download_minecraft_file(
                                        st,
                                        &url,
                                        Some(hash),
                                        Some(asset.size as u64),
                                        &resource_path,
                                        ResourceClass::MinecraftAsset,
                                        ContentValidation::None,
                                        force,
                                        object_progress.clone(),
                                        context,
                                    )
                                },
                            )
                            .await?;
                            if reused {
                                tracing::trace!("Reused asset with hash {hash}");
                            } else {
                                tracing::trace!("Fetched asset with hash {hash}");
                            }
                        }
                        Ok::<_, crate::Error>(())
                    },
                    async {
                        if should_fetch_legacy {
                            download_minecraft_file(
                                st,
                                &url,
                                Some(hash),
                                Some(asset.size as u64),
                                &legacy_resource_path,
                                ResourceClass::MinecraftAsset,
                                ContentValidation::None,
                                force,
                                legacy_progress,
                                InstallErrorContext::new("download Minecraft asset")
                                    .file_path(name.clone())
                                    .target_path(legacy_resource_path.display().to_string())
                                    .build(),
                            )
                            .await?;
                            tracing::trace!("Fetched legacy asset with hash {hash}");
                        }
                        Ok::<_, crate::Error>(())
                    },
                }?;

                tracing::trace!("Loaded asset with hash {hash}");
                Ok(())
                }
            }).await?;
    tracing::debug!("Done loading assets!");
    Ok(())
}

#[tracing::instrument(skip_all, fields(version))]
#[allow(clippy::too_many_arguments)]
pub async fn download_libraries(
    st: &State,
    local_source: Option<&LocalRuntimeSource>,
    libraries: &[Library],
    version: &str,
    loading_bar: Option<&LoadingBarId>,
    loading_amount: f64,
    java_arch: &str,
    force: bool,
    minecraft_updated: bool,
    progress: Option<MinecraftDownloadProgress>,
) -> crate::Result<()> {
    tracing::debug!("Loading libraries");

    tokio::try_join! {
        io::create_dir_all(st.directories.libraries_dir()),
        io::create_dir_all(st.directories.version_natives_dir(version))
    }?;
    let libraries =
        deduplicate_native_downloads(libraries, java_arch, minecraft_updated);
    let num_files = libraries.len();
    loading_try_for_each_concurrent(
		stream::iter(libraries).map(Ok::<&Library, crate::Error>),
		Some(st.download_concurrency().saturating_mul(2)),
        loading_bar,
        loading_amount,
        num_files,
        None,
        |library| {
            let progress = progress.clone();
            async move {
            if let Some(rules) = &library.rules
                && !parse_rules(
                    rules,
                    java_arch,
                    &QuickPlayType::None,
                    minecraft_updated,
                )
            {
                tracing::trace!("Skipped library {}", &library.name);
                return Ok(());
            }

            if !library.downloadable {
                tracing::trace!(
                    "Skipped non-downloadable library {}",
                    &library.name
                );
                return Ok(());
            }

            // When a library has natives, we only need to download such natives, as PrismLauncher does
            if let Some((os_key, classifiers)) =
                library.natives_os_key_and_classifiers(java_arch)
            {
                let parsed_key = os_key
                    .replace("${arch}", crate::util::platform::ARCH_WIDTH);

                if let Some(native) = classifiers.get(&parsed_key) {
                    let native_cache_path = st
                        .directories
                        .caches_dir()
                        .join("minecraft-natives")
                        .join(format!("{}.jar", native.sha1));
                    let context = InstallErrorContext::new(
                        "download Minecraft native library",
                    )
                    .minecraft_version(version.to_string())
                    .file_path(library.name.clone())
                    .target_path(
                        st.directories
                            .version_natives_dir(version)
                            .display()
                            .to_string(),
                    )
                    .build();
                    let local_relative = local_native_library_path(
                        library,
                        native,
                        &parsed_key,
                    )?;
                    let reused = download_or_reuse_local(
                        st,
                        local_source,
                        &local_relative,
                        &native_cache_path,
                        Some(&native.sha1),
                        Some(native.size as u64),
                        progress.as_ref(),
                        context.clone(),
                        force,
                        || {
                            download_minecraft_file(
                                st,
                                &native.url,
                                Some(&native.sha1),
                                Some(native.size as u64),
                                &native_cache_path,
                                ResourceClass::MinecraftLibrary,
                                ContentValidation::Jar,
                                force,
                                progress.clone(),
                                context,
                            )
                        },
                    )
                    .await?;
                    if reused {
                        tracing::trace!("Reused native {}", &library.name);
                    }

                    let native_target =
                        st.directories.version_natives_dir(version);
                    let library_name = library.name.clone();
                    tokio::task::spawn_blocking(move || {
                        let file = std::fs::File::open(&native_cache_path)?;
                        let mut archive = zip::ZipArchive::new(file).map_err(
                            |error| {
                                crate::ErrorKind::LauncherError(format!(
                                    "Failed to open native library archive {library_name}: {error}",
                                ))
                            },
                        )?;
                        archive.extract(native_target).map_err(|error| {
                            crate::ErrorKind::LauncherError(format!(
                                "Failed to extract native library {library_name}: {error}",
                            ))
                        })?;
                        Ok::<_, crate::Error>(())
                    })
                    .await??;
                    tracing::debug!("Fetched native {}", &library.name);
                }
            } else {
                let artifact_path = d::get_path_from_artifact(&library.name)?;
                let path = st.directories.libraries_dir().join(&artifact_path);

                if path.exists() && !force {
                    return Ok(());
                }

                if let Some(d::minecraft::LibraryDownloads {
                    artifact: Some(ref artifact),
                    ..
                }) = library.downloads
                    && !artifact.url.is_empty()
                {
                    let local_relative = local_library_path(&library.name)?;
                    let context = InstallErrorContext::new(
                        "download Minecraft library",
                    )
                    .minecraft_version(version.to_string())
                    .file_path(library.name.clone())
                    .target_path(path.display().to_string())
                    .build();
                    let reused = download_or_reuse_local(
                        st,
                        local_source,
                        &local_relative,
                        &path,
                        Some(&artifact.sha1),
                        Some(artifact.size as u64),
                        progress.as_ref(),
                        context.clone(),
                        force,
                        || {
                            download_minecraft_file(
                                st,
                                &artifact.url,
                                Some(&artifact.sha1),
                                Some(artifact.size as u64),
                                &path,
                                ResourceClass::MinecraftLibrary,
                                ContentValidation::None,
                                force,
                                progress.clone(),
                                context,
                            )
                        },
                    )
                    .await?;
                    if reused {
                        tracing::trace!(
                            "Reused library {} to path {:?}",
                            &library.name,
                            &path
                        );
                    } else {
                        tracing::trace!(
                            "Fetched library {} to path {:?}",
                            &library.name,
                            &path
                        );
                    }
                } else {
                    let Some(urls) = legacy_library_download_urls(
                        library.url.as_deref(),
                        &artifact_path,
                    ) else {
                        return Err(crate::ErrorKind::LauncherError(format!(
                            "No safe Maven repository is known for required library {}",
                            library.name
                        ))
                        .into());
                    };

                    let local_relative = local_library_path(&library.name)?;
                    let context = InstallErrorContext::new(
                        "download loader library",
                    )
                    .minecraft_version(version.to_string())
                    .file_path(library.name.clone())
                    .target_path(path.display().to_string())
                    .build();
                    let reused = download_or_reuse_local(
                        st,
                        local_source,
                        &local_relative,
                        &path,
                        legacy_library_sha1(library),
                        None,
                        progress.as_ref(),
                        context.clone(),
                        force,
                        || {
                            download_minecraft_file_with_candidates(
                                st,
                                &urls,
                                legacy_library_sha1(library),
                                None,
                                &path,
                                ResourceClass::Loader,
                                legacy_library_content_validation(&artifact_path),
                                force,
                                progress.clone(),
                                context,
                            )
                        },
                    )
                    .await?;
                    if reused {
                        tracing::debug!(
                            "Reused legacy library {} to path {:?}",
                            &library.name,
                            &path
                        );
                    } else {
                        tracing::debug!(
                            "Fetched legacy library {} to path {:?}",
                            &library.name,
                            &path
                        );
                    }
                }
            }

            tracing::debug!("Loaded library {}", library.name);
            Ok(())
            }
        },
    )
    .await?;

    tracing::debug!("Done loading libraries!");
    Ok(())
}

fn deduplicate_native_downloads<'a>(
    libraries: &'a [Library],
    java_arch: &str,
    minecraft_updated: bool,
) -> Vec<&'a Library> {
    let mut native_hashes = HashSet::new();
    libraries
        .iter()
        .filter(|library| {
            if let Some(rules) = &library.rules
                && !parse_rules(
                    rules,
                    java_arch,
                    &QuickPlayType::None,
                    minecraft_updated,
                )
            {
                return true;
            }
            if !library.downloadable {
                return true;
            }
            let Some((os_key, classifiers)) =
                library.natives_os_key_and_classifiers(java_arch)
            else {
                return true;
            };
            let parsed_key =
                os_key.replace("${arch}", crate::util::platform::ARCH_WIDTH);
            let Some(native) = classifiers.get(&parsed_key) else {
                return true;
            };
            if native.sha1.is_empty() {
                return true;
            }
            let first = native_hashes.insert(native.sha1.clone());
            if !first {
                tracing::debug!(
                    "Skipped duplicate native archive {} ({})",
                    library.name,
                    native.sha1
                );
            }
            first
        })
        .collect()
}

#[tracing::instrument(skip_all)]
pub async fn download_log_config(
    st: &State,
    local_source: Option<&LocalRuntimeSource>,
    version_info: &GameVersionInfo,
    loading_bar: Option<&LoadingBarId>,
    force: bool,
    progress: Option<MinecraftDownloadProgress>,
) -> crate::Result<bool> {
    let log_download = version_info
        .logging
        .as_ref()
        .and_then(|x| x.get(&LoggingSide::Client));
    let Some(LoggingConfiguration::Log4j2Xml {
        file: log_download, ..
    }) = log_download
    else {
        if let Some(loading_bar) = loading_bar {
            emit_loading(loading_bar, 1.0, None)?;
        }
        return Ok(false);
    };

    let path = st.directories.log_configs_dir().join(&log_download.id);

    if !path.exists() || force {
        let context = InstallErrorContext::new("download Minecraft log config")
            .minecraft_version(version_info.id.clone())
            .file_path(log_download.id.clone())
            .target_path(path.display().to_string())
            .build();
        let reused = download_or_reuse_local(
            st,
            local_source,
            &local_log_config_path(&log_download.id),
            &path,
            Some(&log_download.sha1),
            Some(log_download.size as u64),
            progress.as_ref(),
            context.clone(),
            force,
            || {
                download_minecraft_file(
                    st,
                    &log_download.url,
                    Some(&log_download.sha1),
                    Some(log_download.size as u64),
                    &path,
                    ResourceClass::MinecraftLibrary,
                    ContentValidation::None,
                    force,
                    progress.clone(),
                    context,
                )
            },
        )
        .await?;
        if reused {
            tracing::trace!("Reused log config {}", log_download.id);
        } else {
            tracing::trace!("Fetched log config {}", log_download.id);
        }
    }
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 1.0, None)?;
    }

    tracing::debug!("Log config {} loaded", log_download.id);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(values: &[&str]) -> Option<Vec<String>> {
        Some(values.iter().map(|value| (*value).to_string()).collect())
    }

    #[tokio::test]
    async fn writing_version_info_creates_missing_parent_directory() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join("meta/versions/1.21.11-21.11.44/1.21.11-21.11.44.json");

        assert!(!path.parent().unwrap().exists());
        write_version_info(&path, br#"{"id":"1.21.11-21.11.44"}"#.to_vec())
            .await
            .unwrap();

        assert_eq!(
            io::read(&path).await.unwrap(),
            br#"{"id":"1.21.11-21.11.44"}"#
        );
    }

    #[test]
    fn legacy_launcher_meta_maven_uses_canonical_repositories() {
        assert_eq!(
            legacy_library_download_urls(
                Some("https://launcher-meta.modrinth.com/maven/"),
                "net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
            ),
            urls(&[
                "https://maven.fabricmc.net/net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
                "https://launcher-meta.modrinth.com/maven/net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
                "https://libraries.minecraft.net/net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
            ]),
        );
        assert_eq!(
            legacy_library_download_urls(
                Some("https://launcher-meta.modrinth.com/maven/"),
                "org/ow2/asm/asm/9.10.1/asm-9.10.1.jar",
            ),
            urls(&[
                "https://repo.maven.apache.org/maven2/org/ow2/asm/asm/9.10.1/asm-9.10.1.jar",
                "https://launcher-meta.modrinth.com/maven/org/ow2/asm/asm/9.10.1/asm-9.10.1.jar",
                "https://libraries.minecraft.net/org/ow2/asm/asm/9.10.1/asm-9.10.1.jar",
            ]),
        );
        assert_eq!(
            legacy_library_download_urls(
                Some("https://launcher-meta.modrinth.com/maven/"),
                "com/modrinth/daedalus/forge-installer-extracts/1.20.1-47.4.20/forge-installer-extracts-1.20.1-47.4.20-client.lzma",
            ),
            urls(&[
                "https://launcher-meta.modrinth.com/maven/com/modrinth/daedalus/forge-installer-extracts/1.20.1-47.4.20/forge-installer-extracts-1.20.1-47.4.20-client.lzma",
                "https://libraries.minecraft.net/com/modrinth/daedalus/forge-installer-extracts/1.20.1-47.4.20/forge-installer-extracts-1.20.1-47.4.20-client.lzma",
            ]),
        );
        assert_eq!(
            legacy_library_content_validation("library.jar"),
            ContentValidation::Jar,
        );
        assert_eq!(
            legacy_library_content_validation("library.lzma"),
            ContentValidation::None,
        );
    }

    #[test]
    fn legacy_scala_libraries_use_maven_central() {
        let artifact_path = "org/scala-lang/plugins/scala-continuations-library_2.11/1.0.2/scala-continuations-library_2.11-1.0.2.jar";
        let expected = urls(&[
            "https://repo.maven.apache.org/maven2/org/scala-lang/plugins/scala-continuations-library_2.11/1.0.2/scala-continuations-library_2.11-1.0.2.jar",
            "https://launcher-meta.modrinth.com/maven/org/scala-lang/plugins/scala-continuations-library_2.11/1.0.2/scala-continuations-library_2.11-1.0.2.jar",
            "https://libraries.minecraft.net/org/scala-lang/plugins/scala-continuations-library_2.11/1.0.2/scala-continuations-library_2.11-1.0.2.jar",
        ]);

        assert_eq!(
            legacy_library_download_urls(
                Some("https://launcher-meta.modrinth.com/maven"),
                artifact_path,
            ),
            expected,
        );
        assert_eq!(
            legacy_library_download_urls(
                Some("https://launcher-meta.modrinth.com/maven/"),
                artifact_path,
            ),
            expected,
        );
    }

    #[test]
    fn historical_forge_scala_modules_keep_archived_coordinates() {
        let library: Library = serde_json::from_value(serde_json::json!({
            "name": "org.scala-lang:scala-xml_2.11:1.0.2",
            "url": "https://launcher-meta.modrinth.com/maven/",
            "checksums": [
                "7a80ec00aec122fba7cd4e0d4cdd87ff7e4cb6d0",
                "62736b01689d56b6d09a0164b7ef9da2b0b9633d"
            ]
        }))
        .unwrap();
        let artifact_path = d::get_path_from_artifact(&library.name).unwrap();

        assert_eq!(
            artifact_path,
            "org/scala-lang/scala-xml_2.11/1.0.2/scala-xml_2.11-1.0.2.jar",
        );
        assert_eq!(
            legacy_library_sha1(&library),
            Some("7a80ec00aec122fba7cd4e0d4cdd87ff7e4cb6d0"),
        );

        for artifact_path in [
            "org/scala-lang/scala-parser-combinators_2.11/1.0.1/scala-parser-combinators_2.11-1.0.1.jar",
            "org/scala-lang/scala-swing_2.11/1.0.1/scala-swing_2.11-1.0.1.jar",
            "org/scala-lang/scala-xml_2.11/1.0.2/scala-xml_2.11-1.0.2.jar",
        ] {
            assert_eq!(
                legacy_library_download_urls(
                    Some("https://launcher-meta.modrinth.com/maven/"),
                    artifact_path,
                ),
                Some(vec![
                    format!("{FORGE_MAVEN}/{artifact_path}"),
                    format!("{LAUNCHER_META_MAVEN}/{artifact_path}"),
                    format!("{LIBRARIES_MAVEN}/{artifact_path}"),
                ]),
            );
        }
    }

    #[test]
    fn legacy_forge_maven_central_groups_use_maven_central() {
        for artifact_path in [
            "org/jline/jline/3.5.1/jline-3.5.1.jar",
            "jline/jline/2.13/jline-2.13.jar",
            "net/java/dev/jna/jna/4.4.0/jna-4.4.0.jar",
            "com/typesafe/akka/akka-actor_2.11/2.3.3/akka-actor_2.11-2.3.3.jar",
            "com/typesafe/config/1.2.1/config-1.2.1.jar",
        ] {
            assert_eq!(
                legacy_library_download_urls(
                    Some("https://launcher-meta.modrinth.com/maven"),
                    artifact_path,
                ),
                Some(vec![
                    format!("{MAVEN_CENTRAL}/{artifact_path}"),
                    format!("{LAUNCHER_META_MAVEN}/{artifact_path}"),
                    format!("{LIBRARIES_MAVEN}/{artifact_path}"),
                ]),
            );
        }
    }

    #[test]
    fn legacy_maven_download_uses_trusted_declared_paths() {
        assert_eq!(
            legacy_library_download_urls(
                Some("https://launcher-meta.modrinth.com/maven/"),
                "example/unknown/1/unknown-1.jar",
            ),
            urls(&[
                "https://launcher-meta.modrinth.com/maven/example/unknown/1/unknown-1.jar",
                "https://libraries.minecraft.net/example/unknown/1/unknown-1.jar",
            ]),
        );
        assert_eq!(
            legacy_library_download_urls(
                Some("https://maven.minecraftforge.net"),
                "example/unknown/1/unknown-1.jar",
            ),
            urls(&[
                "https://maven.minecraftforge.net/example/unknown/1/unknown-1.jar",
                "https://libraries.minecraft.net/example/unknown/1/unknown-1.jar",
            ]),
        );
        assert_eq!(
            legacy_library_download_urls(
                Some("https://example.invalid/maven"),
                "net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
            ),
            urls(&[
                "https://example.invalid/maven/net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
                "https://maven.fabricmc.net/net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
                "https://libraries.minecraft.net/net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
            ]),
        );
        for repository in [
            "http://example.invalid/maven",
            "https://user@example.invalid/maven",
            "https://example.invalid/maven?token=secret",
        ] {
            assert_eq!(
                legacy_library_download_urls(
                    Some(repository),
                    "example/unknown/1/unknown-1.jar",
                ),
                None,
            );
        }
        for artifact_path in [
            "/example/unknown.jar",
            "../example/unknown.jar",
            "example\\unknown.jar",
        ] {
            assert_eq!(
                legacy_library_download_urls(
                    Some("https://example.invalid/maven"),
                    artifact_path,
                ),
                None,
            );
        }
    }
}
