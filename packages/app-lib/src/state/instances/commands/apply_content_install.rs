use crate::api::content_search::{
    chinese_file_title_for_modrinth_slug, localized_content_file_name,
};
use crate::state::instances::{
    ContentRequirement, ContentSourceKind, Instance, InstanceFile,
    adapters::sqlite::{content_rows, instance_rows},
};
use crate::state::{
    CacheBehaviour, CachedEntry, Dependency, DependencyType, KnownModrinthFile,
    ModLoader, ProjectType, State, Version, cache_file_hash,
    cache_file_hash_metadata,
};
use crate::util::fetch::{
    self, ContentValidation, DownloadMeta, DownloadReason, DownloadRequest,
    Integrity, ResourceClass, download_to_path,
};
use crate::util::io;
use async_trait::async_trait;
use bytes::Bytes;
use modrinth_content_management::{
    ContentMetadataProvider, ContentType, Error as ResolveError,
    ResolutionPreferences, ResolveContentPlan, ResolveContentRequest,
    ResolvedContent,
};
use std::path::{Path, PathBuf};

pub(crate) struct ContentScope {
    pub instance: Instance,
    pub content_set_id: String,
}

pub(crate) struct InstalledContentFile {
    pub relative_path: String,
    pub project_id: Option<String>,
    pub enabled: bool,
}

pub(crate) struct DownloadedProjectVersion {
    pub file_name: String,
    pub path: PathBuf,
    pub sha1: String,
    pub size: u64,
    pub project_type: ProjectType,
    pub project_id: String,
    pub version_id: String,
}

pub(crate) struct InstanceInstallProjectRequest {
    pub project_id: String,
    pub version_id: Option<String>,
    pub content_type: ContentType,
    pub selected: ResolutionPreferences,
}

struct CachedEntryContentProvider<'a> {
    state: &'a State,
    cache_behaviour: Option<CacheBehaviour>,
}

#[async_trait]
impl ContentMetadataProvider for CachedEntryContentProvider<'_> {
    async fn get_version(
        &mut self,
        version_id: &str,
    ) -> Result<Option<modrinth_content_management::Version>, ResolveError>
    {
        let version = CachedEntry::get_version(
            version_id,
            self.cache_behaviour,
            &self.state.pool,
            &self.state.api_semaphore,
        )
        .await
        .map_err(resolve_provider_error)?;

        Ok(version.map(version_to_resolver))
    }

    async fn get_project_versions(
        &mut self,
        project_id: &str,
    ) -> Result<Vec<modrinth_content_management::Version>, ResolveError> {
        let versions = CachedEntry::get_project_versions(
            project_id,
            self.cache_behaviour,
            &self.state.pool,
            &self.state.api_semaphore,
        )
        .await
        .map_err(resolve_provider_error)?;

        Ok(versions
            .unwrap_or_default()
            .into_iter()
            .map(version_to_resolver)
            .collect())
    }
}

fn resolve_provider_error(error: crate::Error) -> ResolveError {
    ResolveError::Provider(error.to_string())
}

fn resolver_error(error: ResolveError) -> crate::Error {
    crate::ErrorKind::InputError(error.to_string()).into()
}

fn version_to_resolver(
    version: Version,
) -> modrinth_content_management::Version {
    modrinth_content_management::Version {
        id: version.id,
        project_id: version.project_id,
        date_published: version.date_published,
        dependencies: version
            .dependencies
            .into_iter()
            .map(dependency_to_resolver)
            .collect(),
        game_versions: version.game_versions,
        loaders: version.loaders,
    }
}

fn dependency_to_resolver(
    dependency: Dependency,
) -> modrinth_content_management::Dependency {
    modrinth_content_management::Dependency {
        version_id: dependency.version_id,
        project_id: dependency.project_id,
        file_name: dependency.file_name,
        dependency_type: match dependency.dependency_type {
            DependencyType::Required => {
                modrinth_content_management::DependencyType::Required
            }
            DependencyType::Optional => {
                modrinth_content_management::DependencyType::Optional
            }
            DependencyType::Incompatible => {
                modrinth_content_management::DependencyType::Incompatible
            }
            DependencyType::Embedded => {
                modrinth_content_management::DependencyType::Embedded
            }
        },
    }
}

fn target_preferences(
    game_version: String,
    loader: ModLoader,
    content_type: ContentType,
) -> ResolutionPreferences {
    let loader = match content_type {
        ContentType::DataPack => "datapack".to_string(),
        ContentType::ResourcePack => "minecraft".to_string(),
        ContentType::Shader => "iris".to_string(),
        _ => loader.as_str().to_string(),
    };

    ResolutionPreferences {
        game_versions: vec![game_version],
        loaders: vec![loader],
    }
}

pub(crate) async fn resolve_install_plan(
    instance_id: &str,
    request: InstanceInstallProjectRequest,
    state: &State,
) -> crate::Result<ResolveContentPlan> {
    let content_set =
        content_rows::get_applied_content_set(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Instance {instance_id} has no applied content set"
                ))
            })?;
    let existing_project_ids =
        crate::state::get_installed_project_ids_for_instance(
            instance_id,
            None,
            state,
        )
        .await?;
    let provider = CachedEntryContentProvider {
        state,
        cache_behaviour: Some(CacheBehaviour::MustRevalidate),
    };
    let content_type = request.content_type;
    let request = ResolveContentRequest {
        project_id: request.project_id,
        version_id: request.version_id,
        content_type,
        selected: request.selected,
        target: target_preferences(
            content_set.game_version,
            content_set.loader,
            content_type,
        ),
        existing_project_ids,
    };

    modrinth_content_management::resolve_content(provider, request)
        .await
        .map_err(resolver_error)
}

pub(crate) async fn install_resolved_content_plan(
    instance_id: &str,
    plan: &ResolveContentPlan,
    state: &State,
) -> crate::Result<()> {
    add_resolved_content(
        instance_id,
        &plan.primary,
        DownloadReason::Standalone,
        state,
    )
    .await?;
    for dependency in &plan.dependencies {
        add_resolved_content(
            instance_id,
            dependency,
            DownloadReason::Dependency,
            state,
        )
        .await?;
    }

    Ok(())
}

pub(crate) async fn switch_project_version_with_dependencies(
    instance_id: &str,
    project_path: &str,
    version_id: &str,
    state: &State,
) -> crate::Result<String> {
    let version = CachedEntry::get_version(
        version_id,
        Some(CacheBehaviour::MustRevalidate),
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unable to install version id {version_id}. Not found."
        ))
    })?;
    let content_type = ProjectType::get_from_loaders(version.loaders.clone())
        .map(ContentType::from)
        .unwrap_or(ContentType::Mod);
    let plan = resolve_install_plan(
        instance_id,
        InstanceInstallProjectRequest {
            project_id: version.project_id,
            version_id: Some(version_id.to_string()),
            content_type,
            selected: ResolutionPreferences::default(),
        },
        state,
    )
    .await?;

    let was_disabled = project_path.ends_with(".disabled");
    let mut new_path = add_project_from_version(
        instance_id,
        &plan.primary.version_id,
        DownloadReason::Update,
        None,
        ContentSourceKind::Local,
        state,
    )
    .await?;

    if was_disabled {
        new_path =
            toggle_disable_project(instance_id, &new_path, Some(false), state)
                .await?;
    }

    for dependency in &plan.dependencies {
        add_resolved_content(
            instance_id,
            dependency,
            DownloadReason::Dependency,
            state,
        )
        .await?;
    }

    if new_path != project_path {
        remove_project(instance_id, project_path, state).await?;
    }

    Ok(new_path)
}

async fn add_resolved_content(
    instance_id: &str,
    content: &ResolvedContent,
    reason: DownloadReason,
    state: &State,
) -> crate::Result<String> {
    add_project_from_version(
        instance_id,
        &content.version_id,
        reason,
        content.dependent_on_version_id.clone(),
        ContentSourceKind::Local,
        state,
    )
    .await
}

pub(crate) async fn resolve_content_scope(
    instance_id: &str,
    content_set_id: Option<&str>,
    state: &State,
) -> crate::Result<ContentScope> {
    let instance = instance_rows::get_instance_by_id(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;
    let content_set_id = match content_set_id {
        Some(id) => id.to_string(),
        None => instance.applied_content_set_id.clone().ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Instance {} has no applied content set",
                instance.id
            ))
        })?,
    };

    Ok(ContentScope {
        instance,
        content_set_id,
    })
}

pub(crate) async fn add_project_from_version(
    instance_id: &str,
    version_id: &str,
    reason: DownloadReason,
    dependent_on_version_id: Option<String>,
    source_kind: ContentSourceKind,
    state: &State,
) -> crate::Result<String> {
    let downloaded = download_project_version(
        instance_id,
        version_id,
        reason,
        dependent_on_version_id,
        state,
    )
    .await?;

    add_downloaded_project_version(instance_id, downloaded, source_kind, state)
        .await
}

pub(crate) async fn download_project_version(
    instance_id: &str,
    version_id: &str,
    reason: DownloadReason,
    dependent_on_version_id: Option<String>,
    state: &State,
) -> crate::Result<DownloadedProjectVersion> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let content_set =
        content_rows::get_content_set(&scope.content_set_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Unknown content set {}",
                    scope.content_set_id
                ))
            })?;
    let version = CachedEntry::get_version(
        version_id,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unable to install version id {version_id}. Not found."
        ))
    })?;
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "No files for input version present!".to_string(),
            )
        })?;
    let download_meta = DownloadMeta {
        reason,
        game_version: content_set.game_version,
        loader: content_set.loader.as_str().to_string(),
        dependent_on: dependent_on_version_id,
    };
    let file_name_path = Path::new(&file.filename);
    if file_name_path.as_os_str().is_empty()
        || file_name_path.is_absolute()
        || file_name_path.components().count() != 1
        || !matches!(
            file_name_path.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err(crate::ErrorKind::InputError(
            "Modrinth returned an invalid project file name".to_string(),
        )
        .into());
    }
    let path = state
        .directories
        .caches_dir()
        .join("content")
        .join("modrinth")
        .join(&version.id)
        .join(file_name_path);
    let content = if matches!(
        file_name_path.extension().and_then(|value| value.to_str()),
        Some("jar" | "zip" | "mrpack")
    ) {
        ContentValidation::Jar
    } else {
        ContentValidation::None
    };
    let integrity = Integrity {
        size: Some(file.size as u64),
        sha1: file.hashes.get("sha1").cloned(),
        sha512: file.hashes.get("sha512").cloned(),
        content,
        ..Integrity::default()
    };
    let download = download_to_path(
        DownloadRequest::new(&file.url, ResourceClass::Modrinth)
            .with_integrity(integrity)
            .with_download_meta(download_meta),
        &path,
        &state.download_semaphore,
        &state.pool,
        None,
    )
    .await?;
    let sha1 = if let Some(hash) = file.hashes.get("sha1") {
        hash.clone()
    } else {
        fetch::sha1_file_async(&path).await?.1
    };
    let project_type = ProjectType::get_from_loaders(version.loaders.clone())
        .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unable to infer project type for version {version_id}"
        ))
    })?;
    let project_id = version.project_id.clone();
    let version_id = version.id.clone();

    Ok(DownloadedProjectVersion {
        file_name: file.filename.clone(),
        path,
        sha1,
        size: download.size,
        project_type,
        project_id,
        version_id,
    })
}

/// Chooses the instance-relative path for a content file that is about to be
/// installed. Paths already recorded for this instance always win, so repeat
/// installs never duplicate a project across locale switches; otherwise the
/// `[中文名]` candidate is used when the app language is Simplified Chinese.
pub(crate) async fn resolve_content_install_relative_path(
    instance_id: &str,
    original_relative_path: String,
    localized_candidate: Option<String>,
    pool: &sqlx::SqlitePool,
) -> crate::Result<String> {
    if content_rows::get_instance_file_by_relative_path(
        instance_id,
        &original_relative_path,
        pool,
    )
    .await?
    .is_some()
    {
        return Ok(original_relative_path);
    }
    let Some(localized_candidate) = localized_candidate else {
        return Ok(original_relative_path);
    };
    if content_rows::get_instance_file_by_relative_path(
        instance_id,
        &localized_candidate,
        pool,
    )
    .await?
    .is_some()
    {
        return Ok(localized_candidate);
    }
    if crate::state::Settings::get(pool).await?.locale == "zh-CN" {
        return Ok(localized_candidate);
    }
    Ok(original_relative_path)
}

/// Builds the `[中文名]` file-name candidate for a Modrinth project file by
/// resolving the project's slug from cache. Any failure disables the naming.
async fn modrinth_chinese_file_name_candidate(
    project_id: &str,
    file_name: &str,
    state: &State,
) -> Option<String> {
    let project = CachedEntry::get_project(
        project_id,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await
    .ok()??;
    let title = chinese_file_title_for_modrinth_slug(&project.slug?)?;
    localized_content_file_name(file_name, &title)
}

pub(crate) async fn add_downloaded_project_version(
    instance_id: &str,
    downloaded: DownloadedProjectVersion,
    source_kind: ContentSourceKind,
    state: &State,
) -> crate::Result<String> {
    let DownloadedProjectVersion {
        file_name,
        path,
        sha1,
        size,
        project_type,
        project_id,
        version_id,
    } = downloaded;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let localized_candidate =
        modrinth_chinese_file_name_candidate(&project_id, &file_name, state)
            .await
            .map(|file_name| {
                format!("{}/{}", project_type.get_folder(), file_name)
            });
    let relative_path = resolve_content_install_relative_path(
        instance_id,
        format!("{}/{}", project_type.get_folder(), file_name),
        localized_candidate,
        &state.pool,
    )
    .await?;
    let full_path =
        instance_full_path(state, &scope.instance).join(&relative_path);
    materialize_project_download(&path, &full_path).await?;
    cache_file_hash_metadata(
        &scope.instance.id,
        &relative_path,
        size,
        sha1.clone(),
        Some(project_type),
        Some(KnownModrinthFile {
            project_id: &project_id,
            version_id: &version_id,
        }),
        &state.pool,
    )
    .await?;
    record_project_file(
        instance_id,
        &relative_path,
        &sha1,
        size,
        project_type,
        source_kind,
        Some(&project_id),
        Some(&version_id),
        state,
    )
    .await?;
    Ok(relative_path)
}

async fn materialize_project_download(
    source: &Path,
    destination: &Path,
) -> crate::Result<()> {
    if let Some(parent) = destination.parent() {
        io::create_dir_all(parent).await?;
    }
    let mut temporary = destination.as_os_str().to_os_string();
    temporary.push(".installing");
    let temporary = PathBuf::from(temporary);
    if temporary.exists() {
        io::remove_file(&temporary).await?;
    }
    if tokio::fs::hard_link(source, &temporary).await.is_err() {
        io::copy(source, &temporary).await?;
    }
    if destination.exists() {
        io::remove_file(destination).await?;
    }
    tokio::fs::rename(&temporary, destination).await?;
    Ok(())
}

pub(crate) async fn add_project_from_path(
    instance_id: &str,
    path: &Path,
    project_type: Option<ProjectType>,
    state: &State,
) -> crate::Result<String> {
    let file = io::read(path).await?;
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    add_project_bytes(
        instance_id,
        &file_name,
        Bytes::from(file),
        None,
        project_type,
        ContentSourceKind::Local,
        None,
        None,
        state,
    )
    .await
}

pub(crate) async fn add_project_bytes(
    instance_id: &str,
    file_name: &str,
    bytes: Bytes,
    hash: Option<&str>,
    project_type: Option<ProjectType>,
    source_kind: ContentSourceKind,
    project_id: Option<&str>,
    version_id: Option<&str>,
    state: &State,
) -> crate::Result<String> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let project_type = match project_type {
        Some(project_type) => project_type,
        None => infer_project_type(&bytes)?,
    };
    let relative_path = format!("{}/{}", project_type.get_folder(), file_name);
    let full_path =
        instance_full_path(state, &scope.instance).join(&relative_path);
    let sha1 = match hash {
        Some(hash) => hash.to_string(),
        None => fetch::sha1_async(bytes.clone()).await?,
    };

    cache_file_hash(
        bytes.clone(),
        &scope.instance.id,
        &relative_path,
        Some(&sha1),
        Some(project_type),
        project_id.zip(version_id).map(|(project_id, version_id)| {
            KnownModrinthFile {
                project_id,
                version_id,
            }
        }),
        &state.pool,
    )
    .await?;
    fetch::write(&full_path, &bytes, &state.io_semaphore).await?;

    let file = content_rows::upsert_instance_file_from_parts(
        content_rows::UpsertInstanceFile {
            instance_id: &scope.instance.id,
            relative_path: &relative_path,
            file_name,
            enabled: !relative_path.ends_with(".disabled"),
            sha1: &sha1,
            size: bytes.len() as u64,
            missing: false,
        },
        &state.pool,
    )
    .await?;
    upsert_entry_for_file(
        &scope,
        &file,
        project_type,
        project_id,
        version_id,
        source_kind,
        &state.pool,
    )
    .await?;

    Ok(relative_path)
}

pub(crate) async fn record_project_file(
    instance_id: &str,
    relative_path: &str,
    sha1: &str,
    size: u64,
    project_type: ProjectType,
    source_kind: ContentSourceKind,
    project_id: Option<&str>,
    version_id: Option<&str>,
    state: &State,
) -> crate::Result<()> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let file_name = Path::new(relative_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file = content_rows::upsert_instance_file_from_parts(
        content_rows::UpsertInstanceFile {
            instance_id: &scope.instance.id,
            relative_path,
            file_name: &file_name,
            enabled: !relative_path.ends_with(".disabled"),
            sha1,
            size,
            missing: false,
        },
        &state.pool,
    )
    .await?;
    upsert_entry_for_file(
        &scope,
        &file,
        project_type,
        project_id,
        version_id,
        source_kind,
        &state.pool,
    )
    .await
}

pub(crate) async fn toggle_disable_project(
    instance_id: &str,
    project_path: &str,
    desired_enabled: Option<bool>,
    state: &State,
) -> crate::Result<String> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let base = instance_full_path(state, &scope.instance);
    let trimmed = project_path.trim_end_matches(".disabled");
    let current_path = if base.join(project_path).exists() {
        project_path.to_string()
    } else if base.join(format!("{trimmed}.disabled")).exists() {
        format!("{trimmed}.disabled")
    } else if base.join(trimmed).exists() {
        trimmed.to_string()
    } else {
        return Err(crate::ErrorKind::FSError(format!(
            "Could not find project file for '{project_path}' in instance"
        ))
        .into());
    };
    let current_enabled = !current_path.ends_with(".disabled");
    let enabled = desired_enabled.unwrap_or(!current_enabled);
    let new_path = if enabled {
        trimmed.to_string()
    } else {
        format!("{trimmed}.disabled")
    };

    if current_path != new_path {
        io::rename_or_move(&base.join(&current_path), &base.join(&new_path))
            .await?;
    }

    let file_name = Path::new(&new_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file = match content_rows::rename_instance_file(
        &scope.instance.id,
        &current_path,
        &new_path,
        &file_name,
        enabled,
        &state.pool,
    )
    .await?
    {
        Some(file) => file,
        None if current_path != project_path => {
            match content_rows::rename_instance_file(
                &scope.instance.id,
                project_path,
                &new_path,
                &file_name,
                enabled,
                &state.pool,
            )
            .await?
            {
                Some(file) => file,
                None => index_existing_file(&scope, &new_path, state).await?,
            }
        }
        None => index_existing_file(&scope, &new_path, state).await?,
    };
    let updated_entry = content_rows::set_content_entry_enabled_for_file(
        &scope.content_set_id,
        &file.id,
        enabled,
        &state.pool,
    )
    .await?;
    if !updated_entry {
        let project_type = ProjectType::get_from_parent_folder(&new_path)
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Unable to infer project type from {new_path}"
                ))
            })?;
        upsert_entry_for_file(
            &scope,
            &file,
            project_type,
            None,
            None,
            ContentSourceKind::Local,
            &state.pool,
        )
        .await?;
    }

    Ok(new_path)
}

pub(crate) async fn remove_project(
    instance_id: &str,
    project_path: &str,
    state: &State,
) -> crate::Result<()> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let base = instance_full_path(state, &scope.instance);
    let file = content_rows::get_instance_file_by_relative_path(
        &scope.instance.id,
        project_path,
        &state.pool,
    )
    .await?;

    io::remove_file(base.join(project_path)).await?;

    if let Some(file) = file {
        content_rows::remove_content_entries_for_file(
            &scope.content_set_id,
            &file.id,
            &state.pool,
        )
        .await?;
        content_rows::remove_instance_file_by_relative_path(
            &scope.instance.id,
            project_path,
            &state.pool,
        )
        .await?;
    }

    Ok(())
}

pub(crate) async fn list_project_files(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<InstalledContentFile>> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let entries =
        content_rows::get_content_entries(&scope.content_set_id, &state.pool)
            .await?;
    let files =
        content_rows::get_instance_files(&scope.instance.id, &state.pool)
            .await?
            .into_iter()
            .map(|file| (file.id.clone(), file))
            .collect::<std::collections::HashMap<_, _>>();

    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let file = files.get(entry.file_id.as_ref()?)?;
            Some(InstalledContentFile {
                relative_path: file.relative_path.clone(),
                project_id: entry.project_id,
                enabled: entry.enabled && file.enabled,
            })
        })
        .collect())
}

pub(crate) fn instance_full_path(
    state: &State,
    instance: &Instance,
) -> PathBuf {
    state.directories.instances_dir().join(&instance.path)
}

async fn index_existing_file(
    scope: &ContentScope,
    relative_path: &str,
    state: &State,
) -> crate::Result<InstanceFile> {
    let full_path =
        instance_full_path(state, &scope.instance).join(relative_path);
    let (size, sha1) = fetch::sha1_file_async(&full_path).await?;
    let file_name = Path::new(relative_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let project_type = ProjectType::get_from_parent_folder(relative_path)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Unable to infer project type from {relative_path}"
            ))
        })?;

    let file = content_rows::upsert_instance_file_from_parts(
        content_rows::UpsertInstanceFile {
            instance_id: &scope.instance.id,
            relative_path,
            file_name: &file_name,
            enabled: !relative_path.ends_with(".disabled"),
            sha1: &sha1,
            size,
            missing: false,
        },
        &state.pool,
    )
    .await?;
    upsert_entry_for_file(
        scope,
        &file,
        project_type,
        None,
        None,
        ContentSourceKind::Local,
        &state.pool,
    )
    .await?;

    Ok(file)
}

async fn upsert_entry_for_file(
    scope: &ContentScope,
    file: &InstanceFile,
    project_type: ProjectType,
    project_id: Option<&str>,
    version_id: Option<&str>,
    source_kind: ContentSourceKind,
    pool: &sqlx::SqlitePool,
) -> crate::Result<()> {
    content_rows::upsert_content_entry_from_parts(
        content_rows::UpsertContentEntry {
            instance_id: &scope.instance.id,
            content_set_id: &scope.content_set_id,
            file_id: Some(&file.id),
            project_type,
            project_id,
            version_id,
            source_kind,
            server_requirement: ContentRequirement::Required,
            client_requirement: ContentRequirement::Required,
            enabled: file.enabled,
        },
        pool,
    )
    .await?;

    Ok(())
}

fn infer_project_type(bytes: &Bytes) -> crate::Result<ProjectType> {
    let cursor = std::io::Cursor::new(&**bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|_| {
        crate::ErrorKind::InputError(
            "Unable to infer project type for input file".to_string(),
        )
    })?;

    if archive.by_name("fabric.mod.json").is_ok()
        || archive.by_name("quilt.mod.json").is_ok()
        || archive.by_name("META-INF/neoforge.mods.toml").is_ok()
        || archive.by_name("META-INF/mods.toml").is_ok()
        || archive.by_name("mcmod.info").is_ok()
    {
        Ok(ProjectType::Mod)
    } else if archive.by_name("pack.mcmeta").is_ok() {
        if archive.file_names().any(|name| name.starts_with("data/")) {
            Ok(ProjectType::DataPack)
        } else {
            Ok(ProjectType::ResourcePack)
        }
    } else if archive
        .file_names()
        .any(|name| name.starts_with("shaders/"))
    {
        Ok(ProjectType::ShaderPack)
    } else {
        Err(crate::ErrorKind::InputError(
            "Unable to infer project type for input file".to_string(),
        )
        .into())
    }
}
