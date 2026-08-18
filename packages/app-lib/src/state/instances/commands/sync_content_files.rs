use crate::State;
use crate::state::instances::adapters::{filesystem, sqlite};
use crate::state::instances::{Instance, InstanceFile};
use crate::state::{
    CacheBehaviour, CachedEntry, CachedFileUpdate, ContentProvider,
    ContentProviderRef, ModrinthVersionId, ProjectType,
};
use crate::util::fetch::{self, FetchSemaphore};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

pub(crate) async fn sync_content_files(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    let instance =
        sqlite::instance_rows::get_instance_by_id(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError("Unknown instance".to_string())
            })?;

    sync_instance_content_files(&instance, state).await
}

pub(crate) async fn sync_instance_content_files(
    instance: &Instance,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    // Keep the filesystem snapshot stable until its database rows commit.
    let _instance_lock = state.lock_instance_content(&instance.id).await;
    let scanned = filesystem::scan_content_files(
        &state.directories.instances_dir(),
        &instance.path,
    )?;
    let scanned_paths = scanned
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<HashSet<_>>();
    let cache_keys = scanned
        .iter()
        .map(|file| file.hash_cache_key.as_str())
        .collect::<Vec<_>>();
    let hashes = CachedEntry::get_file_hash_many(
        &cache_keys,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let hashes_by_key = hashes
        .into_iter()
        .map(|hash| {
            (
                format!(
                    "{}-{}",
                    hash.size,
                    hash.path.trim_end_matches(".disabled")
                ),
                hash,
            )
        })
        .collect::<HashMap<_, _>>();
    let existing_files =
        sqlite::content_rows::get_instance_files(&instance.id, &state.pool)
            .await?;
    let mut existing_files_by_path = HashMap::new();
    let mut existing_files_by_sha1: HashMap<String, Vec<InstanceFile>> =
        HashMap::new();
    for file in existing_files {
        existing_files_by_sha1
            .entry(file.sha1.clone())
            .or_default()
            .push(file.clone());
        existing_files_by_path.insert(file.relative_path.clone(), file);
    }
    let entry_file_ids = match sqlite::content_rows::get_applied_content_set(
        &instance.id,
        &state.pool,
    )
    .await?
    {
        Some(content_set) => sqlite::content_rows::get_content_entries(
            &content_set.id,
            &state.pool,
        )
        .await?
        .into_iter()
        .filter_map(|entry| entry.file_id)
        .collect(),
        None => HashSet::new(),
    };

    let now = Utc::now();
    let mut files: Vec<InstanceFile> = Vec::new();
    let mut reclaims: HashMap<String, String> = HashMap::new();
    let mut merges: HashMap<String, String> = HashMap::new();
    let mut claimed_reclaim_ids = HashSet::new();

    for file in scanned {
        let hash_key = file.hash_cache_key.trim_end_matches(".disabled");
        let Some(hash) = hashes_by_key.get(hash_key) else {
            continue;
        };
        let existing_file = existing_files_by_path.get(&file.relative_path);
        let reclaim_candidate = if existing_file.is_some() {
            None
        } else {
            reclaimable_existing_file(
                &hash.hash,
                &file.relative_path,
                &existing_files_by_sha1,
                &scanned_paths,
                &claimed_reclaim_ids,
            )
        };
        let merge_candidate = existing_file
            .filter(|file| !entry_file_ids.contains(&file.id))
            .and_then(|file| {
                mergeable_tracked_file(
                    &hash.hash,
                    &file.relative_path,
                    &file.id,
                    &existing_files_by_sha1,
                    &entry_file_ids,
                    &scanned_paths,
                    &claimed_reclaim_ids,
                )
            });
        let source_file = existing_file.or(reclaim_candidate);
        if let Some(candidate) = reclaim_candidate {
            claimed_reclaim_ids.insert(candidate.id.clone());
            reclaims.insert(
                file.relative_path.clone(),
                candidate.relative_path.clone(),
            );
        }
        if let Some(candidate) = merge_candidate {
            claimed_reclaim_ids.insert(candidate.id.clone());
            merges.insert(file.relative_path.clone(), candidate.id.clone());
        }

        files.push(InstanceFile {
            id: source_file
                .map(|file| file.id.clone())
                .unwrap_or_else(instance_file_id),
            instance_id: instance.id.clone(),
            relative_path: file.relative_path,
            file_name: file.file_name,
            enabled: file.enabled,
            sha1: hash.hash.clone(),
            size: file.size,
            missing: false,
            added_at: source_file.map(|file| file.added_at).unwrap_or(now),
            modified_at: now,
            local_mod_data: source_file.and_then(|f| f.local_mod_data.clone()),
            icon_path: source_file.and_then(|f| f.icon_path.clone()),
        });
    }

    // Extract local mod metadata (Mod JARs) and cached icons (Mod JARs and
    // resource packs) for files that don't have them yet. This also backfills
    // rows created before these features existed; `icon_path` distinguishes
    // not-attempted (NULL), no-icon (empty string), and cached (path).
    let instance_dir = state.directories.instances_dir().join(&instance.path);
    let icon_cache_dir = state.directories.caches_dir().join("icons");
    for file in &mut files {
        let Some(project_type) = project_type_for_file(file) else {
            continue;
        };
        // Re-extract metadata written before dependency extraction existed:
        // legacy JSON parses with `dependencies: None` and needs one pass
        // through the updated extractor.
        let extract_metadata = project_type == ProjectType::Mod
            && file
                .local_mod_data
                .as_ref()
                .and_then(|json| {
                    serde_json::from_str::<
                        crate::mod_metadata::LocalModMetadata,
                    >(json)
                    .ok()
                })
                .and_then(|metadata| metadata.dependencies)
                .is_none();
        let extract_icon = file.icon_path.is_none()
            && matches!(
                project_type,
                ProjectType::Mod | ProjectType::ResourcePack
            );
        if !extract_metadata && !extract_icon {
            continue;
        }

        let path = instance_dir.join(&file.relative_path);

        // Resource packs are read entry-wise so large archives are not
        // materialized in memory just to fetch `pack.png`.
        if extract_icon && project_type == ProjectType::ResourcePack {
            let icon =
                crate::mod_metadata::icon::extract_resource_pack_icon(&path);
            file.icon_path = Some(
                cache_extracted_icon(icon, &file.sha1, &icon_cache_dir, state)
                    .await,
            );
            continue;
        }

        // Mods: one in-memory read serves both metadata and icon extraction.
        let bytes = match tokio::fs::read(&path).await {
            Ok(data) => bytes::Bytes::from(data),
            Err(_) => {
                // File temporarily inaccessible; skip silently.
                continue;
            }
        };

        if extract_metadata
            && let Some(meta) =
                crate::mod_metadata::extract_mod_metadata(&bytes)
            && let Ok(json) = serde_json::to_string(&meta)
        {
            file.local_mod_data = Some(json);
        }

        if extract_icon {
            let meta = file.local_mod_data.as_ref().and_then(|json| {
                serde_json::from_str::<crate::mod_metadata::LocalModMetadata>(
                    json,
                )
                .ok()
            });
            let icon = crate::mod_metadata::icon::extract_mod_icon(
                &bytes,
                meta.as_ref(),
            );
            file.icon_path = Some(
                cache_extracted_icon(icon, &file.sha1, &icon_cache_dir, state)
                    .await,
            );
        }
    }

    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlite::content_rows::ensure_instance_exists(&instance.id, &mut tx).await?;
    sqlite::content_rows::mark_instance_files_missing(&instance.id, &mut tx)
        .await?;

    // Upsert with a fresh id lookup inside the transaction. The ids assigned
    // during the scan may be stale if a concurrent operation (e.g. batch
    // disable renaming files to `.disabled`) moved a row after the snapshot;
    // reusing a stale id against the moved row would trip the UNIQUE
    // constraint on `instance_files.id` (code 1555).
    let mut synced_files: Vec<InstanceFile> = Vec::with_capacity(files.len());
    for file in &files {
        let synced =
            if let Some(tracked_file_id) = merges.get(&file.relative_path) {
                sqlite::content_rows::adopt_untracked_file_in_transaction(
                    &instance.id,
                    &file.relative_path,
                    tracked_file_id,
                    &mut tx,
                )
                .await?;
                upsert_scanned_file(&instance.id, file, &mut tx).await?
            } else if let Some(old_relative_path) =
                reclaims.get(&file.relative_path)
            {
                match sqlite::content_rows::move_instance_file_in_transaction(
                    &instance.id,
                    old_relative_path,
                    &file.relative_path,
                    &file.file_name,
                    file.enabled,
                    &file.sha1,
                    file.size,
                    file.local_mod_data.as_deref(),
                    file.icon_path.as_deref(),
                    &mut tx,
                )
                .await?
                {
                    Some(file) => file,
                    None => {
                        upsert_scanned_file(&instance.id, file, &mut tx).await?
                    }
                }
            } else {
                upsert_scanned_file(&instance.id, file, &mut tx).await?
            };
        synced_files.push(synced);
    }

    tx.commit().await?;

    Ok(synced_files)
}

async fn cache_extracted_icon(
    icon: Option<(String, Vec<u8>)>,
    sha1: &str,
    icon_cache_dir: &Path,
    state: &State,
) -> String {
    let Some((entry_name, icon_bytes)) = icon else {
        return String::new();
    };

    let extension = icon_extension(&entry_name);
    let cache_path = icon_cache_dir.join(format!("{sha1}.{extension}"));
    match fetch::write(&cache_path, &icon_bytes, &state.io_semaphore).await {
        Ok(()) => crate::util::io::canonicalize(&cache_path)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|_| cache_path.to_string_lossy().into_owned()),
        Err(_) => String::new(),
    }
}

fn icon_extension(entry_name: &str) -> &str {
    let extension = Path::new(entry_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if matches!(
        extension.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg"
    ) {
        extension
    } else {
        "png"
    }
}

pub(crate) fn project_type_for_file(
    file: &InstanceFile,
) -> Option<ProjectType> {
    filesystem::project_type_from_relative_path(&file.relative_path)
}

pub(crate) fn installed_modrinth_version_id(
    provider_refs: &[ContentProviderRef],
) -> Option<ModrinthVersionId> {
    provider_refs.iter().find_map(|reference| match reference {
        ContentProviderRef::Modrinth { version_id, .. } => {
            version_id.as_ref().cloned()
        }
        ContentProviderRef::CurseForge { .. } => None,
    })
}

pub(crate) async fn fetch_content_file_updates(
    update_key_refs: &[&str],
    cache_behaviour: Option<CacheBehaviour>,
    refresh: bool,
    pool: &sqlx::SqlitePool,
    fetch_semaphore: &FetchSemaphore,
) -> crate::Result<Vec<CachedFileUpdate>> {
    let update_behaviour = if refresh {
        Some(CacheBehaviour::Bypass)
    } else {
        cache_behaviour
    };

    match CachedEntry::get_file_update_many(
        update_key_refs,
        update_behaviour,
        pool,
        fetch_semaphore,
    )
    .await
    {
        Ok(updates) => Ok(updates),
        Err(error) if refresh => {
            tracing::warn!(
                "Content update refresh failed, using cached update data: {error}"
            );
            CachedEntry::get_file_update_many(
                update_key_refs,
                Some(CacheBehaviour::CacheOnly),
                pool,
                fetch_semaphore,
            )
            .await
        }
        Err(error) => Err(error),
    }
}

/// Whether a file may receive Modrinth update suggestions.
///
/// Files installed from Modrinth (origin `Modrinth`) always qualify. Untracked
/// or locally recorded files (no origin) qualify as long as no CurseForge
/// reference ties them to a different provider; CurseForge-origin files are
/// handled by the CurseForge update path instead.
pub(crate) fn modrinth_update_enabled(
    origin_provider: Option<ContentProvider>,
    provider_refs: &[ContentProviderRef],
) -> bool {
    match origin_provider {
        Some(ContentProvider::Modrinth) => true,
        Some(ContentProvider::CurseForge) | Some(ContentProvider::Local) => {
            false
        }
        None => provider_refs.iter().all(|reference| {
            matches!(reference, ContentProviderRef::Modrinth { .. })
        }),
    }
}

fn instance_file_id() -> String {
    format!("instance-file:{}", Uuid::new_v4())
}

async fn upsert_scanned_file(
    instance_id: &str,
    file: &InstanceFile,
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> crate::Result<InstanceFile> {
    sqlite::content_rows::upsert_instance_file_from_parts_in_transaction(
        sqlite::content_rows::UpsertInstanceFile {
            instance_id,
            relative_path: &file.relative_path,
            file_name: &file.file_name,
            enabled: file.enabled,
            sha1: &file.sha1,
            size: file.size,
            missing: false,
            local_mod_data: file.local_mod_data.as_deref(),
            icon_path: file.icon_path.as_deref(),
        },
        tx,
    )
    .await
}

/// Finds the single missing file row that can safely inherit a scanned file's
/// installed identity. A rename should keep its provider refs, entry, and
/// install history; ambiguous duplicates and cross-type moves stay untracked.
fn reclaimable_existing_file<'a>(
    sha1: &str,
    new_relative_path: &str,
    existing_files_by_sha1: &'a HashMap<String, Vec<InstanceFile>>,
    scanned_paths: &HashSet<String>,
    claimed_ids: &HashSet<String>,
) -> Option<&'a InstanceFile> {
    let candidates = existing_files_by_sha1.get(sha1)?;
    if candidates
        .iter()
        .any(|file| scanned_paths.contains(file.relative_path.as_str()))
    {
        return None;
    }
    let missing = candidates
        .iter()
        .filter(|file| {
            file.missing || !scanned_paths.contains(&file.relative_path)
        })
        .collect::<Vec<_>>();
    if missing.len() != 1 || claimed_ids.contains(&missing[0].id) {
        return None;
    }
    let candidate = missing[0];
    let new_type =
        filesystem::project_type_from_relative_path(new_relative_path)?;
    let old_type =
        filesystem::project_type_from_relative_path(&candidate.relative_path)?;
    (new_type == old_type).then_some(candidate)
}

/// Finds the single tracked row that should hand its installed identity to an
/// untracked row at a new path. This repairs instances that were already
/// broken by a rename before reclaim-on-move existed.
fn mergeable_tracked_file<'a>(
    sha1: &str,
    new_relative_path: &str,
    untracked_file_id: &str,
    existing_files_by_sha1: &'a HashMap<String, Vec<InstanceFile>>,
    entry_file_ids: &HashSet<String>,
    scanned_paths: &HashSet<String>,
    claimed_ids: &HashSet<String>,
) -> Option<&'a InstanceFile> {
    let candidates = existing_files_by_sha1.get(sha1)?;
    let tracked = candidates
        .iter()
        .filter(|file| entry_file_ids.contains(&file.id))
        .collect::<Vec<_>>();
    if tracked.is_empty()
        || tracked.iter().any(|file| {
            file.id != untracked_file_id
                && scanned_paths.contains(file.relative_path.as_str())
        })
    {
        return None;
    }
    let missing = tracked
        .iter()
        .filter(|file| {
            file.id != untracked_file_id
                && (file.missing
                    || !scanned_paths.contains(file.relative_path.as_str()))
        })
        .collect::<Vec<_>>();
    if missing.len() != 1 || claimed_ids.contains(&missing[0].id) {
        return None;
    }
    let candidate = missing[0];
    let new_type =
        filesystem::project_type_from_relative_path(new_relative_path)?;
    let old_type =
        filesystem::project_type_from_relative_path(&candidate.relative_path)?;
    (new_type == old_type).then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        CurseForgeFileId, CurseForgeProjectId, ModrinthProjectId,
        ModrinthVersionId,
    };
    use std::fs;

    fn modrinth_ref() -> ContentProviderRef {
        ContentProviderRef::Modrinth {
            project_id: ModrinthProjectId::new("project").unwrap(),
            version_id: Some(ModrinthVersionId::new("version").unwrap()),
        }
    }

    fn curseforge_ref() -> ContentProviderRef {
        ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(42).unwrap(),
            file_id: Some(CurseForgeFileId::new(7).unwrap()),
        }
    }

    #[test]
    fn untracked_files_qualify_for_modrinth_updates() {
        assert!(modrinth_update_enabled(None, &[]));
        assert!(modrinth_update_enabled(None, &[modrinth_ref()]));
    }

    #[test]
    fn curseforge_tracked_files_do_not_qualify_for_modrinth_updates() {
        assert!(!modrinth_update_enabled(
            Some(ContentProvider::CurseForge),
            &[curseforge_ref()],
        ));
        assert!(!modrinth_update_enabled(None, &[curseforge_ref()]));
    }

    #[test]
    fn modrinth_origin_always_qualifies() {
        assert!(modrinth_update_enabled(
            Some(ContentProvider::Modrinth),
            &[curseforge_ref(), modrinth_ref()],
        ));
    }

    #[test]
    fn content_scan_preserves_install_temporary_files() {
        let root = tempfile::tempdir().unwrap();
        let mods = root.path().join("instance/mods");
        fs::create_dir_all(&mods).unwrap();
        let temporary_names = [
            "example.jar.installing.download",
            "example.jar.installing",
            "example.jar.installing.previous",
        ];
        for name in temporary_names {
            fs::write(mods.join(name), name).unwrap();
        }

        let scanned =
            filesystem::scan_content_files(root.path(), "instance").unwrap();

        assert!(scanned.is_empty());
        for name in temporary_names {
            assert!(mods.join(name).is_file());
        }
    }
}
