use std::path::PathBuf;

use serde::Deserialize;

use crate::{
    State,
    install::{InstallPhaseDetails, InstallProgressReporter},
    state::{
        AppliedContentSetPatch, ContentSourceKind, EditInstance,
        InstanceInstallStage, InstanceLaunchOverridesPatch, ModLoader,
        ReleaseChannel, instances::InstanceLaunchOverridesData,
    },
    util::io,
};

use super::{ImportOverrides, finish_import, generic, recache_icon};

const CONFIG_FILE_NAME: &str = "axolotl_config.json";
const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct AxolotlConfigFile {
    pub schema_version: u32,
    pub instance_id: String,
    pub path: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub name: String,
    pub icon_path: Option<String>,
    pub update_channel: String,
    pub symlink_target: Option<String>,
    pub groups: Vec<String>,
    pub content_set: AxolotlContentSet,
    pub link: crate::state::instances::InstanceLink,
    pub launch_overrides: InstanceLaunchOverridesData,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct AxolotlContentSet {
    pub source_kind: String,
    pub game_version: String,
    pub protocol_version: Option<u32>,
    pub loader: String,
    pub loader_version: Option<String>,
}

/// Imports an Axolotl instance by reading `axolotl_config.json` and applying
/// the migratable fields to the new profile. Invalid or unsupported configs
/// fall back to the generic instance import so the game files still arrive.
pub(crate) async fn import_axolotl(
    source_path: PathBuf,
    instance_id: &str,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
    symlink: bool,
    overrides: &ImportOverrides,
) -> crate::Result<()> {
    let config_path = source_path.join(CONFIG_FILE_NAME);
    let content = io::read_any_encoding_to_string(&config_path)
        .await
        .unwrap_or_else(|error| {
            tracing::warn!(
                "Axolotl import: could not read {}: {error}; falling back to generic import",
                config_path.display()
            );
            (String::new(), encoding_rs::UTF_8)
        })
        .0;

    let config = match serde_json::from_str::<AxolotlConfigFile>(&content) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!(
                "Axolotl import: invalid config {}: {error}; falling back to generic import",
                config_path.display()
            );
            return generic::import_generic(
                source_path,
                instance_id,
                reporter,
                details,
                symlink,
                overrides,
                None,
            )
            .await;
        }
    };
    tracing::debug!(
        "Axolotl import: config instance_id={} path={} generated_at={} symlink_target={:?}",
        config.instance_id,
        config.path,
        config.generated_at,
        config.symlink_target
    );

    if config.schema_version != CONFIG_SCHEMA_VERSION
        || config.content_set.game_version.trim().is_empty()
    {
        tracing::warn!(
            "Axolotl import: unsupported schema or missing game version for {}; falling back to generic import",
            config_path.display()
        );
        return generic::import_generic(
            source_path,
            instance_id,
            reporter,
            details,
            symlink,
            overrides,
            None,
        )
        .await;
    }

    let icon = match config.icon_path.as_ref() {
        Some(path) => recache_icon(source_path.join(path)).await?,
        None => None,
    };
    let source_kind =
        ContentSourceKind::from_str(&config.content_set.source_kind)
            .unwrap_or(ContentSourceKind::Local);
    let loader = ModLoader::try_from_string(&config.content_set.loader)?;
    let update_channel = ReleaseChannel::from_key(&config.update_channel);

    let state = State::get().await?;
    crate::state::edit_instance(
        instance_id,
        EditInstance {
            install_stage: Some(InstanceInstallStage::PackInstalling),
            name: Some(config.name.clone()),
            icon_path: Some(icon.map(|p| p.to_string_lossy().to_string())),
            update_channel: Some(update_channel),
            groups: Some(config.groups.clone()),
            link: Some(config.link.clone()),
            content_set_patch: Some(AppliedContentSetPatch {
                source_kind: Some(source_kind),
                game_version: Some(config.content_set.game_version.clone()),
                protocol_version: Some(config.content_set.protocol_version),
                loader: Some(loader),
                loader_version: Some(config.content_set.loader_version.clone()),
            }),
            launch_overrides: Some(InstanceLaunchOverridesPatch {
                java_path: Some(config.launch_overrides.java_path.clone()),
                extra_launch_args: Some(
                    config.launch_overrides.extra_launch_args.clone(),
                ),
                custom_env_vars: Some(
                    config.launch_overrides.custom_env_vars.clone(),
                ),
                memory: Some(config.launch_overrides.memory),
                force_fullscreen: Some(
                    config.launch_overrides.force_fullscreen,
                ),
                game_resolution: Some(config.launch_overrides.game_resolution),
                hooks: Some(config.launch_overrides.hooks.clone()),
            }),
            ..EditInstance::default()
        },
        &state.pool,
    )
    .await?;

    finish_import(
        instance_id,
        source_path,
        &state.io_semaphore,
        reporter,
        details,
        symlink,
    )
    .await
}
