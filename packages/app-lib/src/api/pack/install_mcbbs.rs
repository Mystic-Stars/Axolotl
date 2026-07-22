//! Installer for MCBBS modpacks.
//!
//! MCBBS packs are zips carrying either an `mcbbs.packmeta` file or a
//! `manifest.json` with an `addons` array. Game and loader versions come from
//! the addons list, bundled content ships in `overrides/`, and optional
//! launch settings come from `launchInfo`.

use std::path::PathBuf;

use serde::Deserialize;

use super::archive_util;
use crate::State;
use crate::data::ModLoader;
use crate::install::{
    InstallJobEventKind, InstallPhaseDetails, InstallPhaseId,
    InstallProgressReporter,
};
use crate::pack::detect::{CURSEFORGE_MANIFEST, MCBBS_MANIFEST};
use crate::state::{
    AppliedContentSetPatch, ContentSourceKind, EditInstance,
    InstanceInstallStage, InstanceLaunchOverridesPatch, InstanceLink,
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct McbbsManifest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    addons: Vec<McbbsAddon>,
    #[serde(default)]
    files: Vec<McbbsFile>,
    #[serde(default)]
    launch_info: Option<McbbsLaunchInfo>,
}

#[derive(Deserialize, Debug)]
struct McbbsAddon {
    id: String,
    version: String,
}

/// A `files` entry; `curse` entries carry CurseForge project/file ids while
/// `addition` entries ship inside the overrides folder and need no download.
#[derive(Deserialize, Debug)]
struct McbbsFile {
    #[serde(default, rename = "type")]
    type_: Option<String>,
    #[serde(default, alias = "projectID", alias = "projectId")]
    project_id: Option<u32>,
    #[serde(default, alias = "fileID", alias = "fileId")]
    file_id: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct McbbsLaunchInfo {
    #[serde(default)]
    java_argument: Option<serde_json::Value>,
}

fn join_arguments(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(value) => vec![value.clone()],
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) async fn install_mcbbs_pack_with_reporter(
    instance_id: String,
    archive_path: PathBuf,
    base_folder: String,
    source_filename: Option<String>,
    reporter: InstallProgressReporter,
) -> crate::Result<()> {
    let state = State::get().await?;

    let manifest_json = match archive_util::read_archive_entry_to_string(
        archive_path.clone(),
        format!("{base_folder}{MCBBS_MANIFEST}"),
    )
    .await
    {
        Ok(contents) => contents,
        Err(_) => {
            archive_util::read_archive_entry_to_string(
                archive_path.clone(),
                format!("{base_folder}{CURSEFORGE_MANIFEST}"),
            )
            .await?
        }
    };
    let manifest: McbbsManifest = serde_json::from_str(&manifest_json)?;

    let mut game_version = None;
    let mut loader = ModLoader::Vanilla;
    let mut loader_version = None;
    let mut optifine_version = None;
    for addon in &manifest.addons {
        match addon.id.to_ascii_lowercase().as_str() {
            "game" => game_version = Some(addon.version.clone()),
            "forge" => {
                loader = ModLoader::Forge;
                loader_version = Some(addon.version.clone());
            }
            "neoforge" => {
                loader = ModLoader::NeoForge;
                loader_version = Some(addon.version.clone());
            }
            "fabric" => {
                loader = ModLoader::Fabric;
                loader_version = Some(addon.version.clone());
            }
            "quilt" => {
                loader = ModLoader::Quilt;
                loader_version = Some(addon.version.clone());
            }
            "optifine" => optifine_version = Some(addon.version.clone()),
            other => {
                tracing::warn!(
                    "Ignoring unsupported MCBBS addon {other} {}",
                    addon.version
                );
            }
        }
    }
    let Some(game_version) = game_version else {
        return Err(crate::ErrorKind::InputError(
            "MCBBS modpack did not specify a Minecraft version".to_string(),
        )
        .into());
    };

    // Standalone OptiFine packs install OptiFine as the loader; packs that
    // also declare Forge/NeoForge get OptiFine dropped into mods/ instead.
    let mut optifine_as_mod = None;
    if let Some(optifine_version) = optifine_version {
        match loader {
            ModLoader::Vanilla => {
                loader = ModLoader::OptiFine;
                loader_version = Some(optifine_version);
            }
            ModLoader::Forge | ModLoader::NeoForge => {
                optifine_as_mod = Some(optifine_version);
            }
            _ => {
                tracing::warn!(
                    "Skipping OptiFine {optifine_version}: not compatible with {}",
                    loader.as_str()
                );
            }
        }
    }

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
        .unwrap_or_else(|| "MCBBS Modpack".to_string());
    let pack_details = InstallPhaseDetails::Modpack {
        project_id: None,
        version_id: None,
        title: Some(pack_name.clone()),
    };
    reporter
        .update(InstallPhaseId::ResolvingPack, None, pack_details.clone())
        .await?;

    let resolved_loader_version = if loader != ModLoader::Vanilla {
        crate::launcher::get_loader_version_from_profile(
            &game_version,
            loader,
            loader_version.as_deref(),
        )
        .await?
    } else {
        None
    };

    let launch_overrides =
        manifest.launch_info.as_ref().and_then(|launch_info| {
            let jvm_args = launch_info
                .java_argument
                .as_ref()
                .map(|value| join_arguments(value))
                .filter(|args| !args.is_empty())?;
            Some(InstanceLaunchOverridesPatch {
                extra_launch_args: Some(Some(jvm_args)),
                ..InstanceLaunchOverridesPatch::default()
            })
        });

    crate::api::instance::edit(
        &instance_id,
        EditInstance {
            install_stage: Some(InstanceInstallStage::PackInstalling),
            name: Some(pack_name.clone()),
            link: Some(InstanceLink::ImportedModpack {
                project_id: None,
                version_id: None,
                name: Some(pack_name.clone()),
                version_number: manifest.version.clone(),
                filename: source_filename,
            }),
            launch_overrides,
            content_set_patch: Some(AppliedContentSetPatch {
                source_kind: Some(ContentSourceKind::ImportedModpack),
                game_version: Some(game_version.clone()),
                protocol_version: Some(None),
                loader: Some(loader),
                loader_version: Some(
                    resolved_loader_version.map(|version| version.id),
                ),
            }),
            ..EditInstance::default()
        },
    )
    .await?;

    let curse_files = manifest
        .files
        .iter()
        .filter(|file| {
            file.type_
                .as_deref()
                .is_none_or(|kind| kind.eq_ignore_ascii_case("curse"))
        })
        .filter_map(|file| {
            Some(crate::api::curseforge::CurseForgeManifestFile {
                project_id: file.project_id?,
                file_id: file.file_id?,
                required: true,
            })
        })
        .collect::<Vec<_>>();
    if !curse_files.is_empty() {
        let content_loader = (loader != ModLoader::Vanilla
            && loader != ModLoader::OptiFine)
            .then(|| loader.as_str().to_string());
        crate::api::curseforge::install_local_manifest_files(
            &instance_id,
            curse_files,
            false,
            &game_version,
            content_loader.as_deref(),
            pack_details.clone(),
            &reporter,
        )
        .await?;
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
    archive_util::extract_archive_subdir(
        archive_path,
        format!("{base_folder}overrides/"),
        instance_path.clone(),
    )
    .await?;

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        &instance_id,
        false,
        Some(reporter.clone()),
    )
    .await?;

    if let Some(optifine_version) = optifine_as_mod
        && let Err(error) = install_optifine_mod(
            &state,
            &instance_id,
            &game_version,
            &optifine_version,
            &instance_path,
        )
        .await
    {
        tracing::warn!(
            "Failed to install OptiFine {optifine_version} as a mod: {error}"
        );
        reporter
            .update_with_events(
                InstallPhaseId::DownloadingContent,
                None,
                pack_details.clone(),
                vec![InstallJobEventKind::ContentFileSkipped {
                    path: format!("OptiFine {optifine_version}"),
                    reason: format!(
                        "OptiFine could not be installed automatically: {error}"
                    ),
                    project_id: None,
                    version_id: None,
                    manual_url: Some(
                        "https://optifine.net/downloads".to_string(),
                    ),
                }],
            )
            .await?;
    }

    reporter.clear_context().await?;
    Ok(())
}

/// Installs OptiFine into the instance's mods folder for packs that pair it
/// with Forge or NeoForge. Requires the instance's Minecraft install to have
/// completed so the client jar and a Java runtime are available.
pub(crate) async fn install_optifine_mod(
    state: &State,
    instance_id: &str,
    game_version: &str,
    optifine_version: &str,
    instance_path: &std::path::Path,
) -> crate::Result<()> {
    let metadata =
        crate::api::instance::get(instance_id)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Unknown instance {instance_id}"
                ))
            })?;
    let version_jar = match &metadata.applied_content_set.loader_version {
        Some(loader_version) => format!("{game_version}-{loader_version}"),
        None => game_version.to_string(),
    };
    let client_jar = state
        .directories
        .version_dir(&version_jar)
        .join(format!("{version_jar}.jar"));

    let (manifest, version_index) =
        crate::launcher::resolve_minecraft_manifest(game_version, state)
            .await?;
    let version_info = crate::launcher::download::download_version_info(
        state,
        &manifest.versions[version_index],
        ModLoader::Vanilla,
        None,
        None,
        None,
        None,
    )
    .await?;
    let java_key = version_info
        .java_version
        .as_ref()
        .map_or(8, |java| java.major_version);
    let java = crate::state::JavaVersion::get(java_key, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(format!(
                "No Java {java_key} runtime is available for the OptiFine installer"
            ))
        })?;

    crate::launcher::optifine::install_optifine_as_mod(
        state,
        std::path::Path::new(&java.path),
        game_version,
        optifine_version,
        &client_jar,
        &instance_path.join("mods"),
    )
    .await?;
    Ok(())
}
