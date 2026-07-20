use crate::event::LoadingBarType;
use crate::event::emit::{
    emit_loading, init_loading, loading_try_for_each_concurrent,
};
use crate::install::{
    InstallJobEventKind, InstallPhaseDetails, InstallPhaseId, InstallProgress,
    InstallProgressReporter, InstallProgressSecondary,
};
use crate::state::ContentProvider;
use crate::state::{
    ContentSourceKind, EditInstance, InstanceLink, ModLoader, ProjectType,
};
use crate::util::fetch::{DownloadMirrorSettings, resolve_download_url};
use crate::{ErrorKind, State};
use bytes::{Bytes, BytesMut};
use futures::{StreamExt, stream};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Seek};
use std::path::{Component, Path};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

const API_BASE_URL: &str = "https://api.curseforge.com";
const API_MIRROR_BASE_URL: &str = "https://mod.mcimirror.top/curseforge";
const MINECRAFT_GAME_ID: u32 = 432;
const MAX_PAGE_SIZE: u32 = 50;
const MODPACK_CONTENT_DOWNLOAD_CONCURRENCY: usize = 8;
const MODPACK_FILE_INSTALL_ATTEMPTS: usize = 3;
const CDN_DOWNLOAD_ATTEMPTS: usize = 3;
const CDN_MAX_REDIRECTS: usize = 5;
const CDN_PROGRESS_LOG_INTERVAL: u64 = 8 * 1024 * 1024;
const PROJECT_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

static UNAUTHORIZED: AtomicBool = AtomicBool::new(false);
static CATEGORY_CACHE: LazyLock<RwLock<Option<Vec<CurseForgeCategory>>>> =
    LazyLock::new(|| RwLock::new(None));
static PROJECT_CACHE: LazyLock<RwLock<HashMap<u32, CachedCurseForgeProject>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static MODPACK_ARCHIVE_CACHE: LazyLock<
    Mutex<Option<CachedCurseForgeModpackArchive>>,
> = LazyLock::new(|| Mutex::new(None));

#[derive(Clone)]
struct CachedCurseForgeModpackArchive {
    project_id: u32,
    file_id: u32,
    bytes: Bytes,
}

#[derive(Clone)]
struct CachedCurseForgeProject {
    project: CurseForgeProject,
    cached_at: Instant,
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
    pub categories: Vec<String>,
    pub versions: Vec<String>,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub date_created: String,
    pub date_modified: String,
    pub latest_version: Option<String>,
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
    #[serde(default)]
    pub categories: Vec<CurseForgeCategory>,
    pub class_id: Option<u32>,
    #[serde(default)]
    pub authors: Vec<CurseForgeAuthor>,
    pub logo: Option<CurseForgeAsset>,
    #[serde(default)]
    pub screenshots: Vec<CurseForgeAsset>,
    pub main_file_id: u32,
    #[serde(default)]
    pub latest_files: Vec<CurseForgeFile>,
    #[serde(default)]
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
    #[serde(default)]
    pub hashes: Vec<CurseForgeFileHash>,
    pub file_date: String,
    pub file_length: u64,
    pub download_count: u64,
    pub file_size_on_disk: Option<u64>,
    pub download_url: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub sortable_game_versions: Vec<CurseForgeSortableGameVersion>,
    #[serde(default)]
    pub dependencies: Vec<CurseForgeFileDependency>,
    pub expose_as_alternative: Option<bool>,
    pub parent_project_file_id: Option<u32>,
    pub alternate_file_id: Option<u32>,
    pub is_server_pack: Option<bool>,
    pub server_pack_file_id: Option<u32>,
    pub is_early_access_content: Option<bool>,
    pub early_access_end_date: Option<String>,
    pub file_fingerprint: u64,
    #[serde(default)]
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
    pub website_url: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeInstallResult {
    pub installed: Vec<CurseForgeInstalledFile>,
    pub manual_downloads: Vec<CurseForgeManualDownload>,
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
    files: Vec<CurseForgeManifestFile>,
    overrides: String,
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
struct CurseForgeManifestFile {
    // CurseForge modpack manifests use projectID/fileID (capital ID), not projectId.
    #[serde(alias = "projectID", alias = "projectId")]
    project_id: u32,
    #[serde(alias = "fileID", alias = "fileId")]
    file_id: u32,
    #[serde(default = "default_true")]
    required: bool,
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
    if let Some(project) = cached_project(project_id) {
        return Ok(project);
    }
    let response: CurseForgeResponse<CurseForgeProject> = request_json(
        Method::GET,
        &format!("/v1/mods/{project_id}"),
        Vec::new(),
        None,
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    cache_projects(std::slice::from_ref(&response.data));
    Ok(response.data)
}

pub async fn get_projects(
    project_ids: Vec<u32>,
) -> crate::Result<Vec<CurseForgeProject>> {
    let mut projects = Vec::new();
    let mut missing_ids = Vec::new();
    for project_id in project_ids.into_iter().collect::<HashSet<_>>() {
        if let Some(project) = cached_project(project_id) {
            projects.push(project);
        } else {
            missing_ids.push(project_id);
        }
    }
    if missing_ids.is_empty() {
        return Ok(projects);
    }
    let response: CurseForgeResponse<Vec<CurseForgeProject>> = request_json(
        Method::POST,
        "/v1/mods",
        Vec::new(),
        Some(json!({ "modIds": missing_ids, "filterPcOnly": true })),
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    cache_projects(&response.data);
    projects.extend(response.data);
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

pub async fn get_files(
    project_id: u32,
    request: CurseForgeFilesRequest,
) -> crate::Result<CurseForgeFilesResponse> {
    let page_size = request.page_size.clamp(1, MAX_PAGE_SIZE);
    let mut query = vec![
        ("index".to_string(), request.index.to_string()),
        ("pageSize".to_string(), page_size.to_string()),
    ];
    push_query(&mut query, "gameVersion", request.game_version);
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
        index: request.index,
        page_size,
        result_count: response.data.len() as u32,
        total_count: response.data.len() as u32,
    });

    Ok(CurseForgeFilesResponse {
        files: response.data,
        pagination,
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
    let response: CurseForgeResponse<Option<String>> = request_json(
        Method::GET,
        &format!("/v1/mods/{project_id}/files/{file_id}/download-url"),
        Vec::new(),
        None,
        MirrorPolicy::MirrorFirst,
    )
    .await?;
    Ok(response.data)
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
                    2 => {
                        result.optional_dependencies.push(dependency_ref.mod_id)
                    }
                    5 => result
                        .incompatible_dependencies
                        .push(dependency_ref.mod_id),
                    3 | 6 => {
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
                                ProjectType::Mod,
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
            result.manual_downloads.push(CurseForgeManualDownload {
                project_id,
                file_id,
                file_name: file.file_name,
                website_url: project.links.website_url,
            });
            continue;
        };

        validate_file_name(&file.file_name)?;
        let bytes = download_cdn_file_with_attempts(&download_url, 1).await?;
        verify_file(&file, &bytes)?;
        let relative_path = write_installed_file(
            &request.instance_id,
            &file.file_name,
            &bytes,
            item_type,
            request.world_name.as_deref(),
        )
        .await?;
        register_provider_ref(
            &request.instance_id,
            &relative_path,
            project_id,
            file_id,
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
    let pack_bytes = download_cdn_file_with_progress(
        &download_url,
        CDN_DOWNLOAD_ATTEMPTS,
        Some(&loading_bar),
    )
    .await?;
    verify_file(&pack_file, &pack_bytes)?;
    let parse_bytes = pack_bytes.clone();
    let target = tokio::task::spawn_blocking(move || {
        let mut archive = zip::ZipArchive::new(Cursor::new(&parse_bytes[..]))
            .map_err(modpack_zip_error)?;
        let manifest = read_modpack_manifest(&mut archive)?;
        modpack_target(&manifest)
    })
    .await??;
    cache_modpack_archive(project_id, file_id, pack_bytes);
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
        return Ok(CurseForgeModpackInstallResult {
            content: CurseForgeInstallResult {
                manual_downloads: vec![CurseForgeManualDownload {
                    project_id: request.project_id,
                    file_id: request.file_id,
                    file_name: pack_file.file_name,
                    website_url: project.links.website_url,
                }],
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

    // Persist the managed-pack association as early as possible so the instance
    // settings UI can treat this like a Modrinth-linked pack even if later
    // content downloads partially fail.
    crate::api::instance::edit(
        &request.instance_id,
        EditInstance {
            name: Some(project.name.clone()),
            icon_path: cached_icon_path
                .as_ref()
                .map(|path| Some(path.to_string_lossy().to_string())),
            link: Some(InstanceLink::CurseForgeModpack {
                project_id: request.project_id.to_string(),
                version_id: request.file_id.to_string(),
            }),
            content_set_patch: Some(crate::state::AppliedContentSetPatch {
                source_kind: Some(ContentSourceKind::CurseForge),
                game_version: None,
                protocol_version: None,
                loader: None,
                loader_version: None,
            }),
            ..EditInstance::default()
        },
    )
    .await?;

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
    let pack_bytes = if let Some(bytes) =
        cached_modpack_archive(request.project_id, request.file_id)
    {
        tracing::info!(
            project_id = request.project_id,
            file_id = request.file_id,
            bytes = bytes.len(),
            "Using cached CurseForge modpack archive"
        );
        bytes
    } else {
        download_cdn_file(&download_url).await?
    };
    if let Some(reporter) = reporter.as_ref() {
        reporter
            .update(
                InstallPhaseId::DownloadingPackFile,
                Some(InstallProgress {
                    current: pack_bytes.len() as u64,
                    total: pack_file.file_length.max(pack_bytes.len() as u64),
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
    verify_file(&pack_file, &pack_bytes)?;
    let (manifest, overrides) =
        tokio::task::spawn_blocking(move || read_modpack_archive(&pack_bytes))
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
        return Err(ErrorKind::InputError(format!(
            "This modpack targets Minecraft {} with {}, while the selected instance uses {} with {}",
            manifest.minecraft.version,
            loader.as_deref().unwrap_or("vanilla"),
            instance_game_version,
            instance_loader
        ))
        .into());
    }

    // Keep the linked pack metadata in sync with the resolved pack target.
    crate::api::instance::edit(
        &request.instance_id,
        EditInstance {
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

    let selected_files = manifest
        .files
        .into_iter()
        .filter(|file| file.required || request.install_optional)
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
    let projects = Arc::new(projects);
    let file_meta = Arc::new(file_meta);
    let files_done = Arc::new(AtomicU64::new(0));
    let bytes_done = Arc::new(AtomicU64::new(0));
    let active_downloads = Arc::new(AtomicU64::new(0));
    let instance_id = request.instance_id.clone();
    let minecraft_version = manifest.minecraft.version.clone();

    loading_try_for_each_concurrent(
        stream::iter(selected_files.into_iter().map(Ok::<_, crate::Error>)),
        Some(MODPACK_CONTENT_DOWNLOAD_CONCURRENCY),
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
                    match install_file(CurseForgeInstallRequest {
                        instance_id: instance_id.clone(),
                        project_id: manifest_file.project_id,
                        file_id: manifest_file.file_id,
                        project_type: project_type.to_string(),
                        game_version: Some(minecraft_version.clone()),
                        mod_loader_type: loader_type_value,
                        world_name: None,
                        install_dependencies: false,
                    })
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
                            failed_result = Some(item_result);
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
                    if failed_result.manual_downloads.is_empty() {
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
                        failed_result.manual_downloads.push(
                            CurseForgeManualDownload {
                                project_id: manifest_file.project_id,
                                file_id: manifest_file.file_id,
                                file_name: file_name.clone(),
                                website_url: project.links.website_url.clone(),
                            },
                        );
                    }
                    let manual_download =
                        failed_result.manual_downloads.first().cloned();
                    let skipped_path = manual_download
                        .as_ref()
                        .map(|file| file.file_name.clone())
                        .unwrap_or_else(|| {
                            format!(
                                "project-{}-file-{}",
                                manifest_file.project_id, manifest_file.file_id
                            )
                        });
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
                        InstallJobEventKind::ContentFileSkipped {
                            path: skipped_path,
                            reason: format!(
                                "Failed after {} attempts: {}",
                                MODPACK_FILE_INSTALL_ATTEMPTS, failure_reason
                            ),
                            project_id: Some(
                                manifest_file.project_id.to_string(),
                            ),
                            version_id: Some(manifest_file.file_id.to_string()),
                            manual_url: manual_download
                                .and_then(|file| file.website_url),
                        },
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

    let instance_path =
        crate::api::instance::get_full_path(&request.instance_id).await?;
    if let Some(reporter) = reporter.as_ref() {
        reporter
            .update(InstallPhaseId::ExtractingOverrides, None, pack_details)
            .await?;
    }
    let mut overrides_written = 0;
    for (relative_path, bytes) in overrides {
        let target = instance_path.join(&relative_path);
        crate::util::fetch::write(
            &target,
            &Bytes::from(bytes),
            &state.io_semaphore,
        )
        .await?;
        overrides_written += 1;
    }

    clear_cached_modpack_archive(request.project_id, request.file_id);
    Ok(CurseForgeModpackInstallResult {
        content,
        overrides_written,
        minecraft_version: manifest.minecraft.version,
        loader,
    })
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

    // Replace previous pack contents before installing the new file set.
    remove_existing_curseforge_pack_content(instance_id, &metadata, &state)
        .await?;

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

    crate::api::instance::edit(
        instance_id,
        EditInstance {
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

    install_modpack(CurseForgeModpackInstallRequest {
        instance_id: instance_id.to_string(),
        project_id,
        file_id,
        install_optional: false,
    })
    .await
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

async fn remove_existing_curseforge_pack_content(
    instance_id: &str,
    metadata: &crate::state::InstanceMetadata,
    state: &State,
) -> crate::Result<()> {
    use crate::state::instances::adapters::sqlite::content_rows;

    let entries = content_rows::get_content_entries(
        &metadata.applied_content_set.id,
        &state.pool,
    )
    .await?;
    let files = content_rows::get_instance_files(instance_id, &state.pool)
        .await?
        .into_iter()
        .map(|file| (file.id.clone(), file))
        .collect::<HashMap<_, _>>();
    let base = state
        .directories
        .instances_dir()
        .join(&metadata.instance.path);

    let mut removed_file_ids = HashSet::new();
    for entry in entries {
        if entry.source_kind != ContentSourceKind::CurseForge {
            continue;
        }
        let Some(file_id) = entry.file_id else {
            continue;
        };
        if !removed_file_ids.insert(file_id.clone()) {
            continue;
        }
        let Some(file) = files.get(&file_id) else {
            continue;
        };
        let _ =
            crate::util::io::remove_file(base.join(&file.relative_path)).await;
        content_rows::remove_content_entries_for_file(
            &metadata.applied_content_set.id,
            &file.id,
            &state.pool,
        )
        .await?;
        content_rows::remove_instance_file_by_relative_path(
            instance_id,
            &file.relative_path,
            &state.pool,
        )
        .await?;
    }

    Ok(())
}

fn read_modpack_archive(
    bytes: &Bytes,
) -> crate::Result<(CurseForgeModpackManifest, Vec<(String, Vec<u8>)>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes[..]))
        .map_err(modpack_zip_error)?;
    let manifest = read_modpack_manifest(&mut archive)?;
    let prefix = format!("{}/", manifest.overrides.trim_matches('/'));
    let mut output = Vec::new();
    let mut total_size = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(modpack_zip_error)?;
        if entry.is_dir() || !entry.name().starts_with(&prefix) {
            continue;
        }
        let relative = &entry.name()[prefix.len()..];
        let safe_path = safe_archive_relative_path(relative)?;
        total_size = total_size.saturating_add(entry.size());
        if total_size > 2 * 1024 * 1024 * 1024 {
            return Err(ErrorKind::InputError(
                "CurseForge modpack overrides exceed the extraction limit"
                    .to_string(),
            )
            .into());
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data)?;
        output.push((safe_path, data));
    }
    Ok((manifest, output))
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

fn loader_family(loader_id: &str) -> &str {
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
        "SELECT ref.project_id, ref.version_id, entry.project_type,
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
        .try_get::<String, _>("project_id")?
        .parse::<u32>()
        .map_err(|_| {
            ErrorKind::InputError(
                "Stored CurseForge project ID is invalid".to_string(),
            )
        })?;
    let current_file_id = row
        .try_get::<Option<String>, _>("version_id")?
        .and_then(|value| value.parse::<u32>().ok());
    let project_type = row.try_get::<String, _>("project_type")?;
    let game_version = row.try_get::<String, _>("game_version")?;
    let loader = row.try_get::<String, _>("loader")?;
    let mod_loader_type = match loader.as_str() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    };
    let latest = get_files(
        project_id,
        CurseForgeFilesRequest {
            game_version: Some(game_version.clone()),
            mod_loader_type,
            game_version_type_id: None,
            index: 0,
            page_size: MAX_PAGE_SIZE,
        },
    )
    .await?
    .files
    .into_iter()
    .find(|file| file.is_available)
    .ok_or_else(|| {
        ErrorKind::InputError(
            "No compatible CurseForge update was found".to_string(),
        )
    })?;
    if current_file_id == Some(latest.id) {
        return Ok(CurseForgeInstallResult::default());
    }

    let result = install_file(CurseForgeInstallRequest {
        instance_id: instance_id.to_string(),
        project_id,
        file_id: latest.id,
        project_type,
        game_version: Some(game_version),
        mod_loader_type,
        world_name: None,
        install_dependencies: true,
    })
    .await?;
    if result.installed.iter().any(|file| {
        !file.dependency
            && file.project_id == project_id
            && file.relative_path != relative_path
    }) {
        crate::api::instance::remove_project(instance_id, relative_path)
            .await?;
    }
    Ok(result)
}

pub async fn recognize_instance_files(
    instance_id: &str,
) -> crate::Result<CurseForgeRecognitionResult> {
    let instance_files =
        crate::api::instance::sync_content_files(instance_id).await?;
    let instance_path =
        crate::api::instance::get_full_path(instance_id).await?;
    let mut fingerprints = Vec::new();
    let mut paths_by_fingerprint = HashMap::<u64, Vec<String>>::new();

    for file in instance_files.into_iter().filter(|file| !file.missing) {
        let bytes =
            tokio::fs::read(instance_path.join(&file.relative_path)).await?;
        let fingerprint = compute_fingerprint(&bytes) as u64;
        fingerprints.push(fingerprint);
        paths_by_fingerprint
            .entry(fingerprint)
            .or_default()
            .push(file.relative_path);
    }

    let mut matches = HashMap::new();
    for chunk in fingerprints.chunks(1000) {
        let response = match_fingerprints(chunk.to_vec()).await?;
        for matched in response.exact_matches {
            matches.insert(matched.file.file_fingerprint, matched.file);
        }
    }

    let mut result = CurseForgeRecognitionResult {
        scanned: fingerprints.len() as u32,
        ..Default::default()
    };
    for (fingerprint, paths) in paths_by_fingerprint {
        if let Some(file) = matches.get(&fingerprint) {
            for path in paths {
                register_provider_ref(instance_id, &path, file.mod_id, file.id)
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
            result.unmatched_paths.extend(paths);
        }
    }
    result.unmatched_paths.sort();
    Ok(result)
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

fn cached_modpack_archive(project_id: u32, file_id: u32) -> Option<Bytes> {
    MODPACK_ARCHIVE_CACHE
        .lock()
        .ok()
        .and_then(|cache| cache.as_ref().cloned())
        .filter(|cached| {
            cached.project_id == project_id && cached.file_id == file_id
        })
        .map(|cached| cached.bytes)
}

fn cache_modpack_archive(project_id: u32, file_id: u32, bytes: Bytes) {
    if let Ok(mut cache) = MODPACK_ARCHIVE_CACHE.lock() {
        *cache = Some(CachedCurseForgeModpackArchive {
            project_id,
            file_id,
            bytes,
        });
    }
}

fn clear_cached_modpack_archive(project_id: u32, file_id: u32) {
    if let Ok(mut cache) = MODPACK_ARCHIVE_CACHE.lock()
        && cache.as_ref().is_some_and(|cached| {
            cached.project_id == project_id && cached.file_id == file_id
        })
    {
        *cache = None;
    }
}

fn cached_project(project_id: u32) -> Option<CurseForgeProject> {
    PROJECT_CACHE
        .read()
        .ok()
        .and_then(|cache| cache.get(&project_id).cloned())
        .filter(|cached| cached.cached_at.elapsed() < PROJECT_CACHE_TTL)
        .map(|cached| cached.project)
}

fn cache_projects(projects: &[CurseForgeProject]) {
    if let Ok(mut cache) = PROJECT_CACHE.write() {
        let cached_at = Instant::now();
        for project in projects {
            cache.insert(
                project.id,
                CachedCurseForgeProject {
                    project: project.clone(),
                    cached_at,
                },
            );
        }
    }
}

async fn download_cdn_file(url: &str) -> crate::Result<Bytes> {
    download_cdn_file_with_attempts(url, CDN_DOWNLOAD_ATTEMPTS).await
}

async fn download_cdn_file_with_attempts(
    url: &str,
    max_attempts: usize,
) -> crate::Result<Bytes> {
    download_cdn_file_with_progress(url, max_attempts, None).await
}

async fn download_cdn_file_with_progress(
    url: &str,
    max_attempts: usize,
    loading_bar: Option<&crate::event::LoadingBarId>,
) -> crate::Result<Bytes> {
    let state = State::get().await?;
    let original_url = reqwest::Url::parse(url).map_err(|_| {
        ErrorKind::InputError(
            "CurseForge returned an invalid download URL".to_string(),
        )
    })?;
    validate_cdn_url(&original_url)?;

    let resolved_url = resolve_download_url(
        url,
        DownloadMirrorSettings {
            curseforge: state.use_curseforge_mirror(),
            ..DownloadMirrorSettings::default()
        },
    );
    let key = api_key();
    let mut routes = Vec::new();
    if resolved_url.is_mirror {
        let mirror_url =
            reqwest::Url::parse(&resolved_url.url).map_err(|_| {
                ErrorKind::InputError(
                    "CurseForge mirror produced an invalid download URL"
                        .to_string(),
                )
            })?;
        validate_cdn_url(&mirror_url)?;
        routes.push((mirror_url, None, RequestRouteSource::Mirror));
        if let Some(key) = key {
            routes.push((
                original_url,
                Some(key),
                RequestRouteSource::Official,
            ));
        }
    } else {
        let key = key.ok_or_else(|| {
            ErrorKind::InputError(
                "CurseForge integration is waiting for an API key".to_string(),
            )
        })?;
        routes.push((original_url, Some(key), RequestRouteSource::Official));
    }
    let mut last_error = String::new();

    for (route_index, (url, key, source)) in routes.iter().enumerate() {
        for attempt in 1..=max_attempts {
            let use_system_proxy = attempt % 2 == 0;
            tracing::info!(
                source = ?source,
                route = route_index + 1,
                attempt,
                max_attempts,
                url = %url,
                use_system_proxy,
                "Attempting CurseForge CDN download"
            );
            let started = Instant::now();
            let attempt_progress = AtomicU64::new(0);
            let result = download_cdn_file_attempt(
                url.clone(),
                key.clone(),
                use_system_proxy,
                attempt,
                loading_bar,
                &attempt_progress,
            )
            .await;
            match result {
                Ok(bytes) => {
                    tracing::info!(
                        attempt,
                        bytes = bytes.len(),
                        elapsed_ms = started.elapsed().as_millis(),
                        "Completed CurseForge CDN download"
                    );
                    return Ok(bytes);
                }
                Err(err) => last_error = err.to_string(),
            }
            let downloaded = attempt_progress.load(Ordering::Relaxed);
            if downloaded > 0
                && let Some(loading_bar) = loading_bar
            {
                emit_loading(
                    loading_bar,
                    -(downloaded as f64),
                    Some("Retrying CurseForge download"),
                )?;
            }
            tracing::warn!(
                attempt,
                max_attempts,
                elapsed_ms = started.elapsed().as_millis(),
                error = %last_error,
                "CurseForge CDN download attempt failed"
            );
            if attempt < max_attempts {
                tokio::time::sleep(Duration::from_millis(250 * attempt as u64))
                    .await;
            }
        }

        if route_index + 1 < routes.len() {
            tracing::warn!(
                source = ?source,
                url = %url,
                error = %last_error,
                "CurseForge mirror failed; falling back to official CDN"
            );
        }
    }

    Err(ErrorKind::OtherError(format!(
        "CurseForge download failed after all routes: {last_error}"
    ))
    .into())
}

async fn download_cdn_file_attempt(
    original_url: reqwest::Url,
    key: Option<String>,
    use_system_proxy: bool,
    attempt: usize,
    loading_bar: Option<&crate::event::LoadingBarId>,
    attempt_progress: &AtomicU64,
) -> crate::Result<Bytes> {
    let state = State::get().await?;
    let mut url = original_url;
    let mut key = key;
    for redirect_count in 0..=CDN_MAX_REDIRECTS {
        validate_cdn_url(&url)?;
        let permit = state.api_semaphore.0.acquire().await?;
        let mut request = request_client(url.as_str(), use_system_proxy)
            .get(url.clone())
            .header("accept", "application/octet-stream");
        if is_forge_cdn_url(&url)
            && let Some(key) = &key
        {
            request = request.header("x-api-key", key);
        }
        let response = request.send().await?;

        if !response.status().is_success() {
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| {
                        ErrorKind::OtherError(
                            "CurseForge CDN redirect omitted its destination"
                                .to_string(),
                        )
                    })?;
                if let Some(response_key) = response
                    .headers()
                    .get("x-api-key")
                    .and_then(|value| value.to_str().ok())
                {
                    key = Some(response_key.to_string());
                }
                if redirect_count == CDN_MAX_REDIRECTS {
                    return Err(ErrorKind::OtherError(
                        "CurseForge CDN returned too many redirects"
                            .to_string(),
                    )
                    .into());
                }
                url = url.join(location).map_err(|_| {
                    ErrorKind::InputError(
                        "CurseForge CDN returned an invalid redirect"
                            .to_string(),
                    )
                })?;
                drop(permit);
                continue;
            }
            return Err(ErrorKind::OtherError(format!(
                "CurseForge download failed with HTTP {}",
                response.status().as_u16()
            ))
            .into());
        }

        let expected_bytes = response.content_length();
        tracing::info!(
            attempt,
            url = %url,
            expected_bytes = ?expected_bytes,
            "Receiving CurseForge CDN response body"
        );
        let mut body = BytesMut::new();
        let mut stream = response.bytes_stream();
        let mut next_progress_log = CDN_PROGRESS_LOG_INTERVAL;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if let Some(loading_bar) = loading_bar {
                emit_loading(
                    loading_bar,
                    chunk.len() as f64,
                    Some("Downloading CurseForge modpack"),
                )?;
            }
            body.extend_from_slice(&chunk);
            let downloaded = body.len() as u64;
            attempt_progress.store(downloaded, Ordering::Relaxed);
            if downloaded >= next_progress_log {
                tracing::info!(
                    attempt,
                    downloaded_bytes = downloaded,
                    expected_bytes = ?expected_bytes,
                    "CurseForge CDN download progress"
                );
                while next_progress_log <= downloaded {
                    next_progress_log = next_progress_log
                        .saturating_add(CDN_PROGRESS_LOG_INTERVAL);
                }
            }
        }
        drop(permit);
        return Ok(body.freeze());
    }

    unreachable!("redirect loop always returns or continues")
}

fn is_forge_cdn_url(url: &reqwest::Url) -> bool {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    host == "forgecdn.net" || host.ends_with(".forgecdn.net")
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

fn verify_file(file: &CurseForgeFile, bytes: &Bytes) -> crate::Result<()> {
    if bytes.len() as u64 != file.file_length {
        return Err(ErrorKind::OtherError(format!(
            "CurseForge file size mismatch for {}",
            file.file_name
        ))
        .into());
    }

    if let Some(expected) = file.hashes.iter().find(|hash| hash.algo == 1) {
        let actual = sha1_smol::Sha1::from(&bytes[..]).hexdigest();
        if !actual.eq_ignore_ascii_case(&expected.value) {
            return Err(ErrorKind::OtherError(format!(
                "CurseForge SHA-1 mismatch for {}",
                file.file_name
            ))
            .into());
        }
    } else if let Some(expected) =
        file.hashes.iter().find(|hash| hash.algo == 2)
    {
        let actual = format!("{:x}", md5::compute(&bytes[..]));
        if !actual.eq_ignore_ascii_case(&expected.value) {
            return Err(ErrorKind::OtherError(format!(
                "CurseForge MD5 mismatch for {}",
                file.file_name
            ))
            .into());
        }
    }

    Ok(())
}

async fn write_installed_file(
    instance_id: &str,
    file_name: &str,
    bytes: &Bytes,
    project_type: ProjectType,
    world_name: Option<&str>,
) -> crate::Result<String> {
    let state = State::get().await?;
    // Pack installs (and normal content installs without a world target) should
    // place datapacks in the instance datapacks/ folder. Only pin to a world
    // when the caller explicitly asks for one.
    if project_type != ProjectType::DataPack || world_name.is_none() {
        return crate::state::add_project_bytes(
            instance_id,
            file_name,
            bytes.clone(),
            None,
            Some(project_type),
            ContentSourceKind::CurseForge,
            None,
            None,
            &state,
        )
        .await;
    }

    let world_name = world_name.expect("checked above");
    validate_file_name(world_name)?;
    let relative_path = format!("saves/{world_name}/datapacks/{file_name}");
    let full_path = crate::api::instance::get_full_path(instance_id)
        .await?
        .join(&relative_path);
    crate::util::fetch::write(&full_path, bytes, &state.io_semaphore).await?;
    let sha1 = sha1_smol::Sha1::from(&bytes[..]).hexdigest();
    crate::state::record_project_file(
        instance_id,
        &relative_path,
        &sha1,
        bytes.len() as u64,
        ProjectType::DataPack,
        ContentSourceKind::CurseForge,
        None,
        None,
        &state,
    )
    .await?;
    Ok(relative_path)
}

async fn register_provider_ref(
    instance_id: &str,
    relative_path: &str,
    project_id: u32,
    file_id: u32,
) -> crate::Result<()> {
    let state = State::get().await?;
    let entry_id = sqlx::query_scalar::<_, String>(
        "SELECT entry.id
         FROM instance_content_entries entry
         INNER JOIN instance_files file ON file.id = entry.file_id
         WHERE entry.instance_id = ? AND file.relative_path = ?
         ORDER BY entry.modified_at DESC
         LIMIT 1",
    )
    .bind(instance_id)
    .bind(relative_path)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ErrorKind::OtherError(
            "Installed CurseForge file was not registered".to_string(),
        )
    })?;

    sqlx::query(
        "INSERT INTO instance_content_provider_refs (
            content_entry_id, provider, project_id, version_id, primary_ref
         ) VALUES (
            ?, 'curseforge', ?, ?,
            CASE WHEN EXISTS (
                SELECT 1 FROM instance_content_provider_refs
                WHERE content_entry_id = ? AND primary_ref = 1
            ) THEN 0 ELSE 1 END
         )
         ON CONFLICT(content_entry_id, provider) DO UPDATE SET
            project_id = excluded.project_id,
            version_id = excluded.version_id",
    )
    .bind(&entry_id)
    .bind(project_id.to_string())
    .bind(file_id.to_string())
    .bind(&entry_id)
    .execute(&state.pool)
    .await?;

    Ok(())
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

fn request_routes(
    path: &str,
    mirror_policy: MirrorPolicy,
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

    let mut routes = Vec::new();
    if mirror_policy == MirrorPolicy::MirrorFirst {
        routes.push(RequestRoute {
            url: format!("{API_MIRROR_BASE_URL}{path}"),
            use_api_key: false,
            use_system_proxy: false,
            source: RequestRouteSource::Mirror,
        });
    }
    routes.push(RequestRoute {
        url: format!("{API_BASE_URL}{path}"),
        use_api_key: true,
        use_system_proxy: false,
        source: RequestRouteSource::Official,
    });
    routes.push(RequestRoute {
        url: format!("{API_BASE_URL}{path}"),
        use_api_key: true,
        use_system_proxy: true,
        source: RequestRouteSource::Official,
    });
    routes
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
    let mirror_policy = if state.use_curseforge_mirror() {
        mirror_policy
    } else {
        MirrorPolicy::OfficialOnly
    };
    let routes = request_routes(path, mirror_policy);
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
            let key = key.as_ref().ok_or_else(|| {
                ErrorKind::InputError(
                    "CurseForge integration is waiting for an API key"
                        .to_string(),
                )
            })?;
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
        && route.source == RequestRouteSource::Mirror
        && status == StatusCode::NOT_FOUND
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
    fn official_http_errors_are_final() {
        let route = RequestRoute {
            url: String::new(),
            use_api_key: true,
            use_system_proxy: false,
            source: RequestRouteSource::Official,
        };

        assert!(!should_try_next_route(&route, StatusCode::FORBIDDEN, true));
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
    fn archive_paths_stay_inside_the_instance() {
        assert_eq!(
            safe_archive_relative_path("config/example.toml").unwrap(),
            "config/example.toml"
        );
        assert!(safe_archive_relative_path("../options.txt").is_err());
        assert!(safe_archive_relative_path("/options.txt").is_err());
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
}
