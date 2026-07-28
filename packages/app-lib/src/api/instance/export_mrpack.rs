use super::content::get_projects;
use super::get::get;
use super::paths::get_full_path;
use crate::api::content_search::original_content_relative_path;
use crate::event::LoadingBarType;
use crate::event::emit::{emit_loading, init_loading};
use crate::pack::install_from::{
    EnvType, PackDependency, PackFile, PackFileHash, PackFormat,
};
use crate::state::{
    CacheBehaviour, CachedEntry, ContentProviderRef, InstanceMetadata,
    ModLoader, ModrinthVersionId, SideType, State,
};
use crate::util::io::{self, IOError};
use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
use path_util::SafeRelativeUtf8UnixPathBuf;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[tracing::instrument(skip_all)]
pub async fn export_mrpack(
    instance_id: &str,
    export_path: PathBuf,
    included_export_candidates: Vec<String>,
    version_id: Option<String>,
    description: Option<String>,
    _name: Option<String>,
) -> crate::Result<()> {
    let state = State::get().await?;
    let _permit: tokio::sync::SemaphorePermit =
        state.io_semaphore.0.acquire().await?;
    let metadata = get(instance_id).await?.ok_or_else(|| {
        crate::ErrorKind::OtherError(format!(
            "Tried to export a nonexistent instance {instance_id}!"
        ))
    })?;
    let included_export_candidates = included_export_candidates
        .into_iter()
        .filter(|x| {
            if let Some(f) = PathBuf::from(x).file_name()
                && f.to_string_lossy().starts_with(".DS_Store")
            {
                return false;
            }
            true
        })
        .collect::<Vec<_>>();

    let instance_base_path = get_full_path(instance_id).await?;
    let mut file = File::create(&export_path)
        .await
        .map_err(|e| IOError::with_path(e, &export_path))?;
    let mut writer = ZipFileWriter::with_tokio(&mut file);
    let version_id = version_id.unwrap_or("1.0.0".to_string());
    let mut packfile =
        create_mrpack_json(&metadata, version_id, description).await?;
    packfile.files.retain(|f| {
        is_export_candidate_included(
            f.path.as_str(),
            &included_export_candidates,
        )
    });
    strip_localized_pack_file_paths(&mut packfile.files);

    let mut path_list = Vec::new();
    add_all_recursive_folder_paths(&instance_base_path, &mut path_list).await?;
    let loading_bar = init_loading(
        LoadingBarType::ZipExtract {
            instance_id: metadata.instance.id.clone(),
            instance_name: metadata.instance.name.clone(),
        },
        path_list.len() as f64,
        "Exporting instance to .mrpack",
    )
    .await?;

    let disk_paths = path_list
        .iter()
        .filter(|path| path.is_file())
        .filter_map(|path| {
            pack_get_relative_path(&instance_base_path, path)
                .ok()
                .map(|relative_path| relative_path.as_str().to_string())
        })
        .collect::<HashSet<_>>();
    let mut written_override_paths = HashSet::new();
    for path in path_list {
        emit_loading(&loading_bar, 1.0, None)?;
        let relative_path = pack_get_relative_path(&instance_base_path, &path)?;
        let exported_path =
            original_content_relative_path(relative_path.as_str());

        if packfile
            .files
            .iter()
            .any(|f| f.path.as_str() == exported_path)
            || !is_export_candidate_included(
                relative_path.as_str(),
                &included_export_candidates,
            )
        {
            continue;
        }

        if path.is_file() {
            let mut file = File::open(&path)
                .await
                .map_err(|e| IOError::with_path(e, &path))?;
            let mut data = Vec::new();
            file.read_to_end(&mut data).await.map_err(IOError::from)?;
            let entry_path = if exported_path != relative_path.as_str()
                && !disk_paths.contains(&exported_path)
                && written_override_paths.insert(exported_path.clone())
            {
                exported_path
            } else {
                relative_path.as_str().to_string()
            };
            let builder = ZipEntryBuilder::new(
                format!("overrides/{entry_path}").into(),
                Compression::Deflate,
            );
            writer.write_entry_whole(builder, &data).await?;
        }
    }

    let data = serde_json::to_vec_pretty(&packfile)?;
    let builder = ZipEntryBuilder::new(
        "modrinth.index.json".to_string().into(),
        Compression::Deflate,
    );
    writer.write_entry_whole(builder, &data).await?;
    writer.close().await?;

    Ok(())
}

fn is_export_candidate_included(
    path: &str,
    included_export_candidates: &[String],
) -> bool {
    included_export_candidates.iter().any(|candidate| {
        path == candidate
            || path
                .strip_prefix(candidate)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

/// Rewrites `[中文名]`-prefixed install paths back to their original names so
/// exported packs stay free of localized file names. Entries whose stripped
/// path would collide with another entry keep their on-disk name.
fn strip_localized_pack_file_paths(files: &mut [PackFile]) {
    let mut used = files
        .iter()
        .map(|file| file.path.as_str().to_string())
        .collect::<HashSet<_>>();
    for file in files {
        let stripped = original_content_relative_path(file.path.as_str());
        if stripped == file.path.as_str() || used.contains(&stripped) {
            continue;
        }
        let Ok(path) = SafeRelativeUtf8UnixPathBuf::try_from(stripped.clone())
        else {
            continue;
        };
        used.insert(stripped);
        file.path = path;
    }
}

#[tracing::instrument]
pub async fn get_pack_export_candidates(
    instance_id: &str,
) -> crate::Result<Vec<SafeRelativeUtf8UnixPathBuf>> {
    let mut path_list = Vec::new();
    let instance_base_dir = get_full_path(instance_id).await?;
    let mut read_dir = io::read_dir(&instance_base_dir).await?;
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, &instance_base_dir))?
    {
        let path = entry.path();
        if path.is_dir() {
            let mut read_dir = io::read_dir(&path).await?;
            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| IOError::with_path(e, &instance_base_dir))?
            {
                path_list.push(pack_get_relative_path(
                    &instance_base_dir,
                    &entry.path(),
                )?);
            }
        } else {
            path_list.push(pack_get_relative_path(&instance_base_dir, &path)?);
        }
    }
    Ok(path_list)
}

fn pack_get_relative_path(
    instance_path: &PathBuf,
    path: &PathBuf,
) -> crate::Result<SafeRelativeUtf8UnixPathBuf> {
    Ok(SafeRelativeUtf8UnixPathBuf::try_from(
        path.strip_prefix(instance_path)
            .map_err(|_| {
                crate::ErrorKind::FSError(format!(
                    "Path {path:?} does not correspond to an instance"
                ))
            })?
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )?)
}

#[tracing::instrument(skip_all)]
pub async fn create_mrpack_json(
    metadata: &InstanceMetadata,
    version_id: String,
    description: Option<String>,
) -> crate::Result<PackFormat> {
    let mut dependencies = HashMap::new();
    match (
        metadata.applied_content_set.loader,
        metadata.applied_content_set.loader_version.clone(),
    ) {
        (ModLoader::Forge, Some(v)) => {
            dependencies.insert(PackDependency::Forge, v)
        }
        (ModLoader::NeoForge, Some(v)) => {
            dependencies.insert(PackDependency::NeoForge, v)
        }
        (ModLoader::Fabric, Some(v)) => {
            dependencies.insert(PackDependency::FabricLoader, v)
        }
        (ModLoader::Quilt, Some(v)) => {
            dependencies.insert(PackDependency::QuiltLoader, v)
        }
        (ModLoader::Vanilla, _) => None,
        (ModLoader::OptiFine, _) => {
            return Err(crate::ErrorKind::OtherError(
                "OptiFine instances cannot be exported to mrpack, as the format has no OptiFine dependency type".to_string(),
            )
            .into());
        }
        _ => {
            return Err(crate::ErrorKind::OtherError(
                "Loader version mismatch".to_string(),
            )
            .into());
        }
    };
    dependencies.insert(
        PackDependency::Minecraft,
        metadata.applied_content_set.game_version.clone(),
    );

    let state = State::get().await?;
    let projects = get_projects(
        &metadata.instance.id,
        Some(CacheBehaviour::MustRevalidate),
    )
    .await?
    .into_iter()
    .collect::<Vec<_>>();
    let instance_path = get_full_path(&metadata.instance.id).await?;
    let mut modrinth_version_ids = projects
        .iter()
        .flat_map(|(_, file)| file.provider_refs.iter())
        .filter_map(|reference| match reference {
            ContentProviderRef::Modrinth {
                version_id: Some(version_id),
                ..
            } => Some(version_id.to_string()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    for file in projects.iter().map(|(_, file)| file) {
        if let Some(metadata) = &file.modrinth {
            modrinth_version_ids.insert(metadata.version_id.to_string());
        }
    }
    let modrinth_version_id_refs = modrinth_version_ids
        .iter()
        .map(|id| ModrinthVersionId::new(id.clone()))
        .collect::<crate::Result<Vec<_>>>()?;
    let versions = CachedEntry::get_version_many(
        &modrinth_version_id_refs,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let versions_by_id = versions
        .into_iter()
        .map(|version| (version.id.clone(), version))
        .collect::<HashMap<_, _>>();
    let mut files = Vec::new();
    let mut remote_paths = HashSet::new();
    for (path, content_file) in projects {
        let disk_path = instance_path.join(path.as_str());
        let Ok((disk_size, local_sha1)) =
            crate::util::fetch::sha1_file_async(&disk_path).await
        else {
            continue;
        };
        let Some(file_size) = u32::try_from(disk_size).ok() else {
            continue;
        };
        let mut remote: Option<(HashMap<PackFileHash, String>, String)> = None;
        for reference in &content_file.provider_refs {
            match reference {
                ContentProviderRef::Modrinth {
                    version_id: Some(version_id),
                    ..
                } => {
                    let Some(version) = versions_by_id.get(version_id.as_str())
                    else {
                        continue;
                    };
                    if let Some(version_file) =
                        version.files.iter().find(|file| {
                            file.size == file_size
                                && file.hashes.get("sha1").is_some_and(|hash| {
                                    hash.eq_ignore_ascii_case(&local_sha1)
                                })
                                && !file.url.trim().is_empty()
                        })
                    {
                        remote = Some((
                            version_file
                                .hashes
                                .clone()
                                .into_iter()
                                .map(|(kind, hash)| {
                                    (PackFileHash::from(kind), hash)
                                })
                                .collect(),
                            version_file.url.clone(),
                        ));
                        break;
                    }
                }
                ContentProviderRef::CurseForge {
                    project_id,
                    file_id: Some(file_id),
                } => {
                    let Ok(file) = crate::api::curseforge::get_file(
                        project_id.get(),
                        file_id.get(),
                    )
                    .await
                    else {
                        continue;
                    };
                    let allowed =
                        crate::api::curseforge::get_project(project_id.get())
                            .await
                            .ok()
                            .and_then(|project| project.allow_mod_distribution)
                            .unwrap_or(true);
                    if !allowed {
                        continue;
                    }
                    let hash = file
                        .hashes
                        .iter()
                        .find(|hash| hash.algo == 1)
                        .map(|hash| hash.value.as_str());
                    if file.file_length == u64::from(file_size)
                        && hash.is_some_and(|hash| {
                            hash.eq_ignore_ascii_case(&local_sha1)
                        })
                        && file
                            .download_url
                            .as_deref()
                            .is_some_and(|url| !url.trim().is_empty())
                    {
                        let mut hashes = HashMap::new();
                        if let Some(hash) = hash {
                            hashes.insert(PackFileHash::Sha1, hash.to_string());
                        }
                        remote = Some((
                            hashes,
                            file.download_url.unwrap_or_default(),
                        ));
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some((hashes, download)) = remote else {
            continue;
        };
        let Ok(path) = SafeRelativeUtf8UnixPathBuf::try_from(
            original_content_relative_path(path.as_str()),
        ) else {
            continue;
        };
        if !remote_paths.insert(path.as_str().to_string()) {
            continue;
        }
        let mut env = HashMap::new();
        env.insert(EnvType::Client, SideType::Required);
        env.insert(EnvType::Server, SideType::Required);
        files.push(PackFile {
            path,
            hashes,
            env: Some(env),
            downloads: vec![download],
            file_size,
        });
    }

    Ok(PackFormat {
        game: "minecraft".to_string(),
        format_version: 1,
        version_id,
        name: metadata.instance.name.clone(),
        summary: description,
        files,
        dependencies,
    })
}

#[async_recursion::async_recursion]
async fn add_all_recursive_folder_paths(
    folder: &PathBuf,
    output: &mut Vec<PathBuf>,
) -> crate::Result<()> {
    let mut read_dir = io::read_dir(folder).await?;
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, folder))?
    {
        let path = entry.path();
        if path.is_dir() {
            add_all_recursive_folder_paths(&path, output).await?;
        } else {
            output.push(path);
        }
    }

    Ok(())
}
