use super::sync_content_files::{
    project_type_for_file, sync_instance_content_files,
};
use crate::State;
use crate::pack::install_from::{PackFileHash, PackFormat};
use crate::state::instances::adapters::sqlite;
use crate::state::instances::{
    ContentSet, ContentSourceKind, Instance, InstanceInstallCandidate,
    InstanceInstallTarget, InstanceLink,
};
use crate::state::{
    CacheBehaviour, CachedEntry, ContentFile, ContentItem, ContentItemOwner,
    ContentItemProject, ContentItemUpdate, ContentItemVersion, ContentProvider,
    ContentProviderRef, Dependency, LinkedModpackInfo, ModLoader,
    ModrinthFileMatch, ModrinthProjectId, ModrinthVersionId, Organization,
    OwnerType, Project, ProjectType, ReleaseChannel, TeamMember, Version,
};
use crate::util::fetch::{
    ContentValidation, DownloadMeta, DownloadReason, DownloadRequest,
    FetchSemaphore, Integrity, ResourceClass, download_to_path, sha1_async,
};
use async_zip::tokio::read::fs::ZipFileReader;
use dashmap::DashMap;
use sqlx::{Row, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};

#[derive(Clone, Debug)]
struct ResolvedContentScope {
    instance: Instance,
    content_set: ContentSet,
}

#[derive(Clone, Copy, Debug)]
enum ContentFilter<'a> {
    All,
    ExcludeModpack(&'a ModpackIdentifiers),
    ExcludeSourceKind {
        source_kind: ContentSourceKind,
        exclude_untracked: bool,
    },
    OnlyModpack(&'a ModpackIdentifiers),
    OnlySourceKind {
        source_kind: ContentSourceKind,
        include_untracked: bool,
    },
}

pub(crate) async fn list_content_sets(
    instance_id: &str,
    pool: &SqlitePool,
) -> crate::Result<Vec<ContentSet>> {
    let instance = sqlite::instance_rows::get_instance_by_id(instance_id, pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;

    sqlite::content_rows::get_content_sets_for_instance(&instance.id, pool)
        .await
}

pub(crate) async fn get_content_projects(
    instance_id: &str,
    content_set_id: Option<&str>,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<DashMap<String, ContentFile>> {
    let resolved = resolve_content_scope_with_instance(
        instance_id,
        content_set_id,
        &state.pool,
    )
    .await?;

    content_projects_for_scope(
        &resolved,
        cache_behaviour,
        state,
        ContentFilter::All,
    )
    .await
}

pub(crate) async fn get_installed_project_ids_for_instance(
    instance_id: &str,
    content_set_id: Option<&str>,
    state: &State,
) -> crate::Result<Vec<String>> {
    let resolved = resolve_content_scope_with_instance(
        instance_id,
        content_set_id,
        &state.pool,
    )
    .await?;
    let projects =
        get_content_projects(instance_id, content_set_id, None, state).await?;

    let mut project_ids = projects
        .into_iter()
        .filter_map(|(_, file)| {
            file.modrinth
                .map(|metadata| metadata.project_id.to_string())
        })
        .collect::<HashSet<_>>();
    let provider_rows = sqlx::query(
        "SELECT DISTINCT ref.provider, ref.provider_project_id
         FROM instance_content_entries entry
         INNER JOIN instance_files file ON file.id = entry.file_id
         INNER JOIN instance_content_provider_refs ref
            ON ref.content_entry_id = entry.id
         WHERE entry.content_set_id = ? AND file.missing = 0",
    )
    .bind(&resolved.content_set.id)
    .fetch_all(&state.pool)
    .await?;
    for row in provider_rows {
        let provider = row.try_get::<String, _>("provider")?;
        let project_id = row.try_get::<String, _>("provider_project_id")?;
        if provider == "curseforge" {
            project_ids.insert(format!("curseforge:{project_id}"));
        } else {
            project_ids.insert(project_id);
        }
    }

    Ok(project_ids.into_iter().collect())
}

#[derive(sqlx::FromRow)]
struct InstanceInstallCandidateRow {
    id: String,
    name: String,
    icon_path: Option<String>,
    game_version: String,
    loader: String,
    installed: i64,
}

pub(crate) async fn get_instance_install_candidates(
    project_id: &str,
    project_type: ProjectType,
    targets: &[InstanceInstallTarget],
    pool: &SqlitePool,
) -> crate::Result<Vec<InstanceInstallCandidate>> {
    let rows = sqlx::query_as::<_, InstanceInstallCandidateRow>(
        r#"
		SELECT
			i.id,
			i.name,
			i.icon_path,
			cs.game_version,
			cs.loader,
			CASE
				WHEN EXISTS (
					SELECT 1
					FROM instance_content_entries entry
					INNER JOIN instance_files file
						ON file.id = entry.file_id
					WHERE entry.content_set_id = cs.id
						AND EXISTS (
							SELECT 1
							FROM instance_content_provider_refs ref
							WHERE ref.content_entry_id = entry.id
								AND ref.provider = CASE
									WHEN ? LIKE 'curseforge:%' THEN 'curseforge'
									ELSE 'modrinth'
								END
								AND ref.provider_project_id = CASE
									WHEN ? LIKE 'curseforge:%' THEN substr(?, 12)
									ELSE ?
									END
						)
						AND file.missing = 0
				)
					THEN 1
				ELSE 0
			END AS installed
		FROM instances i
		INNER JOIN instance_content_sets cs
			ON cs.id = i.applied_content_set_id
		LEFT JOIN instance_links link
			ON link.instance_id = i.id
		WHERE COALESCE(link.link_kind, 'unmanaged') NOT IN (
			'server_project',
			'server_project_modpack'
		)
		ORDER BY i.name ASC
		"#,
    )
    .bind(project_id)
    .bind(project_id)
    .bind(project_id)
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let loader = ModLoader::from_string(&row.loader);
            let compatible = instance_matches_targets(
                project_type,
                &row.game_version,
                loader.as_str(),
                targets,
            );

            InstanceInstallCandidate {
                id: row.id,
                name: row.name,
                icon_path: row.icon_path,
                game_version: row.game_version,
                loader,
                installed: row.installed != 0,
                compatible,
            }
        })
        .collect())
}

fn instance_matches_targets(
    project_type: ProjectType,
    game_version: &str,
    loader: &str,
    targets: &[InstanceInstallTarget],
) -> bool {
    targets.iter().any(|target| {
        target.game_version == game_version
            && (project_type != ProjectType::Mod
                || target.loader == loader
                || target.loader == "datapack")
    })
}

pub(crate) async fn list_content(
    instance_id: &str,
    content_set_id: Option<&str>,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<Vec<ContentItem>> {
    let resolved = resolve_content_scope_with_instance(
        instance_id,
        content_set_id,
        &state.pool,
    )
    .await?;
    let link = sqlite::instance_rows::get_instance_link(
        &resolved.instance.id,
        &state.pool,
    )
    .await?;
    let imported_modpack_scope = is_imported_modpack_scope(&link);
    let curseforge_modpack_scope = is_curseforge_modpack_scope(&link);
    let linked_modpack_source_kind = linked_modpack_source_kind(&link);
    let modpack_ids = if imported_modpack_scope || curseforge_modpack_scope {
        None
    } else {
        match linked_modrinth_modpack_ids(&link) {
            Some((_, version_id)) => {
                get_cached_modpack_identifiers(
                    version_id,
                    &state.pool,
                    &state.api_semaphore,
                )
                .await?
            }
            None => None,
        }
    };
    let filter = if imported_modpack_scope {
        ContentFilter::ExcludeSourceKind {
            source_kind: ContentSourceKind::ImportedModpack,
            exclude_untracked: resolved.instance.install_stage
                != crate::state::InstanceInstallStage::Installed,
        }
    } else if curseforge_modpack_scope {
        ContentFilter::ExcludeSourceKind {
            source_kind: ContentSourceKind::CurseForge,
            exclude_untracked: resolved.instance.install_stage
                != crate::state::InstanceInstallStage::Installed,
        }
    } else if let Some(ids) = modpack_ids.as_ref() {
        ContentFilter::ExcludeModpack(ids)
    } else if let Some(source_kind) = linked_modpack_source_kind {
        ContentFilter::ExcludeSourceKind {
            source_kind,
            exclude_untracked: true,
        }
    } else {
        ContentFilter::All
    };
    let files =
        content_projects_for_scope(&resolved, cache_behaviour, state, filter)
            .await?;
    let files = files.into_iter().collect::<Vec<_>>();

    content_files_to_content_items(
        &resolved.instance,
        &files,
        cache_behaviour,
        state,
    )
    .await
}

pub(crate) async fn list_linked_modpack_content(
    instance_id: &str,
    content_set_id: Option<&str>,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<Vec<ContentItem>> {
    let resolved = resolve_content_scope_with_instance(
        instance_id,
        content_set_id,
        &state.pool,
    )
    .await?;
    let link = sqlite::instance_rows::get_instance_link(
        &resolved.instance.id,
        &state.pool,
    )
    .await?;
    if is_imported_modpack_scope(&link) {
        let files = content_projects_for_scope(
            &resolved,
            cache_behaviour,
            state,
            ContentFilter::All,
        )
        .await?;
        let files = files.into_iter().collect::<Vec<_>>();

        return content_files_to_content_items(
            &resolved.instance,
            &files,
            cache_behaviour,
            state,
        )
        .await;
    }

    if is_curseforge_modpack_scope(&link) {
        let files = content_projects_for_scope(
            &resolved,
            cache_behaviour,
            state,
            ContentFilter::All,
        )
        .await?;
        let files = files.into_iter().collect::<Vec<_>>();

        return content_files_to_content_items(
            &resolved.instance,
            &files,
            cache_behaviour,
            state,
        )
        .await;
    }

    let Some((_, version_id)) = linked_modrinth_modpack_ids(&link) else {
        return Ok(Vec::new());
    };
    let ids = match get_modpack_identifiers(
        version_id,
        &resolved.content_set,
        &state.pool,
        &state.api_semaphore,
    )
    .await
    {
        Ok(ids) => ids,
        Err(err) => {
            tracing::warn!("Failed to fetch modpack identifiers: {}", err);
            return Ok(Vec::new());
        }
    };
    let files = content_projects_for_scope(
        &resolved,
        cache_behaviour,
        state,
        ContentFilter::OnlyModpack(&ids),
    )
    .await?;
    let files = files.into_iter().collect::<Vec<_>>();

    content_files_to_content_items(
        &resolved.instance,
        &files,
        cache_behaviour,
        state,
    )
    .await
}

pub(crate) async fn get_linked_modpack_info(
    instance_id: &str,
    content_set_id: Option<&str>,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<Option<LinkedModpackInfo>> {
    let resolved = resolve_content_scope_with_instance(
        instance_id,
        content_set_id,
        &state.pool,
    )
    .await?;
    let link = sqlite::instance_rows::get_instance_link(
        &resolved.instance.id,
        &state.pool,
    )
    .await?;
    if let Some((project_id, version_id)) = linked_curseforge_modpack_ids(&link)
    {
        return get_curseforge_linked_modpack_info(
            project_id,
            version_id,
            resolved.instance.update_channel,
        )
        .await;
    }

    let Some((project_id, version_id)) = linked_modrinth_modpack_ids(&link)
    else {
        return Ok(None);
    };

    let project_id_ref = ModrinthProjectId::new(project_id.to_string())?;
    let version_id_ref = ModrinthVersionId::new(version_id.to_string())?;
    let (project, version, all_versions) = tokio::try_join!(
        CachedEntry::get_project(
            &project_id_ref,
            cache_behaviour,
            &state.pool,
            &state.api_semaphore,
        ),
        CachedEntry::get_version(
            &version_id_ref,
            cache_behaviour,
            &state.pool,
            &state.api_semaphore,
        ),
        CachedEntry::get_project_versions(
            &project_id_ref,
            cache_behaviour,
            &state.pool,
            &state.api_semaphore,
        ),
    )?;
    let version = version.ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Linked modpack version {version_id} not found"
        ))
    })?;
    let (project, all_versions) = if version.project_id != project_id {
        let modpack_project_id_ref =
            ModrinthProjectId::new(version.project_id.clone())?;
        let (modpack_project, modpack_versions) = tokio::try_join!(
            CachedEntry::get_project(
                &modpack_project_id_ref,
                cache_behaviour,
                &state.pool,
                &state.api_semaphore,
            ),
            CachedEntry::get_project_versions(
                &modpack_project_id_ref,
                cache_behaviour,
                &state.pool,
                &state.api_semaphore,
            ),
        )?;
        (modpack_project.or(project), modpack_versions)
    } else {
        (project, all_versions)
    };
    let project = project.ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Linked modpack project {project_id} not found"
        ))
    })?;
    let owner = if let Some(org_id) = &project.organization {
        let org = CachedEntry::get_organization(
            org_id,
            cache_behaviour,
            &state.pool,
            &state.api_semaphore,
        )
        .await?;
        org.map(|org| ContentItemOwner {
            id: org.id,
            name: org.name,
            avatar_url: org.icon_url,
            owner_type: OwnerType::Organization,
        })
    } else {
        let team = CachedEntry::get_team(
            &project.team,
            cache_behaviour,
            &state.pool,
            &state.api_semaphore,
        )
        .await?;
        team.and_then(|team| {
            team.into_iter()
                .find(|member| member.is_owner)
                .map(|member| ContentItemOwner {
                    id: member.user.id,
                    name: member.user.username,
                    avatar_url: member.user.avatar_url,
                    owner_type: OwnerType::User,
                })
        })
    };
    let (_, update_version_id, update_version) = check_modpack_update(
        &version_id,
        &version,
        all_versions,
        resolved.instance.update_channel,
    );
    let modpack_project_id = ModrinthProjectId::new(project.id.clone())?;
    let current_version_id = ModrinthVersionId::new(version.id.clone())?;
    let update = update_version_id
        .map(ModrinthVersionId::new)
        .transpose()?
        .map(|target_version_id| ContentItemUpdate::Modrinth {
            project_id: modpack_project_id.clone(),
            current_version_id: current_version_id.clone(),
            target_version_id,
        });

    Ok(Some(LinkedModpackInfo {
        project,
        version,
        owner,
        update,
        update_version,
    }))
}

async fn get_curseforge_linked_modpack_info(
    project_id: &str,
    version_id: &str,
    preferred_update_channel: ReleaseChannel,
) -> crate::Result<Option<LinkedModpackInfo>> {
    let numeric_project_id = project_id.parse::<u32>().map_err(|_| {
        crate::ErrorKind::InputError(format!(
            "Linked CurseForge project ID {project_id} is invalid"
        ))
    })?;
    let numeric_file_id = version_id.parse::<u32>().map_err(|_| {
        crate::ErrorKind::InputError(format!(
            "Linked CurseForge file ID {version_id} is invalid"
        ))
    })?;

    let project =
        crate::api::curseforge::get_project(numeric_project_id).await?;
    let installed_file =
        crate::api::curseforge::get_file(numeric_project_id, numeric_file_id)
            .await?;
    let files = crate::api::curseforge::get_files(
        numeric_project_id,
        crate::api::curseforge::CurseForgeFilesRequest {
            game_version: None,
            mod_loader_type: None,
            game_version_type_id: None,
            index: 0,
            page_size: 50,
        },
    )
    .await?
    .files;

    let project_model = curseforge_project_to_project(&project);
    let version = curseforge_file_to_version(&installed_file);
    let all_versions = files
        .into_iter()
        .filter(|file| file.is_available)
        .map(|file| curseforge_file_to_version(&file))
        .collect::<Vec<_>>();
    let owner = project.authors.first().map(|author| ContentItemOwner {
        id: author.id.to_string(),
        name: author.name.clone(),
        avatar_url: None,
        owner_type: OwnerType::User,
    });
    let (_, update_version_id, update_version) = check_modpack_update(
        &version.id,
        &version,
        Some(all_versions),
        preferred_update_channel,
    );

    Ok(Some(LinkedModpackInfo {
        project: project_model,
        version,
        owner,
        update: update_version_id.and_then(|target| {
            Some(ContentItemUpdate::CurseForge {
                project_id: crate::state::CurseForgeProjectId::new(
                    numeric_project_id,
                )
                .ok()?,
                current_file_id: crate::state::CurseForgeFileId::new(
                    numeric_file_id,
                )
                .ok()?,
                target_file_id: crate::state::CurseForgeFileId::new(
                    target.parse().ok()?,
                )
                .ok()?,
            })
        }),
        update_version,
    }))
}

fn curseforge_project_to_project(
    project: &crate::api::curseforge::CurseForgeProject,
) -> Project {
    let mut game_versions = Vec::new();
    let mut loaders = Vec::new();
    let mut seen_versions = HashSet::new();
    let mut seen_loaders = HashSet::new();
    for index in &project.latest_files_indexes {
        if seen_versions.insert(index.game_version.clone()) {
            game_versions.push(index.game_version.clone());
        }
        if let Some(loader) = index.mod_loader.and_then(curseforge_loader_name)
            && seen_loaders.insert(loader)
        {
            loaders.push(loader.to_string());
        }
    }

    Project {
        id: format!("curseforge:{}", project.id),
        slug: Some(project.slug.clone()),
        project_type: "modpack".to_string(),
        team: String::new(),
        organization: None,
        title: project.name.clone(),
        description: project.summary.clone(),
        body: project.summary.clone(),
        published: parse_curseforge_datetime(&project.date_created),
        updated: parse_curseforge_datetime(&project.date_modified),
        approved: None,
        status: "approved".to_string(),
        license: crate::state::License {
            id: String::new(),
            name: String::new(),
            url: None,
        },
        client_side: crate::state::SideType::Unknown,
        server_side: crate::state::SideType::Unknown,
        downloads: project.download_count.min(u32::MAX as u64) as u32,
        followers: 0,
        categories: project
            .categories
            .iter()
            .map(|category| category.slug.clone())
            .collect(),
        additional_categories: Vec::new(),
        game_versions,
        loaders,
        versions: project
            .latest_files
            .iter()
            .map(|file| file.id.to_string())
            .collect(),
        icon_url: project.logo.as_ref().map(|logo| logo.thumbnail_url.clone()),
        issues_url: project.links.issues_url.clone(),
        source_url: project.links.source_url.clone(),
        wiki_url: project.links.wiki_url.clone(),
        discord_url: None,
        donation_urls: None,
        gallery: project
            .screenshots
            .iter()
            .map(|screenshot| crate::state::GalleryItem {
                url: screenshot.url.clone(),
                raw_url: screenshot.url.clone(),
                featured: false,
                title: Some(screenshot.title.clone()),
                description: Some(screenshot.description.clone()),
                created: parse_curseforge_datetime(&project.date_created),
                ordering: 0,
            })
            .collect(),
        color: None,
    }
}

fn curseforge_file_to_version(
    file: &crate::api::curseforge::CurseForgeFile,
) -> Version {
    let loaders = file
        .game_versions
        .iter()
        .filter_map(|value| curseforge_loader_from_name(value))
        .collect::<HashSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let game_versions = file
        .game_versions
        .iter()
        .filter(|value| curseforge_loader_from_name(value).is_none())
        .cloned()
        .collect::<Vec<_>>();
    let hashes = file
        .hashes
        .iter()
        .filter_map(|hash| {
            let algo = match hash.algo {
                1 => "sha1",
                2 => "md5",
                _ => return None,
            };
            Some((algo.to_string(), hash.value.clone()))
        })
        .collect();

    Version {
        id: file.id.to_string(),
        project_id: format!("curseforge:{}", file.mod_id),
        author_id: String::new(),
        featured: false,
        name: file.display_name.clone(),
        version_number: file.display_name.clone(),
        changelog: None,
        changelog_url: None,
        date_published: parse_curseforge_datetime(&file.file_date),
        downloads: file.download_count.min(u32::MAX as u64) as u32,
        version_type: match file.release_type {
            1 => "release".to_string(),
            2 => "beta".to_string(),
            _ => "alpha".to_string(),
        },
        files: vec![crate::state::VersionFile {
            hashes,
            url: file.download_url.clone().unwrap_or_default(),
            filename: file.file_name.clone(),
            primary: true,
            size: file.file_length.min(u32::MAX as u64) as u32,
            file_type: None,
        }],
        dependencies: Vec::new(),
        game_versions,
        loaders: if loaders.is_empty() {
            vec!["minecraft".to_string()]
        } else {
            loaders
        },
    }
}

fn parse_curseforge_datetime(value: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
                .map(|datetime| datetime.and_utc())
        })
        .unwrap_or_else(|_| chrono::Utc::now())
}

fn curseforge_loader_name(value: u32) -> Option<&'static str> {
    match value {
        1 => Some("forge"),
        4 => Some("fabric"),
        5 => Some("quilt"),
        6 => Some("neoforge"),
        _ => None,
    }
}

fn curseforge_loader_from_name(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().replace(' ', "").as_str() {
        "forge" => Some("forge"),
        "fabric" | "fabricloader" => Some("fabric"),
        "quilt" => Some("quilt"),
        "neoforge" => Some("neoforge"),
        _ => None,
    }
}

pub(crate) async fn dependencies_to_content_items(
    dependencies: &[Dependency],
    cache_behaviour: Option<CacheBehaviour>,
    pool: &SqlitePool,
    fetch_semaphore: &FetchSemaphore,
) -> crate::Result<Vec<ContentItem>> {
    let project_ids = dependencies
        .iter()
        .filter_map(|dependency| dependency.project_id.clone())
        .collect::<HashSet<_>>();
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    let version_ids = dependencies
        .iter()
        .filter_map(|dependency| dependency.version_id.clone())
        .collect::<HashSet<_>>();
    let meta = resolve_metadata(
        &project_ids,
        &version_ids,
        cache_behaviour,
        pool,
        fetch_semaphore,
    )
    .await?;
    let mut items = dependencies
        .iter()
        .filter_map(|dependency| {
            let project_id = dependency.project_id.as_ref()?;
            let project = meta
                .projects
                .iter()
                .find(|project| &project.id == project_id)?;
            let version =
                dependency.version_id.as_ref().and_then(|version_id| {
                    meta.versions
                        .iter()
                        .find(|version| &version.id == version_id)
                });
            let owner =
                resolve_owner(project, &meta.teams, &meta.organizations);
            let project_type =
                project_type_from_api_name(&project.project_type);

            Some(ContentItem {
                file_name: version
                    .and_then(|version| version.files.first())
                    .map(|file| file.filename.clone())
                    .unwrap_or_else(|| {
                        format!(
                            "{}.jar",
                            project.slug.as_deref().unwrap_or(&project.id)
                        )
                    }),
                file_path: String::new(),
                id: String::new(),
                size: version
                    .and_then(|version| version.files.first())
                    .map(|file| file.size as u64)
                    .unwrap_or(0),
                enabled: true,
                project_type,
                project: Some(ContentItemProject {
                    id: project.id.clone(),
                    slug: project.slug.clone(),
                    title: project.title.clone(),
                    icon_url: project.icon_url.clone(),
                }),
                version: version.map(|version| ContentItemVersion {
                    id: version.id.clone(),
                    version_number: version.version_number.clone(),
                    file_name: version
                        .files
                        .first()
                        .map(|file| file.filename.clone())
                        .unwrap_or_default(),
                    date_published: Some(version.date_published.to_rfc3339()),
                }),
                owner,
                update: None,
                date_added: None,
                provider_refs: vec![ContentProviderRef::Modrinth {
                    project_id: crate::state::ModrinthProjectId::new(
                        project.id.clone(),
                    )
                    .ok()?,
                    version_id: version
                        .map(|version| {
                            crate::state::ModrinthVersionId::new(
                                version.id.clone(),
                            )
                        })
                        .transpose()
                        .ok()?,
                }],
                origin_provider: Some(ContentProvider::Modrinth),
            })
        })
        .collect::<Vec<_>>();
    sort_content_items(&mut items);

    Ok(items)
}

async fn resolve_content_scope_with_instance(
    instance_id: &str,
    content_set_id: Option<&str>,
    pool: &SqlitePool,
) -> crate::Result<ResolvedContentScope> {
    let instance = sqlite::instance_rows::get_instance_by_id(instance_id, pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;
    let content_set = match content_set_id {
        Some(content_set_id) => {
            let content_set =
                sqlite::content_rows::get_content_set(content_set_id, pool)
                    .await?
                    .ok_or_else(|| {
                        crate::ErrorKind::InputError(format!(
                            "Unknown content set {content_set_id}"
                        ))
                    })?;

            if content_set.instance_id != instance.id {
                return Err(crate::ErrorKind::InputError(format!(
					"Content set {content_set_id} does not belong to instance {}",
					instance.id
				))
                .into());
            }

            content_set
        }
        None => {
            sqlite::content_rows::get_applied_content_set(&instance.id, pool)
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError(format!(
                        "Instance {} has no applied content set",
                        instance.id
                    ))
                })?
        }
    };

    Ok(ResolvedContentScope {
        instance,
        content_set,
    })
}

async fn content_projects_for_scope(
    resolved: &ResolvedContentScope,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
    filter: ContentFilter<'_>,
) -> crate::Result<DashMap<String, ContentFile>> {
    let files = sync_instance_content_files(&resolved.instance, state).await?;
    let entries = sqlite::content_rows::get_content_entries(
        &resolved.content_set.id,
        &state.pool,
    )
    .await?;
    let entries_by_file_id = entries
        .iter()
        .filter_map(|entry| {
            entry.file_id.as_deref().map(|file_id| (file_id, entry))
        })
        .collect::<HashMap<_, _>>();
    let mut provider_refs_by_file_id = HashMap::new();
    let mut origin_provider_by_file_id = HashMap::new();
    for entry in &entries {
        let Some(file_id) = entry.file_id.as_deref() else {
            continue;
        };
        provider_refs_by_file_id.insert(
            file_id.to_string(),
            sqlite::content_rows::get_content_provider_refs(
                &entry.id,
                &state.pool,
            )
            .await?,
        );
        if let Some(origin) = sqlite::content_rows::get_content_origin_provider(
            &entry.id,
            &state.pool,
        )
        .await?
        {
            origin_provider_by_file_id.insert(file_id.to_string(), origin);
        }
    }
    let hashes = files
        .iter()
        .filter(|file| {
            let refs = entries_by_file_id
                .get(file.id.as_str())
                .and_then(|_| provider_refs_by_file_id.get(&file.id));
            refs.is_none_or(|refs| {
                refs.is_empty()
                    || refs.iter().any(|reference| {
                        reference.provider() == ContentProvider::Modrinth
                    })
            })
        })
        .map(|file| file.sha1.as_str())
        .collect::<Vec<_>>();
    let file_info = CachedEntry::get_file_many(
        &hashes,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let file_info_by_hash = file_info
        .into_iter()
        .map(|file| (file.hash.clone(), file))
        .collect::<HashMap<_, _>>();
    let installed_channels = get_installed_update_channels(
        &file_info_by_hash,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let update_keys = files
        .iter()
        .filter(|file| file_info_by_hash.contains_key(&file.sha1))
        .filter_map(|file| {
            let project_type = project_type_for_file(file)?;
            let channel = resolved.instance.update_channel.least_stable(
                installed_channels
                    .get(&file.sha1)
                    .copied()
                    .unwrap_or(resolved.instance.update_channel),
            );
            Some(file_update_cache_key(
                &file.sha1,
                project_type,
                &resolved.content_set,
                channel,
            ))
        })
        .collect::<Vec<_>>();
    let update_key_refs =
        update_keys.iter().map(String::as_str).collect::<Vec<_>>();
    let file_updates = CachedEntry::get_file_update_many(
        &update_key_refs,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let mut updates_by_hash: HashMap<String, Vec<String>> = HashMap::new();
    for update in file_updates {
        updates_by_hash
            .entry(update.hash)
            .or_default()
            .push(update.update_version_id);
    }
    let output = DashMap::new();

    for file in files {
        if file.missing {
            continue;
        }

        let Some(project_type) = project_type_for_file(&file) else {
            continue;
        };
        let metadata = file_info_by_hash.get(&file.sha1).cloned();
        let entry = entries_by_file_id.get(file.id.as_str()).copied();
        let provider_refs = provider_refs_by_file_id
            .get(&file.id)
            .cloned()
            .unwrap_or_default();
        let origin_provider = entry
            .and_then(|_| origin_provider_by_file_id.get(&file.id))
            .copied();
        let modrinth_metadata = if provider_refs.is_empty()
            || origin_provider == Some(ContentProvider::Modrinth)
        {
            metadata.as_ref().and_then(modrinth_file_match)
        } else {
            None
        };

        match filter {
            ContentFilter::All => {}
            ContentFilter::ExcludeModpack(ids) => {
                if ids.is_modpack_file(
                    &file.sha1,
                    metadata.as_ref(),
                    provider_refs.iter().find_map(
                        |reference| match reference {
                            ContentProviderRef::Modrinth {
                                project_id, ..
                            } => Some(project_id.as_str()),
                            ContentProviderRef::CurseForge { .. } => None,
                        },
                    ),
                ) {
                    continue;
                }
            }
            ContentFilter::ExcludeSourceKind {
                source_kind,
                exclude_untracked,
            } => {
                if entry.is_some_and(|entry| entry.source_kind == source_kind)
                    || (exclude_untracked && entry.is_none())
                {
                    continue;
                }
            }
            ContentFilter::OnlyModpack(ids) => {
                if !ids.is_modpack_file(
                    &file.sha1,
                    metadata.as_ref(),
                    provider_refs.iter().find_map(
                        |reference| match reference {
                            ContentProviderRef::Modrinth {
                                project_id, ..
                            } => Some(project_id.as_str()),
                            ContentProviderRef::CurseForge { .. } => None,
                        },
                    ),
                ) {
                    continue;
                }
            }
            ContentFilter::OnlySourceKind {
                source_kind,
                include_untracked,
            } => {
                if !(entry
                    .is_some_and(|entry| entry.source_kind == source_kind)
                    || include_untracked && entry.is_none())
                {
                    continue;
                }
            }
        }

        let update = (origin_provider == Some(ContentProvider::Modrinth))
            .then(|| {
                modrinth_metadata.as_ref().and_then(|metadata| {
                    let update_ids =
                        updates_by_hash.remove(&file.sha1).unwrap_or_default();
                    update_ids
                        .into_iter()
                        .find(|update_id| {
                            update_id != metadata.version_id.as_str()
                        })
                        .and_then(|target| {
                            Some(ContentItemUpdate::Modrinth {
                                project_id: metadata.project_id.clone(),
                                current_version_id: metadata.version_id.clone(),
                                target_version_id:
                                    crate::state::ModrinthVersionId::new(target)
                                        .ok()?,
                            })
                        })
                })
            })
            .flatten();

        output.insert(
            file.relative_path.clone(),
            ContentFile {
                update,
                hash: file.sha1,
                file_name: file.file_name,
                enabled: entry.map_or(file.enabled, |entry| {
                    entry.enabled && file.enabled
                }),
                size: file.size,
                modrinth: modrinth_metadata,
                provider_refs,
                origin_provider,
                project_type,
                local_mod_data: file.local_mod_data,
            },
        );
    }

    Ok(output)
}

async fn get_installed_update_channels(
    file_info_by_hash: &HashMap<String, crate::state::ModrinthHashMatch>,
    cache_behaviour: Option<CacheBehaviour>,
    pool: &SqlitePool,
    fetch_semaphore: &FetchSemaphore,
) -> crate::Result<HashMap<String, ReleaseChannel>> {
    let version_ids = file_info_by_hash
        .values()
        .map(|file| file.version_id.as_str())
        .collect::<HashSet<_>>();
    if version_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let version_id_refs = version_ids
        .iter()
        .filter_map(|id| ModrinthVersionId::new((*id).to_string()).ok())
        .collect::<Vec<_>>();
    let versions = CachedEntry::get_version_many(
        &version_id_refs,
        cache_behaviour,
        pool,
        fetch_semaphore,
    )
    .await?;
    let channels_by_version_id = versions
        .into_iter()
        .map(|version| {
            (
                version.id,
                ReleaseChannel::from_version_type(&version.version_type),
            )
        })
        .collect::<HashMap<_, _>>();

    Ok(file_info_by_hash
        .iter()
        .filter_map(|(hash, file)| {
            channels_by_version_id
                .get(&file.version_id)
                .copied()
                .map(|channel| (hash.clone(), channel))
        })
        .collect())
}

fn file_update_cache_key(
    hash: &str,
    project_type: ProjectType,
    content_set: &ContentSet,
    channel: ReleaseChannel,
) -> String {
    let loader_key = if project_type == ProjectType::Mod {
        content_set.loader.as_str().to_string()
    } else {
        project_type.get_loaders().join("+")
    };

    format!(
        "{}-{}-{}-{}",
        hash,
        loader_key,
        channel.key(),
        content_set.game_version
    )
}

async fn content_files_to_content_items(
    instance: &Instance,
    files: &[(String, ContentFile)],
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<Vec<ContentItem>> {
    let mut provider_refs_by_path =
        HashMap::<String, Vec<ContentProviderRef>>::new();
    let mut origin_provider_by_path = HashMap::<String, ContentProvider>::new();
    let provider_rows = sqlx::query(
        "SELECT file.relative_path, ref.provider, ref.provider_project_id,
                ref.provider_release_id, ref.is_origin
         FROM instance_files file
         INNER JOIN instance_content_entries entry ON entry.file_id = file.id
         INNER JOIN instance_content_provider_refs ref
            ON ref.content_entry_id = entry.id
         WHERE file.instance_id = ?",
    )
    .bind(&instance.id)
    .fetch_all(&state.pool)
    .await?;
    for row in provider_rows {
        let provider = ContentProvider::from_str(row.try_get("provider")?)?;
        provider_refs_by_path
            .entry(row.try_get("relative_path")?)
            .or_default()
            .push(ContentProviderRef::from_database(
                provider.as_str(),
                row.try_get("provider_project_id")?,
                row.try_get::<Option<String>, _>("provider_release_id")?
                    .as_deref(),
            )?);
        if row.try_get::<i64, _>("is_origin")? != 0 {
            origin_provider_by_path
                .insert(row.try_get("relative_path")?, provider);
        }
    }
    let curseforge_project_ids = provider_refs_by_path
        .values()
        .flatten()
        .filter_map(|reference| match reference {
            ContentProviderRef::CurseForge { project_id, .. } => {
                Some(project_id.get())
            }
            ContentProviderRef::Modrinth { .. } => None,
        })
        .collect::<HashSet<_>>();
    let curseforge_projects = if curseforge_project_ids.is_empty()
        || crate::api::curseforge::capability().status
            != crate::api::curseforge::CurseForgeCapabilityStatus::Ready
    {
        HashMap::new()
    } else {
        crate::api::curseforge::get_projects(
            curseforge_project_ids.into_iter().collect(),
        )
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|project| (project.id.to_string(), project))
        .collect::<HashMap<_, _>>()
    };
    let content_set = sqlite::content_rows::get_applied_content_set(
        &instance.id,
        &state.pool,
    )
    .await?;
    let project_ids = files
        .iter()
        .filter_map(|(_, file)| {
            file.modrinth
                .as_ref()
                .map(|metadata| metadata.project_id.to_string())
        })
        .collect::<HashSet<_>>();
    let version_ids = files
        .iter()
        .filter_map(|(_, file)| {
            file.modrinth
                .as_ref()
                .map(|metadata| metadata.version_id.to_string())
        })
        .collect::<HashSet<_>>();
    let meta = resolve_metadata(
        &project_ids,
        &version_ids,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let instance_path = state.directories.instances_dir().join(&instance.path);
    let paths = files
        .iter()
        .map(|(path, _)| instance_path.join(path))
        .collect::<Vec<_>>();
    let modification_times: Vec<Option<String>> =
        tokio::task::spawn_blocking(move || {
            paths
                .iter()
                .map(|path| {
                    std::fs::metadata(path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                        .map(|time| {
                            chrono::DateTime::<chrono::Utc>::from(time)
                                .to_rfc3339()
                        })
                })
                .collect()
        })
        .await?;
    let mut items = files
        .iter()
        .enumerate()
        .map(|(index, (path, file))| {
            let provider_refs = provider_refs_by_path
                .get(path)
                .cloned()
                .or_else(|| {
                    file.modrinth.as_ref().map(|metadata| {
                        vec![ContentProviderRef::Modrinth {
                            project_id: metadata.project_id.clone(),
                            version_id: Some(metadata.version_id.clone()),
                        }]
                    })
                })
                .unwrap_or_default();
            let origin_provider = origin_provider_by_path.get(path).copied();
            let curseforge_ref = provider_refs.iter().find(|reference| {
                reference.provider() == ContentProvider::CurseForge
            });
            let curseforge_project = curseforge_ref.and_then(|reference| {
                let ContentProviderRef::CurseForge { project_id, .. } =
                    reference
                else {
                    return None;
                };
                curseforge_projects.get(&project_id.get().to_string())
            });
            let curseforge_file = curseforge_ref.and_then(|reference| {
                let ContentProviderRef::CurseForge { file_id, .. } = reference
                else {
                    return None;
                };
                let file_id = file_id.as_ref()?.get();
                curseforge_project?
                    .latest_files
                    .iter()
                    .find(|file| file.id == file_id)
            });
            let curseforge_update_id = curseforge_ref.and_then(|reference| {
                if origin_provider != Some(ContentProvider::CurseForge) {
                    return None;
                }
                let ContentProviderRef::CurseForge { file_id, .. } = reference
                else {
                    return None;
                };
                let current_file_id = file_id.as_ref()?.get();
                let content_set = content_set.as_ref()?;
                let loader_type = match content_set.loader.as_str() {
                    "forge" => Some(1),
                    "fabric" => Some(4),
                    "quilt" => Some(5),
                    "neoforge" => Some(6),
                    _ => None,
                };
                let index = curseforge_project?
                    .latest_files_indexes
                    .iter()
                    .find(|index| {
                        index.game_version == content_set.game_version
                            && (file.project_type != ProjectType::Mod
                                || index.mod_loader == loader_type)
                    })?;
                (index.file_id != current_file_id)
                    .then(|| index.file_id.to_string())
            });
            let project = file.modrinth.as_ref().and_then(|metadata| {
                meta.projects
                    .iter()
                    .find(|project| project.id == metadata.project_id.as_str())
            });
            let version = file.modrinth.as_ref().and_then(|metadata| {
                meta.versions
                    .iter()
                    .find(|version| version.id == metadata.version_id.as_str())
            });
            let owner = project.and_then(|project| {
                resolve_owner(project, &meta.teams, &meta.organizations)
            });

            // Parse local_mod_data for fallback display when Modrinth /
            // CurseForge has no match for this file.
            let local_mod = file.local_mod_data.as_ref().and_then(|json| {
                serde_json::from_str::<crate::mod_metadata::LocalModMetadata>(
                    json,
                )
                .ok()
            });

            ContentItem {
                file_name: file.file_name.clone(),
                file_path: path.clone(),
                id: file.hash.clone(),
                size: file.size,
                enabled: file.enabled,
                project_type: file.project_type,
                project: project
                    .map(|project| ContentItemProject {
                        id: project.id.clone(),
                        slug: project.slug.clone(),
                        title: project.title.clone(),
                        icon_url: project.icon_url.clone(),
                    })
                    .or_else(|| {
                        curseforge_project.map(|project| ContentItemProject {
                            id: project.id.to_string(),
                            slug: Some(project.slug.clone()),
                            title: project.name.clone(),
                            icon_url: project
                                .logo
                                .as_ref()
                                .map(|logo| logo.thumbnail_url.clone()),
                        })
                    })
                    .or_else(|| {
                        local_mod.as_ref().map(|meta| ContentItemProject {
                            id: format!("local:{}", meta.mod_id),
                            slug: Some(meta.mod_id.clone()),
                            title: meta
                                .name
                                .clone()
                                .unwrap_or_else(|| meta.mod_id.clone()),
                            icon_url: None,
                        })
                    }),
                version: version
                    .map(|version| ContentItemVersion {
                        id: version.id.clone(),
                        version_number: version.version_number.clone(),
                        file_name: file.file_name.clone(),
                        date_published: Some(
                            version.date_published.to_rfc3339(),
                        ),
                    })
                    .or_else(|| {
                        curseforge_file.map(|version| ContentItemVersion {
                            id: version.id.to_string(),
                            version_number: version.display_name.clone(),
                            file_name: version.file_name.clone(),
                            date_published: Some(version.file_date.clone()),
                        })
                    })
                    .or_else(|| {
                        local_mod.as_ref().and_then(|meta| {
                            meta.version.clone().map(|v| ContentItemVersion {
                                id: format!("local:{}", meta.mod_id),
                                version_number: v,
                                file_name: file.file_name.clone(),
                                date_published: None,
                            })
                        })
                    }),
                owner: owner
                    .or_else(|| {
                        curseforge_project
                            .and_then(|project| project.authors.first())
                            .map(|author| ContentItemOwner {
                                id: author.id.to_string(),
                                name: author.name.clone(),
                                avatar_url: None,
                                owner_type: OwnerType::User,
                            })
                    })
                    .or_else(|| {
                        local_mod.as_ref().and_then(|meta| {
                            meta.authors.first().map(|author| {
                                ContentItemOwner {
                                    id: format!("local:{author}"),
                                    name: author.clone(),
                                    avatar_url: None,
                                    owner_type: OwnerType::User,
                                }
                            })
                        })
                    }),
                update: (origin_provider == Some(ContentProvider::Modrinth))
                    .then(|| file.update.clone())
                    .flatten()
                    .or_else(|| {
                        curseforge_update_id.and_then(|target| {
                            let reference =
                                provider_refs.iter().find_map(|reference| {
                                    match reference {
                                        ContentProviderRef::CurseForge {
                                            project_id,
                                            file_id: Some(current_file_id),
                                        } => Some((
                                            *project_id,
                                            *current_file_id,
                                        )),
                                        _ => None,
                                    }
                                })?;
                            Some(ContentItemUpdate::CurseForge {
                                project_id: reference.0,
                                current_file_id: reference.1,
                                target_file_id:
                                    crate::state::CurseForgeFileId::new(
                                        target.parse().ok()?,
                                    )
                                    .ok()?,
                            })
                        })
                    }),
                date_added: modification_times[index].clone(),
                provider_refs,
                origin_provider,
            }
        })
        .collect::<Vec<_>>();
    sort_content_items(&mut items);

    Ok(items)
}

struct ResolvedMetadata {
    projects: Vec<Project>,
    versions: Vec<Version>,
    teams: Vec<Vec<TeamMember>>,
    organizations: Vec<Organization>,
}

async fn resolve_metadata(
    project_ids: &HashSet<String>,
    version_ids: &HashSet<String>,
    cache_behaviour: Option<CacheBehaviour>,
    pool: &SqlitePool,
    fetch_semaphore: &FetchSemaphore,
) -> crate::Result<ResolvedMetadata> {
    let project_id_refs = project_ids
        .iter()
        .map(|id| ModrinthProjectId::new(id.clone()))
        .collect::<crate::Result<Vec<_>>>()?;
    let version_id_refs = version_ids
        .iter()
        .map(|id| ModrinthVersionId::new(id.clone()))
        .collect::<crate::Result<Vec<_>>>()?;
    let (projects, versions) =
        if !project_ids.is_empty() || !version_ids.is_empty() {
            tokio::try_join!(
                async {
                    if project_ids.is_empty() {
                        Ok(Vec::new())
                    } else {
                        CachedEntry::get_project_many(
                            &project_id_refs,
                            cache_behaviour,
                            pool,
                            fetch_semaphore,
                        )
                        .await
                    }
                },
                async {
                    if version_ids.is_empty() {
                        Ok(Vec::new())
                    } else {
                        CachedEntry::get_version_many(
                            &version_id_refs,
                            cache_behaviour,
                            pool,
                            fetch_semaphore,
                        )
                        .await
                    }
                }
            )?
        } else {
            (Vec::new(), Vec::new())
        };
    let team_ids = projects
        .iter()
        .map(|project| project.team.clone())
        .collect::<HashSet<_>>();
    let org_ids = projects
        .iter()
        .filter_map(|project| project.organization.clone())
        .collect::<HashSet<_>>();
    let team_id_refs = team_ids.iter().map(String::as_str).collect::<Vec<_>>();
    let org_id_refs = org_ids.iter().map(String::as_str).collect::<Vec<_>>();
    let (teams, organizations) = if !team_ids.is_empty() || !org_ids.is_empty()
    {
        tokio::try_join!(
            async {
                if team_ids.is_empty() {
                    Ok(Vec::new())
                } else {
                    CachedEntry::get_team_many(
                        &team_id_refs,
                        cache_behaviour,
                        pool,
                        fetch_semaphore,
                    )
                    .await
                }
            },
            async {
                if org_ids.is_empty() {
                    Ok(Vec::new())
                } else {
                    CachedEntry::get_organization_many(
                        &org_id_refs,
                        cache_behaviour,
                        pool,
                        fetch_semaphore,
                    )
                    .await
                }
            }
        )?
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(ResolvedMetadata {
        projects,
        versions,
        teams,
        organizations,
    })
}

fn resolve_owner(
    project: &Project,
    teams: &[Vec<TeamMember>],
    organizations: &[Organization],
) -> Option<ContentItemOwner> {
    if let Some(org_id) = &project.organization {
        organizations
            .iter()
            .find(|organization| &organization.id == org_id)
            .map(|organization| ContentItemOwner {
                id: organization.id.clone(),
                name: organization.name.clone(),
                avatar_url: organization.icon_url.clone(),
                owner_type: OwnerType::Organization,
            })
    } else {
        teams
            .iter()
            .find(|team| {
                team.first()
                    .is_some_and(|member| member.team_id == project.team)
            })
            .and_then(|team| team.iter().find(|member| member.is_owner))
            .map(|member| ContentItemOwner {
                id: member.user.id.clone(),
                name: member.user.username.clone(),
                avatar_url: member.user.avatar_url.clone(),
                owner_type: OwnerType::User,
            })
    }
}

fn modrinth_file_match(
    cached: &crate::state::ModrinthHashMatch,
) -> Option<ModrinthFileMatch> {
    Some(ModrinthFileMatch {
        project_id: crate::state::ModrinthProjectId::new(
            cached.project_id.clone(),
        )
        .ok()?,
        version_id: crate::state::ModrinthVersionId::new(
            cached.version_id.clone(),
        )
        .ok()?,
    })
}

fn is_imported_modpack_scope(link: &InstanceLink) -> bool {
    matches!(link, InstanceLink::ImportedModpack { .. })
}

fn is_curseforge_modpack_scope(link: &InstanceLink) -> bool {
    matches!(link, InstanceLink::CurseForgeModpack { .. })
}

fn linked_modrinth_modpack_ids(link: &InstanceLink) -> Option<(&str, &str)> {
    match link {
        InstanceLink::ModrinthModpack {
            project_id,
            version_id,
        } => Some((project_id, version_id)),
        InstanceLink::ServerProjectModpack {
            content_project_id,
            content_version_id,
            ..
        } => Some((content_project_id, content_version_id)),
        _ => None,
    }
}

fn linked_curseforge_modpack_ids(link: &InstanceLink) -> Option<(&str, &str)> {
    match link {
        InstanceLink::CurseForgeModpack {
            project_id,
            version_id,
        } => Some((project_id, version_id)),
        _ => None,
    }
}

fn linked_modpack_source_kind(
    link: &InstanceLink,
) -> Option<ContentSourceKind> {
    match link {
        InstanceLink::ModrinthModpack { .. } => {
            Some(ContentSourceKind::ModrinthModpack)
        }
        InstanceLink::CurseForgeModpack { .. } => {
            Some(ContentSourceKind::CurseForge)
        }
        InstanceLink::ServerProjectModpack { .. } => {
            Some(ContentSourceKind::ServerProject)
        }
        _ => None,
    }
}

fn check_modpack_update(
    installed_version_id: &str,
    installed_version: &Version,
    all_versions: Option<Vec<Version>>,
    preferred_update_channel: ReleaseChannel,
) -> (bool, Option<String>, Option<Version>) {
    let Some(versions) = all_versions else {
        return (false, None, None);
    };
    let installed_channel =
        ReleaseChannel::from_version_type(&installed_version.version_type);
    let effective_channel =
        preferred_update_channel.least_stable(installed_channel);

    for version_types in effective_channel.version_type_fallbacks() {
        if !versions.iter().any(|version| {
            version_types.contains(&version.version_type.as_str())
        }) {
            continue;
        }

        let mut newer_versions = versions
            .iter()
            .filter(|version| {
                version.id != installed_version_id
                    && version.date_published > installed_version.date_published
                    && version_types.contains(&version.version_type.as_str())
            })
            .collect::<Vec<_>>();
        newer_versions
            .sort_by_key(|version| std::cmp::Reverse(version.date_published));

        if let Some(newest) = newer_versions.first() {
            return (true, Some(newest.id.clone()), Some((*newest).clone()));
        }

        return (false, None, None);
    }

    (false, None, None)
}

#[derive(Clone, Debug)]
struct ModpackIdentifiers {
    hashes: HashSet<String>,
    project_ids: HashSet<String>,
}

impl ModpackIdentifiers {
    fn is_modpack_file(
        &self,
        hash: &str,
        file: Option<&crate::state::ModrinthHashMatch>,
        entry_project_id: Option<&str>,
    ) -> bool {
        self.hashes.contains(hash)
            || entry_project_id
                .is_some_and(|project_id| self.project_ids.contains(project_id))
            || file
                .is_some_and(|file| self.project_ids.contains(&file.project_id))
    }
}

async fn get_cached_modpack_identifiers(
    version_id: &str,
    pool: &SqlitePool,
    fetch_semaphore: &FetchSemaphore,
) -> crate::Result<Option<ModpackIdentifiers>> {
    let Some(cached) =
        CachedEntry::get_modpack_files(version_id, pool, fetch_semaphore)
            .await?
    else {
        return Ok(None);
    };

    if cached.project_ids.is_empty() {
        return Ok(None);
    }

    Ok(Some(ModpackIdentifiers {
        hashes: cached.file_hashes.into_iter().collect(),
        project_ids: cached.project_ids.into_iter().collect(),
    }))
}

async fn get_modpack_identifiers(
    version_id: &str,
    content_set: &ContentSet,
    pool: &SqlitePool,
    fetch_semaphore: &FetchSemaphore,
) -> crate::Result<ModpackIdentifiers> {
    if let Some(cached) =
        CachedEntry::get_modpack_files(version_id, pool, fetch_semaphore)
            .await?
    {
        if !cached.project_ids.is_empty() {
            return Ok(ModpackIdentifiers {
                hashes: cached.file_hashes.into_iter().collect(),
                project_ids: cached.project_ids.into_iter().collect(),
            });
        }

        let hash_refs = cached
            .file_hashes
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let files =
            CachedEntry::get_file_many(&hash_refs, None, pool, fetch_semaphore)
                .await?;
        let project_ids = files
            .iter()
            .map(|file| file.project_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        CachedEntry::cache_modpack_files(
            version_id,
            cached.file_hashes.clone(),
            project_ids.clone(),
            pool,
        )
        .await?;

        return Ok(ModpackIdentifiers {
            hashes: cached.file_hashes.into_iter().collect(),
            project_ids: project_ids.into_iter().collect(),
        });
    }

    let version = CachedEntry::get_version(
        &ModrinthVersionId::new(version_id.to_string())?,
        None,
        pool,
        fetch_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Modpack version {version_id} not found"
        ))
    })?;
    let primary_file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "No files found for modpack version {version_id}"
            ))
        })?;
    let download_meta = DownloadMeta {
        reason: DownloadReason::Modpack,
        game_version: content_set.game_version.clone(),
        loader: content_set.loader.as_str().to_string(),
        dependent_on: Some(version_id.to_string()),
    };
    let state = State::get().await?;
    let file_name = Path::new(&primary_file.filename);
    if file_name.components().count() != 1
        || !matches!(file_name.components().next(), Some(Component::Normal(_)))
    {
        return Err(crate::ErrorKind::InputError(
            "Modrinth returned an invalid modpack file name".to_string(),
        )
        .into());
    }
    // Stream the pack to the same cache location the installer uses instead
    // of buffering multi-gigabyte .mrpacks in memory; a later install of this
    // version can then reuse the already-verified file.
    let pack_path = state
        .directories
        .caches_dir()
        .join("modpacks")
        .join(&version.project_id)
        .join(version_id)
        .join(file_name);
    download_to_path(
        DownloadRequest::new(&primary_file.url, ResourceClass::Modpack)
            .with_integrity(Integrity {
                size: Some(primary_file.size as u64),
                sha1: primary_file.hashes.get("sha1").cloned(),
                sha512: primary_file.hashes.get("sha512").cloned(),
                content: ContentValidation::Jar,
                ..Integrity::default()
            })
            .with_download_meta(download_meta),
        &pack_path,
        &state.download_semaphore,
        pool,
        None,
    )
    .await?;
    let zip_reader = ZipFileReader::new(&pack_path).await.map_err(|_| {
        crate::ErrorKind::InputError("Failed to read modpack zip".to_string())
    })?;
    let manifest_idx = zip_reader
        .file()
        .entries()
        .iter()
        .position(|file| {
            matches!(file.filename().as_str(), Ok("modrinth.index.json"))
        })
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "No modrinth.index.json found in mrpack".to_string(),
            )
        })?;
    let mut manifest = String::new();
    let mut entry_reader = zip_reader.reader_with_entry(manifest_idx).await?;
    entry_reader.read_to_string_checked(&mut manifest).await?;
    let pack: PackFormat = serde_json::from_str(&manifest)?;
    let mut hashes = pack
        .files
        .iter()
        .filter_map(|file| file.hashes.get(&PackFileHash::Sha1).cloned())
        .collect::<Vec<_>>();
    let project_ids = pack
        .files
        .iter()
        .filter_map(|file| {
            file.downloads.iter().find_map(|url| {
                let parts = url.split('/').collect::<Vec<_>>();
                let data_idx = parts.iter().position(|part| *part == "data")?;
                parts.get(data_idx + 1).map(|part| part.to_string())
            })
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let override_entries = zip_reader
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let filename = entry.filename().as_str().ok()?;
            let is_override = (filename.starts_with("overrides/")
                || filename.starts_with("client-overrides/")
                || filename.starts_with("server-overrides/"))
                && !filename.ends_with('/');
            is_override.then_some(index)
        })
        .collect::<Vec<_>>();

    for index in override_entries {
        let mut file_bytes = Vec::new();
        let mut entry_reader = zip_reader.reader_with_entry(index).await?;
        entry_reader.read_to_end_checked(&mut file_bytes).await?;
        hashes.push(sha1_async(bytes::Bytes::from(file_bytes)).await?);
    }

    CachedEntry::cache_modpack_files(
        version_id,
        hashes.clone(),
        project_ids.clone(),
        pool,
    )
    .await?;

    Ok(ModpackIdentifiers {
        hashes: hashes.into_iter().collect(),
        project_ids: project_ids.into_iter().collect(),
    })
}

fn project_type_from_api_name(project_type: &str) -> ProjectType {
    match project_type {
        "resourcepack" => ProjectType::ResourcePack,
        "shader" => ProjectType::ShaderPack,
        "datapack" => ProjectType::DataPack,
        _ => ProjectType::Mod,
    }
}

fn sort_content_items(items: &mut [ContentItem]) {
    items.sort_by(|left, right| {
        let left_name = left
            .project
            .as_ref()
            .map(|project| project.title.as_str())
            .unwrap_or(&left.file_name);
        let right_name = right
            .project
            .as_ref()
            .map(|project| project.title.as_str())
            .unwrap_or(&right.file_name);

        left_name
            .to_lowercase()
            .cmp(&right_name.to_lowercase())
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
}

#[cfg(test)]
mod tests {
    use super::{linked_curseforge_modpack_ids, linked_modrinth_modpack_ids};
    use crate::state::instances::InstanceLink;

    #[test]
    fn linked_modpack_ids_stay_provider_qualified() {
        let modrinth = InstanceLink::ModrinthModpack {
            project_id: "mr-project".to_string(),
            version_id: "mr-version".to_string(),
        };
        let curseforge = InstanceLink::CurseForgeModpack {
            project_id: "123".to_string(),
            version_id: "456".to_string(),
        };
        let imported = InstanceLink::ImportedModpack {
            project_id: Some("legacy-project".to_string()),
            version_id: Some("legacy-version".to_string()),
            name: None,
            version_number: None,
            filename: None,
        };

        assert_eq!(
            linked_modrinth_modpack_ids(&modrinth),
            Some(("mr-project", "mr-version")),
        );
        assert_eq!(linked_curseforge_modpack_ids(&modrinth), None);
        assert_eq!(linked_modrinth_modpack_ids(&curseforge), None);
        assert_eq!(
            linked_curseforge_modpack_ids(&curseforge),
            Some(("123", "456")),
        );
        assert_eq!(linked_modrinth_modpack_ids(&imported), None);
        assert_eq!(linked_curseforge_modpack_ids(&imported), None);
    }
}
