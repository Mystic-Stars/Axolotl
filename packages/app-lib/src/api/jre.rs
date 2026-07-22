//! Authentication flow interface
use crate::event::emit::{emit_loading, init_loading};
use crate::install::{
    InstallErrorContext, InstallJavaStep, InstallPhaseDetails, InstallPhaseId,
    InstallProgress, InstallProgressReporter,
};
use crate::state::JavaVersion;
use crate::util::fetch::{
    ContentValidation, DownloadRequest, FetchProgressFn, Integrity,
    ResourceClass, download_to_path, fetch_json,
};
use dashmap::DashMap;
use futures::{TryStreamExt, stream};
use reqwest::Method;
use serde::Deserialize;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use sysinfo::{MemoryRefreshKind, RefreshKind};

use crate::util::io;
use crate::util::jre::extract_java_version;
use crate::{
    LoadingBarType, State,
    util::jre::{self},
};

pub async fn get_java_versions() -> crate::Result<DashMap<u32, JavaVersion>> {
    let state = State::get().await?;

    JavaVersion::get_all(&state.pool).await
}

pub async fn set_java_version(java_version: JavaVersion) -> crate::Result<()> {
    let state = State::get().await?;
    java_version.upsert(&state.pool).await?;
    Ok(())
}

// Searches for jres on the system given a java version (ex: 1.8, 1.17, 1.18)
// Allow higher allows for versions higher than the given version to be returned ('at least')
pub async fn find_filtered_jres(
    java_version: Option<u32>,
) -> crate::Result<Vec<JavaVersion>> {
    let jres = jre::get_all_jre().await?;

    // Filter out JREs that are not 1.17 or higher
    Ok(if let Some(java_version) = java_version {
        jres.into_iter()
            .filter(|jre| {
                let jre_version = extract_java_version(&jre.version);
                if let Ok(jre_version) = jre_version {
                    jre_version == java_version
                } else {
                    false
                }
            })
            .collect()
    } else {
        jres
    })
}

pub async fn auto_install_java(java_version: u32) -> crate::Result<PathBuf> {
    auto_install_java_with_loading(java_version, true).await
}

pub async fn auto_install_java_with_loading(
    java_version: u32,
    show_loading: bool,
) -> crate::Result<PathBuf> {
    auto_install_java_inner(java_version, show_loading, None).await
}

pub async fn auto_install_java_with_reporter(
    java_version: u32,
    reporter: InstallProgressReporter,
) -> crate::Result<PathBuf> {
    auto_install_java_inner(java_version, false, Some(reporter)).await
}

const JAVA_INSTALL_STEPS: u64 = 4;
const JAVA_DOWNLOAD_PROGRESS_MIN_BYTES: u64 = 256 * 1024;
const MOJANG_RUNTIME_INDEX_URL: &str = "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

static JAVA_INSTALL_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

type MojangRuntimeIndex =
    HashMap<String, HashMap<String, Vec<MojangRuntimeRelease>>>;

#[derive(Clone, Deserialize)]
struct RuntimeDownload {
    sha1: String,
    size: u64,
    url: String,
}

#[derive(Deserialize)]
struct MojangRuntimeRelease {
    manifest: RuntimeDownload,
}

#[derive(Deserialize)]
struct MojangRuntimeManifest {
    files: HashMap<String, MojangRuntimeFile>,
}

#[derive(Clone, Deserialize)]
struct MojangRuntimeDownloads {
    raw: RuntimeDownload,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum MojangRuntimeFile {
    Directory,
    File {
        downloads: MojangRuntimeDownloads,
        #[serde(default)]
        executable: bool,
    },
    Link {
        target: String,
    },
}

#[derive(Deserialize)]
struct AzulPackageSummary {
    package_uuid: String,
}

#[derive(Deserialize)]
struct AzulPackage {
    download_url: String,
    name: PathBuf,
    sha256_hash: String,
    size: u64,
}

#[derive(Clone, Default)]
struct JavaDownloadMetrics {
    source: Arc<Mutex<Option<String>>>,
    fallback_count: Arc<AtomicU64>,
}

impl JavaDownloadMetrics {
    fn record(&self, result: &crate::util::fetch::DownloadResult) {
        if result.attempts > 0
            && let Ok(mut source) = self.source.lock()
        {
            *source = Some(result.source.as_str().to_string());
        }
        self.fallback_count
            .fetch_add(result.fallback_count as u64, Ordering::Relaxed);
    }

    async fn finish(
        &self,
        reporter: Option<&InstallProgressReporter>,
    ) -> crate::Result<()> {
        let source = self.source.lock().ok().and_then(|source| source.clone());
        if let (Some(reporter), Some(source)) = (reporter, source) {
            reporter
                .record_download_metrics(
                    source,
                    self.fallback_count.load(Ordering::Relaxed),
                )
                .await?;
        }
        Ok(())
    }
}

async fn update_java_install_progress(
    reporter: Option<&InstallProgressReporter>,
    java_version: u32,
    step: InstallJavaStep,
    progress: Option<InstallProgress>,
) -> crate::Result<()> {
    if let Some(reporter) = reporter {
        reporter
            .update(
                InstallPhaseId::PreparingJava,
                progress,
                InstallPhaseDetails::Java {
                    major_version: java_version,
                    step,
                },
            )
            .await?;
    }

    Ok(())
}

fn java_step_progress(current: u64) -> InstallProgress {
    InstallProgress {
        current,
        total: JAVA_INSTALL_STEPS,
        secondary: None,
    }
}

async fn auto_install_java_inner(
    java_version: u32,
    show_loading: bool,
    reporter: Option<InstallProgressReporter>,
) -> crate::Result<PathBuf> {
    let state = State::get().await?;
    let _install_guard = JAVA_INSTALL_LOCK.lock().await;

    let loading_bar = if show_loading {
        Some(
            init_loading(
                LoadingBarType::JavaDownload {
                    version: java_version,
                },
                100.0,
                "Downloading java version",
            )
            .await?,
        )
    } else {
        None
    };

    if let Some(loading_bar) = &loading_bar {
        emit_loading(loading_bar, 0.0, Some("Fetching java version"))?;
    }
    update_java_install_progress(
        reporter.as_ref(),
        java_version,
        InstallJavaStep::FetchingMetadata,
        Some(java_step_progress(1)),
    )
    .await?;

    if let Some(path) = install_mojang_runtime(
        &state,
        java_version,
        loading_bar.as_ref(),
        reporter.as_ref(),
    )
    .await?
    {
        return Ok(path);
    }

    install_azul_runtime(
        &state,
        java_version,
        loading_bar.as_ref(),
        reporter.as_ref(),
    )
    .await
}

fn mojang_runtime_component(java_version: u32) -> Option<&'static str> {
    match java_version {
        8 => Some("jre-legacy"),
        16 => Some("java-runtime-alpha"),
        17 => Some("java-runtime-gamma"),
        21 => Some("java-runtime-delta"),
        25 => Some("java-runtime-epsilon"),
        _ => None,
    }
}

fn mojang_runtime_platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("windows-x64"),
        ("windows", "x86") => Some("windows-x86"),
        ("windows", "aarch64") => Some("windows-arm64"),
        ("macos", "x86_64") => Some("mac-os"),
        ("macos", "aarch64") => Some("mac-os-arm64"),
        ("linux", "x86_64") => Some("linux"),
        ("linux", "x86") => Some("linux-i386"),
        _ => None,
    }
}

fn runtime_executable_relative(platform: &str) -> PathBuf {
    if platform.starts_with("mac-os") {
        PathBuf::from("jre.bundle/Contents/Home/bin/java")
    } else {
        PathBuf::from("bin").join(jre::JAVA_BIN)
    }
}

fn safe_runtime_path(root: &Path, relative: &str) -> crate::Result<PathBuf> {
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(crate::ErrorKind::InputError(
            "Java runtime manifest contains an invalid path".to_string(),
        )
        .into());
    }

    Ok(root.join(relative))
}

fn resolved_runtime_link_target(
    link_path: &str,
    target: &str,
) -> crate::Result<PathBuf> {
    let mut resolved = PathBuf::new();
    if let Some(parent) = Path::new(link_path).parent() {
        resolved.push(parent);
    }

    for component in Path::new(target).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(component) => resolved.push(component),
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err(crate::ErrorKind::InputError(
                        "Java runtime manifest link escapes its install directory"
                            .to_string(),
                    )
                    .into());
                }
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(crate::ErrorKind::InputError(
                    "Java runtime manifest contains an invalid link target"
                        .to_string(),
                )
                .into());
            }
        }
    }

    Ok(resolved)
}

async fn remove_path_if_present(path: &Path) -> crate::Result<()> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };

    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        io::remove_dir_all(path).await?;
    } else {
        io::remove_file(path).await?;
    }
    Ok(())
}

async fn create_runtime_link(
    root: &Path,
    link_path: &str,
    target: &str,
) -> crate::Result<()> {
    let path = safe_runtime_path(root, link_path)?;
    let resolved_target = resolved_runtime_link_target(link_path, target)?;
    if let Some(parent) = path.parent() {
        io::create_dir_all(parent).await?;
    }
    remove_path_if_present(&path).await?;

    #[cfg(unix)]
    {
        let _ = resolved_target;
        let path = path.clone();
        let target = target.to_string();
        tokio::task::spawn_blocking(move || {
            std::os::unix::fs::symlink(target, path)
        })
        .await??;
    }

    #[cfg(windows)]
    {
        tokio::fs::copy(root.join(resolved_target), &path).await?;
    }

    #[cfg(not(any(unix, windows)))]
    {
        tokio::fs::copy(root.join(resolved_target), &path).await?;
    }

    Ok(())
}

async fn set_runtime_executable(path: &Path) -> crate::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = tokio::fs::metadata(path).await?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        tokio::fs::set_permissions(path, permissions).await?;
    }

    Ok(())
}

async fn validate_installed_java(
    path: PathBuf,
    java_version: u32,
    reporter: Option<&InstallProgressReporter>,
    loading_bar: Option<&crate::event::LoadingBarId>,
) -> crate::Result<PathBuf> {
    update_java_install_progress(
        reporter,
        java_version,
        InstallJavaStep::Validating,
        Some(java_step_progress(4)),
    )
    .await?;
    if !test_jre(path.clone(), java_version).await? {
        return Err(crate::ErrorKind::LauncherError(format!(
            "Downloaded Java runtime did not report major version {java_version}"
        ))
        .into());
    }
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 10.0, Some("Done installing java"))?;
    }
    Ok(path)
}

async fn install_mojang_runtime(
    state: &State,
    java_version: u32,
    loading_bar: Option<&crate::event::LoadingBarId>,
    reporter: Option<&InstallProgressReporter>,
) -> crate::Result<Option<PathBuf>> {
    let Some(component) = mojang_runtime_component(java_version) else {
        return Ok(None);
    };
    let Some(platform) = mojang_runtime_platform() else {
        return Ok(None);
    };

    if let Some(reporter) = reporter {
        reporter
            .set_context(
                InstallErrorContext::new("fetch Mojang Java runtime index")
                    .urls(vec![MOJANG_RUNTIME_INDEX_URL.to_string()])
                    .java_version(java_version)
                    .os(std::env::consts::OS)
                    .arch(std::env::consts::ARCH)
                    .build(),
            )
            .await?;
    }
    let index = fetch_json::<MojangRuntimeIndex>(
        Method::GET,
        MOJANG_RUNTIME_INDEX_URL,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    let Some(release) = index
        .get(platform)
        .and_then(|components| components.get(component))
        .and_then(|releases| releases.first())
    else {
        return Ok(None);
    };

    let manifest_path = state
        .directories
        .caches_dir()
        .join("java")
        .join("manifests")
        .join(format!("{}.json", release.manifest.sha1));
    let manifest_integrity = Integrity::sha1(&release.manifest.sha1)
        .with_size(release.manifest.size)
        .with_content_validation(ContentValidation::Json);
    if let Some(reporter) = reporter {
        reporter
            .set_context(
                InstallErrorContext::new("fetch Mojang Java runtime manifest")
                    .urls(vec![release.manifest.url.clone()])
                    .expected_hash(release.manifest.sha1.clone())
                    .expected_size(release.manifest.size)
                    .target_path(manifest_path.display().to_string())
                    .java_version(java_version)
                    .os(std::env::consts::OS)
                    .arch(std::env::consts::ARCH)
                    .build(),
            )
            .await?;
    }
    let download_result = download_to_path(
        DownloadRequest::new(&release.manifest.url, ResourceClass::Java)
            .with_integrity(manifest_integrity),
        &manifest_path,
        &state.download_semaphore,
        &state.pool,
        None,
    )
    .await?;
    if let Some(reporter) = reporter
        && download_result.attempts > 0
    {
        reporter
            .record_download_metrics(
                download_result.source.as_str(),
                download_result.fallback_count as u64,
            )
            .await?;
    }
    let manifest: MojangRuntimeManifest =
        serde_json::from_slice(&io::read(&manifest_path).await?)?;

    let install_name = format!(
        "mojang-{component}-{platform}-{}",
        &release.manifest.sha1[..12.min(release.manifest.sha1.len())]
    );
    let java_root = state.directories.java_versions_dir();
    let final_root = java_root.join(&install_name);
    let staging_root = java_root.join(format!(".{install_name}.installing"));
    let executable_relative = runtime_executable_relative(platform);
    let final_executable = final_root.join(&executable_relative);
    if final_executable.is_file() {
        return Ok(Some(
            validate_installed_java(
                final_executable,
                java_version,
                reporter,
                loading_bar,
            )
            .await?,
        ));
    }

    io::create_dir_all(&staging_root).await?;
    let mut directories = Vec::new();
    let mut files = Vec::new();
    let mut links = Vec::new();
    for (relative_path, entry) in manifest.files {
        match entry {
            MojangRuntimeFile::Directory => directories.push(relative_path),
            MojangRuntimeFile::File {
                downloads,
                executable,
            } => files.push((relative_path, downloads.raw, executable)),
            MojangRuntimeFile::Link { target } => {
                links.push((relative_path, target));
            }
        }
    }
    directories.sort_by_key(|path| Path::new(path).components().count());
    for directory in directories {
        io::create_dir_all(safe_runtime_path(&staging_root, &directory)?)
            .await?;
    }

    let total_bytes = files
        .iter()
        .map(|(_, download, _)| download.size)
        .sum::<u64>();
    update_java_install_progress(
        reporter,
        java_version,
        InstallJavaStep::Downloading,
        Some(InstallProgress {
            current: 0,
            total: total_bytes.max(1),
            secondary: None,
        }),
    )
    .await?;
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 10.0, Some("Downloading java version"))?;
    }

    let downloaded_bytes = Arc::new(AtomicU64::new(0));
    let last_reported = Arc::new(AtomicU64::new(0));
    let download_metrics = JavaDownloadMetrics::default();
    stream::iter(files.into_iter().map(Ok::<_, crate::Error>))
        .try_for_each_concurrent(
            None,
            |(relative_path, download, executable)| {
                let downloaded_bytes = downloaded_bytes.clone();
                let last_reported = last_reported.clone();
                let staging_root = staging_root.clone();
                let reporter = reporter.cloned();
                let download_metrics = download_metrics.clone();
                async move {
                    let path =
                        safe_runtime_path(&staging_root, &relative_path)?;
                    if let Some(reporter) = reporter.as_ref() {
                        reporter
                            .set_transient_context(
                                InstallErrorContext::new(
                                    "download Mojang Java runtime file",
                                )
                                .urls(vec![download.url.clone()])
                                .file_path(relative_path.clone())
                                .target_path(path.display().to_string())
                                .expected_hash(download.sha1.clone())
                                .expected_size(download.size)
                                .java_version(java_version)
                                .os(std::env::consts::OS)
                                .arch(std::env::consts::ARCH)
                                .build(),
                            )
                            .await?;
                    }

                    let mut file_progress = 0_u64;
                    let mut progress = |current: u64,
                                        _total: u64|
                     -> Pin<
                        Box<dyn Future<Output = crate::Result<()>> + Send>,
                    > {
                        let delta = current.saturating_sub(file_progress);
                        file_progress = current;
                        let current = downloaded_bytes
                            .fetch_add(delta, Ordering::Relaxed)
                            .saturating_add(delta)
                            .min(total_bytes);
                        let previous = last_reported.load(Ordering::Relaxed);
                        let min_delta = (total_bytes / 200)
                            .max(JAVA_DOWNLOAD_PROGRESS_MIN_BYTES);
                        if current < total_bytes
                            && current.saturating_sub(previous) < min_delta
                        {
                            return Box::pin(async { Ok(()) });
                        }
                        last_reported.store(current, Ordering::Relaxed);
                        let reporter = reporter.clone();
                        Box::pin(async move {
                            update_java_install_progress(
                                reporter.as_ref(),
                                java_version,
                                InstallJavaStep::Downloading,
                                Some(InstallProgress {
                                    current,
                                    total: total_bytes.max(1),
                                    secondary: None,
                                }),
                            )
                            .await
                        })
                    };
                    let result = download_to_path(
                        DownloadRequest::new(
                            &download.url,
                            ResourceClass::Java,
                        )
                        .with_integrity(
                            Integrity::sha1(&download.sha1)
                                .with_size(download.size),
                        ),
                        &path,
                        &state.download_semaphore,
                        &state.pool,
                        Some(&mut progress as &mut FetchProgressFn<'_>),
                    )
                    .await?;
                    download_metrics.record(&result);
                    if executable {
                        set_runtime_executable(&path).await?;
                    }
                    Ok(())
                }
            },
        )
        .await?;
    download_metrics.finish(reporter).await?;

    update_java_install_progress(
        reporter,
        java_version,
        InstallJavaStep::Extracting,
        Some(java_step_progress(3)),
    )
    .await?;
    for (link_path, target) in links {
        create_runtime_link(&staging_root, &link_path, &target).await?;
    }
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 80.0, Some("Installing java runtime"))?;
    }

    if final_root.exists() {
        io::remove_dir_all(&final_root).await?;
    }
    tokio::fs::rename(&staging_root, &final_root).await?;
    let executable = final_root.join(executable_relative);
    Ok(Some(
        validate_installed_java(
            executable,
            java_version,
            reporter,
            loading_bar,
        )
        .await?,
    ))
}

fn validate_archive_file_name(name: &Path) -> crate::Result<()> {
    if name.as_os_str().is_empty()
        || name.is_absolute()
        || name.components().count() != 1
        || !matches!(name.components().next(), Some(Component::Normal(_)))
    {
        return Err(crate::ErrorKind::InputError(
            "Java package metadata contains an invalid file name".to_string(),
        )
        .into());
    }
    Ok(())
}

fn extract_azul_archive(
    archive_path: &Path,
    staging_root: &Path,
) -> crate::Result<PathBuf> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| {
        crate::ErrorKind::InputError(format!(
            "Failed to read Java archive: {error}"
        ))
    })?;
    let root = archive
        .file_names()
        .find_map(|name| {
            name.split('/').find(|component| !component.is_empty())
        })
        .map(PathBuf::from)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "Java archive does not contain an install directory"
                    .to_string(),
            )
        })?;
    validate_archive_file_name(&root)?;
    archive.extract(staging_root).map_err(|error| {
        crate::ErrorKind::InputError(format!(
            "Failed to extract Java archive: {error}"
        ))
    })?;
    Ok(root)
}

async fn install_azul_runtime(
    state: &State,
    java_version: u32,
    loading_bar: Option<&crate::event::LoadingBarId>,
    reporter: Option<&InstallProgressReporter>,
) -> crate::Result<PathBuf> {
    let metadata_url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages?arch={}&java_version={}&os={}&archive_type=zip&javafx_bundled=false&java_package_type=jre&page_size=1",
        std::env::consts::ARCH,
        java_version,
        std::env::consts::OS
    );
    if let Some(reporter) = reporter {
        reporter
            .set_context(
                InstallErrorContext::new("fetch Azul Java package metadata")
                    .urls(vec![metadata_url.clone()])
                    .java_version(java_version)
                    .os(std::env::consts::OS)
                    .arch(std::env::consts::ARCH)
                    .build(),
            )
            .await?;
    }
    let packages = fetch_json::<Vec<AzulPackageSummary>>(
        Method::GET,
        &metadata_url,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    let summary = packages.first().ok_or_else(|| {
        crate::ErrorKind::LauncherError(format!(
            "No Java Version found for Java version {}, OS {}, and Architecture {}",
            java_version,
            std::env::consts::OS,
            std::env::consts::ARCH,
        ))
    })?;
    let details_url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages/{}",
        summary.package_uuid
    );
    let download = fetch_json::<AzulPackage>(
        Method::GET,
        &details_url,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    validate_archive_file_name(&download.name)?;

    let archive_path = state
        .directories
        .caches_dir()
        .join("java")
        .join("azul")
        .join(&summary.package_uuid)
        .join(&download.name);
    if let Some(reporter) = reporter {
        reporter
            .set_context(
                InstallErrorContext::new("download Azul Java archive")
                    .urls(vec![download.download_url.clone()])
                    .file_path(download.name.display().to_string())
                    .target_path(archive_path.display().to_string())
                    .expected_hash(download.sha256_hash.clone())
                    .expected_size(download.size)
                    .java_version(java_version)
                    .os(std::env::consts::OS)
                    .arch(std::env::consts::ARCH)
                    .build(),
            )
            .await?;
    }
    update_java_install_progress(
        reporter,
        java_version,
        InstallJavaStep::Downloading,
        Some(InstallProgress {
            current: 0,
            total: download.size,
            secondary: None,
        }),
    )
    .await?;
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 10.0, Some("Downloading java version"))?;
    }
    let mut last_reported_bytes = 0_u64;
    let download_reporter = reporter.cloned();
    let mut progress =
        |current: u64,
         total: u64|
         -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>> {
            let min_delta = (total / 200).max(JAVA_DOWNLOAD_PROGRESS_MIN_BYTES);
            if current < total
                && current.saturating_sub(last_reported_bytes) < min_delta
            {
                return Box::pin(async { Ok(()) });
            }
            last_reported_bytes = current;
            let reporter = download_reporter.clone();
            Box::pin(async move {
                update_java_install_progress(
                    reporter.as_ref(),
                    java_version,
                    InstallJavaStep::Downloading,
                    Some(InstallProgress {
                        current,
                        total,
                        secondary: None,
                    }),
                )
                .await
            })
        };
    let download_result = download_to_path(
        DownloadRequest::new(&download.download_url, ResourceClass::Java)
            .with_integrity(Integrity {
                size: Some(download.size),
                sha256: Some(download.sha256_hash.clone()),
                content: ContentValidation::Jar,
                ..Integrity::default()
            }),
        &archive_path,
        &state.download_semaphore,
        &state.pool,
        Some(&mut progress as &mut FetchProgressFn<'_>),
    )
    .await?;
    if let Some(reporter) = reporter
        && download_result.attempts > 0
    {
        reporter
            .record_download_metrics(
                download_result.source.as_str(),
                download_result.fallback_count as u64,
            )
            .await?;
    }
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 80.0, Some("Extracting java"))?;
    }
    update_java_install_progress(
        reporter,
        java_version,
        InstallJavaStep::Extracting,
        Some(java_step_progress(3)),
    )
    .await?;

    let java_root = state.directories.java_versions_dir();
    let staging_root =
        java_root.join(format!(".azul-{}.installing", summary.package_uuid));
    remove_path_if_present(&staging_root).await?;
    io::create_dir_all(&staging_root).await?;
    let archive_path_for_extract = archive_path.clone();
    let staging_for_extract = staging_root.clone();
    let extracted_root = tokio::task::spawn_blocking(move || {
        extract_azul_archive(&archive_path_for_extract, &staging_for_extract)
    })
    .await??;
    let extracted_path = staging_root.join(&extracted_root);
    let final_root = java_root.join(&extracted_root);
    remove_path_if_present(&final_root).await?;
    tokio::fs::rename(&extracted_path, &final_root).await?;
    remove_path_if_present(&staging_root).await?;

    let executable = if cfg!(target_os = "macos") {
        final_root.join("Contents/Home/bin/java")
    } else {
        final_root.join("bin").join(jre::JAVA_BIN)
    };
    validate_installed_java(executable, java_version, reporter, loading_bar)
        .await
}

// Validates JRE at a given at a given path
pub async fn check_jre(path: PathBuf) -> crate::Result<JavaVersion> {
    jre::check_java_at_filepath(&path).await
}

// Test JRE at a given path
pub async fn test_jre(
    path: PathBuf,
    major_version: u32,
) -> crate::Result<bool> {
    let jre = match jre::check_java_at_filepath(&path).await {
        Ok(jre) => jre,
        Err(e) => {
            tracing::warn!("Invalid Java at {}: {e}", path.display());
            return Ok(false);
        }
    };
    let version = extract_java_version(&jre.version)?;
    tracing::info!(
        "Expected Java version {major_version}, and found {version} at {}",
        path.display()
    );
    Ok(version == major_version)
}

fn system_memory_bytes() -> u64 {
    sysinfo::System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    )
    .total_memory()
}

/// Recommended default max heap (MiB) for new instances based on system RAM.
pub fn default_memory_max_mb() -> u32 {
    const BYTES_PER_GIB: u64 = 1024 * 1024 * 1024;
    let system_gib = system_memory_bytes() / BYTES_PER_GIB;

    if system_gib < 8 {
        1024 * 2
    } else if system_gib >= 24 {
        1024 * 6
    } else {
        1024 * 4
    }
}

// Gets maximum memory in KiB.
pub async fn get_max_memory() -> crate::Result<u64> {
    Ok(system_memory_bytes() / 1024)
}
