use crate::State;
use crate::api::content_search::{
    chinese_file_title_for_modrinth_slug, localized_content_relative_path,
    original_content_relative_path,
};
use crate::event::emit::loading_try_for_each_concurrent;
use crate::install::model::{
    InstallPauseReason, MissingModpackContentState, MissingModpackFileState,
};
use crate::install::{
    InstallErrorContext, InstallJobEventKind, InstallPhaseDetails,
    InstallPhaseId, InstallProgress, InstallProgressReporter,
    InstallProgressSecondary,
};
use crate::pack::install_from::{
    EnvType, PackFile, PackFileHash, set_instance_information,
};
use crate::state::instances::ContentSourceKind;
use crate::state::instances::adapters::sqlite::content_rows;
use crate::state::{
    CachedEntry, ContentProviderRef, EditInstance, InstanceInstallStage,
    KnownModrinthFile, ModrinthHashMatch, ModrinthProjectId, ModrinthVersionId,
    Settings, SideType,
};
use crate::util::fetch::{
    ContentValidation, DownloadMeta, DownloadReason, DownloadRequest,
    FetchProgressFn, Integrity, ResourceClass, download_to_path,
    sha1_file_async,
};
use crate::util::io;
use async_zip::base::read::seek::ZipFileReader as SeekZipFileReader;
use async_zip::base::read::{WithEntry, ZipEntryReader};
use async_zip::tokio::read::fs::ZipFileReader as FsZipFileReader;
use futures::StreamExt;
use path_util::SafeRelativeUtf8UnixPathBuf;
use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use super::install_from::{CreatePack, CreatePackFile, PackFormat};
use crate::data::ProjectType;
use std::io::{Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

type ExtractProgressFn<'a> = dyn FnMut(u64) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + 'a>>
    + Send
    + 'a;

const AGGREGATE_FAILURE_PATH_LIMIT: usize = 3;
const AGGREGATE_FAILURE_PATH_CHAR_LIMIT: usize = 160;
const ITEM_FAILURE_REASON_CHAR_LIMIT: usize = 1_024;
/// Batch-level automatic retry passes for failed modpack files, in addition
/// to the per-file retries inside `download_to_path`. Failed files are retried
/// over a single connection (no range segmentation) and only after every pass
/// is exhausted does the install ask the user about missing content.
const AUTO_RETRY_PASSES: usize = 2;

pub(crate) enum MrpackInstallOutcome {
    #[allow(dead_code)]
    Completed(String),
    WaitingForUser(InstallPauseReason),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RequiredFileFailure {
    manifest_index: usize,
    path: String,
    reason: String,
}

impl RequiredFileFailure {
    fn new(manifest_index: usize, path: &str, reason: &str) -> Self {
        Self {
            manifest_index,
            path: bounded_text(path, AGGREGATE_FAILURE_PATH_CHAR_LIMIT),
            reason: bounded_text(reason, ITEM_FAILURE_REASON_CHAR_LIMIT),
        }
    }
}

#[cfg(test)]
struct RequiredFileFailures<'a>(&'a [RequiredFileFailure]);

#[cfg(test)]
impl fmt::Display for RequiredFileFailures<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let failures = self.0;
        write!(
            formatter,
            "{} required modpack file{} failed",
            failures.len(),
            if failures.len() == 1 { "" } else { "s" }
        )?;
        let shown = failures
            .iter()
            .take(AGGREGATE_FAILURE_PATH_LIMIT)
            .map(|failure| failure.path.as_str())
            .collect::<Vec<_>>();
        if !shown.is_empty() {
            write!(formatter, ": {}", shown.join(", "))?;
        }
        if failures.len() > shown.len() {
            write!(formatter, " (+{} more)", failures.len() - shown.len())?;
        }
        Ok(())
    }
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let bounded = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{bounded}...")
    } else {
        bounded
    }
}

fn estimated_file_max_attempts(file: &PackFile) -> usize {
    file.downloads.len().saturating_mul(2).max(1)
}

async fn collect_required_file_failures_concurrently<T, F, Fut>(
    items: Vec<T>,
    limit: Option<usize>,
    mut task: F,
) -> crate::Result<Vec<RequiredFileFailure>>
where
    T: Send,
    F: FnMut(T) -> Fut + Send,
    Fut: Future<Output = Result<(), RequiredFileFailure>> + Send,
{
    let failures = Arc::new(Mutex::new(Vec::<RequiredFileFailure>::new()));
    let collected_failures = failures.clone();
    let num_items = items.len();
    loading_try_for_each_concurrent(
        futures::stream::iter(items).map(Ok::<T, crate::Error>),
        limit,
        None,
        70.0,
        num_items,
        None,
        move |item| {
            let future = task(item);
            let failures = failures.clone();
            async move {
                if let Err(failure) = future.await {
                    failures.lock().await.push(failure);
                }
                Ok(())
            }
        },
    )
    .await?;
    let mut failures = collected_failures.lock().await.clone();
    failures.sort_unstable_by_key(|failure| failure.manifest_index);
    Ok(failures)
}

fn missing_required_content_pause(
    failures: &[RequiredFileFailure],
) -> Option<InstallPauseReason> {
    (!failures.is_empty()).then(|| InstallPauseReason::MissingRequiredContent {
        failed_files: failures.len() as u64,
        paths: failures
            .iter()
            .take(AGGREGATE_FAILURE_PATH_LIMIT)
            .map(|failure| failure.path.clone())
            .collect(),
    })
}

#[derive(Clone)]
struct ModpackContentInstallContext {
    instance_id: String,
    instance_full_path: PathBuf,
    download_meta: DownloadMeta,
    pack_version_id: Option<String>,
    pack_project_id: Option<String>,
    reporter: InstallProgressReporter,
    modpack_details: InstallPhaseDetails,
    content_progress: Arc<AtomicU64>,
    content_bytes_progress: Arc<AtomicU64>,
    active_download_bytes: Arc<Mutex<HashMap<String, u64>>>,
    active_download_total: Arc<AtomicU64>,
    download_source: Arc<Mutex<Option<String>>>,
    fallback_count: Arc<AtomicU64>,
    file_infos_by_hash: Arc<Mutex<Option<HashMap<String, ModrinthHashMatch>>>>,
    file_infos_loading: Arc<Mutex<()>>,
    file_info_hashes: Arc<Vec<String>>,
    chinese_titles_by_sha1: Arc<HashMap<String, String>>,
    existing_paths_by_original: Arc<HashMap<String, String>>,
    num_files: usize,
    content_total_bytes: u64,
}

/// Resolves Modrinth file metadata by SHA-1 hash. All completions share a
/// single lookup (double-checked locking through `file_infos_loading`);
/// failures are cached as an empty map so a broken lookup is not retried once
/// per remaining file.
async fn load_modpack_file_infos(
    file_infos_by_hash: &Arc<Mutex<Option<HashMap<String, ModrinthHashMatch>>>>,
    file_infos_loading: &Arc<Mutex<()>>,
    file_info_hashes: &Arc<Vec<String>>,
    pool: &sqlx::SqlitePool,
    api_semaphore: &crate::util::fetch::FetchSemaphore,
) {
    {
        let guard = file_infos_by_hash.lock().await;
        if guard.is_some() {
            return;
        }
    }
    let _loading = file_infos_loading.lock().await;
    {
        let guard = file_infos_by_hash.lock().await;
        if guard.is_some() {
            return;
        }
    }
    let hash_refs = file_info_hashes
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let loaded =
        CachedEntry::get_file_many(&hash_refs, None, pool, api_semaphore)
            .await
            .map(|files| {
                files
                    .into_iter()
                    .map(|file| (file.hash.clone(), file))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
    *file_infos_by_hash.lock().await = Some(loaded);
}

/// Maps manifest file hashes to sanitized Chinese titles by resolving the
/// files' Modrinth projects from cache. Failures only disable the naming.
async fn resolve_chinese_titles_by_sha1(
    file_infos_by_hash: &HashMap<String, ModrinthHashMatch>,
    state: &State,
) -> HashMap<String, String> {
    let project_ids = file_infos_by_hash
        .values()
        .filter_map(|file| ModrinthProjectId::new(file.project_id.clone()).ok())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if project_ids.is_empty() {
        return HashMap::new();
    }
    let projects = match CachedEntry::get_project_many(
        &project_ids,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await
    {
        Ok(projects) => projects,
        Err(error) => {
            tracing::warn!(
                "Failed to resolve project metadata for Chinese content file names: {error}"
            );
            return HashMap::new();
        }
    };
    let titles_by_project_id = projects
        .into_iter()
        .filter_map(|project| {
            let slug = project.slug?;
            let title = chinese_file_title_for_modrinth_slug(&slug)?;
            Some((project.id, title))
        })
        .collect::<HashMap<_, _>>();
    file_infos_by_hash
        .iter()
        .filter_map(|(sha1, file)| {
            titles_by_project_id
                .get(&file.project_id)
                .map(|title| (sha1.clone(), title.clone()))
        })
        .collect()
}

impl ModpackContentInstallContext {
    /// Picks the instance-relative install path for a pack file. Paths already
    /// recorded for this instance win so repairs and locale switches never
    /// produce duplicate files; new non-mod content may get a `[中文名]` prefix
    /// when Chinese file naming is active. Mod files always keep manifest names.
    fn resolve_install_path(&self, file: &PackFile) -> String {
        let manifest_path = file.path.as_str();
        if let Some(existing) =
            self.existing_paths_by_original.get(manifest_path)
        {
            return existing.clone();
        }
        let Some(project_type) =
            ProjectType::get_from_parent_folder(manifest_path)
        else {
            return manifest_path.to_string();
        };

        if project_type == ProjectType::Mod {
            return manifest_path.to_string();
        }
        let Some(title) = file
            .hashes
            .get(&PackFileHash::Sha1)
            .and_then(|sha1| self.chinese_titles_by_sha1.get(sha1))
        else {
            return manifest_path.to_string();
        };
        localized_content_relative_path(manifest_path, title)
            .unwrap_or_else(|| manifest_path.to_string())
    }

    async fn report_download_attempt(
        &self,
        path: String,
        file_size: u64,
        attempt: usize,
        max_attempts: usize,
    ) -> crate::Result<()> {
        let active_bytes = self.update_active_download(path.clone(), 0).await;
        let current_bytes = self
            .content_bytes_progress
            .load(Ordering::Relaxed)
            .saturating_add(active_bytes)
            .min(self.content_total_bytes);
        self.reporter
            .update_with_events(
                InstallPhaseId::DownloadingContent,
                Some(InstallProgress {
                    current: self.content_progress.load(Ordering::Relaxed),
                    total: self.num_files as u64,
                    secondary: (self.content_total_bytes > 0).then_some(
                        InstallProgressSecondary {
                            current: current_bytes,
                            total: self.content_total_bytes,
                        },
                    ),
                }),
                self.modpack_details.clone(),
                vec![InstallJobEventKind::ContentFileDownloadAttempt {
                    path,
                    bytes_total: (file_size > 0).then_some(file_size),
                    attempt: attempt as u32,
                    max_attempts: max_attempts as u32,
                }],
            )
            .await
    }

    async fn mark_file_settled(
        &self,
        settled_bytes: u64,
        event: InstallJobEventKind,
    ) -> crate::Result<()> {
        let current = self.content_progress.fetch_add(1, Ordering::Relaxed) + 1;
        let current_bytes = self
            .content_bytes_progress
            .fetch_add(settled_bytes, Ordering::Relaxed)
            + settled_bytes;

        self.reporter
            .update_with_events(
                InstallPhaseId::DownloadingContent,
                Some(InstallProgress {
                    current,
                    total: self.num_files as u64,
                    secondary: (self.content_total_bytes > 0).then_some(
                        InstallProgressSecondary {
                            current: current_bytes
                                .min(self.content_total_bytes),
                            total: self.content_total_bytes,
                        },
                    ),
                }),
                self.modpack_details.clone(),
                vec![event],
            )
            .await
    }

    async fn remove_active_download(&self, path: &str) {
        let mut active_download_bytes = self.active_download_bytes.lock().await;
        if let Some(previous) = active_download_bytes.remove(path) {
            self.active_download_total
                .fetch_sub(previous, Ordering::Relaxed);
        }
    }

    async fn update_active_download(
        &self,
        path: String,
        downloaded: u64,
    ) -> u64 {
        let mut active_download_bytes = self.active_download_bytes.lock().await;
        let previous =
            active_download_bytes.insert(path, downloaded).unwrap_or(0);
        self.active_download_total
            .fetch_add(downloaded.saturating_sub(previous), Ordering::Relaxed);
        self.active_download_total.load(Ordering::Relaxed)
    }

    async fn record_download_result(
        &self,
        result: &crate::util::fetch::DownloadResult,
    ) {
        if result.attempts > 0 {
            *self.download_source.lock().await =
                Some(result.source.as_str().to_string());
        }
        self.fallback_count
            .fetch_add(result.fallback_count as u64, Ordering::Relaxed);
    }

    async fn finish_download_metrics(&self) -> crate::Result<()> {
        let source = self.download_source.lock().await.clone();
        if let Some(source) = source {
            self.reporter
                .record_download_metrics(
                    source,
                    self.fallback_count.load(Ordering::Relaxed),
                )
                .await?;
        }
        Ok(())
    }
}

enum MrpackZipReader {
    Memory(async_zip::tokio::read::seek::ZipFileReader<Cursor<bytes::Bytes>>),
    // Local imports stay on disk so large .mrpacks do not have to fit in memory.
    File(FsZipFileReader),
}

impl MrpackZipReader {
    async fn new(file: &CreatePackFile) -> crate::Result<Self> {
        match file {
            CreatePackFile::Bytes(file) => Ok(Self::Memory(
                SeekZipFileReader::with_tokio(Cursor::new(file.clone()))
                    .await
                    .map_err(|_| {
                        crate::Error::from(crate::ErrorKind::InputError(
                            "Failed to read input modpack zip".to_string(),
                        ))
                    })?,
            )),
            CreatePackFile::Path(path) => Ok(Self::File(
                FsZipFileReader::new(path).await.map_err(|_| {
                    crate::Error::from(crate::ErrorKind::InputError(
                        "Failed to read input modpack zip".to_string(),
                    ))
                })?,
            )),
        }
    }

    fn file(&self) -> &async_zip::ZipFile {
        match self {
            Self::Memory(reader) => reader.file(),
            Self::File(reader) => reader.file(),
        }
    }

    async fn read_entry_to_string(
        &mut self,
        index: usize,
    ) -> crate::Result<String> {
        let mut value = String::new();
        match self {
            Self::Memory(reader) => {
                let mut reader = reader.reader_with_entry(index).await?;
                reader.read_to_string_checked(&mut value).await?;
            }
            Self::File(reader) => {
                let mut reader = reader.reader_with_entry(index).await?;
                reader.read_to_string_checked(&mut value).await?;
            }
        }

        Ok(value)
    }

    async fn hash_entry(
        &mut self,
        index: usize,
    ) -> crate::Result<(u64, String)> {
        match self {
            Self::Memory(reader) => {
                hash_zip_entry(reader.reader_with_entry(index).await?).await
            }
            Self::File(reader) => {
                hash_zip_entry(reader.reader_with_entry(index).await?).await
            }
        }
    }

    async fn extract_entry(
        &mut self,
        index: usize,
        path: &Path,
        semaphore: &crate::util::fetch::IoSemaphore,
        progress: Option<&mut ExtractProgressFn<'_>>,
    ) -> crate::Result<(u64, String)> {
        match self {
            Self::Memory(reader) => {
                extract_zip_entry(
                    reader.reader_with_entry(index).await?,
                    path,
                    semaphore,
                    progress,
                )
                .await
            }
            Self::File(reader) => {
                extract_zip_entry(
                    reader.reader_with_entry(index).await?,
                    path,
                    semaphore,
                    progress,
                )
                .await
            }
        }
    }
}

async fn hash_zip_entry<R>(
    mut reader: ZipEntryReader<'_, R, WithEntry<'_>>,
) -> crate::Result<(u64, String)>
where
    R: futures_lite::io::AsyncBufRead + Unpin,
{
    let expected_crc32 = reader.entry().crc32();
    let mut hasher = sha1_smol::Sha1::new();
    let mut size = 0;
    let mut buffer = vec![0; 262144];

    loop {
        let bytes_read =
            futures_lite::io::AsyncReadExt::read(&mut reader, &mut buffer)
                .await?;
        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
        size += bytes_read as u64;
    }

    if reader.compute_hash() != expected_crc32 {
        return Err(async_zip::error::ZipError::CRC32CheckError.into());
    }

    Ok((size, hasher.digest().to_string()))
}

async fn extract_zip_entry<R>(
    mut reader: ZipEntryReader<'_, R, WithEntry<'_>>,
    path: &Path,
    semaphore: &crate::util::fetch::IoSemaphore,
    mut progress: Option<&mut ExtractProgressFn<'_>>,
) -> crate::Result<(u64, String)>
where
    R: futures_lite::io::AsyncBufRead + Unpin,
{
    let _permit = semaphore.0.acquire().await?;

    if let Some(parent) = path.parent() {
        io::create_dir_all(parent).await?;
    }

    let parent = path.parent().ok_or_else(|| {
        io::IOError::from(std::io::Error::other(
            "could not get parent directory for temporary file",
        ))
    })?;
    let temp_path = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| io::IOError::with_path(e, parent))?
        .into_temp_path();

    // Only replace the profile file after the ZIP entry has passed its CRC check.
    let expected_crc32 = reader.entry().crc32();
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| io::IOError::with_path(e, &temp_path))?;
    let mut hasher = sha1_smol::Sha1::new();
    let mut size = 0;
    let mut buffer = vec![0; 262144];

    loop {
        let bytes_read =
            futures_lite::io::AsyncReadExt::read(&mut reader, &mut buffer)
                .await?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .await
            .map_err(|e| io::IOError::with_path(e, &temp_path))?;
        hasher.update(&buffer[..bytes_read]);
        size += bytes_read as u64;
        if let Some(progress) = progress.as_mut() {
            progress(bytes_read as u64).await?;
        }
    }

    file.flush()
        .await
        .map_err(|e| io::IOError::with_path(e, &temp_path))?;
    drop(file);

    if reader.compute_hash() != expected_crc32 {
        return Err(async_zip::error::ZipError::CRC32CheckError.into());
    }

    temp_path.persist(path).map_err(|e| {
        let tempfile::PathPersistError { error, .. } = e;
        io::IOError::with_path(error, path)
    })?;

    Ok((size, hasher.digest().to_string()))
}

pub(crate) async fn install_zipped_mrpack_files_with_reporter(
    create_pack: CreatePack,
    ignore_lock: bool,
    reason: DownloadReason,
    reporter: InstallProgressReporter,
) -> crate::Result<MrpackInstallOutcome> {
    let state = &State::get().await?;

    let file = create_pack.file;
    let description = create_pack.description.clone();
    let icon = create_pack.description.icon;
    let project_id = create_pack.description.project_id;
    let version_id = create_pack.description.version_id;
    let instance_id = create_pack.description.instance_id;
    let mut icon_exists = icon.is_some();
    let source_path = pack_source_path(&file);

    reporter
        .set_context(
            InstallErrorContext::new("read modpack archive")
                .maybe_project_id(project_id.clone())
                .maybe_version_id(version_id.clone())
                .source_path(source_path.clone())
                .build(),
        )
        .await?;
    let mut zip_reader = MrpackZipReader::new(&file).await?;
    let instance_full_path =
        crate::api::instance::get_full_path(&instance_id).await?;
    let modpack_details = InstallPhaseDetails::Modpack {
        project_id: project_id.clone(),
        version_id: version_id.clone(),
        title: description.override_title.clone(),
    };
    reporter
        .update(
            InstallPhaseId::ReadingPackManifest,
            None,
            modpack_details.clone(),
        )
        .await?;
    reporter
        .set_context(
            InstallErrorContext::new("read modpack manifest")
                .maybe_project_id(project_id.clone())
                .maybe_version_id(version_id.clone())
                .source_path(source_path.clone())
                .entry_path("modrinth.index.json")
                .build(),
        )
        .await?;

    // Extract index of modrinth.index.json, tolerating packs zipped inside a
    // single wrapping folder. A root-level manifest always wins so override
    // files that happen to share the name cannot hijack the base folder.
    let Some((manifest_idx, archive_base)) =
        locate_mrpack_manifest(zip_reader.file())
    else {
        return Err(crate::Error::from(crate::ErrorKind::InputError(
            "No pack manifest found in mrpack".to_string(),
        )));
    };
    let overrides_prefix = format!("{archive_base}overrides/");
    let client_overrides_prefix = format!("{archive_base}client-overrides/");
    let server_overrides_prefix = format!("{archive_base}server-overrides/");
    let mut manifest = String::new();
    manifest.push_str(&zip_reader.read_entry_to_string(manifest_idx).await?);

    let pack: PackFormat = serde_json::from_str(&manifest)?;
    if &*pack.game != "minecraft" {
        return Err(crate::ErrorKind::InputError(
            "Pack does not support Minecraft".to_string(),
        )
        .into());
    }

    reporter
        .update(InstallPhaseId::ResolvingPack, None, modpack_details.clone())
        .await?;

    if !icon_exists {
        let icon_entry =
            zip_reader.file().entries().iter().enumerate().find_map(
                |(index, entry)| {
                    let Ok(name) = entry.filename().as_str() else {
                        return None;
                    };
                    (name == format!("{archive_base}icon.png")
                        || name == format!("{overrides_prefix}icon.png")
                        || name == format!("{client_overrides_prefix}icon.png"))
                    .then_some(index)
                },
            );

        if let Some(icon_entry) = icon_entry {
            let icon_path = instance_full_path.join("icon.png");
            zip_reader
                .extract_entry(
                    icon_entry,
                    &icon_path,
                    &state.io_semaphore,
                    None,
                )
                .await?;
            crate::api::instance::edit_icon(&instance_id, Some(&icon_path))
                .await?;
            icon_exists = true;
        }
    }

    // Cache the modpack file hashes for later filtering of user-added content
    // Includes both manifest file hashes and computed hashes for override files
    if let Some(ref version_id) = version_id {
        let mut file_hashes: Vec<String> = pack
            .files
            .iter()
            .filter_map(|f| f.hashes.get(&PackFileHash::Sha1).cloned())
            .collect();

        // Also hash files from overrides folders (these aren't in modrinth.index.json)
        let override_entries: Vec<usize> = zip_reader
            .file()
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let filename = entry.filename().as_str().ok()?;
                let is_override = (filename.starts_with(&overrides_prefix)
                    || filename.starts_with(&client_overrides_prefix)
                    || filename.starts_with(&server_overrides_prefix))
                    && !filename.ends_with('/');
                is_override.then_some(index)
            })
            .collect();

        for index in override_entries {
            let (_, hash) = zip_reader.hash_entry(index).await?;
            file_hashes.push(hash);
        }

        let project_ids: Vec<String> = pack
            .files
            .iter()
            .filter_map(|f| {
                f.downloads.iter().find_map(|url| {
                    let parts: Vec<&str> = url.split('/').collect();
                    let data_idx = parts.iter().position(|&p| p == "data")?;
                    parts.get(data_idx + 1).map(|s| s.to_string())
                })
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        tracing::info!(
            "Caching {} modpack file hashes and {} project IDs for version {}",
            file_hashes.len(),
            project_ids.len(),
            version_id
        );
        CachedEntry::cache_modpack_files(
            version_id,
            file_hashes,
            project_ids,
            &state.pool,
        )
        .await?;
    } else {
        tracing::warn!(
            "No version_id available, skipping modpack file hash caching"
        );
    }

    set_instance_information(
        instance_id.clone(),
        &description,
        &pack.name,
        Some(&pack.version_id),
        &pack.dependencies,
        ignore_lock,
    )
    .await?;

    let metadata =
        crate::api::instance::get(&instance_id)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Unknown instance {instance_id}"
                ))
            })?;
    let download_meta = DownloadMeta {
        reason,
        game_version: metadata.applied_content_set.game_version.clone(),
        loader: metadata.applied_content_set.loader.as_str().to_string(),
        dependent_on: version_id.clone(),
    };

    let num_files = pack.files.len();
    let content_total_bytes = pack
        .files
        .iter()
        .map(|file| file.file_size as u64)
        .sum::<u64>();
    let content_progress = Arc::new(AtomicU64::new(0));
    let content_bytes_progress = Arc::new(AtomicU64::new(0));
    let active_download_bytes =
        Arc::new(Mutex::new(HashMap::<String, u64>::new()));
    let active_download_total = Arc::new(AtomicU64::new(0));
    let file_info_hashes = Arc::new(
        pack.files
            .iter()
            .filter_map(|file| file.hashes.get(&PackFileHash::Sha1).cloned())
            .collect::<Vec<_>>(),
    );
    let file_infos_by_hash =
        Arc::new(Mutex::new(None::<HashMap<String, ModrinthHashMatch>>));
    let file_infos_loading = Arc::new(Mutex::new(()));
    let chinese_naming_enabled =
        Settings::get(&state.pool).await?.locale == "zh-CN";
    let chinese_titles_by_sha1 = Arc::new(if chinese_naming_enabled {
        // Chinese titles shape the on-disk file names, so they must be known
        // before any download starts. All other installs resolve them lazily
        // on the first completed file.
        load_modpack_file_infos(
            &file_infos_by_hash,
            &file_infos_loading,
            &file_info_hashes,
            &state.pool,
            &state.api_semaphore,
        )
        .await;
        let infos = file_infos_by_hash.lock().await;
        let infos = infos.as_ref().expect("file metadata was just loaded");
        resolve_chinese_titles_by_sha1(infos, state).await
    } else {
        HashMap::new()
    });
    let existing_paths_by_original = Arc::new(
        content_rows::get_instance_files(&instance_id, &state.pool)
            .await?
            .into_iter()
            .map(|file| {
                (
                    original_content_relative_path(&file.relative_path),
                    file.relative_path,
                )
            })
            .collect::<HashMap<_, _>>(),
    );
    let content_context = ModpackContentInstallContext {
        instance_id: instance_id.clone(),
        instance_full_path: instance_full_path.clone(),
        download_meta,
        pack_version_id: version_id.clone(),
        pack_project_id: project_id.clone(),
        reporter: reporter.clone(),
        modpack_details: modpack_details.clone(),
        content_progress,
        content_bytes_progress,
        active_download_bytes,
        active_download_total,
        download_source: Arc::new(Mutex::new(None)),
        fallback_count: Arc::new(AtomicU64::new(0)),
        file_infos_by_hash,
        file_infos_loading,
        file_info_hashes,
        chinese_titles_by_sha1,
        existing_paths_by_original,
        num_files,
        content_total_bytes,
    };
    let skipped_missing_content_paths = Arc::new(
        reporter
            .current_state()
            .await?
            .skipped_missing_content_paths
            .into_iter()
            .collect::<HashSet<_>>(),
    );
    let mut queued_events = Vec::with_capacity(num_files.saturating_add(1));
    queued_events.push(InstallJobEventKind::ContentDownloadStarted {
        files: num_files as u64,
        bytes: (content_total_bytes > 0).then_some(content_total_bytes),
    });
    queued_events.extend(pack.files.iter().map(|file| {
        InstallJobEventKind::ContentFileQueued {
            path: content_context.resolve_install_path(file),
            bytes_total: (file.file_size > 0).then_some(file.file_size as u64),
            max_attempts: estimated_file_max_attempts(file) as u32,
        }
    }));
    queued_events.extend(pack.files.iter().filter_map(|file| {
        let urls = crate::install::missing_content::browser_download_urls(
            &file.downloads,
        );
        (!urls.is_empty()).then(|| {
            InstallJobEventKind::ContentFileBrowserOptions {
                path: content_context.resolve_install_path(file),
                urls,
            }
        })
    }));
    reporter
        .update_with_events(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: 0,
                total: num_files as u64,
                secondary: (content_total_bytes > 0).then_some(
                    InstallProgressSecondary {
                        current: 0,
                        total: content_total_bytes,
                    },
                ),
            }),
            modpack_details.clone(),
            queued_events,
        )
        .await?;
    reporter
        .set_missing_content(Some(MissingModpackContentState {
            files: pack
                .files
                .iter()
                .map(|file| {
                    let target_path =
                        content_context.resolve_install_path(file);
                    MissingModpackFileState {
                        item_id: target_path.clone(),
                        manifest_path: file.path.as_str().to_string(),
                        target_path,
                        expected_size: file.file_size as u64,
                        sha1: file.hashes.get(&PackFileHash::Sha1).cloned(),
                        sha512: file
                            .hashes
                            .get(&PackFileHash::Sha512)
                            .cloned(),
                        download_urls: file.downloads.clone(),
                        browser_urls: crate::install::missing_content::browser_download_urls(
                            &file.downloads,
                        ),
                        validate_as_jar: file
                            .path
                            .as_str()
                            .to_ascii_lowercase()
                            .ends_with(".jar"),
                    }
                })
                .collect(),
        }))
        .await?;
    reporter
        .track_rollback_paths(
            pack.files
                .iter()
                .map(|file| content_context.resolve_install_path(file))
                .collect(),
        )
        .await?;
    // Warm the shared DNS cache for the official Modrinth hosts so the batch
    // download starts with an ordered address list instead of racing queries.
    crate::util::fetch::prewarm_download_dns(&[
        crate::util::fetch::MODRINTH_CDN_OFFICIAL_HOST,
        crate::util::fetch::TIANPAO_HOST,
        "api.modrinth.com",
    ])
    .await;
    let pack_files = pack.files;
    let mut retry_indices = (0..pack_files.len()).collect::<Vec<_>>();
    let mut required_file_failures = Vec::new();
    for pass in 0..=AUTO_RETRY_PASSES {
        if retry_indices.is_empty() {
            break;
        }
        let tasks = retry_indices
            .iter()
            .map(|&index| (index, pack_files[index].clone()))
            .collect::<Vec<_>>();
        let pass_failures =
            collect_required_file_failures_concurrently(
        tasks,
        Some(state.download_concurrency()),
        |(manifest_index, project)| {
            let content_context = content_context.clone();
            let skipped_missing_content_paths =
                skipped_missing_content_paths.clone();
            async move {
                let project_size = project.file_size as u64;
                let project_path =
                    content_context.resolve_install_path(&project);
                let target_path =
                    content_context.instance_full_path.join(&project_path);
                let context =
                    InstallErrorContext::new("download modpack content file")
                        .maybe_project_id(
                            content_context.pack_project_id.clone(),
                        )
                        .maybe_version_id(
                            content_context.pack_version_id.clone(),
                        )
                        .file_path(project_path.clone())
                        .target_path(target_path.display().to_string())
                        .urls(project.downloads.clone())
                        .maybe_expected_hash(
                            project.hashes.get(&PackFileHash::Sha1).cloned(),
                        )
                        .expected_size(project_size)
                        .build();
                let result: crate::Result<()> = async {

                if skipped_missing_content_paths.contains(&project_path) {
                    content_context
                        .mark_file_settled(
                            0,
                            InstallJobEventKind::ContentFileSkipped {
                                path: project_path.clone(),
                                reason: "skipped by user".to_string(),
                                project_id: None,
                                version_id: None,
                                manual_url: None,
                            },
                        )
                        .await?;
                    return Ok(());
                }

                //TODO: Future update: prompt user for optional files in a modpack
                if let Some(env) = project.env.as_ref()
                    && env
                        .get(&EnvType::Client)
                        .is_some_and(|x| x == &SideType::Unsupported)
                {
                    content_context
                        .mark_file_settled(
                            0,
                            InstallJobEventKind::ContentFileSkipped {
                                path: project_path.clone(),
                                reason: "unsupported on client".to_string(),
                                project_id: None,
                                version_id: None,
                                manual_url: None,
                            },
                        )
                        .await?;
                    return Ok(());
                }

                content_context
                    .reporter
                    .set_transient_context(context.clone())
                    .await?;

                let progress_key = project_path.clone();
                let progress_context = content_context.clone();
                let min_download_progress_delta =
                    (project_size / 200).max(256 * 1024);
                let mut last_reported_downloaded = 0_u64;
                let mut report_download_progress = move |downloaded: u64,
                                                         _total_size: u64|
                      -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>> {
                    if downloaded < project_size
                        && downloaded.saturating_sub(last_reported_downloaded)
                            < min_download_progress_delta
                    {
                        return Box::pin(async { Ok(()) });
                    }

                    last_reported_downloaded = downloaded;
                    let progress_context = progress_context.clone();
                    let progress_key = progress_key.clone();
                    Box::pin(async move {
                        let active_bytes = progress_context
                            .update_active_download(progress_key, downloaded)
                            .await;
                        let current_bytes = progress_context
                            .content_bytes_progress
                            .load(Ordering::Relaxed)
                            .saturating_add(active_bytes)
                            .min(progress_context.content_total_bytes);
                        progress_context
                            .reporter
                            .update(
                                InstallPhaseId::DownloadingContent,
                                Some(InstallProgress {
                                    current: progress_context
                                        .content_progress
                                        .load(Ordering::Relaxed),
                                    total: progress_context.num_files as u64,
                                    secondary: (progress_context
                                        .content_total_bytes
                                        > 0)
                                        .then_some(InstallProgressSecondary {
                                            current: current_bytes,
                                            total: progress_context
                                                .content_total_bytes,
                                        }),
                                }),
                                progress_context.modpack_details.clone(),
                            )
                            .await?;
                        Ok(())
                    })
                };
                let progress =
                    &mut report_download_progress as &mut FetchProgressFn<'_>;
                content_context
                    .report_download_attempt(
                        project_path.clone(),
                        project_size,
                        1,
                        estimated_file_max_attempts(&project),
                    )
                    .await?;
                if tokio::fs::try_exists(&target_path).await.unwrap_or(false) {
                    content_context
                        .reporter
                        .record_events(vec![
                            InstallJobEventKind::ContentFileVerificationStarted {
                                path: project_path.clone(),
                            },
                        ])
                        .await?;
                }
                let Some(primary_url) = project.downloads.first() else {
                    return Err(crate::ErrorKind::InputError(format!(
                        "Modpack file {} has no download URL",
                        project.path
                    ))
                    .into());
                };
                let integrity = Integrity {
                    size: Some(project_size),
                    sha1: project.hashes.get(&PackFileHash::Sha1).cloned(),
                    sha512: project
                        .hashes
                        .get(&PackFileHash::Sha512)
                        .cloned(),
                    content: if project
                        .path
                        .as_str()
                        .to_ascii_lowercase()
                        .ends_with(".jar")
                    {
                        ContentValidation::Jar
                    } else {
                        ContentValidation::None
                    },
                    ..Integrity::default()
                };
                let download = match download_to_path(
                    DownloadRequest::new(primary_url, ResourceClass::Modpack)
                        .with_candidate_urls(
                            project.downloads.iter().skip(1).cloned(),
                        )
                        .with_integrity(integrity)
                        .with_download_meta(
                            content_context.download_meta.clone(),
                        )
                        .with_segmented_download(true)
                        .with_install_tracking(
                            content_context.reporter.clone(),
                            project_path.clone(),
                            project_path.clone(),
                        ),
                    &target_path,
                    &state.download_semaphore,
                    &state.pool,
                    Some(progress),
                )
                .await
                {
                    Ok(download) => {
                        content_context
                            .remove_active_download(&project_path)
                            .await;
                        download
                    }
                    Err(error) => {
                        content_context
                            .remove_active_download(&project_path)
                            .await;
                        return Err(error);
                    }
                };
                let downloaded_bytes = download.size;
                content_context.record_download_result(&download).await;
                let path = target_path;
                let sha1 = if let Some(hash) =
                    project.hashes.get(&PackFileHash::Sha1)
                {
                    hash.clone()
                } else {
                    sha1_file_async(&path).await?.1
                };

                load_modpack_file_infos(
                    &content_context.file_infos_by_hash,
                    &content_context.file_infos_loading,
                    &content_context.file_info_hashes,
                    &state.pool,
                    &state.api_semaphore,
                )
                .await;
                let file_info = content_context
                    .file_infos_by_hash
                    .lock()
                    .await
                    .as_ref()
                    .and_then(|infos| infos.get(&sha1))
                    .cloned();
                {
                    let _permit = state.install_db_semaphore.acquire().await?;
                    if let Some(project_type) =
                    ProjectType::get_from_parent_folder(project.path.as_str())
                    {
                    let provider_ref = match file_info.as_ref() {
                        Some(file) => Some(ContentProviderRef::Modrinth {
                            project_id: ModrinthProjectId::new(
                                file.project_id.clone(),
                            )?,
                            version_id: Some(ModrinthVersionId::new(
                                file.version_id.clone(),
                            )?),
                        }),
                        None => None,
                    };
                    content_context
                        .reporter
                        .preserve_failure_context(
                            context.clone(),
                            crate::state::instances::commands::record_project_file_atomic(
                                &content_context.instance_id,
                                &project_path,
                                &sha1,
                                downloaded_bytes,
                                project_type,
								modpack_source_kind(
									content_context.pack_version_id.as_deref(),
								),
								crate::state::instances::ContentOwnershipKind::PackManaged,
                                provider_ref.as_ref(),
                                false,
                                file_info.as_ref().map(|file| KnownModrinthFile {
                                    project_id: &file.project_id,
                                    version_id: &file.version_id,
                                }),
                                state,
                            )
                            .await,
                        )
                        .await?;
                    }
                }

								let recovered = download.attempts == 0; //When recovered, the download attempts were set to 0 by download_to_path_inner()
								if recovered {
									content_context
										.mark_file_settled(
												downloaded_bytes,
												InstallJobEventKind::ContentFileRecovered {
														path: project_path.clone(),
														bytes: downloaded_bytes,
												},
										)
										.await?;
								} else {
									content_context
											.mark_file_settled(
													downloaded_bytes,
													InstallJobEventKind::ContentFileCompleted {
															path: project_path.clone(),
															bytes: downloaded_bytes,
													},
											)
											.await?;
								}
                Ok(())
                }
                .await;
                match result {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        content_context
                            .remove_active_download(&project_path)
                            .await;
                        if pass == AUTO_RETRY_PASSES {
                            content_context
                                .reporter
                                .persist_failure_context(context)
                                .await;
                            let failure = RequiredFileFailure::new(
                                manifest_index,
                                &project_path,
                                &error.to_string(),
                            );
                            tracing::warn!(
                                path = %project_path,
                                reason = %failure.reason,
                                "Failed to install required Modrinth pack file"
                            );
                            if let Err(report_error) = content_context
                                .mark_file_settled(
                                    0,
                                    InstallJobEventKind::ContentFileFailed {
                                        path: project_path.clone(),
                                        reason: failure.reason.clone(),
                                        project_id: None,
                                        version_id: None,
                                    },
                                )
                                .await
                            {
                                return Err(RequiredFileFailure::new(
                                    manifest_index,
                                    &project_path,
                                    &format!(
                                        "{}; failed to record item failure: {report_error}",
                                        failure.reason
                                    ),
                                ));
                            }
                            Err(failure)
                        } else {
                            tracing::warn!(
                                path = %project_path,
                                pass = pass + 1,
                                retries_left =
                                    AUTO_RETRY_PASSES.saturating_sub(pass),
                                reason = %error,
                                "Modpack file failed; scheduling an automatic retry"
                            );
                            Err(RequiredFileFailure::new(
                                manifest_index,
                                &project_path,
                                &error.to_string(),
                            ))
                        }
                    }
                }
            }
        },
    )
    .await?;
        required_file_failures = pass_failures;
        if required_file_failures.is_empty() {
            break;
        }
        if pass < AUTO_RETRY_PASSES {
            tracing::warn!(
                files = required_file_failures.len(),
                pass = pass + 1,
                retries_left = AUTO_RETRY_PASSES.saturating_sub(pass),
                "Automatic retry pass for failed modpack files"
            );
        }
        retry_indices = required_file_failures
            .iter()
            .map(|failure| failure.manifest_index)
            .collect();
    }
    content_context.finish_download_metrics().await?;
    if let Some(reason) =
        missing_required_content_pause(&required_file_failures)
    {
        return Ok(MrpackInstallOutcome::WaitingForUser(reason));
    }

    let override_file_entries = zip_reader
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let filename = file.filename().as_str().unwrap_or_default();
            ((filename.starts_with(&overrides_prefix)
                || filename.starts_with(&client_overrides_prefix))
                && !filename.ends_with('/'))
            .then(|| (index, file.clone()))
        })
        .collect::<Vec<_>>();
    let override_total_bytes = override_file_entries
        .iter()
        .map(|(_, file)| file.uncompressed_size())
        .sum::<u64>();
    reporter
        .track_rollback_paths(
            override_file_entries
                .iter()
                .map(|(_, file)| {
                    override_relative_path(
                        file.filename().as_str().unwrap_or_default(),
                        &archive_base,
                    )
                })
                .collect::<crate::Result<Vec<_>>>()?,
        )
        .await?;
    let progress = (override_total_bytes > 0).then_some(InstallProgress {
        current: 0,
        total: override_total_bytes,
        secondary: None,
    });
    reporter
        .update(
            InstallPhaseId::ExtractingOverrides,
            progress,
            modpack_details.clone(),
        )
        .await?;

    let extracted_override_bytes = Arc::new(AtomicU64::new(0));
    let mut last_reported_override_bytes = 0_u64;
    let reporter_for_overrides = reporter.clone();
    let details_for_overrides = modpack_details.clone();
    let mut report_override_progress = |bytes_read: u64| -> Pin<
        Box<dyn Future<Output = crate::Result<()>> + Send>,
    > {
        let current = extracted_override_bytes
            .fetch_add(bytes_read, Ordering::Relaxed)
            + bytes_read;
        let min_delta = (override_total_bytes / 200).max(256 * 1024);
        if current < override_total_bytes
            && current.saturating_sub(last_reported_override_bytes) < min_delta
        {
            return Box::pin(async { Ok(()) });
        }

        last_reported_override_bytes = current;
        let reporter = reporter_for_overrides.clone();
        let details = details_for_overrides.clone();
        Box::pin(async move {
            reporter
                .update(
                    InstallPhaseId::ExtractingOverrides,
                    Some(InstallProgress {
                        current: current.min(override_total_bytes),
                        total: override_total_bytes,
                        secondary: None,
                    }),
                    details,
                )
                .await?;
            Ok(())
        })
    };
    for (index, file) in override_file_entries {
        let entry_name = file.filename().as_str().unwrap();
        let entry_name =
            entry_name.strip_prefix(&archive_base).unwrap_or(entry_name);
        let relative_override_file_path =
            SafeRelativeUtf8UnixPathBuf::try_from(entry_name.to_string())?;
        let relative_override_file_path = relative_override_file_path
            .strip_prefix("overrides")
            .or_else(|_| relative_override_file_path.strip_prefix("client-overrides"))
            .map_err(|_| {
                crate::Error::from(crate::ErrorKind::OtherError(
                    format!("Failed to strip override prefix from override file path: {relative_override_file_path}")
                ))
            })?;

        let path =
            instance_full_path.join(relative_override_file_path.as_str());
        let override_context =
            InstallErrorContext::new("extract modpack override")
                .maybe_project_id(project_id.clone())
                .maybe_version_id(version_id.clone())
                .source_path(source_path.clone())
                .entry_path(file.filename().as_str().unwrap_or_default())
                .target_path(path.display().to_string())
                .build();
        reporter
            .set_transient_context(override_context.clone())
            .await?;
        let extract_result = if override_total_bytes > 0 {
            let progress =
                &mut report_override_progress as &mut ExtractProgressFn<'_>;
            zip_reader
                .extract_entry(
                    index,
                    &path,
                    &state.io_semaphore,
                    Some(progress),
                )
                .await
        } else {
            zip_reader
                .extract_entry(index, &path, &state.io_semaphore, None)
                .await
        };
        let (size, hash) = reporter
            .preserve_failure_context(override_context, extract_result)
            .await?;

        {
            let _permit = state.install_db_semaphore.acquire().await?;
            let record_context =
                InstallErrorContext::new("record modpack override")
                    .maybe_project_id(project_id.clone())
                    .maybe_version_id(version_id.clone())
                    .source_path(source_path.clone())
                    .entry_path(file.filename().as_str().unwrap_or_default())
                    .target_path(path.display().to_string())
                    .build();
            if let Some(project_type) = ProjectType::get_from_parent_folder(
                relative_override_file_path.as_str(),
            ) {
                reporter
                    .preserve_failure_context(
                        record_context,
                        crate::state::instances::commands::record_project_file_atomic(
                            &instance_id,
                            relative_override_file_path.as_str(),
                            &hash,
                            size,
                            project_type,
							modpack_source_kind(version_id.as_deref()),
							crate::state::instances::ContentOwnershipKind::PackManaged,
							None,
                            false,
                            None,
                            state,
                        )
                        .await,
                    )
                    .await?;
            }
        }
    }

    // If the icon doesn't exist, we expect icon.png to be a potential icon.
    // If it doesn't exist, and an override to icon.png exists, cache and use that
    let potential_icon = instance_full_path.join("icon.png");
    if !icon_exists && potential_icon.exists() {
        crate::api::instance::edit_icon(&instance_id, Some(&potential_icon))
            .await?;
    }

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        &instance_id,
        false,
        Some(reporter.clone()),
        crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
    )
    .await?;
    reporter.clear_context().await?;

    Ok(MrpackInstallOutcome::Completed(instance_id.clone()))
}

fn pack_source_path(file: &CreatePackFile) -> String {
    match file {
        CreatePackFile::Bytes(_) => "downloaded mrpack bytes".to_string(),
        CreatePackFile::Path(path) => path.display().to_string(),
    }
}

/// Finds the `modrinth.index.json` entry and the archive base folder it lives
/// in. The exact root entry is preferred; a manifest one wrapping folder deep
/// is accepted as a fallback.
fn locate_mrpack_manifest(
    zip_file: &async_zip::ZipFile,
) -> Option<(usize, String)> {
    let entries = zip_file.entries();
    entries
        .iter()
        .position(|f| {
            matches!(f.filename().as_str(), Ok("modrinth.index.json"))
        })
        .map(|index| (index, String::new()))
        .or_else(|| {
            entries.iter().enumerate().find_map(|(index, f)| {
                let name = f.filename().as_str().ok()?;
                let (base, rest) = name.split_once('/')?;
                (rest == "modrinth.index.json")
                    .then(|| (index, format!("{base}/")))
            })
        })
}

fn override_relative_path(
    entry_name: &str,
    archive_base: &str,
) -> crate::Result<String> {
    let entry_name =
        entry_name.strip_prefix(archive_base).unwrap_or(entry_name);
    let relative =
        SafeRelativeUtf8UnixPathBuf::try_from(entry_name.to_string())?;
    let relative = relative
        .strip_prefix("overrides")
        .or_else(|_| relative.strip_prefix("client-overrides"))
        .map_err(|_| {
            crate::ErrorKind::InputError(format!(
                "Invalid modpack override path: {relative}"
            ))
        })?;
    Ok(relative.as_str().to_string())
}

pub(crate) async fn related_file_paths(
    mrpack_file: &CreatePackFile,
) -> crate::Result<Vec<String>> {
    let mut zip_reader = MrpackZipReader::new(mrpack_file).await?;
    let Some((manifest_idx, archive_base)) =
        locate_mrpack_manifest(zip_reader.file())
    else {
        return Err(crate::ErrorKind::InputError(
            "No pack manifest found in mrpack".to_string(),
        )
        .into());
    };
    let manifest = zip_reader.read_entry_to_string(manifest_idx).await?;
    let pack: PackFormat = serde_json::from_str(&manifest)?;
    let overrides_prefix = format!("{archive_base}overrides/");
    let client_overrides_prefix = format!("{archive_base}client-overrides/");
    let mut paths = pack
        .files
        .into_iter()
        .map(|file| file.path.as_str().to_string())
        .collect::<Vec<_>>();
    for entry in zip_reader.file().entries() {
        let name = entry.filename().as_str().unwrap_or_default();
        if (name.starts_with(&overrides_prefix)
            || name.starts_with(&client_overrides_prefix))
            && !name.ends_with('/')
        {
            paths.push(override_relative_path(name, &archive_base)?);
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn modpack_source_kind(version_id: Option<&str>) -> ContentSourceKind {
    if version_id.is_some() {
        ContentSourceKind::ModrinthModpack
    } else {
        ContentSourceKind::ImportedModpack
    }
}

#[tracing::instrument(skip(mrpack_file))]

pub async fn remove_all_related_files(
    instance_id: String,
    mrpack_file: CreatePackFile,
) -> crate::Result<()> {
    // Updates can remove files from a locally imported or downloaded pack, so share the same reader path.
    let mut zip_reader = MrpackZipReader::new(&mrpack_file).await?;

    // Extract index of modrinth.index.json
    let Some((manifest_idx, archive_base)) =
        locate_mrpack_manifest(zip_reader.file())
    else {
        return Err(crate::Error::from(crate::ErrorKind::InputError(
            "No pack manifest found in mrpack".to_string(),
        )));
    };
    let overrides_prefix = format!("{archive_base}overrides/");
    let client_overrides_prefix = format!("{archive_base}client-overrides/");

    let manifest = zip_reader.read_entry_to_string(manifest_idx).await?;

    let pack: PackFormat = serde_json::from_str(&manifest)?;

    if &*pack.game != "minecraft" {
        return Err(crate::ErrorKind::InputError(
            "Pack does not support Minecraft".to_string(),
        )
        .into());
    }

    crate::api::instance::edit(
        &instance_id,
        EditInstance {
            install_stage: Some(InstanceInstallStage::PackInstalling),
            ..EditInstance::default()
        },
    )
    .await?;

    // First, remove all modrinth projects by their version hashes
    // Remove all modrinth projects by their version hashes
    // We need to do a fetch to get the project ids from Modrinth
    let state = State::get().await?;
    let metadata =
        crate::api::instance::get(&instance_id)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Unknown instance {instance_id}"
                ))
            })?;
    let instance_full_path =
        crate::api::instance::get_full_path(&instance_id).await?;
    let all_hashes = pack
        .files
        .iter()
        .filter_map(|f| Some(f.hashes.get(&PackFileHash::Sha1)?.clone()))
        .collect::<Vec<_>>();

    // First, get project info by hash
    let file_infos = CachedEntry::get_file_many(
        &all_hashes.iter().map(|x| &**x).collect::<Vec<_>>(),
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;

    let to_remove = file_infos
        .into_iter()
        .map(|p| p.project_id)
        .collect::<Vec<_>>();

    for file in crate::state::instances::commands::list_project_files(
        &metadata.instance.id,
        &state,
    )
    .await?
    {
        let is_modrinth_project = file.provider_refs.iter().any(|provider| {
            matches!(
                provider,
                ContentProviderRef::Modrinth { project_id, .. }
                    if to_remove.contains(&project_id.to_string())
            )
        });
        if is_modrinth_project {
            crate::state::instances::commands::remove_project(
                &metadata.instance.id,
                &file.relative_path,
                &state,
            )
            .await?;
        }
    }

    // Iterate over all Modrinth project file paths in the json, and remove them
    // (There should be few, but this removes any files the .mrpack intended as Modrinth projects but were unrecognized)
    for file in pack.files {
        match io::remove_file(instance_full_path.join(file.path.as_str())).await
        {
            Ok(_) => (),
            Err(err) if err.kind() == ErrorKind::NotFound => (),
            Err(err) => return Err(err.into()),
        }
    }

    // Iterate over each 'overrides' file and remove it
    let override_file_entries =
        zip_reader.file().entries().iter().filter(|file| {
            let filename = file.filename().as_str().unwrap_or_default();
            (filename.starts_with(&overrides_prefix)
                || filename.starts_with(&client_overrides_prefix))
                && !filename.ends_with('/')
        });

    for file in override_file_entries {
        let entry_name = file.filename().as_str().unwrap();
        let entry_name =
            entry_name.strip_prefix(&archive_base).unwrap_or(entry_name);
        let relative_override_file_path =
            SafeRelativeUtf8UnixPathBuf::try_from(entry_name.to_string())?;
        let relative_override_file_path = relative_override_file_path
            .strip_prefix("overrides")
            .or_else(|_| relative_override_file_path.strip_prefix("client-overrides"))
            .map_err(|_| {
                crate::Error::from(crate::ErrorKind::OtherError(
                    format!("Failed to strip override prefix from override file path: {relative_override_file_path}")
                ))
            })?;

        // Remove this file if a corresponding one exists in the filesystem
        match io::remove_file(
            instance_full_path.join(relative_override_file_path.as_str()),
        )
        .await
        {
            Ok(_) => (),
            Err(err) if err.kind() == ErrorKind::NotFound => (),
            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_zip::tokio::write::ZipFileWriter;
    use async_zip::{Compression, ZipEntryBuilder};
    use std::sync::Mutex as StdMutex;

    async fn run_failure_collection(
        failed_indices: &[usize],
    ) -> (Vec<usize>, Vec<RequiredFileFailure>) {
        let attempted = Arc::new(StdMutex::new(Vec::new()));
        let failed_indices = Arc::new(failed_indices.to_vec());
        let failures = collect_required_file_failures_concurrently(
            (0..4).map(|index| (index, index)).collect(),
            Some(1),
            {
                let attempted = attempted.clone();
                move |(manifest_index, value)| {
                    let attempted = attempted.clone();
                    let failed_indices = failed_indices.clone();
                    async move {
                        attempted.lock().unwrap().push(value);
                        if failed_indices.contains(&value) {
                            Err(RequiredFileFailure::new(
                                manifest_index,
                                &format!("mods/{value}.jar"),
                                "test download failure",
                            ))
                        } else {
                            Ok(())
                        }
                    }
                }
            },
        )
        .await
        .unwrap();
        let attempted = attempted.lock().unwrap().clone();
        (attempted, failures)
    }

    #[tokio::test]
    async fn all_required_file_tasks_complete_without_failures() {
        let (attempted, failures) = run_failure_collection(&[]).await;

        assert_eq!(attempted, vec![0, 1, 2, 3]);
        assert!(failures.is_empty());
        assert!(missing_required_content_pause(&failures).is_none());
    }

    #[tokio::test]
    async fn first_required_file_failure_does_not_stop_later_tasks() {
        let (attempted, failures) = run_failure_collection(&[0]).await;

        assert_eq!(attempted, vec![0, 1, 2, 3]);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].path, "mods/0.jar");
    }

    #[tokio::test]
    async fn middle_required_file_failure_does_not_stop_later_tasks() {
        let (attempted, failures) = run_failure_collection(&[1]).await;

        assert_eq!(attempted, vec![0, 1, 2, 3]);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].path, "mods/1.jar");
    }

    #[tokio::test]
    async fn multiple_required_file_failures_are_all_collected() {
        let (attempted, failures) = run_failure_collection(&[0, 2, 3]).await;

        assert_eq!(attempted, vec![0, 1, 2, 3]);
        assert_eq!(
            failures
                .iter()
                .map(|failure| failure.path.as_str())
                .collect::<Vec<_>>(),
            vec!["mods/0.jar", "mods/2.jar", "mods/3.jar"]
        );
        let summary = RequiredFileFailures(&failures).to_string();
        assert!(summary.contains("3 required modpack files failed"));
        assert!(summary.contains("mods/0.jar"));

        let pause = missing_required_content_pause(&failures).unwrap();
        assert_eq!(
            pause,
            InstallPauseReason::MissingRequiredContent {
                failed_files: 3,
                paths: vec![
                    "mods/0.jar".to_string(),
                    "mods/2.jar".to_string(),
                    "mods/3.jar".to_string(),
                ],
            }
        );
    }

    #[test]
    fn aggregate_required_file_error_has_bounded_path_details() {
        let failures = (0..10)
            .map(|index| {
                RequiredFileFailure::new(
                    index,
                    &format!("mods/{index}-{}.jar", "x".repeat(500)),
                    "failed",
                )
            })
            .collect::<Vec<_>>();

        let message = RequiredFileFailures(&failures).to_string();
        assert!(message.contains("10 required modpack files failed"));
        assert!(message.contains("(+7 more)"));
        assert!(message.len() < 600);
    }

    #[tokio::test]
    async fn local_mrpack_uses_the_file_backed_reader() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("local.mrpack");
        let mut file = tokio::fs::File::create(&path).await.unwrap();
        let mut writer = ZipFileWriter::with_tokio(&mut file);
        writer
            .write_entry_whole(
                ZipEntryBuilder::new(
                    "modrinth.index.json".to_string().into(),
                    Compression::Stored,
                ),
                br#"{"formatVersion":1,"game":"minecraft","versionId":"test","name":"test","files":[]}"#,
            )
            .await
            .unwrap();
        writer.close().await.unwrap();
        drop(file);

        let mut reader = MrpackZipReader::new(&CreatePackFile::Path(path))
            .await
            .unwrap();
        assert!(matches!(&reader, MrpackZipReader::File(_)));
        let index = reader
            .file()
            .entries()
            .iter()
            .position(|entry| {
                matches!(entry.filename().as_str(), Ok("modrinth.index.json"))
            })
            .unwrap();
        assert_eq!(
            reader.read_entry_to_string(index).await.unwrap(),
            r#"{"formatVersion":1,"game":"minecraft","versionId":"test","name":"test","files":[]}"#
        );
    }
}
