use crate::state::instances::{
    ContentEntry, InstanceFile,
    adapters::sqlite::{content_rows, instance_rows},
};
use crate::state::{
    CacheBehaviour, CachedEntry, ContentProvider, ContentProviderRef,
    CurseForgeFileId, CurseForgeProjectId, ModrinthProjectId,
    ModrinthVersionId, ProjectType, ReleaseChannel, State,
};
use std::collections::HashMap;

use super::sync_content_files::{
    project_type_for_file, sync_instance_content_files,
};

#[derive(Clone, Debug)]
pub(crate) enum ContentUpdate {
    Modrinth {
        relative_path: String,
        project_id: ModrinthProjectId,
        current_version_id: ModrinthVersionId,
        update_version_id: ModrinthVersionId,
    },
    CurseForge {
        relative_path: String,
        project_id: CurseForgeProjectId,
        current_file_id: CurseForgeFileId,
        target_file_id: CurseForgeFileId,
    },
}

impl ContentUpdate {
    pub fn relative_path(&self) -> &str {
        match self {
            Self::Modrinth { relative_path, .. }
            | Self::CurseForge { relative_path, .. } => relative_path,
        }
    }

    pub fn modrinth_ids(
        &self,
    ) -> Option<(&ModrinthProjectId, &ModrinthVersionId, &ModrinthVersionId)>
    {
        match self {
            Self::Modrinth {
                project_id,
                current_version_id,
                update_version_id,
                ..
            } => Some((project_id, current_version_id, update_version_id)),
            Self::CurseForge { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
struct UpdateCandidate {
    entry: Option<ContentEntry>,
    file: InstanceFile,
    project_type: ProjectType,
    project_id: ModrinthProjectId,
    current_version_id: ModrinthVersionId,
}

pub(crate) async fn check_content_updates(
    instance_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<Vec<ContentUpdate>> {
    let instance = instance_rows::get_instance_by_id(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;
    let content_set =
        content_rows::get_applied_content_set(&instance.id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Instance {} has no applied content set",
                    instance.id
                ))
            })?;
    let entries =
        content_rows::get_content_entries(&content_set.id, &state.pool).await?;
    let entries_by_file_id = entries
        .iter()
        .filter_map(|entry| {
            entry.file_id.as_deref().map(|file_id| (file_id, entry))
        })
        .collect::<HashMap<_, _>>();
    let files = sync_instance_content_files(&instance, state).await?;
    let files_by_id = files
        .iter()
        .map(|file| (file.id.clone(), file.clone()))
        .collect::<HashMap<_, _>>();
    let mut provider_refs_by_file_id = HashMap::new();
    let mut origin_provider_by_file_id = HashMap::new();
    for entry in &entries {
        let Some(file_id) = entry.file_id.as_deref() else {
            continue;
        };
        provider_refs_by_file_id.insert(
            file_id.to_string(),
            content_rows::get_content_provider_refs(&entry.id, &state.pool)
                .await?,
        );
        origin_provider_by_file_id.insert(
            file_id.to_string(),
            content_rows::get_content_origin_provider(&entry.id, &state.pool)
                .await?,
        );
    }
    let hashes = files
        .iter()
        .filter(|file| {
            let refs = provider_refs_by_file_id.get(&file.id);
            let origin = origin_provider_by_file_id
                .get(&file.id)
                .and_then(|provider| *provider);
            refs.is_none_or(|refs| {
                refs.is_empty()
                    || (refs.iter().all(|reference| {
                        matches!(reference, ContentProviderRef::Modrinth { .. })
                    }) && origin != Some(ContentProvider::CurseForge))
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
    let mut candidates = Vec::new();
    for file in files {
        let refs = provider_refs_by_file_id.get(&file.id);
        if refs.is_some_and(|refs| {
            !refs.is_empty()
                && !refs.iter().any(|reference| {
                    matches!(reference, ContentProviderRef::Modrinth { .. })
                })
        }) {
            continue;
        }
        let Some(metadata) = file_info_by_hash.get(&file.sha1) else {
            continue;
        };
        if origin_provider_by_file_id
            .get(&file.id)
            .and_then(|provider| *provider)
            != Some(ContentProvider::Modrinth)
        {
            continue;
        }
        let Some(project_type) = project_type_for_file(&file) else {
            continue;
        };
        let project_id = ModrinthProjectId::new(metadata.project_id.clone())?;
        let current_version_id =
            ModrinthVersionId::new(metadata.version_id.clone())?;
        candidates.push(UpdateCandidate {
            entry: entries_by_file_id.get(file.id.as_str()).copied().cloned(),
            file,
            project_type,
            project_id,
            current_version_id,
        });
    }

    let mut output = Vec::new();
    if !candidates.is_empty() {
        let installed_channels =
            installed_update_channels(&candidates, cache_behaviour, state)
                .await?;
        let update_keys = candidates
            .iter()
            .map(|candidate| {
                update_cache_key(
                    &candidate.file,
                    candidate.project_type,
                    effective_update_channel(
                        instance.update_channel,
                        installed_channels.get(&candidate.file.sha1).copied(),
                    ),
                    &content_set.game_version,
                    content_set.loader.as_str(),
                )
            })
            .collect::<Vec<_>>();
        let update_key_refs = update_keys
            .iter()
            .map(|key| key.as_str())
            .collect::<Vec<_>>();
        let updates = CachedEntry::get_file_update_many(
            &update_key_refs,
            cache_behaviour,
            &state.pool,
            &state.api_semaphore,
        )
        .await?;
        let mut updates_by_hash: HashMap<String, Vec<String>> = HashMap::new();
        for update in updates {
            updates_by_hash
                .entry(update.hash)
                .or_default()
                .push(update.update_version_id);
        }

        for candidate in candidates {
            let update_version_id = updates_by_hash
                .remove(&candidate.file.sha1)
                .unwrap_or_default()
                .into_iter()
                .find(|update_version_id| {
                    update_version_id != candidate.current_version_id.as_str()
                })
                .map(|update_version_id| {
                    ModrinthVersionId::new(update_version_id)
                })
                .transpose()?;

            if let Some(entry) = &candidate.entry {
                content_rows::upsert_content_update_check(
                    &entry.id,
                    instance.update_channel,
                    Some(ContentProvider::Modrinth),
                    Some(candidate.project_id.as_str()),
                    update_version_id.as_ref().map(ModrinthVersionId::as_str),
                    &state.pool,
                )
                .await?;
            }

            if let Some(update_version_id) = update_version_id {
                output.push(ContentUpdate::Modrinth {
                    relative_path: candidate.file.relative_path,
                    project_id: candidate.project_id,
                    current_version_id: candidate.current_version_id,
                    update_version_id,
                });
            }
        }
    }

    let curseforge_projects =
        curseforge_projects_for_updates(&provider_refs_by_file_id).await?;
    for (file_id, refs) in &provider_refs_by_file_id {
        if origin_provider_by_file_id
            .get(file_id)
            .and_then(|provider| *provider)
            != Some(ContentProvider::CurseForge)
        {
            continue;
        }
        let Some(ContentProviderRef::CurseForge {
            project_id,
            file_id: Some(current_file_id),
        }) = refs.iter().find(|reference| {
            matches!(reference, ContentProviderRef::CurseForge { .. })
        })
        else {
            continue;
        };
        let Some(instance_file) = files_by_id.get(file_id) else {
            continue;
        };
        let Some(project_type) = project_type_for_file(instance_file) else {
            continue;
        };
        let Some(project) = curseforge_projects.get(&project_id.get()) else {
            continue;
        };
        let Some(target_file_id) =
            project.latest_files_indexes.iter().find_map(|index| {
                if index.game_version != content_set.game_version
                    || (project_type == ProjectType::Mod
                        && index.mod_loader
                            != curseforge_loader_type(
                                content_set.loader.as_str(),
                            ))
                {
                    return None;
                }
                (index.file_id != current_file_id.get())
                    .then_some(index.file_id)
            })
        else {
            continue;
        };
        let target_file_id = CurseForgeFileId::new(target_file_id)?;
        if let Some(entry) = entries_by_file_id.get(file_id.as_str()) {
            let project_id_string = project_id.get().to_string();
            let target_file_id_string = target_file_id.get().to_string();
            content_rows::upsert_content_update_check(
                &entry.id,
                instance.update_channel,
                Some(ContentProvider::CurseForge),
                Some(&project_id_string),
                Some(&target_file_id_string),
                &state.pool,
            )
            .await?;
        }
        output.push(ContentUpdate::CurseForge {
            relative_path: instance_file.relative_path.clone(),
            project_id: *project_id,
            current_file_id: *current_file_id,
            target_file_id,
        });
    }

    Ok(output)
}

async fn curseforge_projects_for_updates(
    provider_refs_by_file_id: &HashMap<String, Vec<ContentProviderRef>>,
) -> crate::Result<HashMap<u32, crate::api::curseforge::CurseForgeProject>> {
    if crate::api::curseforge::capability().status
        != crate::api::curseforge::CurseForgeCapabilityStatus::Ready
    {
        return Ok(HashMap::new());
    }
    let project_ids = provider_refs_by_file_id
        .values()
        .flatten()
        .filter_map(|reference| match reference {
            ContentProviderRef::CurseForge { project_id, .. } => {
                Some(project_id.get())
            }
            ContentProviderRef::Modrinth { .. } => None,
        })
        .collect::<std::collections::HashSet<_>>();
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(
        crate::api::curseforge::get_projects(project_ids.into_iter().collect())
            .await?
            .into_iter()
            .map(|project| (project.id, project))
            .collect(),
    )
}

fn curseforge_loader_type(loader: &str) -> Option<u32> {
    match loader {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

async fn installed_update_channels(
    candidates: &[UpdateCandidate],
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> crate::Result<HashMap<String, ReleaseChannel>> {
    let version_ids = candidates
        .iter()
        .filter_map(|candidate| Some(candidate.current_version_id.clone()))
        .collect::<Vec<_>>();
    let versions = CachedEntry::get_version_many(
        &version_ids,
        cache_behaviour,
        &state.pool,
        &state.api_semaphore,
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

    Ok(candidates
        .iter()
        .filter_map(|candidate| {
            channels_by_version_id
                .get(candidate.current_version_id.as_str())
                .copied()
                .map(|channel| (candidate.file.sha1.clone(), channel))
        })
        .collect())
}

fn effective_update_channel(
    preferred: ReleaseChannel,
    installed: Option<ReleaseChannel>,
) -> ReleaseChannel {
    installed.map_or(preferred, |channel| preferred.least_stable(channel))
}

fn update_cache_key(
    file: &InstanceFile,
    project_type: ProjectType,
    channel: ReleaseChannel,
    game_version: &str,
    loader: &str,
) -> String {
    format!(
        "{}-{}-{}-{}",
        file.sha1,
        if project_type == ProjectType::Mod {
            loader.to_string()
        } else {
            project_type.get_loaders().join("+")
        },
        channel.key(),
        game_version
    )
}
