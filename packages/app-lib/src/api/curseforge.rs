use crate::api::content_search::{
    chinese_file_title_for_curseforge_slug, localized_content_file_name,
};
use crate::event::LoadingBarType;
use crate::event::emit::{
    emit_loading, init_loading, loading_try_for_each_concurrent,
};
use crate::install::{
    InstallJobEventKind, InstallPhaseDetails, InstallPhaseId, InstallProgress,
    InstallProgressReporter, InstallProgressSecondary,
};
use crate::state::{
    CachedEntry, ContentProvider, ContentProviderRef, ContentSourceKind,
    CurseForgeFileId, CurseForgeProjectId, DownloadSourceMode, EditInstance,
    InstanceLink, ModLoader, ProjectType, ReleaseChannel,
};
use crate::util::fetch::{
    ContentValidation, DownloadRequest, DownloadResult, DownloadRouteSource,
    FetchProgressFn, Integrity, ProxyPolicy, ResourceClass, download_to_path,
    resolve_download_routes_for, sha1_file_async,
};
use crate::{ErrorKind, State};
use futures::stream;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

const API_BASE_URL: &str = "https://api.curseforge.com";
const MINECRAFT_GAME_ID: u32 = 432;
const MAX_PAGE_SIZE: u32 = 50;
const MODPACK_FILE_INSTALL_ATTEMPTS: usize = 3;
const DEPENDENCY_RELATION_OPTIONAL: u32 = 2;
const DEPENDENCY_RELATION_REQUIRED: u32 = 3;
const DEPENDENCY_RELATION_INCOMPATIBLE: u32 = 5;
const DEPENDENCY_RELATION_INCLUDE: u32 = 6;

static UNAUTHORIZED: AtomicBool = AtomicBool::new(false);
static CATEGORY_CACHE: LazyLock<RwLock<Option<Vec<CurseForgeCategory>>>> =
    LazyLock::new(|| RwLock::new(None));
static MANUAL_IMPORT_SCAN_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

#[derive(Default)]
struct CurseForgeDownloadMetrics {
    source: Mutex<Option<String>>,
    fallback_count: AtomicU64,
    reporter: Option<InstallProgressReporter>,
}

impl CurseForgeDownloadMetrics {
    fn with_reporter(reporter: InstallProgressReporter) -> Self {
        Self {
            reporter: Some(reporter),
            ..Self::default()
        }
    }

    fn record(&self, result: &DownloadResult) {
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
        reporter: &InstallProgressReporter,
    ) -> crate::Result<()> {
        let source = self.source.lock().ok().and_then(|source| source.clone());
        if let Some(source) = source {
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
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(crate::launcher_user_agent())
        .no_proxy()
        .build()
        .expect("CurseForge client configuration should be valid")
});

static PROXY_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(crate::launcher_user_agent())
        .build()
        .expect("CurseForge proxy client configuration should be valid")
});

#[cfg(debug_assertions)]
static LOCAL_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(crate::launcher_user_agent())
        .no_proxy()
        .build()
        .expect("Local CurseForge client configuration should be valid")
});

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurseForgeCapabilityStatus {
    MissingKey,
    Ready,
    Unauthorized,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurseForgeCapability {
    pub status: CurseForgeCapabilityStatus,
    pub configured: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgePagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u32,
}

#[derive(Clone, Debug, Deserialize)]
struct CurseForgeResponse<T> {
    data: T,
    #[serde(default)]
    pagination: Option<CurseForgePagination>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeSearchRequest {
    pub class_id: u32,
    #[serde(default)]
    pub category_id: Option<u32>,
    #[serde(default)]
    pub category_ids: Vec<u32>,
    #[serde(default)]
    pub search_filter: Option<String>,
    #[serde(default)]
    pub game_version: Option<String>,
    #[serde(default)]
    pub mod_loader_type: Option<u32>,
    #[serde(default)]
    pub sort_field: Option<u32>,
    #[serde(default)]
    pub sort_order: Option<String>,
    #[serde(default)]
    pub index: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page_size() -> u32 {
    20
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnifiedSearchResponse {
    pub provider: ContentProvider,
    pub hits: Vec<UnifiedSearchHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnifiedSearchHit {
    pub provider: ContentProvider,
    pub project_id: String,
    pub slug: Option<String>,
    pub author: String,
    pub author_url: Option<String>,
    pub title: String,
    pub description: String,
    pub project_type: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub categories: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub versions: Vec<String>,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub date_created: String,
    pub date_modified: String,
    pub latest_version: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub gallery: Vec<String>,
    pub website_url: Option<String>,
    pub source_url: Option<String>,
    pub allow_mod_distribution: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeAuthor {
    pub id: u32,
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeLinks {
    pub website_url: Option<String>,
    pub wiki_url: Option<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeAsset {
    pub id: u32,
    pub mod_id: u32,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeCategory {
    pub id: u32,
    pub game_id: u32,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub icon_url: Option<String>,
    pub date_modified: String,
    #[serde(default)]
    pub is_class: Option<bool>,
    pub class_id: Option<u32>,
    pub parent_category_id: Option<u32>,
    pub display_index: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileIndex {
    pub game_version: String,
    pub file_id: u32,
    pub filename: String,
    pub release_type: u32,
    pub game_version_type_id: Option<u32>,
    pub mod_loader: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeProject {
    pub id: u32,
    pub game_id: u32,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub links: CurseForgeLinks,
    pub summary: String,
    pub status: u32,
    pub download_count: u64,
    pub is_featured: bool,
    pub primary_category_id: u32,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub categories: Vec<CurseForgeCategory>,
    pub class_id: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub authors: Vec<CurseForgeAuthor>,
    pub logo: Option<CurseForgeAsset>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub screenshots: Vec<CurseForgeAsset>,
    pub main_file_id: u32,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub latest_files: Vec<CurseForgeFile>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub latest_files_indexes: Vec<CurseForgeFileIndex>,
    pub date_created: String,
    pub date_modified: String,
    pub date_released: String,
    pub allow_mod_distribution: Option<bool>,
    pub game_popularity_rank: Option<i32>,
    pub is_available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileHash {
    pub value: String,
    pub algo: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileDependency {
    pub mod_id: u32,
    pub relation_type: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeSortableGameVersion {
    pub game_version_name: String,
    pub game_version_padded: Option<String>,
    pub game_version: Option<String>,
    pub game_version_release_date: Option<String>,
    pub game_version_type_id: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileModule {
    pub name: String,
    pub fingerprint: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFile {
    pub id: u32,
    pub game_id: u32,
    pub mod_id: u32,
    pub is_available: bool,
    pub display_name: String,
    pub file_name: String,
    pub release_type: u32,
    pub file_status: u32,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub hashes: Vec<CurseForgeFileHash>,
    pub file_date: String,
    pub file_length: u64,
    pub download_count: u64,
    pub file_size_on_disk: Option<u64>,
    pub download_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub game_versions: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub sortable_game_versions: Vec<CurseForgeSortableGameVersion>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub dependencies: Vec<CurseForgeFileDependency>,
    pub expose_as_alternative: Option<bool>,
    pub parent_project_file_id: Option<u32>,
    pub alternate_file_id: Option<u32>,
    pub is_server_pack: Option<bool>,
    pub server_pack_file_id: Option<u32>,
    pub is_early_access_content: Option<bool>,
    pub early_access_end_date: Option<String>,
    pub file_fingerprint: u64,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub modules: Vec<CurseForgeFileModule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFilesRequest {
    #[serde(default)]
    pub game_version: Option<String>,
    #[serde(default)]
    pub mod_loader_type: Option<u32>,
    #[serde(default)]
    pub game_version_type_id: Option<u32>,
    #[serde(default)]
    pub index: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurseForgeFilesResponse {
    pub files: Vec<CurseForgeFile>,
    pub pagination: CurseForgePagination,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeInstallRequest {
    pub instance_id: String,
    pub project_id: u32,
    pub file_id: u32,
    pub project_type: String,
    #[serde(default)]
    pub ownership_kind: crate::state::instances::ContentOwnershipKind,
    #[serde(default)]
    pub manual_operation_kind:
        crate::state::instances::ManualDownloadOperationKind,
    #[serde(default)]
    pub game_version: Option<String>,
    #[serde(default)]
    pub mod_loader_type: Option<u32>,
    #[serde(default)]
    pub world_name: Option<String>,
    #[serde(default = "default_true")]
    pub install_dependencies: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeInstalledFile {
    pub project_id: u32,
    pub file_id: u32,
    pub relative_path: String,
    pub dependency: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeManualDownload {
    pub project_id: u32,
    pub file_id: u32,
    pub file_name: String,
    #[serde(default)]
    pub ownership_kind: crate::state::instances::ContentOwnershipKind,
    #[serde(default)]
    pub operation_kind: crate::state::instances::ManualDownloadOperationKind,
    pub website_url: Option<String>,
    #[serde(default)]
    pub project_type: String,
    #[serde(default)]
    pub project_slug: String,
    #[serde(default)]
    pub target_folder: String,
    #[serde(default)]
    pub hashes: Vec<CurseForgeFileHash>,
    #[serde(default)]
    pub file_length: u64,
    #[serde(default)]
    pub file_fingerprint: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeManualDownloadImport {
    pub project_id: u32,
    pub file_id: u32,
    pub relative_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeManualDownloadImportError {
    pub project_id: u32,
    pub file_id: u32,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeManualDownloadScanResult {
    pub download_directory: Option<String>,
    pub imported: Vec<CurseForgeManualDownloadImport>,
    pub errors: Vec<CurseForgeManualDownloadImportError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFailedDownload {
    pub project_id: u32,
    pub file_id: u32,
    pub file_name: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeInstallResult {
    pub installed: Vec<CurseForgeInstalledFile>,
    pub manual_downloads: Vec<CurseForgeManualDownload>,
    #[serde(default)]
    pub failed_downloads: Vec<CurseForgeFailedDownload>,
    pub optional_dependencies: Vec<u32>,
    pub incompatible_dependencies: Vec<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeRecognitionResult {
    pub scanned: u32,
    pub matched: u32,
    pub linked: Vec<CurseForgeInstalledFile>,
    pub unmatched_paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModpackInstallRequest {
    pub instance_id: String,
    pub project_id: u32,
    pub file_id: u32,
    #[serde(default)]
    pub install_optional: bool,
    #[serde(default)]
    pub allow_target_change: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModpackInstallResult {
    pub content: CurseForgeInstallResult,
    pub overrides_written: u32,
    pub minecraft_version: String,
    pub loader: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurseForgeModpackTarget {
    pub game_version: String,
    pub loader: ModLoader,
    pub loader_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeModpackManifest {
    minecraft: CurseForgeManifestMinecraft,
    #[serde(default)]
    files: Vec<CurseForgeManifestFile>,
    #[serde(default = "default_overrides")]
    overrides: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

fn default_overrides() -> String {
    "overrides".to_string()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeManifestMinecraft {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<CurseForgeManifestLoader>,
}

#[derive(Clone, Debug, Deserialize)]
struct CurseForgeManifestLoader {
    id: String,
    primary: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CurseForgeManifestFile {
    // CurseForge modpack manifests use projectID/fileID (capital ID), not projectId.
    #[serde(alias = "projectID", alias = "projectId")]
    pub(crate) project_id: u32,
    #[serde(alias = "fileID", alias = "fileId")]
    pub(crate) file_id: u32,
    #[serde(default = "default_true")]
    pub(crate) required: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct CurseForgePackExpectedMember {
    pub project_id: u32,
    pub file_id: u32,
    pub project_type: ProjectType,
    pub expected_relative_path: String,
    pub required: bool,
    pub expected_sha1: Option<String>,
    pub expected_size: Option<u64>,
    pub expected_fingerprint: Option<u64>,
    pub manual_download: Option<CurseForgeManualDownload>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CurseForgePackExpectedOverride {
    pub project_type: ProjectType,
    pub expected_relative_path: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CurseForgePackExpectedContent {
    pub members: Vec<CurseForgePackExpectedMember>,
    pub overrides: Vec<CurseForgePackExpectedOverride>,
}

pub(crate) async fn get_modpack_expected_members(
    project_id: u32,
    file_id: u32,
) -> crate::Result<CurseForgePackExpectedContent> {
    let pack_file = get_file(project_id, file_id).await?;
    let project = get_project(project_id).await?;
    let download_url = if project.allow_mod_distribution == Some(false) {
		None
	} else {
		match pack_file.download_url.clone() {
			Some(url) => Some(url),
			None => get_download_url(project_id, file_id).await?,
		}
	}
	.ok_or_else(|| {
		ErrorKind::InputError(
			"The linked CurseForge pack archive requires a manual download, so membership cannot be calibrated automatically"
				.to_string(),
		)
	})?;
    let pack_download = download_curseforge_archive(
        project_id,
        file_id,
        &pack_file,
        &download_url,
        None,
        None,
    )
    .await?;
    let archive_path = pack_download.path;
    let (manifest, overrides) = tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(archive_path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(modpack_zip_error)?;
        let manifest = read_modpack_manifest(&mut archive)?;
        let overrides = read_modpack_override_content(&mut archive, &manifest)?;
        Ok::<_, crate::Error>((manifest, overrides))
    })
    .await??;

    let file_ids = manifest
        .files
        .iter()
        .map(|member| member.file_id)
        .collect::<Vec<_>>();
    let project_ids = manifest
        .files
        .iter()
        .map(|member| member.project_id)
        .collect::<Vec<_>>();
    let mut files = HashMap::new();
    for chunk in file_ids.chunks(50) {
        for file in get_files_many(chunk.to_vec()).await? {
            files.insert(file.id, file);
        }
    }
    let mut projects = HashMap::new();
    for chunk in project_ids.chunks(50) {
        for project in get_projects(chunk.to_vec()).await? {
            projects.insert(project.id, project);
        }
    }

    let mut expected = Vec::with_capacity(manifest.files.len());
    for member in manifest.files {
        let file = files.get(&member.file_id).ok_or_else(|| {
            ErrorKind::OtherError(format!(
                "CurseForge file metadata {} is missing",
                member.file_id
            ))
        })?;
        let project = projects.get(&member.project_id).ok_or_else(|| {
            ErrorKind::OtherError(format!(
                "CurseForge project metadata {} is missing",
                member.project_id
            ))
        })?;
        let project_type =
            managed_project_type(project_type_for_class(project.class_id))?;
        validate_file_name(&file.file_name)?;
        let target_folder = content_target_folder(project_type, None)?;
        let manual_download_required =
            if project.allow_mod_distribution == Some(false) {
                true
            } else if file.download_url.is_some() {
                false
            } else {
                get_download_url(member.project_id, member.file_id)
                    .await?
                    .is_none()
            };
        let manual_download = manual_download_required.then(|| {
            manual_download_from_file(
                member.project_id,
                member.file_id,
                file,
                project,
                project_type.get_name(),
                target_folder.clone(),
                crate::state::instances::ContentOwnershipKind::PackManaged,
                crate::state::instances::ManualDownloadOperationKind::PackUpdate,
            )
        });
        expected.push(CurseForgePackExpectedMember {
            project_id: member.project_id,
            file_id: member.file_id,
            project_type,
            expected_relative_path: format!(
                "{}/{}",
                target_folder, file.file_name
            ),
            required: member.required,
            expected_sha1: file
                .hashes
                .iter()
                .find(|hash| hash.algo == 1)
                .map(|hash| hash.value.clone()),
            expected_size: Some(file.file_length),
            expected_fingerprint: Some(file.file_fingerprint),
            manual_download,
        });
    }
    Ok(CurseForgePackExpectedContent {
        members: expected,
        overrides,
    })
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFingerprintMatch {
    pub id: u32,
    pub file: CurseForgeFile,
    pub latest_files: Vec<CurseForgeFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFingerprintResult {
    pub is_cache_built: bool,
    #[serde(default)]
    pub exact_matches: Vec<CurseForgeFingerprintMatch>,
    #[serde(default)]
    pub exact_fingerprints: Vec<u64>,
    #[serde(default)]
    pub partial_matches: Vec<Value>,
    #[serde(default)]
    pub partial_match_fingerprints: Value,
    #[serde(default)]
    pub installed_fingerprints: Vec<u64>,
    #[serde(default)]
    pub unmatched_fingerprints: Vec<u64>,
}

pub fn capability() -> CurseForgeCapability {
    let configured = api_key().is_some();
    let status = if !configured {
        CurseForgeCapabilityStatus::MissingKey
    } else if UNAUTHORIZED.load(Ordering::Relaxed) {
        CurseForgeCapabilityStatus::Unauthorized
    } else {
        CurseForgeCapabilityStatus::Ready
    };

    CurseForgeCapability { status, configured }
}

pub async fn validate_credentials() -> crate::Result<CurseForgeCapability> {
    let _: CurseForgeResponse<Vec<CurseForgeProject>> = request_json(
        Method::GET,
        "/v1/mods/search",
        vec![
            ("gameId".to_string(), MINECRAFT_GAME_ID.to_string()),
            ("classId".to_string(), "6".to_string()),
            ("pageSize".to_string(), "1".to_string()),
        ],
        None,
        MirrorPolicy::OfficialOnly,
    )
    .await?;
    UNAUTHORIZED.store(false, Ordering::Relaxed);
    Ok(capability())
}

pub async fn search_projects(
    request: CurseForgeSearchRequest,
) -> crate::Result<UnifiedSearchResponse> {
    let page_size = request.page_size.clamp(1, MAX_PAGE_SIZE);
    let mut query = vec![
        ("gameId".to_string(), MINECRAFT_GAME_ID.to_string()),
        ("classId".to_string(), request.class_id.to_string()),
        ("index".to_string(), request.index.to_string()),
        ("pageSize".to_string(), page_size.to_string()),
    ];
    if request.category_ids.is_empty() {
        push_query(&mut query, "categoryId", request.category_id);
    } else if request.category_ids.len() == 1 {
        // Single-category requests are more widely compatible as categoryId.
        push_query(
            &mut query,
            "categoryId",
            request.category_ids.first().copied(),
        );
    } else {
        let category_ids = request
            .category_ids
            .iter()
            .copied()
            .take(10)
            .collect::<Vec<_>>();
        query.push((
            "categoryIds".to_string(),
            serde_json::to_string(&category_ids)?,
        ));
    }
    push_query(&mut query, "searchFilter", request.search_filter);
    push_query(&mut query, "gameVersion", request.game_version);
    push_query(&mut query, "modLoaderType", request.mod_loader_type);
    push_query(&mut query, "sortField", request.sort_field);
    push_query(&mut query, "sortOrder", request.sort_order);

    let response: CurseForgeResponse<Vec<CurseForgeProject>> = request_json(
        Method::GET,
        "/v1/mods/search",
        query,
        None,
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    let pagination = response.pagination.unwrap_or(CurseForgePagination {
        index: request.index,
        page_size,
        result_count: response.data.len() as u32,
        total_count: response.data.len() as u32,
    });

    Ok(UnifiedSearchResponse {
        provider: ContentProvider::CurseForge,
        hits: response
            .data
            .into_iter()
            .map(UnifiedSearchHit::from)
            .collect(),
        offset: pagination.index,
        limit: pagination.page_size,
        total_hits: pagination.total_count,
    })
}

pub async fn get_project(project_id: u32) -> crate::Result<CurseForgeProject> {
    let state = State::get().await?;
    CachedEntry::get_curseforge_project(
        &CurseForgeProjectId::new(project_id)?,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        ErrorKind::OtherError(format!(
            "CurseForge project {project_id} was not found"
        ))
        .as_error()
    })
}

pub async fn get_projects(
    project_ids: Vec<u32>,
) -> crate::Result<Vec<CurseForgeProject>> {
    let project_ids = project_ids
        .into_iter()
        .map(CurseForgeProjectId::new)
        .collect::<crate::Result<Vec<_>>>()?;
    let state = State::get().await?;
    CachedEntry::get_curseforge_project_many(
        &project_ids,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await
}

pub(crate) async fn get_projects_uncached(
    project_ids: Vec<u32>,
) -> crate::Result<Vec<CurseForgeProject>> {
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut projects = Vec::new();
    for project_ids in project_ids.chunks(MAX_PAGE_SIZE as usize) {
        let response: CurseForgeResponse<Vec<CurseForgeProject>> =
            request_json(
                Method::POST,
                "/v1/mods",
                Vec::new(),
                Some(json!({
                    "modIds": project_ids,
                    "filterPcOnly": true
                })),
                MirrorPolicy::MirrorFirst,
            )
            .await?;
        projects.extend(response.data);
    }
    Ok(projects)
}

pub async fn get_description(project_id: u32) -> crate::Result<String> {
    let response: CurseForgeResponse<String> = request_json(
        Method::GET,
        &format!("/v1/mods/{project_id}/description"),
        Vec::new(),
        None,
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    Ok(response.data)
}

/// Fetches every page of a CurseForge project's files and returns the complete
/// file list, since the API caps each page at `MAX_PAGE_SIZE` entries.
pub async fn get_files(
    project_id: u32,
    request: CurseForgeFilesRequest,
) -> crate::Result<CurseForgeFilesResponse> {
    let page_size = request.page_size.clamp(1, MAX_PAGE_SIZE);
    let mut files = Vec::new();
    let mut index = 0u32;
    let mut total_count;

    loop {
        let mut query = vec![
            ("index".to_string(), index.to_string()),
            ("pageSize".to_string(), page_size.to_string()),
        ];
        push_query(&mut query, "gameVersion", request.game_version.clone());
        push_query(&mut query, "modLoaderType", request.mod_loader_type);
        push_query(
            &mut query,
            "gameVersionTypeId",
            request.game_version_type_id,
        );

        let response: CurseForgeResponse<Vec<CurseForgeFile>> = request_json(
            Method::GET,
            &format!("/v1/mods/{project_id}/files"),
            query,
            None,
            MirrorPolicy::MirrorFirst,
        )
        .await?;
        let pagination = response.pagination.unwrap_or(CurseForgePagination {
            index,
            page_size,
            result_count: response.data.len() as u32,
            total_count: response.data.len() as u32,
        });
        total_count = pagination.total_count;
        files.extend(response.data);

        if files.len() as u32 >= total_count || pagination.result_count == 0 {
            break;
        }
        index += pagination.result_count;
    }

    let result_count = files.len() as u32;
    Ok(CurseForgeFilesResponse {
        files,
        pagination: CurseForgePagination {
            index: request.index,
            page_size,
            result_count,
            total_count,
        },
    })
}

pub async fn get_file(
    project_id: u32,
    file_id: u32,
) -> crate::Result<CurseForgeFile> {
    let response: CurseForgeResponse<CurseForgeFile> = request_json(
        Method::GET,
        &format!("/v1/mods/{project_id}/files/{file_id}"),
        Vec::new(),
        None,
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    Ok(response.data)
}

pub async fn get_files_many(
    file_ids: Vec<u32>,
) -> crate::Result<Vec<CurseForgeFile>> {
    let response: CurseForgeResponse<Vec<CurseForgeFile>> = request_json(
        Method::POST,
        "/v1/mods/files",
        Vec::new(),
        Some(json!({ "fileIds": file_ids })),
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    Ok(response.data)
}

pub async fn get_changelog(
    project_id: u32,
    file_id: u32,
) -> crate::Result<String> {
    let response: CurseForgeResponse<String> = request_json(
        Method::GET,
        &format!("/v1/mods/{project_id}/files/{file_id}/changelog"),
        Vec::new(),
        None,
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    Ok(response.data)
}

pub async fn get_download_url(
    project_id: u32,
    file_id: u32,
) -> crate::Result<Option<String>> {
    let path = format!("/v1/mods/{project_id}/files/{file_id}/download-url");
    let response: CurseForgeResponse<Option<String>> = request_json(
        Method::GET,
        &path,
        Vec::new(),
        None,
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    if let Some(url) = normalized_download_url(response.data) {
        return Ok(Some(url));
    }
    if api_key().is_none() {
        return Ok(None);
    }

    let response: CurseForgeResponse<Option<String>> = request_json(
        Method::GET,
        &path,
        Vec::new(),
        None,
        MirrorPolicy::OfficialOnly,
    )
    .await?;
    Ok(normalized_download_url(response.data))
}

fn normalized_download_url(url: Option<String>) -> Option<String> {
    url.and_then(|url| {
        let url = url.trim();
        (!url.is_empty()).then(|| url.to_string())
    })
}

pub async fn get_categories(
    class_id: Option<u32>,
) -> crate::Result<Vec<CurseForgeCategory>> {
    let cached = CATEGORY_CACHE.read().ok().and_then(|cache| cache.clone());
    let categories = if let Some(categories) = cached {
        categories
    } else {
        let response: CurseForgeResponse<Vec<CurseForgeCategory>> =
            request_json(
                Method::GET,
                "/v1/categories",
                vec![("gameId".to_string(), MINECRAFT_GAME_ID.to_string())],
                None,
                MirrorPolicy::MirrorFirst,
            )
            .await?;

        if let Ok(mut cache) = CATEGORY_CACHE.write() {
            *cache = Some(response.data.clone());
        }
        response.data
    };

    Ok(filter_categories(categories, class_id))
}

pub async fn match_fingerprints(
    fingerprints: Vec<u64>,
) -> crate::Result<CurseForgeFingerprintResult> {
    let response: CurseForgeResponse<CurseForgeFingerprintResult> =
        request_json(
            Method::POST,
            &format!("/v1/fingerprints/{MINECRAFT_GAME_ID}"),
            Vec::new(),
            Some(json!({ "fingerprints": fingerprints })),
            MirrorPolicy::MirrorFirst,
        )
        .await?;
    Ok(response.data)
}

pub async fn install_file(
    request: CurseForgeInstallRequest,
) -> crate::Result<CurseForgeInstallResult> {
    install_file_with_metrics(request, None).await
}

pub async fn install_file_with_reporter(
    request: CurseForgeInstallRequest,
    reporter: InstallProgressReporter,
) -> crate::Result<CurseForgeInstallResult> {
    let metrics = CurseForgeDownloadMetrics::with_reporter(reporter.clone());
    let result = install_file_with_metrics(request, Some(&metrics)).await?;
    metrics.finish(&reporter).await?;
    Ok(result)
}

async fn install_file_with_metrics(
    request: CurseForgeInstallRequest,
    download_metrics: Option<&CurseForgeDownloadMetrics>,
) -> crate::Result<CurseForgeInstallResult> {
    let project_type = managed_project_type(&request.project_type)?;
    let mut result = CurseForgeInstallResult::default();
    let mut visited = HashSet::new();
    let mut projects = HashMap::<u32, CurseForgeProject>::new();
    let mut pending =
        vec![(request.project_id, request.file_id, project_type, false)];

    while let Some((project_id, file_id, item_type, dependency)) = pending.pop()
    {
        if !visited.insert((project_id, file_id)) {
            continue;
        }

        let file = get_file(project_id, file_id).await?;
        let project = match projects.get(&project_id) {
            Some(project) => project.clone(),
            None => {
                let project = get_project(project_id).await?;
                projects.insert(project_id, project.clone());
                project
            }
        };
        if request.install_dependencies {
            for dependency_ref in &file.dependencies {
                match dependency_ref.relation_type {
                    DEPENDENCY_RELATION_OPTIONAL => {
                        result.optional_dependencies.push(dependency_ref.mod_id)
                    }
                    DEPENDENCY_RELATION_INCOMPATIBLE => result
                        .incompatible_dependencies
                        .push(dependency_ref.mod_id),
                    DEPENDENCY_RELATION_REQUIRED
                    | DEPENDENCY_RELATION_INCLUDE => {
                        let dependency_project = match projects
                            .get(&dependency_ref.mod_id)
                        {
                            Some(project) => project.clone(),
                            None => {
                                let project =
                                    get_project(dependency_ref.mod_id).await?;
                                projects.insert(
                                    dependency_ref.mod_id,
                                    project.clone(),
                                );
                                project
                            }
                        };
                        let Some(dependency_type) = recognized_project_type(
                            dependency_project.class_id,
                        ) else {
                            result.failed_downloads.push(
                                CurseForgeFailedDownload {
                                    project_id: dependency_ref.mod_id,
                                    file_id: 0,
                                    file_name: dependency_project.name.clone(),
                                    reason:
                                        "The dependency project type is not supported"
                                            .to_string(),
                                },
                            );
                            continue;
                        };
                        if let Some(dependency_file) = select_dependency_file(
                            dependency_ref.mod_id,
                            request.game_version.clone(),
                            request.mod_loader_type,
                        )
                        .await?
                        {
                            pending.push((
                                dependency_ref.mod_id,
                                dependency_file.id,
                                dependency_type,
                                true,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        let download_url = if project.allow_mod_distribution == Some(false) {
            None
        } else {
            match file.download_url.clone() {
                Some(url) => Some(url),
                None => get_download_url(project_id, file_id).await?,
            }
        };
        let Some(download_url) = download_url else {
            let target_folder = content_target_folder(
                item_type,
                request.world_name.as_deref(),
            )?;
            let manual_download = manual_download_from_file(
                project_id,
                file_id,
                &file,
                &project,
                item_type.get_name(),
                target_folder,
                request.ownership_kind,
                request.manual_operation_kind,
            );
            persist_manual_download(&request.instance_id, &manual_download)
                .await?;
            result.manual_downloads.push(manual_download);
            continue;
        };

        validate_file_name(&file.file_name)?;
        let relative_path = download_installed_file(
            &request.instance_id,
            &download_url,
            &file,
            item_type,
            request.world_name.as_deref(),
            project_id,
            file_id,
            &project.slug,
            request.ownership_kind,
            download_metrics,
        )
        .await?;
        result.installed.push(CurseForgeInstalledFile {
            project_id,
            file_id,
            relative_path,
            dependency,
        });
    }

    result.optional_dependencies.sort_unstable();
    result.optional_dependencies.dedup();
    result.incompatible_dependencies.sort_unstable();
    result.incompatible_dependencies.dedup();
    Ok(result)
}

pub async fn install_modpack(
    request: CurseForgeModpackInstallRequest,
) -> crate::Result<CurseForgeModpackInstallResult> {
    install_modpack_with_reporter(request, None).await
}

pub async fn get_modpack_target(
    project_id: u32,
    file_id: u32,
) -> crate::Result<CurseForgeModpackTarget> {
    let pack_file = get_file(project_id, file_id).await?;
    let project = get_project(project_id).await?;
    let download_url = if project.allow_mod_distribution == Some(false) {
        None
    } else {
        match pack_file.download_url.clone() {
            Some(url) => Some(url),
            None => get_download_url(project_id, file_id).await?,
        }
    }
    .ok_or_else(|| {
        ErrorKind::InputError(
            "The CurseForge modpack manifest cannot be downloaded automatically"
                .to_string(),
        )
    })?;

    let icon_url = project.logo.as_ref().and_then(|logo| {
        if !logo.thumbnail_url.is_empty() {
            Some(logo.thumbnail_url.clone())
        } else if !logo.url.is_empty() {
            Some(logo.url.clone())
        } else {
            None
        }
    });
    let loading_bar = init_loading(
        LoadingBarType::PackFileDownload {
            instance_id: String::new(),
            pack_name: project.name.clone(),
            icon: icon_url,
            pack_version: pack_file.display_name.clone(),
        },
        pack_file.file_length.max(1) as f64,
        &format!("Downloading {}", pack_file.file_name),
    )
    .await?;
    let mut last_downloaded = 0_u64;
    let mut progress = |current: u64,
                        _total: u64|
     -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<()>> + Send>,
    > {
        let delta = current.saturating_sub(last_downloaded);
        last_downloaded = current;
        let result = emit_loading(
            &loading_bar,
            delta as f64,
            Some("Downloading CurseForge modpack"),
        );
        Box::pin(async move { result })
    };
    let pack_download = download_curseforge_archive(
        project_id,
        file_id,
        &pack_file,
        &download_url,
        Some(&mut progress as &mut FetchProgressFn<'_>),
        None,
    )
    .await?;
    let pack_path = pack_download.path;
    let target = tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&pack_path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(modpack_zip_error)?;
        let manifest = read_modpack_manifest(&mut archive)?;
        modpack_target(&manifest)
    })
    .await??;
    Ok(target)
}

pub async fn install_modpack_with_reporter(
    request: CurseForgeModpackInstallRequest,
    reporter: Option<InstallProgressReporter>,
) -> crate::Result<CurseForgeModpackInstallResult> {
    let pack_file = get_file(request.project_id, request.file_id).await?;
    let project = get_project(request.project_id).await?;
    let icon_url = project
        .logo
        .as_ref()
        .map(|logo| {
            if !logo.thumbnail_url.is_empty() {
                logo.thumbnail_url.clone()
            } else {
                logo.url.clone()
            }
        })
        .filter(|url| !url.is_empty());
    let download_url = if project.allow_mod_distribution == Some(false) {
        None
    } else {
        match pack_file.download_url.clone() {
            Some(url) => Some(url),
            None => {
                get_download_url(request.project_id, request.file_id).await?
            }
        }
    };
    let Some(download_url) = download_url else {
        let manual_download = CurseForgeManualDownload {
			project_id: request.project_id,
			file_id: request.file_id,
			file_name: pack_file.file_name.clone(),
			ownership_kind: crate::state::instances::ContentOwnershipKind::PackManaged,
			operation_kind: crate::state::instances::ManualDownloadOperationKind::PackInstall,
			website_url: curseforge_file_page_url(
				project.links.website_url.as_deref(),
				request.file_id,
			),
			project_type: "modpack".to_string(),
			project_slug: project.slug.clone(),
			target_folder: String::new(),
			hashes: pack_file.hashes.clone(),
			file_length: pack_file.file_length,
			file_fingerprint: pack_file.file_fingerprint,
		};
        persist_manual_download(&request.instance_id, &manual_download).await?;
        return Ok(CurseForgeModpackInstallResult {
            content: CurseForgeInstallResult {
                manual_downloads: vec![manual_download],
                ..Default::default()
            },
            ..Default::default()
        });
    };

    let cached_icon_path = if let Some(icon_url) = icon_url.as_ref() {
        match cache_instance_icon_from_url(icon_url).await {
            Ok(path) => {
                let _ = crate::api::instance::edit_icon(
                    &request.instance_id,
                    Some(path.as_path()),
                )
                .await;
                Some(path)
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to cache CurseForge modpack icon: {err}"
                );
                None
            }
        }
    } else {
        None
    };

    let pack_details = InstallPhaseDetails::Modpack {
        project_id: Some(request.project_id.to_string()),
        version_id: Some(request.file_id.to_string()),
        title: Some(project.name.clone()),
    };
    if let Some(reporter) = reporter.as_ref() {
        reporter
            .update(
                InstallPhaseId::DownloadingPackFile,
                Some(InstallProgress {
                    current: 0,
                    total: pack_file.file_length.max(1),
                    secondary: None,
                }),
                pack_details.clone(),
            )
            .await?;
    }
    let mut last_downloaded = 0_u64;
    let progress_reporter = reporter.clone();
    let progress_details = pack_details.clone();
    let mut progress = move |current: u64,
                             total: u64|
          -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<()>> + Send>,
    > {
        let min_delta = (total / 200).max(256 * 1024);
        if current < total
            && current.saturating_sub(last_downloaded) < min_delta
        {
            return Box::pin(async { Ok(()) });
        }
        last_downloaded = current;
        let reporter = progress_reporter.clone();
        let details = progress_details.clone();
        Box::pin(async move {
            if let Some(reporter) = reporter {
                reporter
                    .update(
                        InstallPhaseId::DownloadingPackFile,
                        Some(InstallProgress {
                            current,
                            total,
                            secondary: None,
                        }),
                        details,
                    )
                    .await?;
            }
            Ok(())
        })
    };
    let progress = reporter
        .is_some()
        .then_some(&mut progress as &mut FetchProgressFn<'_>);
    let pack_download = download_curseforge_archive(
        request.project_id,
        request.file_id,
        &pack_file,
        &download_url,
        progress,
        reporter.as_ref(),
    )
    .await?;
    if let Some(reporter) = reporter.as_ref()
        && pack_download.attempts > 0
    {
        reporter
            .record_download_metrics(
                pack_download.source.as_str(),
                pack_download.fallback_count as u64,
            )
            .await?;
    }
    let pack_path = pack_download.path;
    if let Some(reporter) = reporter.as_ref() {
        reporter
            .update(
                InstallPhaseId::DownloadingPackFile,
                Some(InstallProgress {
                    current: pack_file.file_length,
                    total: pack_file.file_length.max(1),
                    secondary: None,
                }),
                pack_details.clone(),
            )
            .await?;
        reporter
            .update(
                InstallPhaseId::ReadingPackManifest,
                None,
                pack_details.clone(),
            )
            .await?;
    }
    let pack_path_for_manifest = pack_path.clone();
    let manifest = tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&pack_path_for_manifest)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(modpack_zip_error)?;
        read_modpack_manifest(&mut archive)
    })
    .await??;

    let state = State::get().await?;
    use sqlx::Row;
    let instance_target = sqlx::query(
        "SELECT content_set.game_version, content_set.loader
         FROM instances instance
         INNER JOIN instance_content_sets content_set
            ON content_set.id = instance.applied_content_set_id
         WHERE instance.id = ?",
    )
    .bind(&request.instance_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ErrorKind::InputError(
            "The selected instance has no active Minecraft installation"
                .to_string(),
        )
    })?;
    let instance_game_version =
        instance_target.try_get::<String, _>("game_version")?;
    let instance_loader = instance_target.try_get::<String, _>("loader")?;
    let target = modpack_target(&manifest)?;
    let loader = (target.loader != ModLoader::Vanilla)
        .then(|| target.loader.as_str().to_string());
    if instance_game_version != manifest.minecraft.version
        || target.loader.as_str() != instance_loader
    {
        if !request.allow_target_change {
            return Err(ErrorKind::InputError(format!(
				"This modpack targets Minecraft {} with {}, while the selected instance uses {} with {}",
				manifest.minecraft.version,
				loader.as_deref().unwrap_or("vanilla"),
				instance_game_version,
				instance_loader
			))
			.into());
        }
    }

    let content_set = crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
			&request.instance_id,
			&state.pool,
		)
		.await?
		.ok_or_else(|| {
			ErrorKind::InputError(
				"Instance has no applied content set".to_string(),
			)
		})?;
    let pack_members = crate::state::instances::adapters::sqlite::content_rows::get_pack_members(
			&content_set.id,
			&state.pool,
		)
		.await?;
    let preserved_pack_projects = pack_members
        .iter()
        .filter(|member| {
            matches!(
                member.override_kind,
                crate::state::instances::PackMemberOverrideKind::Version
                    | crate::state::instances::PackMemberOverrideKind::Removed
            )
        })
        .filter_map(|member| member.provider_project_id.clone())
        .collect::<HashSet<_>>();
    let disabled_pack_projects = pack_members
        .iter()
        .filter(|member| {
            member.override_kind
                == crate::state::instances::PackMemberOverrideKind::Disabled
        })
        .filter_map(|member| member.provider_project_id.clone())
        .collect::<HashSet<_>>();
    let installed_pack_releases = pack_members
        .iter()
        .filter(|member| {
            member.materialization_state
				== crate::state::instances::PackMemberMaterializationState::Present
        })
        .filter_map(|member| {
            Some((
                member.provider_project_id.clone()?,
                member.provider_release_id.clone()?,
            ))
        })
        .collect::<HashSet<_>>();
    let selected_files = manifest
        .files
        .into_iter()
        .filter(|file| file.required || request.install_optional)
        .filter(|file| {
            !preserved_pack_projects.contains(&file.project_id.to_string())
        })
        .filter(|file| {
            !installed_pack_releases.contains(&(
                file.project_id.to_string(),
                file.file_id.to_string(),
            ))
        })
        .collect::<Vec<_>>();
    let loader_type_value = loader.as_deref().and_then(loader_type);
    let project_ids = selected_files
        .iter()
        .map(|file| file.project_id)
        .collect::<Vec<_>>();
    let mut projects = HashMap::new();
    for project_ids in project_ids.chunks(50) {
        for project in get_projects(project_ids.to_vec()).await? {
            projects.insert(project.id, project);
        }
    }
    for project_id in &project_ids {
        if !projects.contains_key(project_id) {
            let project = get_project(*project_id).await?;
            projects.insert(project.id, project);
        }
    }

    let instance_name = crate::api::instance::get(&request.instance_id)
        .await?
        .map(|metadata| metadata.instance.name)
        .unwrap_or_else(|| project.name.clone());
    let total_files = selected_files.len().max(1);
    let mut file_ids = selected_files
        .iter()
        .map(|file| file.file_id)
        .collect::<Vec<_>>();
    file_ids.sort_unstable();
    file_ids.dedup();
    let mut file_meta = HashMap::<u32, CurseForgeFile>::new();
    for chunk in file_ids.chunks(50) {
        for file in get_files_many(chunk.to_vec()).await? {
            file_meta.insert(file.id, file);
        }
    }
    let content_total_bytes = selected_files
        .iter()
        .map(|file| {
            file_meta
                .get(&file.file_id)
                .map(|meta| meta.file_length)
                .unwrap_or(0)
        })
        .sum::<u64>();
    // Keep the LoadingBarId in an Arc. LoadingBarId::Drop removes the bar, so
    // cloning the ID itself would destroy progress as soon as the first task
    // finished. Arc clones only share ownership.
    let loading_bar = if reporter.is_none() {
        Some(Arc::new(
            init_loading(
                LoadingBarType::PackDownload {
                    instance_id: request.instance_id.clone(),
                    pack_name: project.name.clone(),
                    icon: cached_icon_path.clone(),
                    pack_id: Some(request.project_id.to_string()),
                    pack_version: Some(request.file_id.to_string()),
                },
                total_files as f64,
                &format!("Downloading {instance_name}"),
            )
            .await?,
        ))
    } else {
        None
    };
    if let Some(loading_bar) = loading_bar.as_ref() {
        let _ = emit_loading(
            loading_bar.as_ref(),
            0.0,
            Some(&format!(
                "0/{total_files} files · 0 / {}",
                format_bytes(content_total_bytes)
            )),
        );
    }
    if let Some(reporter) = reporter.as_ref() {
        reporter
            .update_with_events(
                InstallPhaseId::DownloadingContent,
                Some(InstallProgress {
                    current: 0,
                    total: total_files as u64,
                    secondary: Some(InstallProgressSecondary {
                        current: 0,
                        total: content_total_bytes,
                    }),
                }),
                pack_details.clone(),
                vec![InstallJobEventKind::ContentDownloadStarted {
                    files: total_files as u64,
                    bytes: Some(content_total_bytes),
                }],
            )
            .await?;
    }

    tracing::info!(
        selected_manifest_files = selected_files.len(),
        "Resolved CurseForge modpack manifest files"
    );
    let content = Arc::new(Mutex::new(CurseForgeInstallResult::default()));
    let download_metrics = reporter.as_ref().map(|reporter| {
        Arc::new(CurseForgeDownloadMetrics::with_reporter(reporter.clone()))
    });
    let projects = Arc::new(projects);
    let file_meta = Arc::new(file_meta);
    let files_done = Arc::new(AtomicU64::new(0));
    let bytes_done = Arc::new(AtomicU64::new(0));
    let active_downloads = Arc::new(AtomicU64::new(0));
    let instance_id = request.instance_id.clone();
    let minecraft_version = manifest.minecraft.version.clone();

    loading_try_for_each_concurrent(
        stream::iter(selected_files.into_iter().map(Ok::<_, crate::Error>)),
        Some(state.download_concurrency()),
        // Progress is updated manually with file+byte counts below.
        None,
        1.0,
        total_files,
        None,
        |manifest_file| {
            let content = content.clone();
            let projects = projects.clone();
            let file_meta = file_meta.clone();
            let files_done = files_done.clone();
            let bytes_done = bytes_done.clone();
            let active_downloads = active_downloads.clone();
            let loading_bar = loading_bar.clone();
            let reporter = reporter.clone();
            let download_metrics = download_metrics.clone();
            let pack_details = pack_details.clone();
            let instance_id = instance_id.clone();
            let minecraft_version = minecraft_version.clone();
            async move {
                let expected_bytes = file_meta
                    .get(&manifest_file.file_id)
                    .map(|file| file.file_length)
                    .unwrap_or(0);
                let project = projects
                    .get(&manifest_file.project_id)
                    .ok_or_else(|| {
                        ErrorKind::OtherError(format!(
                            "CurseForge project metadata is missing for {}",
                            manifest_file.project_id
                        ))
                    })?;
                let project_type = project_type_for_class(project.class_id);
                managed_project_type(project_type)?;

                active_downloads.fetch_add(1, Ordering::Relaxed);
                let mut installed_result = None;
                let mut failed_result = None;
                let mut failure_reason = "no file was installed".to_string();
                for attempt in 1..=MODPACK_FILE_INSTALL_ATTEMPTS {
                    match install_file_with_metrics(
                        CurseForgeInstallRequest {
                            instance_id: instance_id.clone(),
                            project_id: manifest_file.project_id,
                            file_id: manifest_file.file_id,
							project_type: project_type.to_string(),
							ownership_kind: crate::state::instances::ContentOwnershipKind::PackManaged,
							manual_operation_kind: if request.allow_target_change {
								crate::state::instances::ManualDownloadOperationKind::PackUpdate
							} else {
								crate::state::instances::ManualDownloadOperationKind::PackInstall
							},
                            game_version: Some(minecraft_version.clone()),
                            mod_loader_type: loader_type_value,
                            world_name: None,
                            install_dependencies: false,
                        },
                        download_metrics.as_deref(),
                    )
                    .await
                    {
                        Ok(item_result)
                            if !item_result.installed.is_empty() =>
                        {
                            installed_result = Some(item_result);
                            break;
                        }
                        Ok(item_result) => {
                            failure_reason = item_result
                                .manual_downloads
                                .first()
                                .map(|file| {
                                    format!(
                                        "{} requires manual download",
                                        file.file_name
                                    )
                                })
                                .unwrap_or_else(|| {
                                    "no file was installed".to_string()
                                });
                            let manual_download_required =
                                !item_result.manual_downloads.is_empty();
                            failed_result = Some(item_result);
                            if manual_download_required {
                                break;
                            }
                        }
                        Err(err) => {
                            failure_reason = err.to_string();
                        }
                    }
                    tracing::warn!(
                        project_id = manifest_file.project_id,
                        file_id = manifest_file.file_id,
                        attempt,
                        max_attempts = MODPACK_FILE_INSTALL_ATTEMPTS,
                        reason = %failure_reason,
                        "Failed to install required CurseForge file"
                    );
                    if attempt < MODPACK_FILE_INSTALL_ATTEMPTS {
                        tokio::time::sleep(Duration::from_millis(
                            250 * attempt as u64,
                        ))
                        .await;
                    }
                }

                let Some(item_result) = installed_result else {
                    active_downloads.fetch_sub(1, Ordering::Relaxed);
                    let mut failed_result = failed_result.unwrap_or_default();
                    let file_name = file_meta
                        .get(&manifest_file.file_id)
                        .map(|file| file.file_name.clone())
                        .unwrap_or_else(|| {
                            format!(
                                "project-{}-file-{}",
                                manifest_file.project_id,
                                manifest_file.file_id
                            )
                        });
                    let manual_download =
                        failed_result.manual_downloads.first().cloned();
                    let event = if let Some(manual_download) =
                        manual_download.as_ref()
                    {
                        InstallJobEventKind::ContentFileSkipped {
                            path: manual_download.file_name.clone(),
                            reason: "CurseForge requires manual download"
                                .to_string(),
                            project_id: Some(
                                manifest_file.project_id.to_string(),
                            ),
                            version_id: Some(manifest_file.file_id.to_string()),
                            manual_url: manual_download.website_url.clone(),
                        }
                    } else {
                        let reason = format!(
                            "Failed after {MODPACK_FILE_INSTALL_ATTEMPTS} attempts: {failure_reason}"
                        );
                        failed_result.failed_downloads.push(
                            CurseForgeFailedDownload {
                                project_id: manifest_file.project_id,
                                file_id: manifest_file.file_id,
                                file_name: file_name.clone(),
                                reason: reason.clone(),
                            },
                        );
                        InstallJobEventKind::ContentFileFailed {
                            path: file_name,
                            reason,
                            project_id: Some(
                                manifest_file.project_id.to_string(),
                            ),
                            version_id: Some(manifest_file.file_id.to_string()),
                        }
                    };
                    {
                        let mut content =
                            content.lock().expect("content mutex");
                        merge_install_result(&mut content, failed_result);
                    }
                    report_modpack_progress(
                        loading_bar.as_deref(),
                        reporter.as_ref(),
                        pack_details,
                        &files_done,
                        &bytes_done,
                        &active_downloads,
                        total_files as u64,
                        content_total_bytes,
                        0,
                        event,
                    )
                    .await?;
                    return Ok(());
                };
                let completed_path =
                    item_result.installed[0].relative_path.clone();
                {
                    let mut content = content.lock().expect("content mutex");
                    merge_install_result(&mut content, item_result);
                }
                active_downloads.fetch_sub(1, Ordering::Relaxed);
                report_modpack_progress(
                    loading_bar.as_deref(),
                    reporter.as_ref(),
                    pack_details,
                    &files_done,
                    &bytes_done,
                    &active_downloads,
                    total_files as u64,
                    content_total_bytes,
                    expected_bytes,
                    InstallJobEventKind::ContentFileCompleted {
                        path: completed_path,
                        bytes: expected_bytes,
                    },
                )
                .await?;
                Ok(())
            }
        },
    )
    .await?;

    if let (Some(reporter), Some(download_metrics)) =
        (reporter.as_ref(), download_metrics.as_ref())
    {
        download_metrics.finish(reporter).await?;
    }

    let content = Arc::try_unwrap(content)
        .map_err(|_| {
            ErrorKind::OtherError(
                "CurseForge install state was still shared after completion"
                    .to_string(),
            )
        })?
        .into_inner()
        .map_err(|_| {
            ErrorKind::OtherError(
                "CurseForge install state mutex was poisoned".to_string(),
            )
        })?;
    if request.allow_target_change {
        for installed in &content.installed {
            if disabled_pack_projects
                .contains(&installed.project_id.to_string())
                && !installed.relative_path.ends_with(".disabled")
            {
                crate::state::instances::commands::toggle_disable_project(
                    &request.instance_id,
                    &installed.relative_path,
                    Some(false),
                    &state,
                )
                .await?;
            }
        }
    }

    let instance_path =
        crate::api::instance::get_full_path(&request.instance_id).await?;
    if let Some(reporter) = reporter.as_ref() {
        reporter
            .update(InstallPhaseId::ExtractingOverrides, None, pack_details)
            .await?;
    }
    let update_ready = content.manual_downloads.is_empty()
        && content.failed_downloads.is_empty();
    let should_commit = content.manual_downloads.is_empty()
        && (!request.allow_target_change || update_ready);
    let overrides_written = if should_commit {
        crate::api::pack::archive_util::run_blocking_instance_write(
            request.instance_id.clone(),
            reporter
                .as_ref()
                .map(InstallProgressReporter::cancellation_token)
                .unwrap_or_default(),
            move |cancellation| {
                extract_modpack_overrides(
                    &pack_path,
                    &instance_path,
                    Some(cancellation),
                )
            },
        )
        .await?
    } else {
        0
    };
    if should_commit && !request.allow_target_change {
        crate::api::instance::edit(
            &request.instance_id,
            EditInstance {
                name: (!request.allow_target_change)
                    .then_some(project.name.clone()),
                icon_path: (!request.allow_target_change).then(|| {
                    cached_icon_path
                        .as_ref()
                        .map(|path| path.to_string_lossy().to_string())
                }),
                link: Some(InstanceLink::CurseForgeModpack {
                    project_id: request.project_id.to_string(),
                    version_id: request.file_id.to_string(),
                }),
                content_set_patch: Some(crate::state::AppliedContentSetPatch {
                    source_kind: Some(ContentSourceKind::CurseForge),
                    game_version: Some(manifest.minecraft.version.clone()),
                    protocol_version: Some(None),
                    loader: Some(target.loader),
                    loader_version: Some(target.loader_version.clone()),
                }),
                ..EditInstance::default()
            },
        )
        .await?;
        let content_set = crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
			&request.instance_id,
			&state.pool,
		)
		.await?
		.ok_or_else(|| {
			ErrorKind::InputError(
				"Instance has no applied content set".to_string(),
			)
		})?;
        crate::state::sync_content_files(&request.instance_id, &state).await?;
        match get_modpack_expected_members(request.project_id, request.file_id)
            .await
        {
            Ok(expected) => {
                crate::state::instances::commands::reconcile_curseforge_members(
					&request.instance_id,
					&content_set.id,
					&expected,
					&state,
				)
				.await?;
            }
            Err(error) => tracing::warn!(
                "Unable to persist the complete CurseForge pack manifest: {error}"
            ),
        }
    }
    Ok(CurseForgeModpackInstallResult {
        content,
        overrides_written,
        minecraft_version: manifest.minecraft.version,
        loader,
    })
}

/// Installs a CurseForge modpack from a local archive on disk (a zip with a
/// `manifest.json`), downloading the listed files through the CurseForge API
/// and extracting the overrides folder. Unlike [`install_modpack_with_reporter`]
/// this needs no project/file id — undownloadable files land on the manual
/// download list exactly like API-driven installs.
pub async fn install_modpack_from_local_archive_with_reporter(
    instance_id: String,
    archive_path: std::path::PathBuf,
    _base_folder: String,
    source_filename: Option<String>,
    install_optional: bool,
    reporter: InstallProgressReporter,
    completion_policy: crate::launcher::InstanceCompletionPolicy,
) -> crate::Result<CurseForgeModpackInstallResult> {
    let manifest_archive_path = archive_path.clone();
    let manifest = tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&manifest_archive_path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(modpack_zip_error)?;
        read_modpack_manifest(&mut archive)
    })
    .await??;

    let target = modpack_target(&manifest)?;
    let loader = (target.loader != ModLoader::Vanilla)
        .then(|| target.loader.as_str().to_string());
    let pack_name = manifest
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            source_filename.as_ref().map(|name| {
                std::path::Path::new(name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
        })
        .unwrap_or_else(|| "CurseForge Modpack".to_string());
    let pack_details = InstallPhaseDetails::Modpack {
        project_id: None,
        version_id: None,
        title: Some(pack_name.clone()),
    };
    reporter
        .update(InstallPhaseId::ResolvingPack, None, pack_details.clone())
        .await?;

    let loader_version = if target.loader != ModLoader::Vanilla {
        crate::launcher::get_loader_version_from_profile(
            &manifest.minecraft.version,
            target.loader,
            target.loader_version.as_deref(),
        )
        .await?
    } else {
        None
    };

    crate::api::instance::edit(
        &instance_id,
        EditInstance {
            install_stage: Some(
                crate::state::InstanceInstallStage::PackInstalling,
            ),
            name: Some(pack_name.clone()),
            link: Some(InstanceLink::ImportedModpack {
                project_id: None,
                version_id: None,
                name: Some(pack_name.clone()),
                version_number: manifest.version.clone(),
                filename: source_filename,
            }),
            content_set_patch: Some(crate::state::AppliedContentSetPatch {
                source_kind: Some(ContentSourceKind::CurseForge),
                game_version: Some(manifest.minecraft.version.clone()),
                protocol_version: Some(None),
                loader: Some(target.loader),
                loader_version: Some(loader_version.map(|version| version.id)),
            }),
            ..EditInstance::default()
        },
    )
    .await?;

    let content = install_local_manifest_files(
        &instance_id,
        manifest.files.clone(),
        install_optional,
        &manifest.minecraft.version,
        loader.as_deref(),
        pack_details.clone(),
        &reporter,
    )
    .await?;

    if !content.manual_downloads.is_empty() {
        return Ok(CurseForgeModpackInstallResult {
            content,
            overrides_written: 0,
            minecraft_version: manifest.minecraft.version,
            loader,
        });
    }

    reporter
        .update(
            InstallPhaseId::ExtractingOverrides,
            None,
            pack_details.clone(),
        )
        .await?;
    let instance_path =
        crate::api::instance::get_full_path(&instance_id).await?;
    let overrides_archive_path = archive_path.clone();
    let overrides_written =
        crate::api::pack::archive_util::run_blocking_instance_write(
            instance_id.clone(),
            reporter.cancellation_token(),
            move |cancellation| {
                extract_modpack_overrides(
                    &overrides_archive_path,
                    &instance_path,
                    Some(cancellation),
                )
            },
        )
        .await?;

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        &instance_id,
        false,
        Some(reporter.clone()),
        completion_policy,
    )
    .await?;
    reporter.clear_context().await?;

    Ok(CurseForgeModpackInstallResult {
        content,
        overrides_written,
        minecraft_version: manifest.minecraft.version,
        loader,
    })
}

/// Downloads and installs the files listed in a local CurseForge manifest.
/// Mirrors the file loop of [`install_modpack_with_reporter`], simplified to
/// the job-reporter path used by local imports. Also used by the MCBBS
/// installer for its CurseForge-style `files` array.
pub(crate) async fn install_local_manifest_files(
    instance_id: &str,
    manifest_files: Vec<CurseForgeManifestFile>,
    install_optional: bool,
    minecraft_version: &str,
    loader: Option<&str>,
    pack_details: InstallPhaseDetails,
    reporter: &InstallProgressReporter,
) -> crate::Result<CurseForgeInstallResult> {
    let state = State::get().await?;
    let content_set = crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
		instance_id,
		&state.pool,
	)
	.await?
	.ok_or_else(|| {
		ErrorKind::InputError(
			"Instance has no applied content set".to_string(),
		)
	})?;
    let installed_releases = crate::state::instances::adapters::sqlite::content_rows::get_pack_members(
		&content_set.id,
		&state.pool,
	)
	.await?
	.into_iter()
	.filter(|member| {
		member.materialization_state
			== crate::state::instances::PackMemberMaterializationState::Present
	})
	.filter_map(|member| {
		Some((member.provider_project_id?, member.provider_release_id?))
	})
	.collect::<HashSet<_>>();
    let selected_files = manifest_files
        .into_iter()
        .filter(|file| file.required || install_optional)
        .filter(|file| {
            !installed_releases.contains(&(
                file.project_id.to_string(),
                file.file_id.to_string(),
            ))
        })
        .collect::<Vec<_>>();
    let loader_type_value = loader.and_then(loader_type);
    let project_ids = selected_files
        .iter()
        .map(|file| file.project_id)
        .collect::<Vec<_>>();
    let mut projects = HashMap::new();
    for project_ids in project_ids.chunks(50) {
        for project in get_projects(project_ids.to_vec()).await? {
            projects.insert(project.id, project);
        }
    }
    for project_id in &project_ids {
        if !projects.contains_key(project_id) {
            let project = get_project(*project_id).await?;
            projects.insert(project.id, project);
        }
    }

    let total_files = selected_files.len().max(1);
    let mut file_ids = selected_files
        .iter()
        .map(|file| file.file_id)
        .collect::<Vec<_>>();
    file_ids.sort_unstable();
    file_ids.dedup();
    let mut file_meta = HashMap::<u32, CurseForgeFile>::new();
    for chunk in file_ids.chunks(50) {
        for file in get_files_many(chunk.to_vec()).await? {
            file_meta.insert(file.id, file);
        }
    }
    let content_total_bytes = selected_files
        .iter()
        .map(|file| {
            file_meta
                .get(&file.file_id)
                .map(|meta| meta.file_length)
                .unwrap_or(0)
        })
        .sum::<u64>();
    reporter
        .update_with_events(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: 0,
                total: total_files as u64,
                secondary: Some(InstallProgressSecondary {
                    current: 0,
                    total: content_total_bytes,
                }),
            }),
            pack_details.clone(),
            vec![InstallJobEventKind::ContentDownloadStarted {
                files: total_files as u64,
                bytes: Some(content_total_bytes),
            }],
        )
        .await?;

    tracing::info!(
        selected_manifest_files = selected_files.len(),
        "Resolved local CurseForge modpack manifest files"
    );
    let content = Arc::new(Mutex::new(CurseForgeInstallResult::default()));
    let download_metrics =
        Arc::new(CurseForgeDownloadMetrics::with_reporter(reporter.clone()));
    let projects = Arc::new(projects);
    let file_meta = Arc::new(file_meta);
    let files_done = Arc::new(AtomicU64::new(0));
    let bytes_done = Arc::new(AtomicU64::new(0));
    let active_downloads = Arc::new(AtomicU64::new(0));

    loading_try_for_each_concurrent(
        stream::iter(selected_files.into_iter().map(Ok::<_, crate::Error>)),
        Some(state.download_concurrency()),
        None,
        1.0,
        total_files,
        None,
        |manifest_file| {
            let content = content.clone();
            let projects = projects.clone();
            let file_meta = file_meta.clone();
            let files_done = files_done.clone();
            let bytes_done = bytes_done.clone();
            let active_downloads = active_downloads.clone();
            let reporter = reporter.clone();
            let download_metrics = download_metrics.clone();
            let pack_details = pack_details.clone();
            let instance_id = instance_id.to_string();
            let minecraft_version = minecraft_version.to_string();
            async move {
                let expected_bytes = file_meta
                    .get(&manifest_file.file_id)
                    .map(|file| file.file_length)
                    .unwrap_or(0);
                let project = projects
                    .get(&manifest_file.project_id)
                    .ok_or_else(|| {
                        ErrorKind::OtherError(format!(
                            "CurseForge project metadata is missing for {}",
                            manifest_file.project_id
                        ))
                    })?;
                let project_type = project_type_for_class(project.class_id);
                managed_project_type(project_type)?;

                active_downloads.fetch_add(1, Ordering::Relaxed);
                let mut installed_result = None;
                let mut failed_result = None;
                let mut failure_reason = "no file was installed".to_string();
                for attempt in 1..=MODPACK_FILE_INSTALL_ATTEMPTS {
                    match install_file_with_metrics(
                        CurseForgeInstallRequest {
                            instance_id: instance_id.clone(),
                            project_id: manifest_file.project_id,
                            file_id: manifest_file.file_id,
							project_type: project_type.to_string(),
							ownership_kind: crate::state::instances::ContentOwnershipKind::PackManaged,
                            manual_operation_kind: crate::state::instances::ManualDownloadOperationKind::PackUpdate,
                            game_version: Some(minecraft_version.clone()),
                            mod_loader_type: loader_type_value,
                            world_name: None,
                            install_dependencies: false,
                        },
                        Some(&download_metrics),
                    )
                    .await
                    {
                        Ok(item_result)
                            if !item_result.installed.is_empty() =>
                        {
                            installed_result = Some(item_result);
                            break;
                        }
                        Ok(item_result) => {
                            failure_reason = item_result
                                .manual_downloads
                                .first()
                                .map(|file| {
                                    format!(
                                        "{} requires manual download",
                                        file.file_name
                                    )
                                })
                                .unwrap_or_else(|| {
                                    "no file was installed".to_string()
                                });
                            let manual_download_required =
                                !item_result.manual_downloads.is_empty();
                            failed_result = Some(item_result);
                            if manual_download_required {
                                break;
                            }
                        }
                        Err(err) => {
                            failure_reason = err.to_string();
                        }
                    }
                    tracing::warn!(
                        project_id = manifest_file.project_id,
                        file_id = manifest_file.file_id,
                        attempt,
                        max_attempts = MODPACK_FILE_INSTALL_ATTEMPTS,
                        reason = %failure_reason,
                        "Failed to install required CurseForge file"
                    );
                    if attempt < MODPACK_FILE_INSTALL_ATTEMPTS {
                        tokio::time::sleep(Duration::from_millis(
                            250 * attempt as u64,
                        ))
                        .await;
                    }
                }

                let Some(item_result) = installed_result else {
                    active_downloads.fetch_sub(1, Ordering::Relaxed);
                    let mut failed_result = failed_result.unwrap_or_default();
                    let file_name = file_meta
                        .get(&manifest_file.file_id)
                        .map(|file| file.file_name.clone())
                        .unwrap_or_else(|| {
                            format!(
                                "project-{}-file-{}",
                                manifest_file.project_id,
                                manifest_file.file_id
                            )
                        });
                    let manual_download =
                        failed_result.manual_downloads.first().cloned();
                    let event = if let Some(manual_download) =
                        manual_download.as_ref()
                    {
                        InstallJobEventKind::ContentFileSkipped {
                            path: manual_download.file_name.clone(),
                            reason: "CurseForge requires manual download"
                                .to_string(),
                            project_id: Some(
                                manifest_file.project_id.to_string(),
                            ),
                            version_id: Some(manifest_file.file_id.to_string()),
                            manual_url: manual_download.website_url.clone(),
                        }
                    } else {
                        let reason = format!(
                            "Failed after {MODPACK_FILE_INSTALL_ATTEMPTS} attempts: {failure_reason}"
                        );
                        failed_result.failed_downloads.push(
                            CurseForgeFailedDownload {
                                project_id: manifest_file.project_id,
                                file_id: manifest_file.file_id,
                                file_name: file_name.clone(),
                                reason: reason.clone(),
                            },
                        );
                        InstallJobEventKind::ContentFileFailed {
                            path: file_name,
                            reason,
                            project_id: Some(
                                manifest_file.project_id.to_string(),
                            ),
                            version_id: Some(manifest_file.file_id.to_string()),
                        }
                    };
                    {
                        let mut content =
                            content.lock().expect("content mutex");
                        merge_install_result(&mut content, failed_result);
                    }
                    report_modpack_progress(
                        None,
                        Some(&reporter),
                        pack_details,
                        &files_done,
                        &bytes_done,
                        &active_downloads,
                        total_files as u64,
                        content_total_bytes,
                        0,
                        event,
                    )
                    .await?;
                    return Ok(());
                };
                let completed_path =
                    item_result.installed[0].relative_path.clone();
                {
                    let mut content = content.lock().expect("content mutex");
                    merge_install_result(&mut content, item_result);
                }
                active_downloads.fetch_sub(1, Ordering::Relaxed);
                report_modpack_progress(
                    None,
                    Some(&reporter),
                    pack_details,
                    &files_done,
                    &bytes_done,
                    &active_downloads,
                    total_files as u64,
                    content_total_bytes,
                    expected_bytes,
                    InstallJobEventKind::ContentFileCompleted {
                        path: completed_path,
                        bytes: expected_bytes,
                    },
                )
                .await?;
                Ok(())
            }
        },
    )
    .await?;

    download_metrics.finish(reporter).await?;

    Arc::try_unwrap(content)
        .map_err(|_| {
            ErrorKind::OtherError(
                "CurseForge install state was still shared after completion"
                    .to_string(),
            )
        })?
        .into_inner()
        .map_err(|_| {
            ErrorKind::OtherError(
                "CurseForge install state mutex was poisoned".to_string(),
            )
        })
        .map_err(Into::into)
}

pub async fn update_managed_modpack(
    instance_id: &str,
    file_id: u32,
) -> crate::Result<CurseForgeModpackInstallResult> {
    let state = State::get().await?;
    let metadata = crate::state::instances::commands::get_instance_metadata(
        instance_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| ErrorKind::InputError("Unknown instance".to_string()))?;
    let project_id = match &metadata.link {
        InstanceLink::CurseForgeModpack { project_id, .. } => {
            project_id.parse::<u32>().map_err(|_| {
                ErrorKind::InputError(
                    "Linked CurseForge project ID is invalid".to_string(),
                )
            })?
        }
        _ => {
            return Err(ErrorKind::InputError(format!(
                "Instance {instance_id} is not a managed CurseForge pack, or has been disconnected."
            ))
            .into());
        }
    };
    let members = crate::state::instances::adapters::sqlite::content_rows::get_pack_members(
		&metadata.applied_content_set.id,
		&state.pool,
	)
	.await?;
    if members.iter().any(|member| !member.reconciled) {
        return Err(ErrorKind::InputError(
			"CurseForge pack membership is not calibrated yet; refresh the content page while online before updating"
				.to_string(),
		)
		.into());
    }
    let expected = get_modpack_expected_members(project_id, file_id).await?;
    let pack_file = get_file(project_id, file_id).await?;
    let game_version = pack_file
        .game_versions
        .iter()
        .find(|value| loader_type(value).is_none())
        .cloned()
        .unwrap_or_else(|| metadata.applied_content_set.game_version.clone());
    let loader = pack_file
        .game_versions
        .iter()
        .find_map(|value| {
            loader_type(value).map(|_| value.to_ascii_lowercase())
        })
        .or_else(|| {
            Some(metadata.applied_content_set.loader.as_str().to_string())
        });
    for member in members.iter().filter(|member| {
        member.override_kind
            == crate::state::instances::PackMemberOverrideKind::Version
    }) {
        let (Some(member_project_id), Some(member_file_id)) = (
            member.provider_project_id.as_deref(),
            member.provider_release_id.as_deref(),
        ) else {
            continue;
        };
        let override_file = get_file(
            member_project_id.parse().map_err(|_| {
                ErrorKind::InputError(
                    "Stored CurseForge project ID is invalid".to_string(),
                )
            })?,
            member_file_id.parse().map_err(|_| {
                ErrorKind::InputError(
                    "Stored CurseForge file ID is invalid".to_string(),
                )
            })?,
        )
        .await?;
        if !override_file
            .game_versions
            .iter()
            .any(|version| version == &game_version)
        {
            return Err(ErrorKind::InputError(format!(
				"Pack member {} has a local version override that is not compatible with Minecraft {game_version}; restore the pack default before updating",
				member.expected_relative_path
			))
			.into());
        }
    }
    let installed_releases = members
        .iter()
        .filter(|member| {
            member.materialization_state
				== crate::state::instances::PackMemberMaterializationState::Present
        })
        .filter_map(|member| {
            Some((
                member.provider_project_id.clone()?,
                member.provider_release_id.clone()?,
            ))
        })
        .collect::<HashSet<_>>();
    let manual_downloads = expected
        .members
        .iter()
        .filter(|member| member.required)
        .filter(|member| {
            !installed_releases.contains(&(
                member.project_id.to_string(),
                member.file_id.to_string(),
            ))
        })
        .filter_map(|member| member.manual_download.clone())
        .collect::<Vec<_>>();
    if !manual_downloads.is_empty() {
        for download in &manual_downloads {
            persist_manual_download(instance_id, download).await?;
        }
        return Ok(CurseForgeModpackInstallResult {
            content: CurseForgeInstallResult {
                manual_downloads,
                ..CurseForgeInstallResult::default()
            },
            overrides_written: 0,
            minecraft_version: game_version,
            loader,
        });
    }

    let entries = crate::state::instances::adapters::sqlite::content_rows::get_content_entries(
		&metadata.applied_content_set.id,
		&state.pool,
	)
	.await?
	.into_iter()
	.filter_map(|entry| entry.file_id.map(|file_id| (entry.id, file_id)))
	.collect::<HashMap<_, _>>();
    let files = crate::state::instances::adapters::sqlite::content_rows::get_instance_files(
		instance_id,
		&state.pool,
	)
	.await?
	.into_iter()
	.map(|file| (file.id.clone(), file.relative_path))
	.collect::<HashMap<_, _>>();
    let backup = create_curseforge_update_backup(
        &metadata, &members, &entries, &files, &state,
    )
    .await?;
    let result = match install_modpack(CurseForgeModpackInstallRequest {
        instance_id: instance_id.to_string(),
        project_id,
        file_id,
        install_optional: false,
        allow_target_change: true,
    })
    .await
    {
        Ok(result) => result,
        Err(error) => {
            rollback_curseforge_update(
                instance_id,
                &metadata,
                &members,
                &backup,
                &[],
                &state,
            )
            .await?;
            return Err(error);
        }
    };
    if !result.content.manual_downloads.is_empty()
        || !result.content.failed_downloads.is_empty()
    {
        rollback_curseforge_update(
            instance_id,
            &metadata,
            &members,
            &backup,
            &result.content.installed,
            &state,
        )
        .await?;
        return Ok(result);
    }

    let expected_keys = expected
        .members
        .iter()
        .map(|member| {
            format!(
                "curseforge:{}:{}",
                member.project_id,
                member.project_type.get_name()
            )
        })
        .collect::<HashSet<_>>();
    for member in &members {
        if expected_keys.contains(&member.member_key)
            || member.override_kind
                == crate::state::instances::PackMemberOverrideKind::Version
        {
            continue;
        }
        let Some(entry_id) = member.content_entry_id.as_ref() else {
            continue;
        };
        let Some(file_id) = entries.get(entry_id) else {
            continue;
        };
        let Some(relative_path) = files.get(file_id) else {
            continue;
        };
        crate::state::instances::commands::remove_project(
            instance_id,
            relative_path,
            &state,
        )
        .await?;
    }
    crate::state::sync_content_files(instance_id, &state).await?;
    crate::state::instances::commands::reconcile_curseforge_members(
        instance_id,
        &metadata.applied_content_set.id,
        &expected,
        &state,
    )
    .await?;
    crate::api::instance::edit(
        instance_id,
        EditInstance {
            link: Some(InstanceLink::CurseForgeModpack {
                project_id: project_id.to_string(),
                version_id: file_id.to_string(),
            }),
            content_set_patch: Some(crate::state::AppliedContentSetPatch {
                source_kind: Some(ContentSourceKind::CurseForge),
                game_version: Some(game_version),
                protocol_version: Some(None),
                loader: loader
                    .as_deref()
                    .map(crate::data::ModLoader::from_string),
                loader_version: Some(None),
            }),
            ..EditInstance::default()
        },
    )
    .await?;
    Ok(result)
}

struct CurseForgeUpdateBackup {
    _directory: tempfile::TempDir,
    files: Vec<(String, String, std::path::PathBuf)>,
}

async fn create_curseforge_update_backup(
    metadata: &crate::state::InstanceMetadata,
    members: &[crate::state::instances::PackMember],
    entries: &HashMap<String, String>,
    files: &HashMap<String, String>,
    state: &State,
) -> crate::Result<CurseForgeUpdateBackup> {
    crate::util::io::create_dir_all(&state.directories.caches_dir()).await?;
    let directory = tempfile::Builder::new()
        .prefix("curseforge-pack-update-")
        .tempdir_in(state.directories.caches_dir())?;
    let instance_path = state
        .directories
        .instances_dir()
        .join(&metadata.instance.path);
    let mut backups = Vec::new();
    for member in members {
        let Some(entry_id) = member.content_entry_id.as_ref() else {
            continue;
        };
        let Some(file_id) = entries.get(entry_id) else {
            continue;
        };
        let Some(relative_path) = files.get(file_id) else {
            continue;
        };
        let source = instance_path.join(relative_path);
        if !source.is_file() {
            continue;
        }
        let backup_path = directory
            .path()
            .join(format!("{}.backup", uuid::Uuid::new_v4()));
        tokio::fs::copy(&source, &backup_path).await?;
        backups.push((
            member.member_key.clone(),
            relative_path.clone(),
            backup_path,
        ));
    }
    Ok(CurseForgeUpdateBackup {
        _directory: directory,
        files: backups,
    })
}

async fn rollback_curseforge_update(
    instance_id: &str,
    metadata: &crate::state::InstanceMetadata,
    old_members: &[crate::state::instances::PackMember],
    backup: &CurseForgeUpdateBackup,
    installed: &[CurseForgeInstalledFile],
    state: &State,
) -> crate::Result<()> {
    let instance_path = state
        .directories
        .instances_dir()
        .join(&metadata.instance.path);
    let old_paths = backup
        .files
        .iter()
        .map(|(_, relative_path, _)| relative_path.as_str())
        .collect::<HashSet<_>>();
    for installed_file in installed {
        if old_paths.contains(installed_file.relative_path.as_str()) {
            continue;
        }
        crate::state::instances::commands::remove_project(
            instance_id,
            &installed_file.relative_path,
            state,
        )
        .await?;
    }
    for (member_key, relative_path, backup_path) in &backup.files {
        let destination = instance_path.join(relative_path);
        if let Some(parent) = destination.parent() {
            crate::util::io::create_dir_all(parent).await?;
        }
        tokio::fs::copy(backup_path, &destination).await?;
        let Some(member) = old_members
            .iter()
            .find(|member| member.member_key == *member_key)
        else {
            continue;
        };
        let (Some(project_id), Some(file_id)) = (
            member.provider_project_id.as_deref(),
            member.provider_release_id.as_deref(),
        ) else {
            continue;
        };
        let provider_ref = ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(project_id.parse().map_err(
                |_| {
                    ErrorKind::InputError(
                        "Stored CurseForge project ID is invalid".to_string(),
                    )
                },
            )?)?,
            file_id: Some(CurseForgeFileId::new(file_id.parse().map_err(
                |_| {
                    ErrorKind::InputError(
                        "Stored CurseForge file ID is invalid".to_string(),
                    )
                },
            )?)?),
        };
        let (size, sha1) = sha1_file_async(&destination).await?;
        crate::state::record_project_file_atomic(
            instance_id,
            relative_path,
            &sha1,
            size,
            member.project_type,
            ContentSourceKind::CurseForge,
            crate::state::instances::ContentOwnershipKind::PackManaged,
            Some(&provider_ref),
            true,
            None,
            state,
        )
        .await?;
    }

    let current_members = crate::state::instances::adapters::sqlite::content_rows::get_pack_members(
		&metadata.applied_content_set.id,
		&state.pool,
	)
	.await?;
    let _instance_lock = state.lock_instance_content(instance_id).await;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old_keys = old_members
        .iter()
        .map(|member| member.member_key.as_str())
        .collect::<HashSet<_>>();
    for member in current_members {
        if !old_keys.contains(member.member_key.as_str()) {
            sqlx::query("DELETE FROM instance_pack_members WHERE id = ?")
                .bind(member.id)
                .execute(&mut *tx)
                .await?;
        }
    }
    for old_member in old_members {
        let current_entry_id = sqlx::query_scalar::<_, Option<String>>(
            "SELECT content_entry_id FROM instance_pack_members
			 WHERE content_set_id = ? AND member_key = ?",
        )
        .bind(&metadata.applied_content_set.id)
        .bind(&old_member.member_key)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        let mut restored_member = old_member.clone();
        if current_entry_id.is_some() {
            restored_member.content_entry_id = current_entry_id;
        }
        crate::state::instances::adapters::sqlite::content_rows::upsert_pack_member_in_transaction(
			&restored_member,
			&mut tx,
		)
		.await?;
    }
    crate::state::instances::adapters::sqlite::content_rows::bump_content_set_revision_in_transaction(
		&metadata.applied_content_set.id,
		&mut tx,
	)
	.await?;
    tx.commit().await?;
    Ok(())
}

async fn cache_instance_icon_from_url(
    icon_url: &str,
) -> crate::Result<std::path::PathBuf> {
    let state = State::get().await?;
    // CurseForge avatar/CDN assets are frequently broken via local system
    // proxies, so always download icons with a direct client.
    let permit = state.fetch_semaphore.0.acquire().await?;
    let response = CLIENT.get(icon_url).send().await?;
    drop(permit);
    if !response.status().is_success() {
        return Err(ErrorKind::OtherError(format!(
            "CurseForge icon download failed with HTTP {}",
            response.status().as_u16()
        ))
        .into());
    }
    let icon_bytes = response.bytes().await?;
    let filename = icon_url.rsplit('/').next().unwrap_or("icon.png");
    crate::util::fetch::write_cached_icon(
        filename,
        &state.directories.caches_dir(),
        icon_bytes,
        &state.io_semaphore,
    )
    .await
}

fn extract_modpack_overrides(
    archive_path: &Path,
    instance_path: &Path,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
) -> crate::Result<u32> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(modpack_zip_error)?;
    let manifest = read_modpack_manifest(&mut archive)?;
    let prefix = format!("{}/", manifest.overrides.trim_matches('/'));
    let mut files_written = 0_u32;
    let mut total_size = 0_u64;
    for index in 0..archive.len() {
        crate::api::pack::archive_util::check_cancellation(cancellation)?;
        let mut entry = archive.by_index(index).map_err(modpack_zip_error)?;
        let entry_name =
            crate::pack::detect::decode_zip_entry_name(entry.name_raw());
        if entry.is_dir() || !entry_name.starts_with(&prefix) {
            continue;
        }
        let relative = &entry_name[prefix.len()..];
        let safe_path = safe_archive_relative_path(relative)?;
        total_size = total_size.saturating_add(entry.size());
        if total_size > 2 * 1024 * 1024 * 1024 {
            return Err(ErrorKind::InputError(
                "CurseForge modpack overrides exceed the extraction limit"
                    .to_string(),
            )
            .into());
        }
        let target = instance_path.join(safe_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut output = std::fs::File::create(&target)?;
        let written = crate::api::pack::archive_util::copy_with_cancellation(
            &mut entry,
            &mut output,
            cancellation,
            &target,
        )?;
        if written != entry.size() {
            return Err(ErrorKind::InputError(
                "CurseForge modpack override was truncated during extraction"
                    .to_string(),
            )
            .into());
        }
        files_written = files_written.checked_add(1).ok_or_else(|| {
            ErrorKind::InputError(
                "CurseForge modpack contains too many override files"
                    .to_string(),
            )
        })?;
    }
    Ok(files_written)
}

fn read_modpack_manifest<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> crate::Result<CurseForgeModpackManifest> {
    let mut entry = archive.by_name("manifest.json").map_err(|_| {
        ErrorKind::InputError(
            "CurseForge modpack is missing manifest.json".to_string(),
        )
    })?;
    let mut json = String::new();
    entry.read_to_string(&mut json)?;
    Ok(serde_json::from_str::<CurseForgeModpackManifest>(&json)?)
}

fn read_modpack_override_content<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    manifest: &CurseForgeModpackManifest,
) -> crate::Result<Vec<CurseForgePackExpectedOverride>> {
    let prefix = format!("{}/", manifest.overrides.trim_matches('/'));
    let mut overrides = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(modpack_zip_error)?;
        let entry_name =
            crate::pack::detect::decode_zip_entry_name(entry.name_raw());
        if entry.is_dir() || !entry_name.starts_with(&prefix) {
            continue;
        }
        let Some(override_content) =
            curseforge_override_content(&entry_name[prefix.len()..])?
        else {
            continue;
        };
        overrides.push(override_content);
    }
    overrides.sort_by(|left, right| {
        left.expected_relative_path
            .cmp(&right.expected_relative_path)
    });
    overrides.dedup_by(|left, right| {
        left.expected_relative_path == right.expected_relative_path
    });
    Ok(overrides)
}

fn curseforge_override_content(
    relative_path: &str,
) -> crate::Result<Option<CurseForgePackExpectedOverride>> {
    let relative_path = safe_archive_relative_path(relative_path)?;
    let Some(project_type) = crate::state::instances::adapters::filesystem::project_type_from_relative_path(
        &relative_path,
    ) else {
        return Ok(None);
    };
    if !crate::state::instances::adapters::filesystem::is_scannable_project_path(
        project_type,
        &relative_path,
    ) {
        return Ok(None);
    }
    Ok(Some(CurseForgePackExpectedOverride {
        project_type,
        expected_relative_path: relative_path,
    }))
}

fn modpack_target(
    manifest: &CurseForgeModpackManifest,
) -> crate::Result<CurseForgeModpackTarget> {
    let Some(manifest_loader) = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|loader| loader.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first())
    else {
        return Ok(CurseForgeModpackTarget {
            game_version: manifest.minecraft.version.clone(),
            loader: ModLoader::Vanilla,
            loader_version: None,
        });
    };

    let family = loader_family(&manifest_loader.id);
    let loader = match family {
        "forge" => ModLoader::Forge,
        "fabric" => ModLoader::Fabric,
        "quilt" => ModLoader::Quilt,
        "neo" | "neoforge" => ModLoader::NeoForge,
        _ => {
            return Err(ErrorKind::InputError(format!(
                "CurseForge modpack uses unsupported loader {}",
                manifest_loader.id
            ))
            .into());
        }
    };
    let loader_version = manifest_loader
        .id
        .strip_prefix(family)
        .and_then(|version| version.strip_prefix('-'))
        .filter(|version| !version.is_empty())
        .map(str::to_string);

    Ok(CurseForgeModpackTarget {
        game_version: manifest.minecraft.version.clone(),
        loader,
        loader_version,
    })
}

fn modpack_zip_error(error: zip::result::ZipError) -> crate::Error {
    ErrorKind::InputError(format!(
        "CurseForge modpack archive is invalid: {error}"
    ))
    .into()
}

fn safe_archive_relative_path(value: &str) -> crate::Result<String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ErrorKind::InputError(
            "CurseForge modpack contains an invalid override path".to_string(),
        )
        .into());
    }
    Ok(path.to_string_lossy().replace('\\', "/"))
}

pub(crate) fn loader_family(loader_id: &str) -> &str {
    loader_id.split('-').next().unwrap_or(loader_id)
}

fn loader_type(loader: &str) -> Option<u32> {
    match loader {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

fn merge_install_result(
    target: &mut CurseForgeInstallResult,
    mut source: CurseForgeInstallResult,
) {
    target.installed.append(&mut source.installed);
    target.manual_downloads.append(&mut source.manual_downloads);
    target.failed_downloads.append(&mut source.failed_downloads);
    target
        .optional_dependencies
        .append(&mut source.optional_dependencies);
    target
        .incompatible_dependencies
        .append(&mut source.incompatible_dependencies);
}

pub async fn update_installed_file(
    instance_id: &str,
    relative_path: &str,
) -> crate::Result<CurseForgeInstallResult> {
    use sqlx::Row;

    let state = State::get().await?;
    let row = sqlx::query(
		"SELECT ref.provider_project_id, ref.provider_release_id, entry.project_type,
				entry.ownership_kind,
				content_set.game_version, content_set.loader,
				instance.update_channel
         FROM instance_files file
         INNER JOIN instance_content_entries entry ON entry.file_id = file.id
         INNER JOIN instance_content_provider_refs ref
            ON ref.content_entry_id = entry.id AND ref.provider = 'curseforge'
         INNER JOIN instance_content_sets content_set
            ON content_set.id = entry.content_set_id
         INNER JOIN instances instance ON instance.id = file.instance_id
         WHERE file.instance_id = ? AND file.relative_path = ?
         ORDER BY entry.modified_at DESC
         LIMIT 1",
    )
    .bind(instance_id)
    .bind(relative_path)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ErrorKind::InputError(
            "The selected file is not linked to CurseForge".to_string(),
        )
    })?;
    let project_id = row
        .try_get::<String, _>("provider_project_id")?
        .parse::<u32>()
        .map_err(|_| {
            ErrorKind::InputError(
                "Stored CurseForge project ID is invalid".to_string(),
            )
        })?;
    let current_file_id = row
        .try_get::<Option<String>, _>("provider_release_id")?
        .and_then(|value| value.parse::<u32>().ok());
    let project_type = row.try_get::<String, _>("project_type")?;
    let ownership_kind =
        crate::state::instances::ContentOwnershipKind::from_str(
            &row.try_get::<String, _>("ownership_kind")?,
        )?;
    let game_version = row.try_get::<String, _>("game_version")?;
    let loader = row.try_get::<String, _>("loader")?;
    let update_channel = row
        .try_get::<Option<String>, _>("update_channel")?
        .as_deref()
        .map(ReleaseChannel::from_key)
        .unwrap_or(ReleaseChannel::Release);
    let mod_loader_type = match loader.as_str() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    };
    let latest = select_latest_compatible_file(
        project_id,
        Some(game_version.clone()),
        mod_loader_type,
        Some(update_channel),
    )
    .await?
    .ok_or_else(|| {
        ErrorKind::InputError(
            "No compatible CurseForge update was found".to_string(),
        )
    })?;
    if current_file_id == Some(latest.id) {
        return Ok(CurseForgeInstallResult::default());
    }

    install_selected_file(
        instance_id,
        relative_path,
        project_id,
        latest.id,
        project_type,
        ownership_kind,
        game_version,
        mod_loader_type,
    )
    .await
}

pub async fn switch_installed_file_version(
    instance_id: &str,
    relative_path: &str,
    file_id: u32,
) -> crate::Result<CurseForgeInstallResult> {
    use sqlx::Row;

    let state = State::get().await?;
    let row = sqlx::query(
		"SELECT ref.provider_project_id, ref.provider_release_id, entry.project_type,
				entry.ownership_kind,
				content_set.game_version, content_set.loader
         FROM instance_files file
         INNER JOIN instance_content_entries entry ON entry.file_id = file.id
         INNER JOIN instance_content_provider_refs ref
            ON ref.content_entry_id = entry.id AND ref.provider = 'curseforge'
         INNER JOIN instance_content_sets content_set
            ON content_set.id = entry.content_set_id
         WHERE file.instance_id = ? AND file.relative_path = ?
         ORDER BY entry.modified_at DESC
         LIMIT 1",
    )
    .bind(instance_id)
    .bind(relative_path)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ErrorKind::InputError(
            "The selected file is not linked to CurseForge".to_string(),
        )
    })?;
    let project_id = row
        .try_get::<String, _>("provider_project_id")?
        .parse::<u32>()
        .map_err(|_| {
            ErrorKind::InputError(
                "Stored CurseForge project ID is invalid".to_string(),
            )
        })?;
    let current_file_id = row
        .try_get::<Option<String>, _>("provider_release_id")?
        .and_then(|value| value.parse::<u32>().ok());
    let project_type = row.try_get::<String, _>("project_type")?;
    let ownership_kind =
        crate::state::instances::ContentOwnershipKind::from_str(
            &row.try_get::<String, _>("ownership_kind")?,
        )?;
    let game_version = row.try_get::<String, _>("game_version")?;
    let loader = row.try_get::<String, _>("loader")?;
    let mod_loader_type = match loader.as_str() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    };

    if current_file_id == Some(file_id) {
        return Ok(CurseForgeInstallResult::default());
    }

    install_selected_file(
        instance_id,
        relative_path,
        project_id,
        file_id,
        project_type,
        ownership_kind,
        game_version,
        mod_loader_type,
    )
    .await
}

async fn install_selected_file(
    instance_id: &str,
    relative_path: &str,
    project_id: u32,
    file_id: u32,
    project_type: String,
    ownership_kind: crate::state::instances::ContentOwnershipKind,
    game_version: String,
    mod_loader_type: Option<u32>,
) -> crate::Result<CurseForgeInstallResult> {
    let result = install_file(CurseForgeInstallRequest {
        instance_id: instance_id.to_string(),
        project_id,
        file_id,
        project_type,
        ownership_kind,
        manual_operation_kind:
            crate::state::instances::ManualDownloadOperationKind::ContentUpdate,
        game_version: Some(game_version),
        mod_loader_type,
        world_name: None,
        install_dependencies: true,
    })
    .await?;
    if ownership_kind
        == crate::state::instances::ContentOwnershipKind::PackManaged
        && let Some(installed) = result
            .installed
            .iter()
            .find(|file| !file.dependency && file.project_id == project_id)
    {
        mark_pack_member_version_override(
            instance_id,
            &installed.relative_path,
        )
        .await?;
    }
    if let Some(new_path) = result
        .installed
        .iter()
        .find(|file| {
            !file.dependency
                && file.project_id == project_id
                && file.relative_path != relative_path
        })
        .map(|file| file.relative_path.clone())
    {
        let state = State::get().await?;
        if crate::state::instances::commands::archive_project_file(
            instance_id,
            relative_path,
            &new_path,
            &state,
        )
        .await?
        .is_none()
        {
            crate::api::instance::remove_project(instance_id, relative_path)
                .await?;
        }
    }
    Ok(result)
}

pub async fn recognize_instance_files(
    instance_id: &str,
) -> crate::Result<CurseForgeRecognitionResult> {
    let mut result = CurseForgeRecognitionResult::default();
    if capability().status != CurseForgeCapabilityStatus::Ready {
        return Ok(result);
    }

    let state = State::get().await?;
    let instance_files =
        crate::api::instance::sync_content_files(instance_id).await?;
    let tracked_file_ids = match crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
        instance_id,
        &state.pool,
    )
    .await?
    {
        Some(content_set) => {
            crate::state::instances::adapters::sqlite::content_rows::get_content_entries(
                &content_set.id,
                &state.pool,
            )
            .await?
            .into_iter()
            .filter_map(|entry| entry.file_id)
            .collect::<HashSet<_>>()
        }
        None => HashSet::new(),
    };
    let instance_path =
        crate::api::instance::get_full_path(instance_id).await?;
    let mut fingerprints = Vec::new();
    let mut paths_by_fingerprint =
        HashMap::<u64, Vec<(String, ProjectType)>>::new();

    for file in instance_files.into_iter().filter(|file| !file.missing) {
        if tracked_file_ids.contains(&file.id) {
            continue;
        }
        let Some(project_type) =
            crate::state::instances::adapters::filesystem::project_type_from_relative_path(
                &file.relative_path,
            )
        else {
            continue;
        };
        let full_path = instance_path.join(&file.relative_path);
        let Ok(bytes) = tokio::fs::read(&full_path).await else {
            continue;
        };
        let fingerprint = compute_fingerprint(&bytes) as u64;
        fingerprints.push(fingerprint);
        paths_by_fingerprint
            .entry(fingerprint)
            .or_default()
            .push((file.relative_path, project_type));
    }

    let mut matches = HashMap::new();
    for chunk in fingerprints.chunks(1000) {
        let response = match_fingerprints(chunk.to_vec()).await?;
        for matched in response.exact_matches {
            matches.insert(matched.file.file_fingerprint, matched.file);
        }
    }

    result.scanned = fingerprints.len() as u32;
    let matched_project_ids =
        matches.values().map(|file| file.mod_id).collect::<Vec<_>>();
    let matched_projects = if matched_project_ids.is_empty() {
        HashMap::new()
    } else {
        match get_projects(matched_project_ids).await {
            Ok(projects) => projects
                .into_iter()
                .map(|project| (project.id, project))
                .collect(),
            Err(error) => {
                tracing::warn!(
                    %error,
                    "Failed to resolve CurseForge project types while recognizing instance files"
                );
                HashMap::new()
            }
        }
    };

    for (fingerprint, paths) in paths_by_fingerprint {
        if let Some(file) = matches.get(&fingerprint) {
            for (path, path_project_type) in paths {
                let project_type = matched_projects
                    .get(&file.mod_id)
                    .and_then(|project| {
                        recognized_project_type(project.class_id)
                    })
                    .unwrap_or(path_project_type);
                let full_path = instance_path.join(&path);
                let Some((size, sha1)) = verify_recognized_curseforge_file(
                    &full_path,
                    file.file_fingerprint,
                )
                .await?
                else {
                    continue;
                };
                crate::state::record_verified_curseforge_project_file_atomic(
                    instance_id,
                    &path,
                    &sha1,
                    size,
                    project_type,
                    ContentSourceKind::CurseForge,
                    crate::state::instances::ContentOwnershipKind::UserAdded,
                    CurseForgeProjectId::new(file.mod_id)?,
                    CurseForgeFileId::new(file.id)?,
                    true,
                    &state,
                )
                .await?;
                result.linked.push(CurseForgeInstalledFile {
                    project_id: file.mod_id,
                    file_id: file.id,
                    relative_path: path,
                    dependency: false,
                });
                result.matched += 1;
            }
        } else {
            result
                .unmatched_paths
                .extend(paths.into_iter().map(|(path, _)| path));
        }
    }
    result.unmatched_paths.sort();
    Ok(result)
}

async fn verify_recognized_curseforge_file(
    path: &Path,
    expected_fingerprint: u64,
) -> crate::Result<Option<(u64, String)>> {
    if expected_fingerprint == 0 {
        return Ok(None);
    }
    let bytes = tokio::fs::read(path).await?;
    if compute_fingerprint(&bytes) as u64 != expected_fingerprint {
        return Ok(None);
    }
    Ok(Some((
        bytes.len() as u64,
        sha1_smol::Sha1::from(&bytes).hexdigest(),
    )))
}

pub async fn import_manual_downloads(
    instance_id: &str,
    scan_directory: Option<PathBuf>,
) -> crate::Result<CurseForgeManualDownloadScanResult> {
    let Some(download_directory) = scan_directory.or_else(dirs::download_dir)
    else {
        finalize_curseforge_manual_download_import(instance_id, false).await?;
        return Ok(CurseForgeManualDownloadScanResult::default());
    };
    let mut result = CurseForgeManualDownloadScanResult {
        download_directory: Some(download_directory.to_string_lossy().into()),
        ..Default::default()
    };
    let downloads = list_pending_manual_downloads(instance_id).await?;

    for download in downloads {
        let candidate = match find_manual_download_candidate(
            &download_directory,
            &download,
        )
        .await
        {
            Ok(candidate) => candidate,
            Err(error) => {
                result.errors.push(CurseForgeManualDownloadImportError {
                    project_id: download.project_id,
                    file_id: download.file_id,
                    message: error.to_string(),
                });
                continue;
            }
        };
        let Some((source_path, size, sha1)) = candidate else {
            continue;
        };

        match install_manual_download(
            instance_id,
            &download,
            &source_path,
            size,
            &sha1,
        )
        .await
        {
            Ok(relative_path) => {
                result.imported.push(CurseForgeManualDownloadImport {
                    project_id: download.project_id,
                    file_id: download.file_id,
                    relative_path,
                });
            }
            Err(error) => {
                result.errors.push(CurseForgeManualDownloadImportError {
                    project_id: download.project_id,
                    file_id: download.file_id,
                    message: error.to_string(),
                });
            }
        }
    }

    finalize_curseforge_manual_download_import(
        instance_id,
        !result.imported.is_empty(),
    )
    .await?;

    Ok(result)
}

pub async fn configure_manual_download_watcher(
    enabled: bool,
    scan_directory: Option<PathBuf>,
) -> crate::Result<Option<String>> {
    let state = State::get().await?;
    let directory = if enabled {
        let Some(directory) = scan_directory.or_else(dirs::download_dir) else {
            state
                .file_watcher
                .configure_manual_import_directory(None)
                .await?;
            return Ok(None);
        };
        let directory = tokio::fs::canonicalize(directory).await?;
        if !tokio::fs::metadata(&directory).await?.is_dir() {
            return Err(ErrorKind::InputError(
                "Manual import watch path is not a directory".to_string(),
            )
            .into());
        }
        Some(directory)
    } else {
        None
    };

    state
        .file_watcher
        .configure_manual_import_directory(directory.clone())
        .await?;
    Ok(directory.map(|path| path.to_string_lossy().into_owned()))
}

pub(crate) async fn scan_pending_manual_downloads_in(
    download_directory: &Path,
) -> crate::Result<()> {
    let _scan_guard = MANUAL_IMPORT_SCAN_LOCK.lock().await;
    let state = State::get().await?;
    let instance_ids = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT instance_id
         FROM instance_pending_manual_downloads
         WHERE provider = 'curseforge'
            AND state IN ('waiting', 'matched', 'error')",
    )
    .fetch_all(&state.pool)
    .await?;

    for instance_id in instance_ids {
        let result = import_manual_downloads(
            &instance_id,
            Some(download_directory.to_path_buf()),
        )
        .await?;
        for error in result.errors {
            tracing::debug!(
                instance_id,
                project_id = error.project_id,
                file_id = error.file_id,
                "Pending manual download not ready: {}",
                error.message
            );
        }
    }

    let waiting_instances = crate::install::store::list(false, &state)
        .await?
        .into_iter()
        .filter(|job| {
            job.status == crate::install::InstallJobStatus::WaitingForUser
                && is_curseforge_manual_download_job(&job.state)
        })
        .filter_map(|job| job.instance_id)
        .collect::<HashSet<_>>();
    for instance_id in waiting_instances {
        reconcile_curseforge_waiting_jobs_for_instance(&instance_id).await?;
    }

    Ok(())
}

pub async fn list_pending_manual_downloads(
    instance_id: &str,
) -> crate::Result<Vec<CurseForgeManualDownload>> {
    let state = State::get().await?;
    Ok(crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
		instance_id,
		&state.pool,
	)
	.await?
	.into_iter()
	.filter_map(pending_manual_download)
	.collect())
}

fn pending_manual_download(
    item: crate::state::instances::PendingManualDownload,
) -> Option<CurseForgeManualDownload> {
    if item.provider != ContentProvider::CurseForge {
        return None;
    }
    serde_json::from_value::<CurseForgeManualDownload>(item.context.clone())
        .ok()
        .or_else(|| {
            Some(CurseForgeManualDownload {
                project_id: item.provider_project_id.parse().ok()?,
                file_id: item.provider_release_id.parse().ok()?,
                file_name: item.file_name,
                ownership_kind: item
                    .pack_member_id
                    .is_some()
                    .then_some(
                        crate::state::instances::ContentOwnershipKind::PackManaged,
                    )
                    .unwrap_or_default(),
                operation_kind: item.operation_kind,
                website_url: item.website_url,
                project_type: item.project_type.get_name().to_string(),
                project_slug: String::new(),
                target_folder: Path::new(&item.target_relative_path)
                    .parent()
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default(),
                hashes: item
                    .expected_sha1
                    .map(|value| vec![CurseForgeFileHash { value, algo: 1 }])
                    .unwrap_or_default(),
                file_length: item.expected_size.unwrap_or_default(),
                file_fingerprint: item.expected_fingerprint.unwrap_or_default(),
            })
        })
}

#[derive(Clone, Debug)]
struct CurseForgeManualDownloadIntegrityMetadata {
    project_id: u32,
    file_id: u32,
    hashes: Vec<CurseForgeFileHash>,
    file_length: u64,
    file_fingerprint: u64,
}

impl From<CurseForgeFile> for CurseForgeManualDownloadIntegrityMetadata {
    fn from(file: CurseForgeFile) -> Self {
        Self {
            project_id: file.mod_id,
            file_id: file.id,
            hashes: file.hashes,
            file_length: file.file_length,
            file_fingerprint: file.file_fingerprint,
        }
    }
}

fn manual_download_has_integrity_metadata(
    download: &CurseForgeManualDownload,
) -> bool {
    download
        .hashes
        .iter()
        .any(|hash| hash.algo == 1 && !hash.value.trim().is_empty())
        || download.file_fingerprint != 0
}

async fn ensure_manual_download_integrity_metadata(
    download: &CurseForgeManualDownload,
) -> crate::Result<CurseForgeManualDownload> {
    ensure_manual_download_integrity_metadata_with(
        download,
        resolve_manual_download_integrity_metadata,
    )
    .await
}

async fn resolve_manual_download_integrity_metadata(
    project_id: u32,
    file_id: u32,
) -> crate::Result<CurseForgeManualDownloadIntegrityMetadata> {
    Ok(get_file(project_id, file_id).await?.into())
}

async fn ensure_manual_download_integrity_metadata_with<F, Fut>(
    download: &CurseForgeManualDownload,
    resolve_metadata: F,
) -> crate::Result<CurseForgeManualDownload>
where
    F: FnOnce(u32, u32) -> Fut,
    Fut: std::future::Future<
            Output = crate::Result<CurseForgeManualDownloadIntegrityMetadata>,
        >,
{
    if manual_download_has_integrity_metadata(download) {
        return Ok(download.clone());
    }

    let metadata =
        resolve_metadata(download.project_id, download.file_id).await?;
    if metadata.project_id != download.project_id
        || metadata.file_id != download.file_id
    {
        return Err(ErrorKind::InputError(
            "CurseForge returned metadata for a different project or file"
                .to_string(),
        )
        .into());
    }

    let mut hydrated = download.clone();
    hydrated.hashes = metadata.hashes;
    hydrated.file_length = metadata.file_length;
    hydrated.file_fingerprint = metadata.file_fingerprint;
    if !manual_download_has_integrity_metadata(&hydrated) {
        return Err(ErrorKind::InputError(
            "The required CurseForge file has no usable integrity metadata"
                .to_string(),
        )
        .into());
    }
    Ok(hydrated)
}

async fn find_manual_download_candidate(
    download_directory: &Path,
    download: &CurseForgeManualDownload,
) -> crate::Result<Option<(std::path::PathBuf, u64, String)>> {
    validate_file_name(&download.file_name)?;
    let mut entries = match tokio::fs::read_dir(download_directory).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };

    let mut verified_download = None;
    while let Some(entry) = entries.next_entry().await? {
        let actual_file_name = entry.file_name().to_string_lossy().to_string();
        if !crate::util::downloads::browser_download_file_name_matches(
            &actual_file_name,
            &download.file_name,
        ) {
            continue;
        }
        let path = entry.path();
        tracing::debug!(
            project_id = download.project_id,
            file_id = download.file_id,
            candidate_path = %path.display(),
            "Found CurseForge manual download filename candidate"
        );
        if verified_download.is_none() {
            verified_download = Some(
                ensure_manual_download_integrity_metadata(download).await?,
            );
        }
        match verify_manual_download_candidate_with_integrity(
            &path,
            verified_download
                .as_ref()
                .expect("integrity metadata resolved"),
            true,
        )
        .await
        {
            Ok(Some((size, sha1))) => {
                return Ok(Some((path, size, sha1)));
            }
            Ok(None) => continue,
            Err(error) => {
                tracing::debug!(
                    project_id = download.project_id,
                    file_id = download.file_id,
                    candidate_path = %path.display(),
                    %error,
                    "Unable to inspect CurseForge manual download candidate"
                );
                continue;
            }
        }
    }

    Ok(None)
}

async fn verify_manual_download_candidate(
    path: &Path,
    download: &CurseForgeManualDownload,
    require_matching_name: bool,
) -> crate::Result<Option<(u64, String)>> {
    let download = ensure_manual_download_integrity_metadata(download).await?;
    verify_manual_download_candidate_with_integrity(
        path,
        &download,
        require_matching_name,
    )
    .await
}

async fn verify_manual_download_candidate_with_integrity(
    path: &Path,
    download: &CurseForgeManualDownload,
    require_matching_name: bool,
) -> crate::Result<Option<(u64, String)>> {
    let actual_file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    if require_matching_name
        && !crate::util::downloads::browser_download_file_name_matches(
            &actual_file_name,
            &download.file_name,
        )
    {
        trace_manual_download_candidate_rejection(
            path,
            download,
            "name_mismatch",
        );
        return Ok(None);
    }
    let metadata = tokio::fs::symlink_metadata(path).await?;
    if crate::util::io::is_symlink_or_reparse(&metadata) {
        trace_manual_download_candidate_rejection(
            path,
            download,
            "symlink_or_reparse",
        );
        return Ok(None);
    }
    if !metadata.is_file() {
        trace_manual_download_candidate_rejection(path, download, "non_file");
        return Ok(None);
    }
    if download.file_length > 0 && metadata.len() != download.file_length {
        trace_manual_download_candidate_rejection(
            path,
            download,
            "size_mismatch",
        );
        return Ok(None);
    }

    let (size, sha1) = sha1_file_async(path).await?;
    if let Some(expected_sha1) = download
        .hashes
        .iter()
        .find(|hash| hash.algo == 1 && !hash.value.trim().is_empty())
        .map(|hash| hash.value.as_str())
    {
        if !sha1.eq_ignore_ascii_case(expected_sha1) {
            trace_manual_download_candidate_rejection(
                path,
                download,
                "sha1_mismatch",
            );
            return Ok(None);
        }
        return Ok(Some((size, sha1)));
    }
    if download.file_fingerprint == 0 {
        trace_manual_download_candidate_rejection(
            path,
            download,
            "missing_integrity_metadata",
        );
        return Err(ErrorKind::InputError(
            "The required CurseForge file has no usable integrity metadata"
                .to_string(),
        )
        .into());
    }
    let bytes = tokio::fs::read(path).await?;
    if compute_fingerprint(&bytes) as u64 != download.file_fingerprint {
        trace_manual_download_candidate_rejection(
            path,
            download,
            "fingerprint_mismatch",
        );
        return Ok(None);
    }
    Ok(Some((size, sha1)))
}

fn trace_manual_download_candidate_rejection(
    path: &Path,
    download: &CurseForgeManualDownload,
    reason: &'static str,
) {
    tracing::trace!(
        project_id = download.project_id,
        file_id = download.file_id,
        candidate_path = %path.display(),
        expected_file_name = %download.file_name,
        rejection_reason = reason,
        "Rejected CurseForge manual download candidate"
    );
}

pub(crate) async fn import_pending_manual_download_from_path(
    instance_id: &str,
    source_path: &Path,
) -> crate::Result<Option<String>> {
    let state = State::get().await?;
    let pending = crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
        instance_id,
        &state.pool,
    )
    .await?;
    let actual_file_name = source_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut matched_pending_name = false;
    for download in pending.into_iter().filter_map(pending_manual_download) {
        if !crate::util::downloads::browser_download_file_name_matches(
            &actual_file_name,
            &download.file_name,
        ) {
            continue;
        }
        matched_pending_name = true;
        let Some((size, sha1)) =
            verify_manual_download_candidate(source_path, &download, true)
                .await?
        else {
            continue;
        };
        let relative_path = install_manual_download(
            instance_id,
            &download,
            source_path,
            size,
            &sha1,
        )
        .await?;
        finalize_curseforge_manual_download_import(instance_id, false).await?;
        return Ok(Some(relative_path));
    }
    if matched_pending_name {
        return Err(ErrorKind::InputError(
            "The selected file does not match the required CurseForge file"
                .to_string(),
        )
        .into());
    }
    Ok(None)
}

pub async fn import_pending_manual_download_file(
    instance_id: &str,
    project_id: u32,
    file_id: u32,
    source_path: PathBuf,
) -> crate::Result<CurseForgeManualDownloadImport> {
    import_pending_manual_download_file_with_integrity_resolver(
        instance_id,
        project_id,
        file_id,
        source_path,
        resolve_manual_download_integrity_metadata,
    )
    .await
}

async fn import_pending_manual_download_file_with_integrity_resolver<F, Fut>(
    instance_id: &str,
    project_id: u32,
    file_id: u32,
    source_path: PathBuf,
    resolve_metadata: F,
) -> crate::Result<CurseForgeManualDownloadImport>
where
    F: FnOnce(u32, u32) -> Fut,
    Fut: std::future::Future<
            Output = crate::Result<CurseForgeManualDownloadIntegrityMetadata>,
        >,
{
    tracing::debug!(
        instance_id,
        project_id,
        file_id,
        source_path = %source_path.display(),
        "Importing selected CurseForge manual download file"
    );
    let pending = list_pending_manual_downloads(instance_id).await?;
    let pending_count = pending.len();
    let download = pending.into_iter().find(|download| {
        download.project_id == project_id && download.file_id == file_id
    });
    tracing::debug!(
        instance_id,
        project_id,
        file_id,
        pending_count,
        pending_found = download.is_some(),
        "Looked up selected CurseForge pending manual download"
    );
    let download = download.ok_or_else(|| {
        ErrorKind::InputError(
            "The selected CurseForge file is not pending for this instance"
                .to_string(),
        )
    })?;
    let download = ensure_manual_download_integrity_metadata_with(
        &download,
        resolve_metadata,
    )
    .await?;
    let Some((size, sha1)) = verify_manual_download_candidate_with_integrity(
        &source_path,
        &download,
        false,
    )
    .await?
    else {
        return Err(ErrorKind::InputError(
            "The selected file does not match the required CurseForge file"
                .to_string(),
        )
        .into());
    };
    let relative_path = install_manual_download(
        instance_id,
        &download,
        &source_path,
        size,
        &sha1,
    )
    .await?;
    finalize_curseforge_manual_download_import(instance_id, true).await?;
    Ok(CurseForgeManualDownloadImport {
        project_id,
        file_id,
        relative_path,
    })
}

async fn finalize_curseforge_manual_download_import(
    instance_id: &str,
    emit_content_changed: bool,
) -> crate::Result<()> {
    let reconciliation =
        reconcile_curseforge_waiting_jobs_for_instance(instance_id).await;
    let content_changed = if emit_content_changed {
        crate::api::instance::emit_content_changed(instance_id).await
    } else {
        Ok(())
    };
    reconciliation?;
    content_changed
}

pub(crate) async fn reconcile_curseforge_waiting_jobs_for_instance(
    instance_id: &str,
) -> crate::Result<()> {
    let state = State::get().await?;
    reconcile_curseforge_waiting_jobs_for_instance_with_state(
        instance_id,
        &state,
    )
    .await
}

pub(crate) async fn reconcile_persisted_curseforge_waiting_jobs(
    state: &State,
) -> crate::Result<()> {
    let mut resume_job = resume_curseforge_install_job;
    reconcile_persisted_curseforge_waiting_jobs_with_resume(
        state,
        &mut resume_job,
    )
    .await
}

async fn reconcile_persisted_curseforge_waiting_jobs_with_resume<F, Fut>(
    state: &State,
    resume_job: &mut F,
) -> crate::Result<()>
where
    F: FnMut(uuid::Uuid) -> Fut,
    Fut: std::future::Future<Output = crate::Result<()>>,
{
    let jobs = crate::install::store::list(false, state).await?;
    for instance_id in curseforge_waiting_job_instance_ids(&jobs) {
        reconcile_curseforge_waiting_jobs_for_instance_with_resume(
            &instance_id,
            state,
            resume_job,
        )
        .await?;
    }
    Ok(())
}

fn curseforge_waiting_job_instance_ids(
    jobs: &[crate::install::store::InstallJobRecord],
) -> Vec<String> {
    let mut instance_ids = jobs
        .iter()
        .filter(|job| is_reconcilable_curseforge_waiting_job(job))
        .filter_map(|job| job.instance_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    instance_ids.sort_unstable();
    instance_ids
}

async fn reconcile_curseforge_waiting_jobs_for_instance_with_state(
    instance_id: &str,
    state: &State,
) -> crate::Result<()> {
    let mut resume_job = resume_curseforge_install_job;
    reconcile_curseforge_waiting_jobs_for_instance_with_resume(
        instance_id,
        state,
        &mut resume_job,
    )
    .await
}

async fn resume_curseforge_install_job(
    job_id: uuid::Uuid,
) -> crate::Result<()> {
    crate::install::runner::resume_job(job_id).await?;
    Ok(())
}

async fn reconcile_curseforge_waiting_jobs_for_instance_with_resume<F, Fut>(
    instance_id: &str,
    state: &State,
    resume_job: &mut F,
) -> crate::Result<()>
where
    F: FnMut(uuid::Uuid) -> Fut,
    Fut: std::future::Future<Output = crate::Result<()>>,
{
    let pending = crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
		instance_id,
		&state.pool,
	)
	.await?
	.into_iter()
	.filter(|download| download.provider == ContentProvider::CurseForge)
	.map(|download| {
		(download.provider_project_id, download.provider_release_id)
	})
	.collect::<HashSet<_>>();
    let materialized = crate::state::instances::adapters::sqlite::content_rows::get_materialized_curseforge_downloads(
        instance_id,
        &state.pool,
    )
    .await?
    .into_iter()
    .collect::<HashSet<_>>();

    let jobs = crate::install::store::list(false, &state).await?;
    let waiting_job_count = jobs
        .iter()
        .filter(|job| {
            job.instance_id.as_deref() == Some(instance_id)
                && is_reconcilable_curseforge_waiting_job(job)
        })
        .count();
    tracing::debug!(
        instance_id,
        pending_key_count = pending.len(),
        waiting_job_count,
        "Starting CurseForge waiting install job reconciliation"
    );
    let mut recovered_item_count = 0usize;
    let mut resume_job_called = false;
    for job in jobs {
        if job.instance_id.as_deref() != Some(instance_id)
            || !is_reconcilable_curseforge_waiting_job(&job)
        {
            continue;
        }

        let operation_lock = state
            .install_job_operation_locks
            .entry(job.id)
            .or_default()
            .clone();
        let _operation_guard = operation_lock.lock().await;
        let current =
            crate::install::store::get_required(job.id, state).await?;
        if current.instance_id.as_deref() != Some(instance_id)
            || !is_reconcilable_curseforge_waiting_job(&current)
        {
            continue;
        }

        let reconciliation = curseforge_manual_download_reconciliation(
            &current.state.download_items(),
            &pending,
            &materialized,
        );
        for inconsistent in &reconciliation.inconsistent {
            tracing::warn!(
                job_id = %current.id,
                instance_id,
                project_id = %inconsistent.project_id,
                file_id = %inconsistent.file_id,
                item_path = %inconsistent.path,
                reason = inconsistent.reason,
                "CurseForge manual download job state is inconsistent"
            );
        }

        let recovered_count = reconciliation.recovered.len();
        recovered_item_count += recovered_count;
        if recovered_count > 0 {
            InstallProgressReporter::new(current.id, current.state)
                .record_events(reconciliation.recovered)
                .await?;
        }

        let latest = crate::install::store::get_required(job.id, state).await?;
        if latest.status != crate::install::InstallJobStatus::WaitingForUser {
            continue;
        }
        let latest_reconciliation = curseforge_manual_download_reconciliation(
            &latest.state.download_items(),
            &pending,
            &materialized,
        );
        tracing::debug!(
            job_id = %latest.id,
            instance_id,
            pending_count = pending.len(),
            manual_skipped_count = reconciliation.manual_skipped_count,
            materialized_exact_match_count =
                reconciliation.materialized_exact_match_count,
            recovered_count,
            unresolved_pending_count =
                latest_reconciliation.unresolved_pending_count,
            inconsistent_count = latest_reconciliation.inconsistent.len(),
            "Reconciled CurseForge waiting install job"
        );

        if latest_reconciliation.should_resume() {
            resume_job_called = true;
            match resume_job(latest.id).await {
                Ok(_) => {
                    tracing::info!(
                        job_id = %latest.id,
                        instance_id,
                        pending_count = pending.len(),
                        manual_skipped_count =
                            reconciliation.manual_skipped_count,
                        materialized_exact_match_count =
                            reconciliation.materialized_exact_match_count,
                        recovered_count,
                        unresolved_pending_count = 0,
                        inconsistent_count = 0,
                        "Reconciled completed CurseForge manual downloads and resuming install job"
                    );
                }
                Err(error) => {
                    let current =
                        crate::install::store::get_required(latest.id, state)
                            .await?;
                    if current.status
                        == crate::install::InstallJobStatus::WaitingForUser
                    {
                        return Err(error);
                    }
                }
            }
        }
    }
    tracing::debug!(
        instance_id,
        pending_key_count = pending.len(),
        waiting_job_count,
        recovered_item_count,
        resume_job_called,
        "Completed CurseForge waiting install job reconciliation"
    );
    Ok(())
}

#[derive(Debug)]
struct CurseForgeManualDownloadInconsistency {
    path: String,
    project_id: String,
    file_id: String,
    reason: &'static str,
}

#[derive(Debug, Default)]
struct CurseForgeManualDownloadReconciliation {
    recovered: Vec<InstallJobEventKind>,
    inconsistent: Vec<CurseForgeManualDownloadInconsistency>,
    manual_skipped_count: usize,
    materialized_exact_match_count: usize,
    unresolved_pending_count: usize,
}

impl CurseForgeManualDownloadReconciliation {
    fn should_resume(&self) -> bool {
        self.unresolved_pending_count == 0 && self.inconsistent.is_empty()
    }
}

fn curseforge_manual_download_reconciliation(
    items: &[crate::install::model::DownloadItemSnapshot],
    pending: &HashSet<(String, String)>,
    materialized: &HashSet<(String, String)>,
) -> CurseForgeManualDownloadReconciliation {
    let mut result = CurseForgeManualDownloadReconciliation::default();
    for item in items.iter().filter(|item| {
        item.status == crate::install::model::DownloadItemStatus::Skipped
            && item.manual_url.is_some()
            && item.project_id.is_some()
            && item.version_id.is_some()
    }) {
        result.manual_skipped_count += 1;
        let project_id = item.project_id.as_ref().expect("filtered project ID");
        let version_id = item.version_id.as_ref().expect("filtered file ID");
        if pending.contains(&(project_id.clone(), version_id.clone())) {
            result.unresolved_pending_count += 1;
        } else if materialized
            .contains(&(project_id.clone(), version_id.clone()))
        {
            result.materialized_exact_match_count += 1;
            result
                .recovered
                .push(InstallJobEventKind::ContentFileRecovered {
                    path: item.id.clone(),
                    bytes: item.bytes_total.unwrap_or(0),
                });
        } else {
            result
                .inconsistent
                .push(CurseForgeManualDownloadInconsistency {
                    path: item.id.clone(),
                    project_id: project_id.clone(),
                    file_id: version_id.clone(),
                    reason: "pending_missing_but_not_materialized",
                });
        }
    }
    result
}

fn is_reconcilable_curseforge_waiting_job(
    job: &crate::install::store::InstallJobRecord,
) -> bool {
    job.status == crate::install::InstallJobStatus::WaitingForUser
        && job.instance_id.is_some()
        && is_curseforge_manual_download_job(&job.state)
        && matches!(
            &job.state.pause_reason,
            Some(crate::install::model::InstallPauseReason::MissingRequiredContent { .. })
        )
        && job.state.download_items().iter().any(|item| {
            item.status == crate::install::model::DownloadItemStatus::Skipped
                && item.manual_url.is_some()
                && item.project_id.is_some()
                && item.version_id.is_some()
        })
}

fn is_curseforge_manual_download_job(
    job: &crate::install::model::InstallJobState,
) -> bool {
    if !matches!(
        job.provider(),
        crate::install::model::InstallJobProvider::CurseForge
            | crate::install::model::InstallJobProvider::Local
    ) {
        return false;
    }
    job.download_items().iter().any(|item| {
        item.manual_url.is_some()
            && item.project_id.is_some()
            && item.version_id.is_some()
    })
}

async fn install_manual_download(
    instance_id: &str,
    download: &CurseForgeManualDownload,
    source_path: &Path,
    size: u64,
    sha1: &str,
) -> crate::Result<String> {
    if download.project_type == "modpack" {
        let verified_directory = tempfile::tempdir()?;
        let verified_source =
            verified_directory.path().join("manual-download.pack");
        crate::state::materialize_verified_project_download_copy(
            source_path,
            &verified_source,
            size,
            sha1,
        )
        .await?;
        crate::install::install_pack_to_existing_instance(
            instance_id.to_string(),
            crate::api::pack::install_from::CreatePackLocation::FromFile {
                path: verified_source,
            },
            None,
        )
        .await?;
        let state = State::get().await?;
        let content_set = crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
			instance_id,
			&state.pool,
		)
		.await?
		.ok_or_else(|| {
			ErrorKind::InputError(
				"Instance has no applied content set".to_string(),
			)
		})?;
        let _instance_lock = state.lock_instance_content(instance_id).await;
        let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::state::instances::adapters::sqlite::content_rows::complete_pending_manual_download(
			instance_id,
			&download.project_id.to_string(),
			&download.file_id.to_string(),
			None,
			&mut tx,
		)
		.await?;
        crate::state::instances::adapters::sqlite::content_rows::bump_content_set_revision_in_transaction(
			&content_set.id,
			&mut tx,
        )
		.await?;
        tx.commit().await?;
        let relative_path = source_path.to_string_lossy().to_string();
        tracing::info!(
            instance_id,
            project_id = download.project_id,
            file_id = download.file_id,
            relative_path = %relative_path,
            "Completed CurseForge pending manual download record"
        );
        return Ok(relative_path);
    }
    let project_type = managed_project_type(&download.project_type)?;
    let target_folder = manual_download_target_folder(download, project_type)?;
    let state = State::get().await?;
    let localized_candidate =
        chinese_file_title_for_curseforge_slug(&download.project_slug)
            .and_then(|title| {
                localized_content_file_name(&download.file_name, &title)
            })
            .map(|file_name| format!("{target_folder}/{file_name}"));
    let relative_path = crate::state::resolve_content_install_relative_path(
        instance_id,
        format!("{target_folder}/{}", download.file_name),
        localized_candidate,
        &state.pool,
    )
    .await?;
    let full_path = crate::api::instance::get_full_path(instance_id)
        .await?
        .join(&relative_path);
    let previous_path =
        crate::state::materialize_verified_project_download_copy(
            source_path,
            &full_path,
            size,
            sha1,
        )
        .await?;
    let record_result =
        crate::state::record_verified_curseforge_project_file_atomic(
            instance_id,
            &relative_path,
            sha1,
            size,
            project_type,
            ContentSourceKind::CurseForge,
            download.ownership_kind,
            CurseForgeProjectId::new(download.project_id)?,
            CurseForgeFileId::new(download.file_id)?,
            true,
            &state,
        )
        .await;
    match record_result {
        Ok(()) => {
            crate::state::finalize_project_materialization(
                previous_path.as_deref(),
            )
            .await?;
        }
        Err(error) => {
            crate::state::restore_project_materialization(
                &full_path,
                previous_path.as_deref(),
            )
            .await?;
            return Err(error);
        }
    }
    tracing::info!(
        instance_id,
        project_id = download.project_id,
        file_id = download.file_id,
        relative_path = %relative_path,
        "Completed CurseForge pending manual download record"
    );
    if download.ownership_kind
        == crate::state::instances::ContentOwnershipKind::PackManaged
        && download.operation_kind
            == crate::state::instances::ManualDownloadOperationKind::ContentUpdate
    {
        mark_pack_member_version_override(instance_id, &relative_path).await?;
    }
    Ok(relative_path)
}

async fn mark_pack_member_version_override(
    instance_id: &str,
    relative_path: &str,
) -> crate::Result<()> {
    let state = State::get().await?;
    let _instance_lock = state.lock_instance_content(instance_id).await;
    let content_set = crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
        instance_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| {
        ErrorKind::InputError(
            "Instance has no applied content set".to_string(),
        )
    })?;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let updated = sqlx::query(
        "UPDATE instance_pack_members
         SET override_kind = 'version', modified_at = ?
         WHERE content_entry_id IN (
            SELECT entry.id
            FROM instance_content_entries entry
            INNER JOIN instance_files file ON file.id = entry.file_id
            WHERE entry.content_set_id = ? AND file.relative_path = ?
         )",
    )
    .bind(chrono::Utc::now().timestamp())
    .bind(&content_set.id)
    .bind(relative_path)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() > 0 {
        crate::state::instances::adapters::sqlite::content_rows::bump_content_set_revision_in_transaction(
            &content_set.id,
            &mut tx,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn manual_download_target_folder(
    download: &CurseForgeManualDownload,
    project_type: ProjectType,
) -> crate::Result<String> {
    let target_folder = if download.target_folder.is_empty() {
        project_type.get_folder()
    } else {
        &download.target_folder
    };
    if target_folder == project_type.get_folder() {
        return Ok(target_folder.to_string());
    }
    let components = Path::new(target_folder).components().collect::<Vec<_>>();
    if project_type == ProjectType::DataPack
        && components.len() == 3
        && matches!(components[0], Component::Normal(value) if value == "saves")
        && matches!(components[1], Component::Normal(value) if !value.is_empty())
        && matches!(components[2], Component::Normal(value) if value == "datapacks")
    {
        return Ok(target_folder.to_string());
    }
    Err(ErrorKind::InputError(
        "CurseForge manual download has an invalid target folder".to_string(),
    )
    .into())
}

pub(crate) async fn select_latest_compatible_file(
    project_id: u32,
    game_version: Option<String>,
    mod_loader_type: Option<u32>,
    release_channel: Option<ReleaseChannel>,
) -> crate::Result<Option<CurseForgeFile>> {
    let response = get_files(
        project_id,
        CurseForgeFilesRequest {
            game_version,
            mod_loader_type,
            game_version_type_id: None,
            index: 0,
            page_size: MAX_PAGE_SIZE,
        },
    )
    .await?;

    Ok(response.files.into_iter().find(|file| {
        file.is_available
            && match release_channel {
                Some(ReleaseChannel::Release) => file.release_type == 1,
                Some(ReleaseChannel::Beta) => file.release_type <= 2,
                Some(ReleaseChannel::Alpha) | None => true,
            }
    }))
}

async fn select_dependency_file(
    project_id: u32,
    game_version: Option<String>,
    mod_loader_type: Option<u32>,
) -> crate::Result<Option<CurseForgeFile>> {
    let response = get_files(
        project_id,
        CurseForgeFilesRequest {
            game_version,
            mod_loader_type,
            game_version_type_id: None,
            index: 0,
            page_size: MAX_PAGE_SIZE,
        },
    )
    .await?;

    Ok(response.files.into_iter().find(|file| file.is_available))
}

fn managed_project_type(value: &str) -> crate::Result<ProjectType> {
    match value {
        "mod" => Ok(ProjectType::Mod),
        "datapack" => Ok(ProjectType::DataPack),
        "resourcepack" => Ok(ProjectType::ResourcePack),
        "shader" | "shaderpack" => Ok(ProjectType::ShaderPack),
        other => Err(ErrorKind::InputError(format!(
            "CurseForge project type {other} uses its dedicated installer"
        ))
        .into()),
    }
}

fn content_target_folder(
    project_type: ProjectType,
    world_name: Option<&str>,
) -> crate::Result<String> {
    if project_type == ProjectType::DataPack
        && let Some(world_name) = world_name
    {
        validate_file_name(world_name)?;
        Ok(format!("saves/{world_name}/datapacks"))
    } else {
        Ok(project_type.get_folder().to_string())
    }
}

fn manual_download_from_file(
    project_id: u32,
    file_id: u32,
    file: &CurseForgeFile,
    project: &CurseForgeProject,
    project_type: &str,
    target_folder: String,
    ownership_kind: crate::state::instances::ContentOwnershipKind,
    operation_kind: crate::state::instances::ManualDownloadOperationKind,
) -> CurseForgeManualDownload {
    CurseForgeManualDownload {
        project_id,
        file_id,
        file_name: file.file_name.clone(),
        ownership_kind,
        operation_kind,
        website_url: curseforge_file_page_url(
            project.links.website_url.as_deref(),
            file_id,
        ),
        project_type: project_type.to_string(),
        project_slug: project.slug.clone(),
        target_folder,
        hashes: file.hashes.clone(),
        file_length: file.file_length,
        file_fingerprint: file.file_fingerprint,
    }
}

async fn persist_manual_modpack_archive(
    instance_id: &str,
    download: &CurseForgeManualDownload,
) -> crate::Result<()> {
    let state = State::get().await?;
    let _instance_lock = state.lock_instance_content(instance_id).await;
    let content_set = crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
		instance_id,
		&state.pool,
	)
	.await?
	.ok_or_else(|| {
		ErrorKind::InputError(
			"Instance has no applied content set".to_string(),
		)
	})?;
    let now = chrono::Utc::now();
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    crate::state::instances::adapters::sqlite::content_rows::upsert_pending_manual_download_in_transaction(
		&crate::state::instances::PendingManualDownload {
			id: format!("manual-download:{}", uuid::Uuid::new_v4()),
			instance_id: instance_id.to_string(),
			pack_member_id: None,
			content_entry_id: None,
			operation_kind: download.operation_kind,
			operation_target_id: Some(download.file_id.to_string()),
			project_type: ProjectType::Mod,
			provider: ContentProvider::CurseForge,
			provider_project_id: download.project_id.to_string(),
			provider_release_id: download.file_id.to_string(),
			file_name: download.file_name.clone(),
			website_url: download.website_url.clone(),
			target_relative_path: download.file_name.clone(),
			expected_sha1: download
				.hashes
				.iter()
				.find(|hash| hash.algo == 1)
				.map(|hash| hash.value.clone()),
			expected_size: (download.file_length > 0)
				.then_some(download.file_length),
			expected_fingerprint: (download.file_fingerprint > 0)
				.then_some(download.file_fingerprint),
			state: crate::state::instances::ManualDownloadState::Waiting,
			context: serde_json::to_value(download)?,
			created_at: now,
			modified_at: now,
		},
		&mut tx,
	)
	.await?;
    crate::state::instances::adapters::sqlite::content_rows::bump_content_set_revision_in_transaction(
		&content_set.id,
		&mut tx,
	)
	.await?;
    tx.commit().await?;
    Ok(())
}

async fn persist_manual_download(
    instance_id: &str,
    download: &CurseForgeManualDownload,
) -> crate::Result<()> {
    if download.project_type == "modpack" {
        return persist_manual_modpack_archive(instance_id, download).await;
    }
    let project_type = managed_project_type(&download.project_type)?;
    let state = State::get().await?;
    let _instance_lock = state.lock_instance_content(instance_id).await;
    let content_set = crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
        instance_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| {
        ErrorKind::InputError(
            "Instance has no applied content set".to_string(),
        )
    })?;
    let target_relative_path = format!(
        "{}/{}",
        download.target_folder.trim_end_matches('/'),
        download.file_name
    );
    let member_key = format!(
        "curseforge:{}:{}",
        download.project_id,
        project_type.get_name()
    );
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let now = chrono::Utc::now();
    let pack_member_id = if download.ownership_kind
        == crate::state::instances::ContentOwnershipKind::PackManaged
    {
        let existing_id = sqlx::query_scalar::<_, String>(
            "SELECT id FROM instance_pack_members
             WHERE content_set_id = ? AND member_key = ?",
        )
        .bind(&content_set.id)
        .bind(&member_key)
        .fetch_optional(&mut *tx)
        .await?;
        if download.operation_kind
            == crate::state::instances::ManualDownloadOperationKind::PackUpdate
        {
            existing_id
        } else {
            let member_id = existing_id.unwrap_or_else(|| {
                format!("pack-member:{}", uuid::Uuid::new_v4())
            });
            crate::state::instances::adapters::sqlite::content_rows::upsert_pack_member_in_transaction(
			&crate::state::instances::PackMember {
                id: member_id.clone(),
                content_set_id: content_set.id.clone(),
                content_entry_id: None,
                member_key,
                project_type,
                expected_relative_path: target_relative_path.clone(),
                provider: Some(ContentProvider::CurseForge),
                provider_project_id: Some(download.project_id.to_string()),
                provider_release_id: Some(download.file_id.to_string()),
                required: true,
                expected_sha1: download
                    .hashes
                    .iter()
                    .find(|hash| hash.algo == 1)
                    .map(|hash| hash.value.clone()),
                expected_size: (download.file_length > 0)
                    .then_some(download.file_length),
                expected_fingerprint: (download.file_fingerprint > 0)
                    .then_some(download.file_fingerprint),
                materialization_state: crate::state::instances::PackMemberMaterializationState::PendingManual,
                override_kind: crate::state::instances::PackMemberOverrideKind::None,
                reconciled: true,
                created_at: now,
                modified_at: now,
            },
			&mut tx,
		)
		.await?;
            Some(member_id)
        }
    } else {
        None
    };
    crate::state::instances::adapters::sqlite::content_rows::upsert_pending_manual_download_in_transaction(
        &crate::state::instances::PendingManualDownload {
            id: format!("manual-download:{}", uuid::Uuid::new_v4()),
            instance_id: instance_id.to_string(),
            pack_member_id,
            content_entry_id: None,
            operation_kind: download.operation_kind,
            operation_target_id: None,
            project_type,
            provider: ContentProvider::CurseForge,
            provider_project_id: download.project_id.to_string(),
            provider_release_id: download.file_id.to_string(),
            file_name: download.file_name.clone(),
            website_url: download.website_url.clone(),
            target_relative_path,
            expected_sha1: download
                .hashes
                .iter()
                .find(|hash| hash.algo == 1)
                .map(|hash| hash.value.clone()),
            expected_size: (download.file_length > 0)
                .then_some(download.file_length),
            expected_fingerprint: (download.file_fingerprint > 0)
                .then_some(download.file_fingerprint),
            state: crate::state::instances::ManualDownloadState::Waiting,
            context: serde_json::to_value(download)?,
            created_at: now,
            modified_at: now,
        },
        &mut tx,
    )
    .await?;
    crate::state::instances::adapters::sqlite::content_rows::bump_content_set_revision_in_transaction(
        &content_set.id,
        &mut tx,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn validate_file_name(file_name: &str) -> crate::Result<()> {
    let path = Path::new(file_name);
    if file_name.is_empty()
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(ErrorKind::InputError(
            "CurseForge returned an invalid file name".to_string(),
        )
        .into());
    }
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

async fn report_modpack_progress(
    loading_bar: Option<&crate::event::LoadingBarId>,
    reporter: Option<&InstallProgressReporter>,
    details: InstallPhaseDetails,
    files_done: &AtomicU64,
    bytes_done: &AtomicU64,
    active_downloads: &AtomicU64,
    total_files: u64,
    total_bytes: u64,
    file_bytes: u64,
    event: InstallJobEventKind,
) -> crate::Result<()> {
    let current_files = files_done.fetch_add(1, Ordering::Relaxed) + 1;
    let current_bytes =
        bytes_done.fetch_add(file_bytes, Ordering::Relaxed) + file_bytes;
    let active = active_downloads.load(Ordering::Relaxed);
    let message = if total_bytes > 0 {
        format!(
            "{current_files}/{total_files} files · {} / {} · {active} downloading in parallel",
            format_bytes(current_bytes.min(total_bytes)),
            format_bytes(total_bytes)
        )
    } else {
        format!(
            "{current_files}/{total_files} files · {active} downloading in parallel"
        )
    };
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 1.0, Some(&message))?;
    }
    if let Some(reporter) = reporter {
        reporter
            .update_with_events(
                InstallPhaseId::DownloadingContent,
                Some(InstallProgress {
                    current: current_files,
                    total: total_files,
                    secondary: Some(InstallProgressSecondary {
                        current: current_bytes.min(total_bytes),
                        total: total_bytes,
                    }),
                }),
                details,
                vec![event],
            )
            .await?;
    }
    Ok(())
}

fn is_forge_cdn_url(url: &reqwest::Url) -> bool {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    host == "forgecdn.net" || host.ends_with(".forgecdn.net")
}

fn curseforge_file_page_url(
    website_url: Option<&str>,
    file_id: u32,
) -> Option<String> {
    let website_url = website_url?;
    let Ok(mut url) = reqwest::Url::parse(website_url) else {
        return Some(website_url.to_owned());
    };
    if !matches!(
        url.host_str(),
        Some("curseforge.com" | "www.curseforge.com" | "legacy.curseforge.com")
    ) {
        return Some(website_url.to_owned());
    }

    let mut path = url.path().trim_end_matches('/').to_string();
    for marker in ["/files/", "/download/"] {
        if let Some(index) = path.rfind(marker) {
            path.truncate(index);
            break;
        }
    }
    url.set_path(&format!("{path}/download/{file_id}"));
    url.set_query(None);
    url.set_fragment(None);
    Some(url.into())
}

fn validate_cdn_url(url: &reqwest::Url) -> crate::Result<()> {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    #[cfg(debug_assertions)]
    if url.scheme() == "http"
        && matches!(host.as_str(), "127.0.0.1" | "localhost")
    {
        return Ok(());
    }
    if url.scheme() != "https"
        || (host != "mod.mcimirror.top" && !is_forge_cdn_url(url))
    {
        return Err(ErrorKind::InputError(
            "CurseForge returned a download URL outside its CDN".to_string(),
        )
        .into());
    }
    Ok(())
}

fn curseforge_content_validation(file_name: &str) -> ContentValidation {
    match Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jar" | "zip" | "mrpack") => ContentValidation::Jar,
        _ => ContentValidation::None,
    }
}

fn curseforge_integrity(
    file: &CurseForgeFile,
    validation: ContentValidation,
) -> Integrity {
    Integrity {
        size: Some(file.file_length),
        sha1: file
            .hashes
            .iter()
            .find(|hash| hash.algo == 1)
            .map(|hash| hash.value.clone()),
        md5: file
            .hashes
            .iter()
            .find(|hash| hash.algo == 2)
            .map(|hash| hash.value.clone()),
        content: validation,
        ..Integrity::default()
    }
}

fn curseforge_candidate_urls(url: &str) -> crate::Result<Vec<String>> {
    let parsed = reqwest::Url::parse(url).map_err(|_| {
        ErrorKind::InputError(
            "CurseForge returned an invalid download URL".to_string(),
        )
    })?;
    validate_cdn_url(&parsed)?;
    if !is_forge_cdn_url(&parsed) {
        return Ok(Vec::new());
    }

    let original_host = parsed.host_str().unwrap_or_default();
    let mut candidates = Vec::new();
    for host in [
        "edge.forgecdn.net",
        "media.forgecdn.net",
        "mediafilez.forgecdn.net",
    ] {
        if host == original_host {
            continue;
        }
        let mut candidate = parsed.clone();
        candidate.set_host(Some(host)).map_err(|_| {
            ErrorKind::InputError(
                "CurseForge returned an invalid CDN URL".to_string(),
            )
        })?;
        candidates.push(candidate.to_string());
    }
    Ok(candidates)
}

async fn download_curseforge_path(
    url: &str,
    file: &CurseForgeFile,
    destination: &Path,
    validation: ContentValidation,
    progress: Option<&mut FetchProgressFn<'_>>,
    tracking: Option<(&InstallProgressReporter, &str)>,
) -> crate::Result<crate::util::fetch::DownloadResult> {
    let state = State::get().await?;
    let mut request = DownloadRequest::new(url, ResourceClass::CurseForge)
        .with_candidate_urls(curseforge_candidate_urls(url)?)
        .with_integrity(curseforge_integrity(file, validation));
    let parsed = reqwest::Url::parse(url)?;
    if is_forge_cdn_url(&parsed)
        && let Some(key) = api_key()
    {
        request = request.with_header("x-api-key", key);
    }
    if let Some((reporter, item_id)) = tracking {
        request = request.with_install_tracking(
            reporter.clone(),
            item_id,
            file.file_name.clone(),
        );
    }
    download_to_path(
        request,
        destination,
        &state.download_semaphore,
        &state.pool,
        progress,
    )
    .await
}

async fn download_curseforge_archive(
    project_id: u32,
    file_id: u32,
    file: &CurseForgeFile,
    url: &str,
    progress: Option<&mut FetchProgressFn<'_>>,
    reporter: Option<&InstallProgressReporter>,
) -> crate::Result<crate::util::fetch::DownloadResult> {
    validate_file_name(&file.file_name)?;
    let state = State::get().await?;
    let path = state
        .directories
        .caches_dir()
        .join("curseforge")
        .join("modpacks")
        .join(project_id.to_string())
        .join(file_id.to_string())
        .join(&file.file_name);
    let tracking_item_id = path.display().to_string();
    download_curseforge_path(
        url,
        file,
        &path,
        ContentValidation::Jar,
        progress,
        reporter.map(|reporter| (reporter, tracking_item_id.as_str())),
    )
    .await
}

async fn download_installed_file(
    instance_id: &str,
    url: &str,
    file: &CurseForgeFile,
    project_type: ProjectType,
    world_name: Option<&str>,
    project_id: u32,
    file_id: u32,
    project_slug: &str,
    ownership_kind: crate::state::instances::ContentOwnershipKind,
    download_metrics: Option<&CurseForgeDownloadMetrics>,
) -> crate::Result<String> {
    if file.mod_id != project_id || file.id != file_id {
        return Err(ErrorKind::InputError(
            "CurseForge returned metadata for a different project or file"
                .to_string(),
        )
        .into());
    }
    let state = State::get().await?;
    validate_file_name(&file.file_name)?;
    let folder = content_target_folder(project_type, world_name)?;
    let localized_candidate =
        chinese_file_title_for_curseforge_slug(project_slug)
            .and_then(|title| {
                localized_content_file_name(&file.file_name, &title)
            })
            .map(|file_name| format!("{folder}/{file_name}"));
    let relative_path = crate::state::resolve_content_install_relative_path(
        instance_id,
        format!("{folder}/{}", file.file_name),
        localized_candidate,
        &state.pool,
    )
    .await?;
    let full_path = crate::api::instance::get_full_path(instance_id)
        .await?
        .join(&relative_path);
    let mut download_path = full_path.as_os_str().to_os_string();
    download_path.push(".installing.download");
    let download_path = Path::new(&download_path);
    let result = download_curseforge_path(
        url,
        file,
        download_path,
        curseforge_content_validation(&file.file_name),
        None,
        download_metrics
            .and_then(|metrics| metrics.reporter.as_ref())
            .map(|reporter| (reporter, relative_path.as_str())),
    )
    .await?;
    if let Some(download_metrics) = download_metrics {
        download_metrics.record(&result);
    }
    let previous_path =
        crate::state::materialize_project_download(download_path, &full_path)
            .await?;
    crate::util::io::remove_file(download_path).await?;
    let record_result = record_installed_curseforge_file(
        instance_id,
        &relative_path,
        &full_path,
        file,
        project_type,
        ownership_kind,
        &state,
    )
    .await;
    match record_result {
        Ok(()) => {
            crate::state::finalize_project_materialization(
                previous_path.as_deref(),
            )
            .await?;
        }
        Err(error) => {
            crate::state::restore_project_materialization(
                &full_path,
                previous_path.as_deref(),
            )
            .await?;
            return Err(error);
        }
    }
    Ok(relative_path)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CurseForgePendingCompletionProof {
    None,
    AuthoritativeSha1,
    AuthoritativeFingerprint,
}

struct VerifiedInstalledCurseForgeFile {
    size: u64,
    sha1: String,
    pending_completion: CurseForgePendingCompletionProof,
}

async fn verify_installed_curseforge_file(
    path: &Path,
    file: &CurseForgeFile,
) -> crate::Result<VerifiedInstalledCurseForgeFile> {
    if let Some(expected_sha1) = file
        .hashes
        .iter()
        .find(|hash| hash.algo == 1 && !hash.value.trim().is_empty())
        .map(|hash| hash.value.as_str())
    {
        let (size, sha1) = sha1_file_async(path).await?;
        if !sha1.eq_ignore_ascii_case(expected_sha1) {
            return Err(
                ErrorKind::HashError(expected_sha1.to_string(), sha1).into()
            );
        }
        return Ok(VerifiedInstalledCurseForgeFile {
            size,
            sha1,
            pending_completion:
                CurseForgePendingCompletionProof::AuthoritativeSha1,
        });
    }

    if file.file_fingerprint != 0 {
        let bytes = tokio::fs::read(path).await?;
        if compute_fingerprint(&bytes) as u64 != file.file_fingerprint {
            return Err(ErrorKind::InputError(
                "The downloaded file does not match the required CurseForge fingerprint"
                    .to_string(),
            )
            .into());
        }
        return Ok(VerifiedInstalledCurseForgeFile {
            size: bytes.len() as u64,
            sha1: sha1_smol::Sha1::from(&bytes).hexdigest(),
            pending_completion:
                CurseForgePendingCompletionProof::AuthoritativeFingerprint,
        });
    }

    let (size, sha1) = sha1_file_async(path).await?;
    Ok(VerifiedInstalledCurseForgeFile {
        size,
        sha1,
        pending_completion: CurseForgePendingCompletionProof::None,
    })
}

async fn record_installed_curseforge_file(
    instance_id: &str,
    relative_path: &str,
    full_path: &Path,
    file: &CurseForgeFile,
    project_type: ProjectType,
    ownership_kind: crate::state::instances::ContentOwnershipKind,
    state: &State,
) -> crate::Result<()> {
    let verified = verify_installed_curseforge_file(full_path, file).await?;
    match verified.pending_completion {
        CurseForgePendingCompletionProof::None => {
            let provider_ref = ContentProviderRef::CurseForge {
                project_id: CurseForgeProjectId::new(file.mod_id)?,
                file_id: Some(CurseForgeFileId::new(file.id)?),
            };
            crate::state::record_project_file_atomic(
                instance_id,
                relative_path,
                &verified.sha1,
                verified.size,
                project_type,
                ContentSourceKind::CurseForge,
                ownership_kind,
                Some(&provider_ref),
                true,
                None,
                state,
            )
            .await
        }
        CurseForgePendingCompletionProof::AuthoritativeSha1
        | CurseForgePendingCompletionProof::AuthoritativeFingerprint => {
            crate::state::record_verified_curseforge_project_file_atomic(
                instance_id,
                relative_path,
                &verified.sha1,
                verified.size,
                project_type,
                ContentSourceKind::CurseForge,
                ownership_kind,
                CurseForgeProjectId::new(file.mod_id)?,
                CurseForgeFileId::new(file.id)?,
                true,
                state,
            )
            .await
        }
    }
}

pub fn compute_fingerprint(data: &[u8]) -> u32 {
    let normalized = data
        .iter()
        .copied()
        .filter(|byte| !matches!(byte, 9 | 10 | 13 | 32))
        .collect::<Vec<_>>();
    murmur2(&normalized, 1)
}

impl From<CurseForgeProject> for UnifiedSearchHit {
    fn from(project: CurseForgeProject) -> Self {
        let mut versions = Vec::new();
        let mut seen_versions = HashSet::new();
        for index in &project.latest_files_indexes {
            if seen_versions.insert(index.game_version.clone()) {
                versions.push(index.game_version.clone());
            }
        }

        let project_type = project_type_for_class(project.class_id);
        Self {
            provider: ContentProvider::CurseForge,
            project_id: project.id.to_string(),
            slug: Some(project.slug),
            author: project
                .authors
                .first()
                .map(|author| author.name.clone())
                .unwrap_or_default(),
            author_url: project
                .authors
                .first()
                .map(|author| author.url.clone()),
            title: project.name,
            description: project.summary,
            project_type: project_type.to_string(),
            categories: project
                .categories
                .iter()
                .map(|category| category.slug.clone())
                .collect(),
            versions,
            downloads: project.download_count,
            icon_url: project.logo.map(|logo| logo.thumbnail_url),
            date_created: project.date_created,
            date_modified: project.date_modified,
            latest_version: project
                .latest_files
                .first()
                .map(|file| file.id.to_string()),
            gallery: project
                .screenshots
                .into_iter()
                .map(|screenshot| screenshot.url)
                .collect(),
            website_url: project.links.website_url,
            source_url: project.links.source_url,
            allow_mod_distribution: project.allow_mod_distribution,
        }
    }
}

fn project_type_for_class(class_id: Option<u32>) -> &'static str {
    match class_id {
        Some(5) => "plugin",
        Some(6) => "mod",
        Some(12) => "resourcepack",
        Some(17) => "world",
        Some(6945) => "datapack",
        Some(4471) => "modpack",
        Some(6552) => "shader",
        _ => "mod",
    }
}

fn recognized_project_type(class_id: Option<u32>) -> Option<ProjectType> {
    match class_id {
        Some(6) => Some(ProjectType::Mod),
        Some(12) => Some(ProjectType::ResourcePack),
        Some(6552) => Some(ProjectType::ShaderPack),
        Some(6945) => Some(ProjectType::DataPack),
        Some(17) => Some(ProjectType::WorldSave),
        _ => None,
    }
}

fn filter_categories(
    categories: Vec<CurseForgeCategory>,
    class_id: Option<u32>,
) -> Vec<CurseForgeCategory> {
    let Some(class_id) = class_id else {
        return categories;
    };

    categories
        .into_iter()
        .filter(|category| {
            category.id == class_id || category.class_id == Some(class_id)
        })
        .collect()
}

fn push_query<T: ToString>(
    query: &mut Vec<(String, String)>,
    name: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        query.push((name.to_string(), value.to_string()));
    }
}

fn api_key() -> Option<String> {
    std::env::var("AXOLOTL_CURSEFORGE_API_KEY")
        .ok()
        .or_else(|| option_env!("CURSEFORGE_API_KEY").map(str::to_string))
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
}

fn api_base_url() -> String {
    #[cfg(debug_assertions)]
    if let Ok(value) = std::env::var("AXOLOTL_CURSEFORGE_API_BASE_URL")
        && value.starts_with("http://127.0.0.1:")
    {
        return value.trim_end_matches('/').to_string();
    }

    API_BASE_URL.to_string()
}

fn request_client(
    _url: &str,
    use_system_proxy: bool,
) -> &'static reqwest::Client {
    #[cfg(debug_assertions)]
    if _url.starts_with("http://127.0.0.1:") {
        return &LOCAL_CLIENT;
    }

    if use_system_proxy {
        &PROXY_CLIENT
    } else {
        &CLIENT
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MirrorPolicy {
    MirrorFirst,
    OfficialOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequestRouteSource {
    Official,
    Mirror,
}

struct RequestRoute {
    url: String,
    use_api_key: bool,
    use_system_proxy: bool,
    source: RequestRouteSource,
}

#[cfg(test)]
fn request_routes(
    path: &str,
    mirror_policy: MirrorPolicy,
) -> Vec<RequestRoute> {
    let mode = match mirror_policy {
        MirrorPolicy::MirrorFirst => DownloadSourceMode::MirrorPreferred,
        MirrorPolicy::OfficialOnly => DownloadSourceMode::OfficialOnly,
    };
    request_routes_with_mode(path, mode)
}

fn request_routes_with_mode(
    path: &str,
    mode: DownloadSourceMode,
) -> Vec<RequestRoute> {
    let base_url = api_base_url();
    if base_url != API_BASE_URL {
        return vec![RequestRoute {
            url: format!("{base_url}{path}"),
            use_api_key: true,
            use_system_proxy: false,
            source: RequestRouteSource::Official,
        }];
    }

    resolve_download_routes_for(
        &format!("{API_BASE_URL}{path}"),
        ResourceClass::CurseForge,
        mode,
    )
    .into_iter()
    .map(|route| RequestRoute {
        use_api_key: route.allow_sensitive_headers,
        use_system_proxy: route.proxy == ProxyPolicy::System,
        source: match route.source {
            DownloadRouteSource::Bmclapi | DownloadRouteSource::Mcim => {
                RequestRouteSource::Mirror
            }
            DownloadRouteSource::Official | DownloadRouteSource::Alternate => {
                RequestRouteSource::Official
            }
        },
        url: route.url,
    })
    .collect()
}

async fn request_json<T: DeserializeOwned>(
    method: Method,
    path: &str,
    query: Vec<(String, String)>,
    body: Option<Value>,
    mirror_policy: MirrorPolicy,
) -> crate::Result<T> {
    let key = api_key();
    let state = State::get().await?;
    let source_mode = if method != Method::GET
        || mirror_policy == MirrorPolicy::OfficialOnly
    {
        DownloadSourceMode::OfficialOnly
    } else {
        state.curseforge_source()
    };
    let routes = request_routes_with_mode(path, source_mode);
    let mut last_error = None;

    for (route_index, route) in routes.iter().enumerate() {
        let started = Instant::now();
        tracing::info!(
            source = ?route.source,
            method = %method,
            url = %route.url,
            route = route_index + 1,
            use_system_proxy = route.use_system_proxy,
            "Attempting CurseForge API request"
        );
        let permit = state.api_semaphore.0.acquire().await?;
        let mut request = request_client(&route.url, route.use_system_proxy)
            .request(method.clone(), &route.url)
            .header("accept", "application/json")
            .query(&query);
        if route.use_api_key {
            let Some(key) = key.as_ref() else {
                drop(permit);
                let error: crate::Error = ErrorKind::InputError(
                    "CurseForge integration is waiting for an API key"
                        .to_string(),
                )
                .into();
                if route_index + 1 < routes.len() {
                    last_error = Some(error);
                    continue;
                }
                return Err(error);
            };
            request = request.header("x-api-key", key);
        }
        if let Some(body) = &body {
            request = request.json(body);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) if route_index + 1 < routes.len() => {
                drop(permit);
                tracing::warn!(
                    url = %route.url,
                    route = route_index + 1,
                    %error,
                    "CurseForge request failed, retrying with another route"
                );
                last_error = Some(error.into());
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        drop(permit);

        let status = response.status();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| Duration::from_secs(seconds.min(30)));
        let bytes = response.bytes().await?;

        tracing::info!(
            source = ?route.source,
            method = %method,
            url = %route.url,
            route = route_index + 1,
            status = status.as_u16(),
            response_bytes = bytes.len(),
            elapsed_ms = started.elapsed().as_millis(),
            "Completed CurseForge API request"
        );

        if status.is_success() {
            match serde_json::from_slice(&bytes) {
                Ok(value) => {
                    UNAUTHORIZED.store(false, Ordering::Relaxed);
                    return Ok(value);
                }
                Err(error)
                    if route.source == RequestRouteSource::Mirror
                        && route_index + 1 < routes.len() =>
                {
                    tracing::warn!(
                        url = %route.url,
                        route = route_index + 1,
                        %error,
                        "CurseForge mirror returned incompatible response data; falling back to official source"
                    );
                    last_error = Some(error.into());
                    continue;
                }
                Err(error) => return Err(error.into()),
            }
        }

        if status == StatusCode::UNAUTHORIZED {
            UNAUTHORIZED.store(true, Ordering::Relaxed);
        }

        let message = response_error_message(status, &bytes);
        let route_error = ErrorKind::OtherError(format!(
            "CurseForge request to {} failed with HTTP {}: {message}",
            route.url,
            status.as_u16()
        ));

        if should_try_next_route(route, status, route_index + 1 < routes.len())
        {
            if let Some(delay) = retry_after {
                tokio::time::sleep(delay).await;
            }
            tracing::warn!(
                url = %route.url,
                route = route_index + 1,
                status = status.as_u16(),
                "CurseForge route rejected the request, trying another route"
            );
            last_error = Some(route_error.into());
            continue;
        }

        return Err(route_error.into());
    }

    Err(last_error.unwrap_or_else(|| {
        ErrorKind::OtherError("CurseForge request exhausted routes".to_string())
            .into()
    }))
}

fn should_try_next_route(
    route: &RequestRoute,
    status: StatusCode,
    has_next_route: bool,
) -> bool {
    has_next_route
        && (route.source == RequestRouteSource::Mirror
            || status == StatusCode::TOO_MANY_REQUESTS
            || status == StatusCode::FORBIDDEN
            || status.is_server_error())
}

fn response_error_message(status: StatusCode, bytes: &[u8]) -> String {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|value| {
            value
                .get("description")
                .or_else(|| value.get("message"))
                .or_else(|| value.get("error"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("request failed")
                .to_string()
        })
}

fn murmur2(data: &[u8], seed: u32) -> u32 {
    const M: u32 = 0x5bd1e995;
    const R: u32 = 24;
    let mut hash = seed ^ data.len() as u32;
    let mut chunks = data.chunks_exact(4);

    for chunk in &mut chunks {
        let mut value =
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        value = value.wrapping_mul(M);
        value ^= value >> R;
        value = value.wrapping_mul(M);
        hash = hash.wrapping_mul(M);
        hash ^= value;
    }

    match chunks.remainder() {
        [a, b, c] => {
            hash ^= (*c as u32) << 16;
            hash ^= (*b as u32) << 8;
            hash ^= *a as u32;
            hash = hash.wrapping_mul(M);
        }
        [a, b] => {
            hash ^= (*b as u32) << 8;
            hash ^= *a as u32;
            hash = hash.wrapping_mul(M);
        }
        [a] => {
            hash ^= *a as u32;
            hash = hash.wrapping_mul(M);
        }
        [] => {}
        _ => unreachable!(),
    }

    hash ^= hash >> 13;
    hash = hash.wrapping_mul(M);
    hash ^= hash >> 15;
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skipped_curseforge_manual_item(
        id: &str,
        project_id: &str,
        version_id: &str,
    ) -> crate::install::model::DownloadItemSnapshot {
        crate::install::model::DownloadItemSnapshot {
            id: id.to_string(),
            name: id.to_string(),
            project_id: Some(project_id.to_string()),
            version_id: Some(version_id.to_string()),
            status: crate::install::model::DownloadItemStatus::Skipped,
            bytes_downloaded: 0,
            bytes_total: Some(42),
            attempt: Some(1),
            max_attempts: Some(1),
            error: Some("CurseForge requires manual download".to_string()),
            manual_url: Some("https://www.curseforge.com/download".to_string()),
            request_url: None,
            source: None,
        }
    }

    fn waiting_manual_job_record(
        instance_id: &str,
        request: crate::install::model::InstallRequest,
    ) -> crate::install::store::InstallJobRecord {
        let mut state = crate::install::model::InstallJobState::new(request);
        state.record_event(InstallJobEventKind::ContentFileSkipped {
            path: "mods/one.jar".to_string(),
            reason: "manual download required".to_string(),
            project_id: Some("1".to_string()),
            version_id: Some("10".to_string()),
            manual_url: Some("https://www.curseforge.com/download".to_string()),
        });
        state.record_event(InstallJobEventKind::WaitingForUser {
            reason:
                crate::install::model::InstallPauseReason::MissingRequiredContent {
                    failed_files: 1,
                    paths: vec!["mods/one.jar".to_string()],
                },
        });
        state.pause_reason = Some(
            crate::install::model::InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                paths: vec!["mods/one.jar".to_string()],
            },
        );
        let now = chrono::Utc::now();
        crate::install::store::InstallJobRecord {
            id: uuid::Uuid::new_v4(),
            instance_id: Some(instance_id.to_string()),
            kind: crate::install::model::InstallJobKind::InstallContent,
            status: crate::install::model::InstallJobStatus::WaitingForUser,
            state,
            created: now,
            modified: now,
            finished: None,
            dismissed: false,
        }
    }

    fn apply_recovered_events(
        items: &mut [crate::install::model::DownloadItemSnapshot],
        events: &[InstallJobEventKind],
    ) {
        for event in events {
            let InstallJobEventKind::ContentFileRecovered { path, bytes } =
                event
            else {
                continue;
            };
            let item = items.iter_mut().find(|item| item.id == *path).unwrap();
            item.status = crate::install::model::DownloadItemStatus::Completed;
            item.bytes_downloaded = *bytes;
        }
    }

    fn reconcile_and_simulate_resume(
        items: &mut [crate::install::model::DownloadItemSnapshot],
        pending: &HashSet<(String, String)>,
        materialized: &HashSet<(String, String)>,
        resume_count: &mut usize,
    ) -> CurseForgeManualDownloadReconciliation {
        let result = curseforge_manual_download_reconciliation(
            items,
            pending,
            materialized,
        );
        apply_recovered_events(items, &result.recovered);
        let latest = curseforge_manual_download_reconciliation(
            items,
            pending,
            materialized,
        );
        if latest.should_resume() {
            *resume_count += 1;
        }
        result
    }

    #[cfg(not(feature = "tauri"))]
    async fn stage6_state() -> std::sync::Arc<State> {
        crate::event::EventState::init().await.unwrap();
        let state_root = tempfile::tempdir().unwrap().keep();
        State::init_for_test(state_root.to_string_lossy().to_string())
            .await
            .unwrap()
    }

    #[cfg(not(feature = "tauri"))]
    async fn create_stage6_instance(
        label: &str,
    ) -> (std::sync::Arc<State>, String) {
        let state = stage6_state().await;
        let created = crate::api::instance::create(
            format!("Stage 6 {label} {}", uuid::Uuid::new_v4()),
            "1.20.1".to_string(),
            ModLoader::Vanilla,
            None,
            None,
            InstanceLink::Unmanaged,
            None,
        )
        .await
        .unwrap();
        (state, created.instance.id)
    }

    #[cfg(not(feature = "tauri"))]
    fn stage6_manual_download(
        project_id: u32,
        file_id: u32,
        file_name: &str,
        expected_bytes: &[u8],
    ) -> CurseForgeManualDownload {
        CurseForgeManualDownload {
            project_id,
            file_id,
            file_name: file_name.to_string(),
            ownership_kind:
                crate::state::instances::ContentOwnershipKind::PackManaged,
            operation_kind:
                crate::state::instances::ManualDownloadOperationKind::PackInstall,
            website_url: None,
            project_type: "mod".to_string(),
            project_slug: format!("stage-6-{project_id}-{file_id}"),
            target_folder: "mods".to_string(),
            hashes: vec![CurseForgeFileHash {
                value: sha1_smol::Sha1::from(expected_bytes).hexdigest(),
                algo: 1,
            }],
            file_length: expected_bytes.len() as u64,
            file_fingerprint: 0,
        }
    }

    fn stage8_legacy_manual_download(
        project_id: u32,
        file_id: u32,
        file_name: &str,
    ) -> CurseForgeManualDownload {
        CurseForgeManualDownload {
            project_id,
            file_id,
            file_name: file_name.to_string(),
            ownership_kind:
                crate::state::instances::ContentOwnershipKind::PackManaged,
            operation_kind:
                crate::state::instances::ManualDownloadOperationKind::PackInstall,
            website_url: None,
            project_type: "mod".to_string(),
            project_slug: format!("stage-8-{project_id}-{file_id}"),
            target_folder: "mods".to_string(),
            hashes: Vec::new(),
            file_length: 0,
            file_fingerprint: 0,
        }
    }

    fn stage8_integrity_metadata(
        download: &CurseForgeManualDownload,
        hashes: Vec<CurseForgeFileHash>,
        file_length: u64,
        file_fingerprint: u64,
    ) -> CurseForgeManualDownloadIntegrityMetadata {
        CurseForgeManualDownloadIntegrityMetadata {
            project_id: download.project_id,
            file_id: download.file_id,
            hashes,
            file_length,
            file_fingerprint,
        }
    }

    fn stage8_curseforge_file(
        project_id: u32,
        file_id: u32,
        file_name: &str,
        file_length: u64,
        hashes: Vec<CurseForgeFileHash>,
        file_fingerprint: u64,
    ) -> CurseForgeFile {
        CurseForgeFile {
            id: file_id,
            game_id: MINECRAFT_GAME_ID,
            mod_id: project_id,
            is_available: true,
            display_name: file_name.to_string(),
            file_name: file_name.to_string(),
            release_type: 1,
            file_status: 4,
            hashes,
            file_date: String::new(),
            file_length,
            download_count: 0,
            file_size_on_disk: Some(file_length),
            download_url: None,
            game_versions: Vec::new(),
            sortable_game_versions: Vec::new(),
            dependencies: Vec::new(),
            expose_as_alternative: None,
            parent_project_file_id: None,
            alternate_file_id: None,
            is_server_pack: None,
            server_pack_file_id: None,
            is_early_access_content: None,
            early_access_end_date: None,
            file_fingerprint,
            modules: Vec::new(),
        }
    }

    #[cfg(not(feature = "tauri"))]
    async fn stage8_pending_keys(
        state: &State,
        instance_id: &str,
    ) -> HashSet<(String, String)> {
        crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
            instance_id,
            &state.pool,
        )
        .await
        .unwrap()
        .into_iter()
        .map(|download| {
            (
                download.provider_project_id,
                download.provider_release_id,
            )
        })
        .collect()
    }

    #[cfg(not(feature = "tauri"))]
    async fn import_stage6_manual_download(
        instance_id: &str,
        download: &CurseForgeManualDownload,
        bytes: &[u8],
    ) {
        let source_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join(&download.file_name);
        crate::util::io::write(&source, bytes).await.unwrap();
        import_pending_manual_download_file(
            instance_id,
            download.project_id,
            download.file_id,
            source,
        )
        .await
        .unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    fn stage6_waiting_manual_job_state(
        instance_id: &str,
        items: &[(&str, &str, &str)],
    ) -> crate::install::model::InstallJobState {
        let mut state = crate::install::model::InstallJobState::new(
            crate::install::model::InstallRequest::InstallPackToExistingInstance {
                instance_id: instance_id.to_string(),
                location: crate::api::pack::install_from::CreatePackLocation::FromFile {
                    path: PathBuf::from(format!(
                        "missing-stage-6-{}.mrpack",
                        uuid::Uuid::new_v4()
                    )),
                },
                post_install_edit: None,
            },
        );
        let paths = items
            .iter()
            .map(|(path, _, _)| (*path).to_string())
            .collect::<Vec<_>>();
        for (path, project_id, file_id) in items {
            state.record_event(InstallJobEventKind::ContentFileQueued {
                path: (*path).to_string(),
                bytes_total: Some(42),
                max_attempts: 1,
            });
            state.record_event(InstallJobEventKind::ContentFileSkipped {
                path: (*path).to_string(),
                reason: "manual download required".to_string(),
                project_id: Some((*project_id).to_string()),
                version_id: Some((*file_id).to_string()),
                manual_url: Some(
                    "https://www.curseforge.com/download".to_string(),
                ),
            });
        }
        let reason =
            crate::install::model::InstallPauseReason::MissingRequiredContent {
                failed_files: paths.len() as u64,
                paths,
            };
        state.pause_reason = Some(reason.clone());
        state.record_event(InstallJobEventKind::WaitingForUser { reason });
        state
    }

    #[cfg(not(feature = "tauri"))]
    async fn insert_stage6_waiting_manual_job(
        state: &State,
        instance_id: &str,
        items: &[(&str, &str, &str)],
    ) -> uuid::Uuid {
        let job_id = uuid::Uuid::new_v4();
        let job_state = stage6_waiting_manual_job_state(instance_id, items);
        crate::install::store::insert(
            job_id,
            &job_state,
            crate::install::model::InstallJobStatus::WaitingForUser,
            state,
        )
        .await
        .unwrap();
        job_id
    }

    #[cfg(not(feature = "tauri"))]
    async fn assert_stage6_job_is_unresolved(
        state: &State,
        job_id: uuid::Uuid,
        item_path: &str,
    ) {
        let job = crate::install::store::get_required(job_id, state)
            .await
            .unwrap();
        assert_eq!(
            job.status,
            crate::install::model::InstallJobStatus::WaitingForUser
        );
        assert_eq!(
            job.snapshot()
                .items
                .iter()
                .find(|item| item.id == item_path)
                .unwrap()
                .status,
            crate::install::model::DownloadItemStatus::Skipped
        );
        assert_eq!(
            job.state
                .events
                .iter()
                .filter(|event| matches!(
                    event.kind,
                    InstallJobEventKind::ContentFileRecovered { .. }
                ))
                .count(),
            0
        );
    }

    #[test]
    fn curseforge_final_manual_completion_recovers_and_resumes() {
        let mut items =
            vec![skipped_curseforge_manual_item("mods/one.jar", "1", "10")];
        let materialized = HashSet::from([("1".to_string(), "10".to_string())]);
        let mut resume_count = 0;

        let result = reconcile_and_simulate_resume(
            &mut items,
            &HashSet::new(),
            &materialized,
            &mut resume_count,
        );

        assert_eq!(result.recovered.len(), 1);
        assert_eq!(result.materialized_exact_match_count, 1);
        assert_eq!(resume_count, 1);
        assert_eq!(
            items[0].status,
            crate::install::model::DownloadItemStatus::Completed
        );
    }

    #[test]
    fn curseforge_missing_pending_without_materialization_does_not_resume() {
        let mut items =
            vec![skipped_curseforge_manual_item("mods/one.jar", "1", "10")];
        let mut resume_count = 0;

        let result = reconcile_and_simulate_resume(
            &mut items,
            &HashSet::new(),
            &HashSet::new(),
            &mut resume_count,
        );

        assert!(result.recovered.is_empty());
        assert_eq!(result.inconsistent.len(), 1);
        assert_eq!(
            result.inconsistent[0].reason,
            "pending_missing_but_not_materialized"
        );
        assert_eq!(resume_count, 0);
        assert_eq!(
            items[0].status,
            crate::install::model::DownloadItemStatus::Skipped
        );
    }

    #[test]
    fn curseforge_pending_still_exists_does_not_resume() {
        let mut items =
            vec![skipped_curseforge_manual_item("mods/one.jar", "1", "10")];
        let pending = HashSet::from([("1".to_string(), "10".to_string())]);
        let mut resume_count = 0;

        let result = reconcile_and_simulate_resume(
            &mut items,
            &pending,
            &HashSet::new(),
            &mut resume_count,
        );

        assert!(result.recovered.is_empty());
        assert_eq!(result.unresolved_pending_count, 1);
        assert!(result.inconsistent.is_empty());
        assert_eq!(resume_count, 0);
    }

    #[test]
    fn curseforge_partial_manual_completion_does_not_resume() {
        let mut items = vec![
            skipped_curseforge_manual_item("mods/a.jar", "1", "10"),
            skipped_curseforge_manual_item("mods/b.jar", "2", "20"),
            skipped_curseforge_manual_item("mods/c.jar", "3", "30"),
        ];
        let keys = [
            ("1".to_string(), "10".to_string()),
            ("2".to_string(), "20".to_string()),
            ("3".to_string(), "30".to_string()),
        ];
        let mut pending = keys.iter().cloned().collect::<HashSet<_>>();
        let mut materialized = HashSet::new();
        let mut resume_count = 0;

        for (index, key) in keys.into_iter().enumerate() {
            assert!(pending.remove(&key));
            assert!(materialized.insert(key));
            let result = reconcile_and_simulate_resume(
                &mut items,
                &pending,
                &materialized,
                &mut resume_count,
            );

            assert_eq!(result.recovered.len(), 1);
            assert!(items[..=index].iter().all(|item| {
                item.status
                    == crate::install::model::DownloadItemStatus::Completed
            }));
            assert!(items[index + 1..].iter().all(|item| {
                item.status
                    == crate::install::model::DownloadItemStatus::Skipped
            }));
            assert_eq!(resume_count, usize::from(index == 2));
        }
    }

    #[test]
    fn curseforge_repeated_reconciliation_is_idempotent() {
        let mut items =
            vec![skipped_curseforge_manual_item("mods/one.jar", "1", "10")];
        let materialized = HashSet::from([("1".to_string(), "10".to_string())]);
        let mut resume_count = 0;
        let mut waiting_for_user = true;
        let mut recovered_count = 0;

        for _ in 0..3 {
            if !waiting_for_user {
                continue;
            }
            let result = reconcile_and_simulate_resume(
                &mut items,
                &HashSet::new(),
                &materialized,
                &mut resume_count,
            );
            recovered_count += result.recovered.len();
            if resume_count > 0 {
                waiting_for_user = false;
            }
        }

        assert_eq!(recovered_count, 1);
        assert_eq!(resume_count, 1);
    }

    #[test]
    fn curseforge_startup_selects_only_reconcilable_jobs() {
        let curseforge_job = waiting_manual_job_record(
            "curseforge-instance",
            crate::install::model::InstallRequest::InstallCurseForgeContent {
                request: CurseForgeInstallRequest {
                    instance_id: "curseforge-instance".to_string(),
                    project_id: 1,
                    file_id: 10,
                    project_type: "mod".to_string(),
                    ownership_kind:
                        crate::state::instances::ContentOwnershipKind::PackManaged,
                    manual_operation_kind: crate::state::instances::ManualDownloadOperationKind::PackInstall,
                    game_version: None,
                    mod_loader_type: None,
                    world_name: None,
                    install_dependencies: true,
                },
                display_title: "CurseForge".to_string(),
                display_icon: None,
            },
        );
        let modrinth_job = waiting_manual_job_record(
            "modrinth-instance",
            crate::install::model::InstallRequest::InstallPackToExistingInstance {
                instance_id: "modrinth-instance".to_string(),
                location: crate::api::pack::install_from::CreatePackLocation::FromVersionId {
                    project_id: "project".to_string(),
                    version_id: "version".to_string(),
                    title: "Modrinth".to_string(),
                    icon_url: None,
                },
                post_install_edit: None,
            },
        );
        assert_eq!(
            curseforge_waiting_job_instance_ids(&[
                curseforge_job,
                modrinth_job,
            ]),
            vec!["curseforge-instance".to_string()]
        );

        let mut items =
            vec![skipped_curseforge_manual_item("mods/one.jar", "1", "10")];
        let materialized = HashSet::from([("1".to_string(), "10".to_string())]);
        let mut resume_count = 0;

        let result = reconcile_and_simulate_resume(
            &mut items,
            &HashSet::new(),
            &materialized,
            &mut resume_count,
        );

        assert_eq!(result.recovered.len(), 1);
        assert_eq!(resume_count, 1);
    }

    #[test]
    fn curseforge_wrong_instance_or_release_never_recovers() {
        let mut items =
            vec![skipped_curseforge_manual_item("mods/one.jar", "1", "10")];
        let other_instance_or_release = HashSet::from([
            ("1".to_string(), "11".to_string()),
            ("2".to_string(), "10".to_string()),
        ]);
        let mut resume_count = 0;

        let result = reconcile_and_simulate_resume(
            &mut items,
            &HashSet::new(),
            &other_instance_or_release,
            &mut resume_count,
        );

        assert!(result.recovered.is_empty());
        assert_eq!(result.inconsistent.len(), 1);
        assert_eq!(resume_count, 0);
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_pending_is_scoped_by_instance() {
        let (state, instance_a) = create_stage6_instance("scope A").await;
        let (_, instance_b) = create_stage6_instance("scope B").await;
        let completed = stage6_manual_download(
            101,
            1001,
            "scope-one.jar",
            b"scope-one-content",
        );
        let other_release = stage6_manual_download(
            101,
            1002,
            "scope-two.jar",
            b"scope-two-content",
        );
        persist_manual_download(&instance_a, &completed)
            .await
            .unwrap();
        persist_manual_download(&instance_a, &other_release)
            .await
            .unwrap();
        persist_manual_download(&instance_b, &completed)
            .await
            .unwrap();
        let job_a = insert_stage6_waiting_manual_job(
            &state,
            &instance_a,
            &[
                ("mods/scope-one.jar", "101", "1001"),
                ("mods/scope-two.jar", "101", "1002"),
            ],
        )
        .await;
        let job_b = insert_stage6_waiting_manual_job(
            &state,
            &instance_b,
            &[("mods/scope-one.jar", "101", "1001")],
        )
        .await;

        import_stage6_manual_download(
            &instance_a,
            &completed,
            b"scope-one-content",
        )
        .await;

        let pending_a = crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
            &instance_a,
            &state.pool,
        )
        .await
        .unwrap();
        let pending_b = crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
            &instance_b,
            &state.pool,
        )
        .await
        .unwrap();
        assert_eq!(pending_a.len(), 1);
        assert_eq!(pending_a[0].provider_project_id, "101");
        assert_eq!(pending_a[0].provider_release_id, "1002");
        assert_eq!(pending_b.len(), 1);
        assert_eq!(pending_b[0].provider_project_id, "101");
        assert_eq!(pending_b[0].provider_release_id, "1001");

        let reconciled_a = crate::install::store::get_required(job_a, &state)
            .await
            .unwrap();
        assert_eq!(
            reconciled_a.status,
            crate::install::model::InstallJobStatus::WaitingForUser
        );
        let items_a = reconciled_a.snapshot().items;
        assert_eq!(
            items_a
                .iter()
                .find(|item| item.id == "mods/scope-one.jar")
                .unwrap()
                .status,
            crate::install::model::DownloadItemStatus::Completed
        );
        assert_eq!(
            items_a
                .iter()
                .find(|item| item.id == "mods/scope-two.jar")
                .unwrap()
                .status,
            crate::install::model::DownloadItemStatus::Skipped
        );
        assert_eq!(
            reconciled_a
                .state
                .events
                .iter()
                .filter(|event| matches!(
                    event.kind,
                    InstallJobEventKind::ContentFileRecovered { .. }
                ))
                .count(),
            1
        );
        assert_stage6_job_is_unresolved(&state, job_b, "mods/scope-one.jar")
            .await;
        crate::install::store::dismiss(job_a, &state).await.unwrap();
        crate::install::store::dismiss(job_b, &state).await.unwrap();
        crate::api::instance::remove(&instance_a).await.unwrap();
        crate::api::instance::remove(&instance_b).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_missing_pending_without_materialization_keeps_real_job_waiting()
     {
        let (state, instance_id) =
            create_stage6_instance("missing materialization").await;
        let job_id = insert_stage6_waiting_manual_job(
            &state,
            &instance_id,
            &[("mods/missing.jar", "201", "2001")],
        )
        .await;

        reconcile_curseforge_waiting_jobs_for_instance_with_state(
            &instance_id,
            &state,
        )
        .await
        .unwrap();

        assert_stage6_job_is_unresolved(&state, job_id, "mods/missing.jar")
            .await;
        crate::install::store::dismiss(job_id, &state)
            .await
            .unwrap();
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_pending_still_exists_keeps_real_job_waiting() {
        let (state, instance_id) = create_stage6_instance("pending").await;
        let download = stage6_manual_download(
            301,
            3001,
            "pending.jar",
            b"pending-content",
        );
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let job_id = insert_stage6_waiting_manual_job(
            &state,
            &instance_id,
            &[("mods/pending.jar", "301", "3001")],
        )
        .await;

        reconcile_curseforge_waiting_jobs_for_instance_with_state(
            &instance_id,
            &state,
        )
        .await
        .unwrap();

        assert_stage6_job_is_unresolved(&state, job_id, "mods/pending.jar")
            .await;
        crate::install::store::dismiss(job_id, &state)
            .await
            .unwrap();
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_wrong_release_keeps_real_job_waiting() {
        let (state, instance_id) =
            create_stage6_instance("wrong release").await;
        let other_release = stage6_manual_download(
            401,
            4002,
            "other-release.jar",
            b"other-release-content",
        );
        persist_manual_download(&instance_id, &other_release)
            .await
            .unwrap();
        import_stage6_manual_download(
            &instance_id,
            &other_release,
            b"other-release-content",
        )
        .await;
        let job_id = insert_stage6_waiting_manual_job(
            &state,
            &instance_id,
            &[("mods/target-release.jar", "401", "4001")],
        )
        .await;

        reconcile_curseforge_waiting_jobs_for_instance_with_state(
            &instance_id,
            &state,
        )
        .await
        .unwrap();

        assert_stage6_job_is_unresolved(
            &state,
            job_id,
            "mods/target-release.jar",
        )
        .await;
        crate::install::store::dismiss(job_id, &state)
            .await
            .unwrap();
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_startup_reconciliation_recovers_and_resumes_once() {
        let (state, instance_id) = create_stage6_instance("startup").await;
        let download = stage6_manual_download(
            501,
            5001,
            "startup.jar",
            b"startup-content",
        );
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        import_stage6_manual_download(
            &instance_id,
            &download,
            b"startup-content",
        )
        .await;
        let job_id = insert_stage6_waiting_manual_job(
            &state,
            &instance_id,
            &[("mods/startup.jar", "501", "5001")],
        )
        .await;
        let resume_count =
            std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let state_for_resume = std::sync::Arc::clone(&state);
        let resume_count_for_callback = std::sync::Arc::clone(&resume_count);
        let mut resume_job = move |job_id| {
            let state = std::sync::Arc::clone(&state_for_resume);
            let resume_count =
                std::sync::Arc::clone(&resume_count_for_callback);
            async move {
                let current =
                    crate::install::store::get_required(job_id, &state).await?;
                let claimed = crate::install::store::update_status_if(
                    job_id,
                    crate::install::model::InstallJobStatus::WaitingForUser,
                    crate::install::model::InstallJobStatus::Queued,
                    &current.state,
                    &state,
                )
                .await?;
                assert!(
                    claimed.is_some(),
                    "resume status claim must succeed once"
                );
                resume_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        };

        for _ in 0..3 {
            reconcile_persisted_curseforge_waiting_jobs_with_resume(
                &state,
                &mut resume_job,
            )
            .await
            .unwrap();
        }

        let reconciled = crate::install::store::get_required(job_id, &state)
            .await
            .unwrap();
        assert_eq!(
            reconciled.status,
            crate::install::model::InstallJobStatus::Queued
        );
        assert_eq!(
            reconciled
                .snapshot()
                .items
                .iter()
                .find(|item| item.id == "mods/startup.jar")
                .unwrap()
                .status,
            crate::install::model::DownloadItemStatus::Completed
        );
        assert_eq!(
            reconciled
                .state
                .events
                .iter()
                .filter(|event| matches!(
                    event.kind,
                    InstallJobEventKind::ContentFileRecovered { .. }
                ))
                .count(),
            1
        );
        assert_eq!(resume_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        crate::install::store::dismiss(job_id, &state)
            .await
            .unwrap();
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[test]
    fn unified_search_hit_accepts_null_gallery() {
        let hit: UnifiedSearchHit = serde_json::from_value(serde_json::json!({
            "provider": "curseforge",
            "project_id": "250419",
            "slug": null,
            "author": "DarkhaxDev",
            "author_url": null,
            "title": "Enchantment Descriptions",
            "description": "description",
            "project_type": "mod",
            "categories": [],
            "versions": [],
            "downloads": 0,
            "icon_url": null,
            "date_created": "2026-01-01T00:00:00Z",
            "date_modified": "2026-01-01T00:00:00Z",
            "latest_version": null,
            "gallery": null,
            "website_url": null,
            "source_url": null,
            "allow_mod_distribution": null,
        }))
        .unwrap();

        assert!(hit.gallery.is_empty());
    }

    #[test]
    fn curseforge_file_accepts_null_modules() {
        let file: CurseForgeFile = serde_json::from_value(serde_json::json!({
            "id": 4031925,
            "gameId": 432,
            "modId": 250419,
            "isAvailable": true,
            "displayName": "old.jar",
            "fileName": "old.jar",
            "releaseType": 1,
            "fileStatus": 4,
            "fileDate": "2026-01-01T00:00:00Z",
            "fileLength": 1,
            "downloadCount": 0,
            "fileFingerprint": 0,
            "modules": null,
        }))
        .unwrap();

        assert!(file.modules.is_empty());
    }

    #[test]
    fn fingerprint_ignores_curseforge_whitespace() {
        assert_eq!(
            compute_fingerprint(b"abc\r\n def\t"),
            compute_fingerprint(b"abcdef")
        );
    }

    #[test]
    fn project_types_are_provider_qualified() {
        assert_eq!(project_type_for_class(Some(6)), "mod");
        assert_eq!(project_type_for_class(Some(4471)), "modpack");
        assert_eq!(project_type_for_class(Some(6552)), "shader");
        assert_eq!(project_type_for_class(Some(6945)), "datapack");
    }

    #[test]
    fn official_only_requests_exclude_mirror_routes() {
        let routes =
            request_routes("/v1/mods/search", MirrorPolicy::OfficialOnly);

        assert!(
            routes
                .iter()
                .all(|route| route.source == RequestRouteSource::Official)
        );
    }

    #[test]
    fn mirror_first_requests_start_with_mirror() {
        let routes = request_routes(
            "/v1/mods/285109/description",
            MirrorPolicy::MirrorFirst,
        );

        assert_eq!(routes[0].source, RequestRouteSource::Mirror);
        assert_eq!(routes[1].source, RequestRouteSource::Official);
    }

    #[test]
    fn mirror_not_found_tries_next_route() {
        let route = RequestRoute {
            url: String::new(),
            use_api_key: false,
            use_system_proxy: false,
            source: RequestRouteSource::Mirror,
        };

        assert!(should_try_next_route(&route, StatusCode::NOT_FOUND, true));
    }

    #[test]
    fn official_forbidden_response_tries_proxy_route() {
        let route = RequestRoute {
            url: String::new(),
            use_api_key: true,
            use_system_proxy: false,
            source: RequestRouteSource::Official,
        };

        assert!(should_try_next_route(&route, StatusCode::FORBIDDEN, true));
    }

    #[test]
    fn projects_accept_negative_popularity_ranks() {
        let project = serde_json::from_value::<CurseForgeProject>(json!({
            "id": 1,
            "gameId": MINECRAFT_GAME_ID,
            "name": "Fixture",
            "slug": "fixture",
            "links": {},
            "summary": "Fixture project",
            "status": 4,
            "downloadCount": 0,
            "isFeatured": false,
            "primaryCategoryId": 6,
            "categories": [],
            "classId": 6,
            "authors": [],
            "logo": null,
            "screenshots": [],
            "mainFileId": 0,
            "latestFiles": [],
            "latestFilesIndexes": [],
            "dateCreated": "2026-01-01T00:00:00Z",
            "dateModified": "2026-01-01T00:00:00Z",
            "dateReleased": "2026-01-01T00:00:00Z",
            "allowModDistribution": true,
            "gamePopularityRank": -10,
            "isAvailable": true
        }))
        .unwrap();

        assert_eq!(project.game_popularity_rank, Some(-10));
    }

    #[test]
    fn category_cache_can_be_filtered_for_each_project_class() {
        let categories = vec![
            category(6, None, true),
            category(406, Some(6), false),
            category(4471, None, true),
            category(4481, Some(4471), false),
        ];

        let mods = filter_categories(categories.clone(), Some(6));
        assert_eq!(
            mods.iter().map(|category| category.id).collect::<Vec<_>>(),
            vec![6, 406]
        );

        let modpacks = filter_categories(categories, Some(4471));
        assert_eq!(
            modpacks
                .iter()
                .map(|category| category.id)
                .collect::<Vec<_>>(),
            vec![4471, 4481]
        );
    }

    fn category(
        id: u32,
        class_id: Option<u32>,
        is_class: bool,
    ) -> CurseForgeCategory {
        CurseForgeCategory {
            id,
            game_id: MINECRAFT_GAME_ID,
            name: id.to_string(),
            slug: id.to_string(),
            url: String::new(),
            icon_url: None,
            date_modified: String::new(),
            is_class: Some(is_class),
            class_id,
            parent_category_id: class_id,
            display_index: Some(0),
        }
    }

    #[test]
    fn manual_downloads_open_the_official_download_page() {
        assert_eq!(
            curseforge_file_page_url(
                Some("https://www.curseforge.com/minecraft/mc-mods/example"),
                12345,
            ),
            Some(
                "https://www.curseforge.com/minecraft/mc-mods/example/download/12345"
                    .to_string()
            )
        );
        assert_eq!(
            curseforge_file_page_url(
                Some("https://www.curseforge.com/minecraft/mc-mods/example/files/12345?tab=files"),
                12345,
            ),
            Some(
                "https://www.curseforge.com/minecraft/mc-mods/example/download/12345"
                    .to_string()
            )
        );
        assert_eq!(
            curseforge_file_page_url(
                Some("https://example.com/project"),
                12345
            ),
            Some("https://example.com/project".to_string())
        );
    }

    #[test]
    fn browser_duplicate_download_names_match_the_expected_file() {
        assert!(crate::util::downloads::browser_download_file_name_matches(
            "example-mod (1).jar",
            "example-mod.jar"
        ));
        assert!(crate::util::downloads::browser_download_file_name_matches(
            "EXAMPLE-MOD.JAR",
            "example-mod.jar"
        ));
        assert!(!crate::util::downloads::browser_download_file_name_matches(
            "example-mod-fabric.jar",
            "example-mod.jar"
        ));
        assert!(!crate::util::downloads::browser_download_file_name_matches(
            "example-mod.jar.crdownload",
            "example-mod.jar"
        ));
    }

    #[tokio::test]
    async fn curseforge_metadata_less_exact_name_is_not_trusted() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("example-mod.jar");
        crate::util::io::write(&path, b"wrong exact-name jar")
            .await
            .unwrap();
        let download = stage8_legacy_manual_download(1, 2, "example-mod.jar");

        let error = verify_manual_download_candidate_with_integrity(
            &path, &download, true,
        )
        .await
        .expect_err("filename alone must not verify a candidate");

        assert!(matches!(
            error.raw.as_ref(),
            ErrorKind::InputError(message)
                if message
                    == "The required CurseForge file has no usable integrity metadata"
        ));
    }

    #[tokio::test]
    async fn curseforge_hydration_is_scoped_to_exact_release() {
        let download =
            stage8_legacy_manual_download(11, 101, "exact-release.jar");
        let error = ensure_manual_download_integrity_metadata_with(
            &download,
            |project_id, file_id| async move {
                assert_eq!(project_id, 11);
                assert_eq!(file_id, 101);
                Ok(CurseForgeManualDownloadIntegrityMetadata {
                    project_id,
                    file_id: 102,
                    hashes: vec![CurseForgeFileHash {
                        value: "wrong-release-sha1".to_string(),
                        algo: 1,
                    }],
                    file_length: 1,
                    file_fingerprint: 0,
                })
            },
        )
        .await
        .expect_err("metadata for another file must be rejected");

        assert!(matches!(
            error.raw.as_ref(),
            ErrorKind::InputError(message)
                if message
                    == "CurseForge returned metadata for a different project or file"
        ));
    }

    #[tokio::test]
    async fn curseforge_existing_integrity_skips_hydration() {
        let mut sha1_download =
            stage8_legacy_manual_download(12, 201, "existing-sha1.jar");
        sha1_download.hashes = vec![CurseForgeFileHash {
            value: "existing-sha1".to_string(),
            algo: 1,
        }];
        let hydrated = ensure_manual_download_integrity_metadata_with(
            &sha1_download,
            |_, _| async {
                Err(ErrorKind::InputError(
                    "metadata resolver must not run".to_string(),
                )
                .into())
            },
        )
        .await
        .unwrap();
        assert_eq!(hydrated.hashes[0].value, "existing-sha1");

        let mut fingerprint_download =
            stage8_legacy_manual_download(12, 202, "existing-fingerprint.jar");
        fingerprint_download.file_fingerprint = 12345;
        let hydrated = ensure_manual_download_integrity_metadata_with(
            &fingerprint_download,
            |_, _| async {
                Err(ErrorKind::InputError(
                    "metadata resolver must not run".to_string(),
                )
                .into())
            },
        )
        .await
        .unwrap();
        assert_eq!(hydrated.file_fingerprint, 12345);
    }

    #[tokio::test]
    async fn curseforge_hydrated_fingerprint_verifies_candidate() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fingerprint.jar");
        let bytes = b"legacy fingerprint content";
        crate::util::io::write(&path, bytes).await.unwrap();
        let download =
            stage8_legacy_manual_download(13, 301, "fingerprint.jar");
        let metadata = stage8_integrity_metadata(
            &download,
            Vec::new(),
            bytes.len() as u64,
            compute_fingerprint(bytes) as u64,
        );
        let hydrated = ensure_manual_download_integrity_metadata_with(
            &download,
            move |project_id, file_id| async move {
                assert_eq!((project_id, file_id), (13, 301));
                Ok(metadata)
            },
        )
        .await
        .unwrap();

        assert!(
            verify_manual_download_candidate_with_integrity(
                &path, &hydrated, true,
            )
            .await
            .unwrap()
            .is_some()
        );
    }

    #[tokio::test]
    async fn curseforge_manual_materialization_is_copy_isolated() {
        let source_directory = tempfile::tempdir().unwrap();
        let destination_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join("manual.jar");
        let destination = destination_directory.path().join("manual.jar");
        let verified_bytes = b"verified manual bytes";
        crate::util::io::write(&source, verified_bytes)
            .await
            .unwrap();
        let (verified_size, verified_sha1) =
            sha1_file_async(&source).await.unwrap();

        crate::state::materialize_verified_project_download_copy(
            &source,
            &destination,
            verified_size,
            &verified_sha1,
        )
        .await
        .unwrap();
        crate::util::io::write(&source, b"mutated source content")
            .await
            .unwrap();

        assert_eq!(
            crate::util::io::read(&destination).await.unwrap(),
            verified_bytes
        );
        assert_eq!(
            sha1_file_async(&destination).await.unwrap(),
            (verified_size, verified_sha1)
        );
    }

    #[tokio::test]
    async fn curseforge_fingerprint_recognition_rechecks_current_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recognized.jar");
        let matched_bytes = b"matched fingerprint bytes";
        let expected_fingerprint = compute_fingerprint(matched_bytes) as u64;
        crate::util::io::write(&path, matched_bytes).await.unwrap();
        assert!(
            verify_recognized_curseforge_file(&path, expected_fingerprint)
                .await
                .unwrap()
                .is_some()
        );

        crate::util::io::write(&path, b"changed after fingerprint match")
            .await
            .unwrap();
        assert!(
            verify_recognized_curseforge_file(&path, expected_fingerprint)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn curseforge_localized_candidate_still_requires_integrity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("[测试]example-mod (1).jar");
        crate::util::io::write(&path, b"expected curseforge file")
            .await
            .unwrap();
        let (_, sha1) = sha1_file_async(&path).await.unwrap();
        let download = CurseForgeManualDownload {
            project_id: 1,
            file_id: 2,
            file_name: "example-mod.jar".to_string(),
            ownership_kind:
                crate::state::instances::ContentOwnershipKind::PackManaged,
            operation_kind:
                crate::state::instances::ManualDownloadOperationKind::PackInstall,
            website_url: None,
            project_type: "mod".to_string(),
            project_slug: String::new(),
            target_folder: "mods".to_string(),
            hashes: vec![CurseForgeFileHash {
                value: sha1,
                algo: 1,
            }],
            file_length: 24,
            file_fingerprint: 0,
        };

        assert!(
            find_manual_download_candidate(directory.path(), &download)
                .await
                .unwrap()
                .is_some()
        );
        crate::util::io::write(&path, b"same-length-wrong-bytes!")
            .await
            .unwrap();
        assert!(
            find_manual_download_candidate(directory.path(), &download)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_legacy_pending_imports_after_exact_metadata_hydration()
    {
        let (state, instance_id) =
            create_stage6_instance("legacy hydration").await;
        let download =
            stage8_legacy_manual_download(14, 401, "legacy-hydration.jar");
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let source_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join(&download.file_name);
        let bytes = b"legacy hydrated content";
        crate::util::io::write(&source, bytes).await.unwrap();
        let metadata = stage8_integrity_metadata(
            &download,
            vec![CurseForgeFileHash {
                value: sha1_smol::Sha1::from(bytes).hexdigest(),
                algo: 1,
            }],
            bytes.len() as u64,
            0,
        );

        let imported =
            import_pending_manual_download_file_with_integrity_resolver(
                &instance_id,
                download.project_id,
                download.file_id,
                source.clone(),
                move |project_id, file_id| async move {
                    assert_eq!((project_id, file_id), (14, 401));
                    Ok(metadata)
                },
            )
            .await
            .unwrap();

        assert!(source.exists());
        assert_eq!(imported.relative_path, "mods/legacy-hydration.jar");
        assert!(
            crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
                &instance_id,
                &state.pool,
            )
            .await
            .unwrap()
            .is_empty()
        );
        let snapshot = crate::api::instance::get_content_snapshot(&instance_id)
            .await
            .unwrap();
        assert!(snapshot.items.iter().any(|item| {
            item.provider_project_id.as_deref() == Some("14")
                && item.provider_release_id.as_deref() == Some("401")
                && item.materialization_state
                    == crate::state::instances::PackMemberMaterializationState::Present
                && item.content.is_some()
        }));
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_missing_integrity_after_hydration_keeps_pending_unresolved()
     {
        let (state, instance_id) =
            create_stage6_instance("missing hydrated integrity").await;
        let download =
            stage8_legacy_manual_download(15, 501, "missing-integrity.jar");
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let job_id = insert_stage6_waiting_manual_job(
            &state,
            &instance_id,
            &[("mods/missing-integrity.jar", "15", "501")],
        )
        .await;
        let source_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join(&download.file_name);
        crate::util::io::write(&source, b"unverifiable content")
            .await
            .unwrap();
        let metadata = stage8_integrity_metadata(&download, Vec::new(), 20, 0);

        let fetch_error =
            import_pending_manual_download_file_with_integrity_resolver(
                &instance_id,
                download.project_id,
                download.file_id,
                source.clone(),
                |project_id, file_id| async move {
                    assert_eq!((project_id, file_id), (15, 501));
                    Err(ErrorKind::InputError(
                        "simulated CurseForge metadata fetch failure"
                            .to_string(),
                    )
                    .into())
                },
            )
            .await
            .expect_err("metadata fetch failures must block import");
        assert!(matches!(
            fetch_error.raw.as_ref(),
            ErrorKind::InputError(message)
                if message
                    == "simulated CurseForge metadata fetch failure"
        ));

        let error =
            import_pending_manual_download_file_with_integrity_resolver(
                &instance_id,
                download.project_id,
                download.file_id,
                source.clone(),
                move |project_id, file_id| async move {
                    assert_eq!((project_id, file_id), (15, 501));
                    Ok(metadata)
                },
            )
            .await
            .expect_err("missing authoritative integrity must block import");

        assert!(matches!(
            error.raw.as_ref(),
            ErrorKind::InputError(message)
                if message
                    == "The required CurseForge file has no usable integrity metadata"
        ));
        assert!(source.exists());
        assert_eq!(
            crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
                &instance_id,
                &state.pool,
            )
            .await
            .unwrap()
            .len(),
            1
        );
        let target = crate::api::instance::get_full_path(&instance_id)
            .await
            .unwrap()
            .join("mods/missing-integrity.jar");
        assert!(!target.exists());
        assert_stage6_job_is_unresolved(
            &state,
            job_id,
            "mods/missing-integrity.jar",
        )
        .await;
        let job = crate::install::store::get_required(job_id, &state)
            .await
            .unwrap();
        assert_eq!(
            job.state
                .events
                .iter()
                .filter(|event| matches!(
                    event.kind,
                    InstallJobEventKind::ContentFileRecovered { .. }
                ))
                .count(),
            0
        );
        crate::install::store::dismiss(job_id, &state)
            .await
            .unwrap();
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_generic_record_does_not_complete_pending() {
        let (state, instance_id) =
            create_stage6_instance("generic record provenance").await;
        let download =
            stage8_legacy_manual_download(16, 601, "generic-record.jar");
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let relative_path = "mods/generic-record.jar";
        let full_path = crate::api::instance::get_full_path(&instance_id)
            .await
            .unwrap()
            .join(relative_path);
        let bytes = b"generic recorded bytes";
        crate::util::io::write(&full_path, bytes).await.unwrap();
        let (_, sha1) = sha1_file_async(&full_path).await.unwrap();
        let provider_ref = ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(16).unwrap(),
            file_id: Some(CurseForgeFileId::new(601).unwrap()),
        };

        crate::state::record_project_file_atomic(
            &instance_id,
            relative_path,
            &sha1,
            bytes.len() as u64,
            ProjectType::Mod,
            ContentSourceKind::CurseForge,
            crate::state::instances::ContentOwnershipKind::PackManaged,
            Some(&provider_ref),
            true,
            None,
            &state,
        )
        .await
        .unwrap();

        assert!(
            stage8_pending_keys(&state, &instance_id)
                .await
                .contains(&("16".to_string(), "601".to_string()))
        );
        let snapshot = crate::api::instance::get_content_snapshot(&instance_id)
            .await
            .unwrap();
        assert!(snapshot.items.iter().any(|item| {
            item.provider_project_id.as_deref() == Some("16")
                && item.provider_release_id.as_deref() == Some("601")
                && item.materialization_state
                    == crate::state::instances::PackMemberMaterializationState::Present
                && item.content.is_some()
        }));
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_verified_record_completes_only_exact_pending() {
        let (state, instance_a) =
            create_stage6_instance("verified record A").await;
        let (_, instance_b) = create_stage6_instance("verified record B").await;
        let file_1 = stage8_legacy_manual_download(17, 701, "one.jar");
        let file_2 = stage8_legacy_manual_download(17, 702, "two.jar");
        persist_manual_download(&instance_a, &file_1).await.unwrap();
        persist_manual_download(&instance_a, &file_2).await.unwrap();
        persist_manual_download(&instance_b, &file_1).await.unwrap();
        let relative_path = "mods/one.jar";
        let full_path = crate::api::instance::get_full_path(&instance_a)
            .await
            .unwrap()
            .join(relative_path);
        let bytes = b"verified exact record";
        crate::util::io::write(&full_path, bytes).await.unwrap();
        let (_, sha1) = sha1_file_async(&full_path).await.unwrap();

        crate::state::record_verified_curseforge_project_file_atomic(
            &instance_a,
            relative_path,
            &sha1,
            bytes.len() as u64,
            ProjectType::Mod,
            ContentSourceKind::CurseForge,
            crate::state::instances::ContentOwnershipKind::PackManaged,
            CurseForgeProjectId::new(17).unwrap(),
            CurseForgeFileId::new(701).unwrap(),
            true,
            &state,
        )
        .await
        .unwrap();

        assert_eq!(
            stage8_pending_keys(&state, &instance_a).await,
            HashSet::from([("17".to_string(), "702".to_string())])
        );
        assert_eq!(
            stage8_pending_keys(&state, &instance_b).await,
            HashSet::from([("17".to_string(), "701".to_string())])
        );
        crate::api::instance::remove(&instance_a).await.unwrap();
        crate::api::instance::remove(&instance_b).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_manual_materialization_detects_source_change() {
        let (state, instance_id) =
            create_stage6_instance("manual copy TOCTOU").await;
        let download = stage8_legacy_manual_download(18, 801, "toctou.jar");
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let source_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join("toctou.jar");
        crate::util::io::write(&source, b"original verified bytes")
            .await
            .unwrap();
        let (verified_size, verified_sha1) =
            sha1_file_async(&source).await.unwrap();
        crate::util::io::write(&source, b"changed after verification")
            .await
            .unwrap();
        let destination = crate::api::instance::get_full_path(&instance_id)
            .await
            .unwrap()
            .join("mods/toctou.jar");
        let previous_bytes = b"previous destination";
        crate::util::io::write(&destination, previous_bytes)
            .await
            .unwrap();

        crate::state::materialize_verified_project_download_copy(
            &source,
            &destination,
            verified_size,
            &verified_sha1,
        )
        .await
        .expect_err("changed source must fail copied-byte identity");

        assert_eq!(
            crate::util::io::read(&destination).await.unwrap(),
            previous_bytes
        );
        assert!(
            stage8_pending_keys(&state, &instance_id)
                .await
                .contains(&("18".to_string(), "801".to_string()))
        );
        let snapshot = crate::api::instance::get_content_snapshot(&instance_id)
            .await
            .unwrap();
        assert!(!snapshot.items.iter().any(|item| {
            item.provider_project_id.as_deref() == Some("18")
                && item.provider_release_id.as_deref() == Some("801")
                && item.materialization_state
                    == crate::state::instances::PackMemberMaterializationState::Present
                && item.content.is_some()
        }));
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_automatic_sha1_record_can_complete_pending() {
        let (state, instance_id) =
            create_stage6_instance("automatic SHA1").await;
        let download =
            stage8_legacy_manual_download(19, 901, "automatic-sha1.jar");
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let relative_path = "mods/automatic-sha1.jar";
        let full_path = crate::api::instance::get_full_path(&instance_id)
            .await
            .unwrap()
            .join(relative_path);
        let bytes = b"automatic SHA1 bytes";
        crate::util::io::write(&full_path, bytes).await.unwrap();
        let (_, sha1) = sha1_file_async(&full_path).await.unwrap();
        let file = stage8_curseforge_file(
            19,
            901,
            "automatic-sha1.jar",
            bytes.len() as u64,
            vec![CurseForgeFileHash {
                value: sha1,
                algo: 1,
            }],
            0,
        );

        record_installed_curseforge_file(
            &instance_id,
            relative_path,
            &full_path,
            &file,
            ProjectType::Mod,
            crate::state::instances::ContentOwnershipKind::PackManaged,
            &state,
        )
        .await
        .unwrap();

        assert!(stage8_pending_keys(&state, &instance_id).await.is_empty());
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_automatic_fingerprint_record_can_complete_pending() {
        let (state, instance_id) =
            create_stage6_instance("automatic fingerprint").await;
        let download = stage8_legacy_manual_download(
            20,
            1001,
            "automatic-fingerprint.jar",
        );
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let relative_path = "mods/automatic-fingerprint.jar";
        let full_path = crate::api::instance::get_full_path(&instance_id)
            .await
            .unwrap()
            .join(relative_path);
        let bytes = b"automatic fingerprint bytes";
        crate::util::io::write(&full_path, bytes).await.unwrap();
        let file = stage8_curseforge_file(
            20,
            1001,
            "automatic-fingerprint.jar",
            bytes.len() as u64,
            Vec::new(),
            compute_fingerprint(bytes) as u64,
        );

        record_installed_curseforge_file(
            &instance_id,
            relative_path,
            &full_path,
            &file,
            ProjectType::Mod,
            crate::state::instances::ContentOwnershipKind::PackManaged,
            &state,
        )
        .await
        .unwrap();

        assert!(stage8_pending_keys(&state, &instance_id).await.is_empty());
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_automatic_weak_record_keeps_pending() {
        let (state, instance_id) =
            create_stage6_instance("automatic weak metadata").await;
        let download =
            stage8_legacy_manual_download(21, 1101, "automatic-md5.jar");
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let relative_path = "mods/automatic-md5.jar";
        let full_path = crate::api::instance::get_full_path(&instance_id)
            .await
            .unwrap()
            .join(relative_path);
        let bytes = b"automatic MD5-only bytes";
        crate::util::io::write(&full_path, bytes).await.unwrap();
        let file = stage8_curseforge_file(
            21,
            1101,
            "automatic-md5.jar",
            bytes.len() as u64,
            vec![CurseForgeFileHash {
                value: "validated-md5".to_string(),
                algo: 2,
            }],
            0,
        );

        record_installed_curseforge_file(
            &instance_id,
            relative_path,
            &full_path,
            &file,
            ProjectType::Mod,
            crate::state::instances::ContentOwnershipKind::PackManaged,
            &state,
        )
        .await
        .unwrap();

        assert!(
            stage8_pending_keys(&state, &instance_id)
                .await
                .contains(&("21".to_string(), "1101".to_string()))
        );
        let snapshot = crate::api::instance::get_content_snapshot(&instance_id)
            .await
            .unwrap();
        assert!(snapshot.items.iter().any(|item| {
            item.provider_project_id.as_deref() == Some("21")
                && item.provider_release_id.as_deref() == Some("1101")
                && item.materialization_state
                    == crate::state::instances::PackMemberMaterializationState::Present
        }));
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_rollback_identity_record_does_not_complete_pending() {
        let (state, instance_id) =
            create_stage6_instance("rollback provenance").await;
        let download = stage8_legacy_manual_download(22, 1201, "rollback.jar");
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let relative_path = "mods/rollback.jar";
        let full_path = crate::api::instance::get_full_path(&instance_id)
            .await
            .unwrap()
            .join(relative_path);
        let bytes = b"historical rollback bytes";
        crate::util::io::write(&full_path, bytes).await.unwrap();
        let (_, sha1) = sha1_file_async(&full_path).await.unwrap();
        let provider_ref = ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(22).unwrap(),
            file_id: Some(CurseForgeFileId::new(1201).unwrap()),
        };

        crate::state::record_project_file_atomic(
            &instance_id,
            relative_path,
            &sha1,
            bytes.len() as u64,
            ProjectType::Mod,
            ContentSourceKind::CurseForge,
            crate::state::instances::ContentOwnershipKind::PackManaged,
            Some(&provider_ref),
            true,
            None,
            &state,
        )
        .await
        .unwrap();

        assert!(
            stage8_pending_keys(&state, &instance_id)
                .await
                .contains(&("22".to_string(), "1201".to_string()))
        );
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_non_pending_import_fails_before_verification() {
        let (state, instance_id) =
            create_stage6_instance("non-pending import").await;
        let source_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join("invalid.jar");
        crate::util::io::write(&source, b"not the required file")
            .await
            .unwrap();

        let error = import_pending_manual_download_file(
            &instance_id,
            601,
            6001,
            source.clone(),
        )
        .await
        .expect_err("missing pending identity must be rejected");

        assert!(matches!(
            error.raw.as_ref(),
            ErrorKind::InputError(message)
                if message
                    == "The selected CurseForge file is not pending for this instance"
        ));
        assert!(source.exists());
        assert!(
            crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
                &instance_id,
                &state.pool,
            )
            .await
            .unwrap()
            .is_empty()
        );
        let target = crate::api::instance::get_full_path(&instance_id)
            .await
            .unwrap()
            .join("mods/invalid.jar");
        assert!(!target.exists());
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn curseforge_wrong_pending_file_fails_integrity() {
        let (state, instance_id) =
            create_stage6_instance("wrong pending file").await;
        let download = stage6_manual_download(
            701,
            7001,
            "expected.jar",
            b"required pending bytes",
        );
        persist_manual_download(&instance_id, &download)
            .await
            .unwrap();
        let source_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join("selected.jar");
        crate::util::io::write(&source, b"wrong pending bytes")
            .await
            .unwrap();

        let error = import_pending_manual_download_file(
            &instance_id,
            download.project_id,
            download.file_id,
            source.clone(),
        )
        .await
        .expect_err("wrong pending file must fail integrity");

        assert!(matches!(
            error.raw.as_ref(),
            ErrorKind::InputError(message)
                if message
                    == "The selected file does not match the required CurseForge file"
        ));
        assert!(source.exists());
        let pending = crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
            &instance_id,
            &state.pool,
        )
        .await
        .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].provider_project_id, "701");
        assert_eq!(pending[0].provider_release_id, "7001");
        let snapshot = crate::api::instance::get_content_snapshot(&instance_id)
            .await
            .unwrap();
        assert!(!snapshot.items.iter().any(|item| {
            item.provider_project_id.as_deref() == Some("701")
                && item.provider_release_id.as_deref() == Some("7001")
                && item.materialization_state
                    == crate::state::instances::PackMemberMaterializationState::Present
                && item.content.is_some()
        }));
        crate::api::instance::remove(&instance_id).await.unwrap();
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn manual_import_completes_persisted_pack_member() {
        crate::event::EventState::init().await.unwrap();
        let state_root = tempfile::tempdir().unwrap().keep();
        let state =
            State::init_for_test(state_root.to_string_lossy().to_string())
                .await
                .unwrap();
        let created = crate::api::instance::create(
            format!("CurseForge drag {}", uuid::Uuid::new_v4()),
            "1.12.2".to_string(),
            ModLoader::Forge,
            Some("14.23.5.2860".to_string()),
            None,
            InstanceLink::Unmanaged,
            None,
        )
        .await
        .unwrap();
        let source_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join("example-mod.jar");
        let bytes = b"verified curseforge drag";
        crate::util::io::write(&source, bytes).await.unwrap();
        let (_, sha1) = sha1_file_async(&source).await.unwrap();
        let download = CurseForgeManualDownload {
            project_id: 123,
            file_id: 456,
            file_name: "example-mod.jar".to_string(),
            ownership_kind:
                crate::state::instances::ContentOwnershipKind::PackManaged,
            operation_kind:
                crate::state::instances::ManualDownloadOperationKind::PackInstall,
            website_url: None,
            project_type: "mod".to_string(),
            project_slug: "example-mod".to_string(),
            target_folder: "mods".to_string(),
            hashes: vec![CurseForgeFileHash {
                value: sha1,
                algo: 1,
            }],
            file_length: bytes.len() as u64,
            file_fingerprint: 0,
        };
        persist_manual_download(&created.instance.id, &download)
            .await
            .unwrap();

        crate::util::io::write(&source, b"same-length-wrong-bytes!")
            .await
            .unwrap();
        assert!(
            import_pending_manual_download_from_path(
                &created.instance.id,
                &source,
            )
            .await
            .is_err()
        );
        assert_eq!(
            crate::state::instances::adapters::sqlite::content_rows::get_pending_manual_downloads(
                &created.instance.id,
                &state.pool,
            )
            .await
            .unwrap()
            .len(),
            1
        );
        let selected = source_directory.path().join("renamed-local-file.jar");
        crate::util::io::write(&selected, bytes).await.unwrap();

        let relative_path = import_pending_manual_download_file(
            &created.instance.id,
            123,
            456,
            selected.clone(),
        )
        .await
        .unwrap()
        .relative_path;
        let snapshot =
            crate::api::instance::get_content_snapshot(&created.instance.id)
                .await
                .unwrap();
        let imported = snapshot
            .items
            .iter()
            .find(|item| {
                item.provider_project_id.as_deref() == Some("123")
                    && item.provider_release_id.as_deref() == Some("456")
            })
            .unwrap();

        assert!(snapshot.pending_manual_downloads.is_empty());
        assert_eq!(
            imported.ownership_kind,
            crate::state::instances::ContentOwnershipKind::PackManaged
        );
        assert_eq!(
            imported.materialization_state,
            crate::state::instances::PackMemberMaterializationState::Present
        );
        assert_eq!(imported.expected_relative_path, relative_path);
        let instance_path = state
            .directories
            .instances_dir()
            .join(created.instance.path)
            .join(relative_path);
        assert_eq!(crate::util::io::read(&instance_path).await.unwrap(), bytes);
        assert!(source.exists());
        assert!(selected.exists());
        crate::util::io::write(&selected, b"mutated selected source")
            .await
            .unwrap();
        assert_eq!(crate::util::io::read(instance_path).await.unwrap(), bytes);
    }

    #[test]
    fn archive_paths_stay_inside_the_instance() {
        assert_eq!(
            safe_archive_relative_path("config/example.toml").unwrap(),
            "config/example.toml"
        );
        assert!(safe_archive_relative_path("../options.txt").is_err());
        assert!(safe_archive_relative_path("/options.txt").is_err());
    }

    #[test]
    fn curseforge_override_content_joins_the_pack_group() {
        assert_eq!(
            curseforge_override_content("mods/cc-tweaked.jar").unwrap(),
            Some(CurseForgePackExpectedOverride {
                project_type: ProjectType::Mod,
                expected_relative_path: "mods/cc-tweaked.jar".to_string(),
            })
        );
        assert_eq!(
            curseforge_override_content("datapacks/sawmill.zip").unwrap(),
            Some(CurseForgePackExpectedOverride {
                project_type: ProjectType::DataPack,
                expected_relative_path: "datapacks/sawmill.zip".to_string(),
            })
        );
        assert_eq!(
            curseforge_override_content("config/example.toml").unwrap(),
            None
        );
    }

    #[test]
    fn cdn_urls_are_restricted_to_forgecdn() {
        assert!(
            validate_cdn_url(
                &reqwest::Url::parse(
                    "https://edge.forgecdn.net/files/1/2/a.jar"
                )
                .unwrap()
            )
            .is_ok()
        );
        assert!(
            validate_cdn_url(
                &reqwest::Url::parse("https://forgecdn.net.evil.test/a.jar")
                    .unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn blank_download_urls_are_treated_as_unavailable() {
        assert_eq!(normalized_download_url(None), None);
        assert_eq!(normalized_download_url(Some(String::new())), None);
        assert_eq!(normalized_download_url(Some("  \t".to_string())), None);
        assert_eq!(
            normalized_download_url(Some(
                " https://edge.forgecdn.net/files/1/2/a.jar ".to_string(),
            )),
            Some("https://edge.forgecdn.net/files/1/2/a.jar".to_string()),
        );
    }

    #[test]
    fn curseforge_loader_ids_map_to_instance_loaders() {
        assert_eq!(loader_family("forge-47.4.0"), "forge");
        assert_eq!(loader_family("fabric-0.16.10"), "fabric");
        assert_eq!(loader_type("neoforge"), Some(6));
    }

    #[test]
    fn modpack_target_uses_primary_forge_loader_and_version() {
        let manifest = CurseForgeModpackManifest {
            minecraft: CurseForgeManifestMinecraft {
                version: "1.12.2".to_string(),
                mod_loaders: vec![CurseForgeManifestLoader {
                    id: "forge-14.23.5.2860".to_string(),
                    primary: true,
                }],
            },
            files: Vec::new(),
            overrides: "overrides".to_string(),
            name: None,
            version: None,
        };

        assert_eq!(
            modpack_target(&manifest).unwrap(),
            CurseForgeModpackTarget {
                game_version: "1.12.2".to_string(),
                loader: ModLoader::Forge,
                loader_version: Some("14.23.5.2860".to_string()),
            }
        );
    }

    #[test]
    fn modpack_target_without_loader_is_vanilla() {
        let manifest = CurseForgeModpackManifest {
            minecraft: CurseForgeManifestMinecraft {
                version: "1.20.1".to_string(),
                mod_loaders: Vec::new(),
            },
            files: Vec::new(),
            overrides: "overrides".to_string(),
            name: None,
            version: None,
        };

        assert_eq!(
            modpack_target(&manifest).unwrap(),
            CurseForgeModpackTarget {
                game_version: "1.20.1".to_string(),
                loader: ModLoader::Vanilla,
                loader_version: None,
            }
        );
    }

    #[test]
    fn recognized_project_types_map_curseforge_classes() {
        assert_eq!(recognized_project_type(Some(6)), Some(ProjectType::Mod));
        assert_eq!(
            recognized_project_type(Some(12)),
            Some(ProjectType::ResourcePack),
        );
        assert_eq!(
            recognized_project_type(Some(6552)),
            Some(ProjectType::ShaderPack),
        );
        assert_eq!(
            recognized_project_type(Some(6945)),
            Some(ProjectType::DataPack),
        );
        assert_eq!(
            recognized_project_type(Some(17)),
            Some(ProjectType::WorldSave),
        );
        assert_eq!(recognized_project_type(Some(4471)), None);
        assert_eq!(recognized_project_type(None), None);
    }
}
