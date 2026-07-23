//! Installer for HMCL modpacks.
//!
//! HMCL packs are zips carrying a `modpack.json` with the pack name and game
//! version, optionally with an MCBBS-style `addons` array declaring loaders;
//! bundled content ships in a `minecraft/` folder that maps onto the
//! instance's game directory.

use std::path::PathBuf;

use serde::Deserialize;

use super::archive_util;
use crate::State;
use crate::data::ModLoader;
use crate::install::{
    InstallJobEventKind, InstallPhaseDetails, InstallPhaseId,
    InstallProgressReporter,
};
use crate::pack::detect::HMCL_MANIFEST;
use crate::state::{
    AppliedContentSetPatch, ContentSourceKind, EditInstance,
    InstanceInstallStage, InstanceLink,
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct HmclManifest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    addons: Vec<HmclAddon>,
}

#[derive(Deserialize, Debug)]
struct HmclAddon {
    id: String,
    version: String,
}

pub(crate) async fn install_hmcl_pack_with_reporter(
    instance_id: String,
    archive_path: PathBuf,
    base_folder: String,
    source_filename: Option<String>,
    reporter: InstallProgressReporter,
) -> crate::Result<()> {
    let state = State::get().await?;
    let manifest_json = archive_util::read_archive_entry_to_string(
        archive_path.clone(),
        format!("{base_folder}{HMCL_MANIFEST}"),
    )
    .await?;
    let manifest: HmclManifest = serde_json::from_str(&manifest_json)?;

    let mut game_version = manifest
        .game_version
        .clone()
        .filter(|version| !version.trim().is_empty());
    let mut loader = ModLoader::Vanilla;
    let mut loader_version = None;
    let mut optifine_version = None;
    for addon in &manifest.addons {
        match addon.id.to_ascii_lowercase().as_str() {
            "game" => {
                if game_version.is_none() {
                    game_version = Some(addon.version.clone());
                }
            }
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
                    "Ignoring unsupported HMCL addon {other} {}",
                    addon.version
                );
            }
        }
    }
    let Some(game_version) = game_version else {
        return Err(crate::ErrorKind::InputError(
            "HMCL modpack did not specify a Minecraft version".to_string(),
        )
        .into());
    };

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
        .unwrap_or_else(|| "HMCL Modpack".to_string());
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
        format!("{base_folder}minecraft/"),
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
        && let Err(error) = super::install_mcbbs::install_optifine_mod(
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
