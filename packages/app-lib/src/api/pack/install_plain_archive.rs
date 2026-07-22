//! Installer for plain zipped-up game folders.
//!
//! These archives have no pack manifest at all — they are a `.minecraft`
//! folder (optionally wrapped in extra directories) identified by a
//! `versions/<id>/<id>.json` structure. The version JSON is inspected to
//! guess the game version and loader, and the folder contents become the
//! instance's game directory.

use std::path::PathBuf;

use serde::Deserialize;

use super::archive_util;
use crate::data::ModLoader;
use crate::install::{
    InstallPhaseDetails, InstallPhaseId, InstallProgressReporter,
};
use crate::state::{
    AppliedContentSetPatch, ContentSourceKind, EditInstance,
    InstanceInstallStage, InstanceLink,
};

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct PlainVersionJson {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    inherits_from: Option<String>,
    #[serde(default)]
    libraries: Vec<PlainLibrary>,
}

#[derive(Deserialize, Debug)]
struct PlainLibrary {
    #[serde(default)]
    name: Option<String>,
}

fn looks_like_game_version(value: &str) -> bool {
    let mut parts = value.split('.');
    parts.next().is_some_and(|part| part.parse::<u32>().is_ok())
        && value.split('.').skip(1).all(|part| {
            part.split('-')
                .next()
                .is_some_and(|part| part.parse::<u32>().is_ok())
        })
}

struct DetectedTarget {
    game_version: String,
    loader: ModLoader,
    loader_version: Option<String>,
}

fn detect_target(version_json: &PlainVersionJson) -> Option<DetectedTarget> {
    let mut game_version = version_json
        .inherits_from
        .clone()
        .filter(|value| looks_like_game_version(value));
    let mut loader = ModLoader::Vanilla;
    let mut loader_version = None;

    for library in &version_json.libraries {
        let Some(name) = &library.name else {
            continue;
        };
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() < 3 {
            continue;
        }
        let (group, artifact, version) = (parts[0], parts[1], parts[2]);
        match (group, artifact) {
            ("net.fabricmc", "fabric-loader") => {
                loader = ModLoader::Fabric;
                loader_version = Some(version.to_string());
            }
            ("org.quiltmc", "quilt-loader") => {
                loader = ModLoader::Quilt;
                loader_version = Some(version.to_string());
            }
            ("net.neoforged", "neoforge" | "forge") => {
                loader = ModLoader::NeoForge;
                loader_version = Some(version.to_string());
            }
            ("net.minecraftforge", "forge" | "fmlloader") => {
                loader = ModLoader::Forge;
                // Forge versions are usually stored as `<mc>-<forge>`.
                let forge_version = version
                    .split_once('-')
                    .map(|(mc, forge)| {
                        if game_version.is_none()
                            && looks_like_game_version(mc)
                        {
                            game_version = Some(mc.to_string());
                        }
                        forge.to_string()
                    })
                    .unwrap_or_else(|| version.to_string());
                loader_version = Some(forge_version);
            }
            ("optifine", "OptiFine") => {
                if loader == ModLoader::Vanilla {
                    loader = ModLoader::OptiFine;
                    // OptiFine library versions look like `<mc>_HD_U_I6`.
                    loader_version = Some(
                        version
                            .split_once('_')
                            .map(|(_, of)| of.to_string())
                            .unwrap_or_else(|| version.to_string()),
                    );
                }
            }
            _ => {}
        }
    }

    if game_version.is_none()
        && let Some(id) = &version_json.id
        && looks_like_game_version(id)
    {
        game_version = Some(id.clone());
    }

    game_version.map(|game_version| DetectedTarget {
        game_version,
        loader,
        loader_version,
    })
}

/// Reads every `versions/<id>/<id>.json` under the base folder. Archives of
/// modded installs usually contain both the vanilla and the modded version
/// folder, so all candidates are needed to pick the right one.
async fn read_version_candidates(
    archive_path: PathBuf,
    base_folder: String,
) -> crate::Result<Vec<(String, PlainVersionJson)>> {
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive_path).map_err(|error| {
            crate::util::io::IOError::with_path(error, &archive_path)
        })?;
        let mut archive = zip::ZipArchive::new(file).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Modpack archive is invalid: {error}"
            ))
        })?;
        let versions_prefix = format!("{base_folder}versions/");
        let mut candidates = Vec::new();
        for index in 0..archive.len() {
            let name = {
                let entry =
                    archive.by_index_raw(index).map_err(|error| {
                        crate::ErrorKind::InputError(format!(
                            "Failed to read modpack archive entry: {error}"
                        ))
                    })?;
                crate::pack::detect::decode_zip_entry_name(entry.name_raw())
            };
            let Some(rest) = name.strip_prefix(&versions_prefix) else {
                continue;
            };
            let mut segments = rest.split('/');
            let (Some(id), Some(json), None) =
                (segments.next(), segments.next(), segments.next())
            else {
                continue;
            };
            if json.strip_suffix(".json") != Some(id) {
                continue;
            }
            let id = id.to_string();
            let mut entry = archive.by_index(index).map_err(|error| {
                crate::ErrorKind::InputError(format!(
                    "Failed to read modpack archive entry: {error}"
                ))
            })?;
            let mut contents = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut contents)?;
            if let Ok(parsed) =
                serde_json::from_slice::<PlainVersionJson>(&contents)
            {
                candidates.push((id, parsed));
            }
        }
        Ok(candidates)
    })
    .await?
}

pub(crate) async fn install_plain_archive_with_reporter(
    instance_id: String,
    archive_path: PathBuf,
    base_folder: String,
    version_id: String,
    source_filename: Option<String>,
    reporter: InstallProgressReporter,
) -> crate::Result<()> {
    let candidates =
        read_version_candidates(archive_path.clone(), base_folder.clone())
            .await?;
    let mut targets: Vec<(String, DetectedTarget)> = candidates
        .iter()
        .filter_map(|(id, json)| {
            detect_target(json).map(|target| (id.clone(), target))
        })
        .collect();
    let selected = targets
        .iter()
        .position(|(_, target)| target.loader != ModLoader::Vanilla)
        .map(|index| targets.remove(index))
        .or_else(|| {
            if targets.is_empty() {
                None
            } else {
                Some(targets.remove(0))
            }
        });
    let Some((selected_id, target)) = selected else {
        return Err(crate::ErrorKind::InputError(format!(
            "Could not determine the Minecraft version of archived instance {version_id}"
        ))
        .into());
    };

    let pack_name = if selected_id.trim().is_empty() {
        source_filename
            .as_ref()
            .map(|name| {
                std::path::Path::new(name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_else(|| "Imported Instance".to_string())
    } else {
        selected_id.clone()
    };
    let pack_details = InstallPhaseDetails::Modpack {
        project_id: None,
        version_id: None,
        title: Some(pack_name.clone()),
    };
    reporter
        .update(InstallPhaseId::ResolvingPack, None, pack_details.clone())
        .await?;

    let resolved_loader_version = if target.loader != ModLoader::Vanilla {
        crate::launcher::get_loader_version_from_profile(
            &target.game_version,
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
            install_stage: Some(InstanceInstallStage::PackInstalling),
            name: Some(pack_name.clone()),
            link: Some(InstanceLink::ImportedModpack {
                project_id: None,
                version_id: None,
                name: Some(pack_name),
                version_number: None,
                filename: source_filename,
            }),
            content_set_patch: Some(AppliedContentSetPatch {
                source_kind: Some(ContentSourceKind::ImportedModpack),
                game_version: Some(target.game_version.clone()),
                protocol_version: Some(None),
                loader: Some(target.loader),
                loader_version: Some(
                    resolved_loader_version.map(|version| version.id),
                ),
            }),
            ..EditInstance::default()
        },
    )
    .await?;

    reporter
        .update(
            InstallPhaseId::ExtractingOverrides,
            None,
            pack_details.clone(),
        )
        .await?;
    let instance_path =
        crate::api::instance::get_full_path(&instance_id).await?;
    archive_util::extract_archive_subdir(
        archive_path,
        base_folder,
        instance_path,
    )
    .await?;

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        &instance_id,
        false,
        Some(reporter.clone()),
    )
    .await?;
    reporter.clear_context().await?;
    Ok(())
}
