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
use daedalus::minecraft::{LoggingConfiguration, LoggingSide};
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
    future::Future,
    path::Path,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

const MINECRAFT_DOWNLOAD_PROGRESS_MIN_BYTES: u64 = 256 * 1024;

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
    let mirrors = minecraft_library_mirrors(url);
    let mut context = context;
    context.urls.extend(mirrors.iter().cloned());
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
    let request = DownloadRequest::new(url, resource)
        .with_candidate_urls(mirrors.into_iter().skip(1))
        .with_integrity(integrity);
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
            let delta = downloaded.saturating_sub(previous);
            let progress = progress.clone();
            Box::pin(async move { progress.add_bytes(delta).await })
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

fn legacy_library_download_url(
    repository: Option<&str>,
    artifact_path: &str,
) -> Option<String> {
    let repository =
        repository.unwrap_or(LIBRARIES_MAVEN).trim_end_matches('/');
    let repository = match repository {
        LAUNCHER_META_MAVEN => legacy_library_repository(artifact_path)?,
        LIBRARIES_MAVEN | FABRIC_MAVEN | FORGE_MAVEN | NEOFORGE_MAVEN
        | QUILT_MAVEN | SPONGE_MAVEN | MAVEN_CENTRAL => repository,
        _ => return None,
    };
    Some(format!("{repository}/{artifact_path}"))
}

fn legacy_library_repository(artifact_path: &str) -> Option<&'static str> {
    if artifact_path.starts_with("net/fabricmc/") {
        Some(FABRIC_MAVEN)
    } else if artifact_path.starts_with("org/quiltmc/") {
        Some(QUILT_MAVEN)
    } else if artifact_path.starts_with("net/minecraftforge/")
        || artifact_path.starts_with("cpw/mods/")
    {
        Some(FORGE_MAVEN)
    } else if artifact_path.starts_with("net/neoforged/") {
        Some(NEOFORGE_MAVEN)
    } else if artifact_path.starts_with("org/spongepowered/") {
        Some(SPONGE_MAVEN)
    } else if artifact_path.starts_with("net/minecraft/launchwrapper/") {
        Some(LIBRARIES_MAVEN)
    } else if artifact_path.starts_with("org/ow2/") {
        Some(MAVEN_CENTRAL)
    } else if artifact_path.starts_with("com/modrinth/daedalus/") {
        Some(LAUNCHER_META_MAVEN)
    } else {
        None
    }
}

fn legacy_library_content_validation(artifact_path: &str) -> ContentValidation {
    artifact_path
        .ends_with(".jar")
        .then_some(ContentValidation::Jar)
        .unwrap_or(ContentValidation::None)
}

fn legacy_library_sha1(library: &Library) -> Option<&str> {
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
pub async fn download_minecraft(
    st: &State,
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

    // 5
    let assets_index = download_assets_index(
        st,
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

    let amount = if version.processors.as_ref().is_some_and(|x| !x.is_empty()) {
        25.0
    } else {
        40.0
    };

    tokio::try_join! {
        // Total loading sums to 90/60
        download_client(st, version, loading_bar, force, progress.clone()), // 9
        download_log_config(st, version, loading_bar, force, progress.clone()),
        download_assets(st, version.assets == "legacy", &assets_index, loading_bar, amount, force, progress.clone()), // 40
        download_libraries(st, version.libraries.as_slice(), &version.id, loading_bar, amount, java_arch, force, minecraft_updated, progress.clone()) // 40
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
                    fetch_json(
                        Method::GET,
                        &loader.url,
                        None,
                        None,
                        None,
                        &st.api_semaphore,
                        &st.pool,
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
        download_minecraft_file(
            st,
            &client_download.url,
            Some(&client_download.sha1),
            Some(client_download.size as u64),
            &path,
            ResourceClass::MinecraftLibrary,
            ContentValidation::Jar,
            force,
            progress,
            InstallErrorContext::new("download Minecraft client")
                .minecraft_version(version.to_string())
                .file_path(format!("{version}.jar"))
                .target_path(path.display().to_string())
                .build(),
        )
        .await?;
        tracing::trace!("Fetched client version {version}");
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
        download_minecraft_file(
            st,
            &version.asset_index.url,
            Some(&version.asset_index.sha1),
            Some(version.asset_index.size as u64),
            &path,
            ResourceClass::Metadata,
            ContentValidation::Json,
            force,
            progress,
            InstallErrorContext::new("download Minecraft assets index")
                .minecraft_version(version.id.clone())
                .file_path(format!("{}.json", version.asset_index.id))
                .target_path(path.display().to_string())
                .build(),
        )
        .await?;
        let index = serde_json::from_slice(&io::read(&path).await?)?;
        tracing::info!("Fetched assets index");
        Ok(index)
    }?;

    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 5.0, None)?;
    }
    tracing::debug!("Assets index successfully loaded!");
    Ok(res)
}

#[tracing::instrument(skip_all)]

pub async fn download_assets(
    st: &State,
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
            None,
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
                            download_minecraft_file(
                                st,
                                &url,
                                Some(hash),
                                Some(asset.size as u64),
                                &resource_path,
                                ResourceClass::MinecraftAsset,
                                ContentValidation::None,
                                force,
                                object_progress,
                                InstallErrorContext::new("download Minecraft asset")
                                    .file_path(name.clone())
                                    .target_path(resource_path.display().to_string())
                                    .build(),
                            )
                            .await?;
                            tracing::trace!("Fetched asset with hash {hash}");
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
    let num_files = libraries.len();
    loading_try_for_each_concurrent(
        stream::iter(libraries.iter()).map(Ok::<&Library, crate::Error>),
        None,
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
                        InstallErrorContext::new("download Minecraft native library")
                            .minecraft_version(version.to_string())
                            .file_path(library.name.clone())
                            .target_path(
                                st.directories
                                    .version_natives_dir(version)
                                    .display()
                                    .to_string(),
                            )
                            .build(),
                    )
                    .await?;

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
                        InstallErrorContext::new("download Minecraft library")
                            .minecraft_version(version.to_string())
                            .file_path(library.name.clone())
                            .target_path(path.display().to_string())
                            .build(),
                    )
                    .await?;

                    tracing::trace!(
                        "Fetched library {} to path {:?}",
                        &library.name,
                        &path
                    );
                } else {
                    let Some(url) = legacy_library_download_url(
                        library.url.as_deref(),
                        &artifact_path,
                    ) else {
                        return Err(crate::ErrorKind::LauncherError(format!(
                            "No safe Maven repository is known for required library {}",
                            library.name
                        ))
                        .into());
                    };

                    download_minecraft_file(
                        st,
                        &url,
                        legacy_library_sha1(library),
                        None,
                        &path,
                        ResourceClass::Loader,
                        legacy_library_content_validation(&artifact_path),
                        force,
                        progress.clone(),
                        InstallErrorContext::new("download loader library")
                            .minecraft_version(version.to_string())
                            .file_path(library.name.clone())
                            .target_path(path.display().to_string())
                            .build(),
                    )
                    .await?;

                    tracing::debug!(
                        "Fetched legacy library {} to path {:?}",
                        &library.name,
                        &path
                    );
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

#[tracing::instrument(skip_all)]
pub async fn download_log_config(
    st: &State,
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
        download_minecraft_file(
            st,
            &log_download.url,
            Some(&log_download.sha1),
            Some(log_download.size as u64),
            &path,
            ResourceClass::MinecraftLibrary,
            ContentValidation::None,
            force,
            progress,
            InstallErrorContext::new("download Minecraft log config")
                .minecraft_version(version_info.id.clone())
                .file_path(log_download.id.clone())
                .target_path(path.display().to_string())
                .build(),
        )
        .await?;
        tracing::trace!("Fetched log config {}", log_download.id);
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
            legacy_library_download_url(
                Some("https://launcher-meta.modrinth.com/maven/"),
                "net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
            ),
            Some(
                "https://maven.fabricmc.net/net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar".to_string(),
            ),
        );
        assert_eq!(
            legacy_library_download_url(
                Some("https://launcher-meta.modrinth.com/maven/"),
                "org/ow2/asm/asm/9.10.1/asm-9.10.1.jar",
            ),
            Some(
                "https://repo.maven.apache.org/maven2/org/ow2/asm/asm/9.10.1/asm-9.10.1.jar".to_string(),
            ),
        );
        assert_eq!(
            legacy_library_download_url(
                Some("https://launcher-meta.modrinth.com/maven/"),
                "com/modrinth/daedalus/forge-installer-extracts/1.20.1-47.4.20/forge-installer-extracts-1.20.1-47.4.20-client.lzma",
            ),
            Some(
                "https://launcher-meta.modrinth.com/maven/com/modrinth/daedalus/forge-installer-extracts/1.20.1-47.4.20/forge-installer-extracts-1.20.1-47.4.20-client.lzma".to_string(),
            ),
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
    fn legacy_maven_download_rejects_unknown_repositories_and_paths() {
        assert_eq!(
            legacy_library_download_url(
                Some("https://launcher-meta.modrinth.com/maven/"),
                "example/unknown/1/unknown-1.jar",
            ),
            None,
        );
        assert_eq!(
            legacy_library_download_url(
                Some("https://example.com/maven/"),
                "net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.jar",
            ),
            None,
        );
    }
}
