use crate::api::Result;
use crate::api::files::local_instance_icon_path;
use chrono::NaiveDate;
use dashmap::DashMap;
use path_util::SafeRelativeUtf8UnixPathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use theseus::DownloadReason;
use theseus::data::{
    AppliedContentSetPatch, ContentItem, Dependency,
    EditInstance as CoreEditInstance, InstanceInstallCandidate,
    InstanceInstallTarget, InstanceLaunchOverridesPatch,
    InstanceLink as CoreInstanceLink, InstanceMetadata, LinkedModpackInfo,
};
use theseus::instance::InstallProjectWithDependenciesRequest;
use theseus::instance::QuickPlayType;
use theseus::pack::import::ImportLauncherType;
use theseus::prelude::*;
use theseus::server_address::ServerAddress;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("instance")
        .invoke_handler(tauri::generate_handler![
            instance_remove,
            instance_create_direct_link,
            instance_sync_direct_links,
            instance_get,
            instance_get_many,
            instance_list,
            instance_set_pinned,
            instance_get_daily_playtime,
            instance_get_daily_playtime_details,
            instance_get_projects,
            instance_get_installed_project_ids,
            instance_get_install_candidates,
            instance_content,
            instance_get_content_items,
            instance_get_content_items_by_paths,
            instance_get_content_snapshot,
            instance_refresh_content,
            instance_plan_content_updates,
            instance_apply_content_update_plan,
            instance_plan_upgrade,
            instance_get_upgrade_plan,
            instance_update_upgrade_resolution,
            instance_update_upgrade_resolutions,
            instance_reset_upgrade_resolution,
            instance_select_upgrade_solution,
            instance_resolve_custom_upgrade_solution,
            instance_execute_upgrade,
            instance_get_post_upgrade_notice,
            instance_dismiss_post_upgrade_notice,
            instance_get_dependencies_as_content_items,
            instance_get_linked_modpack_info,
            instance_get_linked_modpack_content,
            instance_get_optimal_jre_key,
            instance_list_core_components,
            instance_add_core_jar_mod,
            instance_replace_core_jar,
            instance_move_core_component,
            instance_set_core_component_enabled,
            instance_remove_core_component,
            instance_restore_core_component,
            instance_preview_core_jar,
            instance_install_mcarchive_modloader,
            instance_import_mcarchive_modloader,
            instance_install_mcarchive_content,
            instance_import_mcarchive_content,
            instance_install_planet_minecraft_content,
            instance_import_planet_minecraft_content,
            instance_get_full_path,
            instance_get_mod_full_path,
            instance_check_installed,
            instance_update_all,
            instance_update_project,
            instance_add_project_from_version,
            instance_install_project_with_dependencies,
            instance_preview_project_with_dependencies,
            instance_preview_project_with_dependencies_for_target,
            instance_queue_project_with_dependencies,
            instance_queue_curseforge_content,
            instance_queue_curseforge_world,
            instance_switch_project_version_with_dependencies,
            instance_add_project_from_path,
            instance_import_world_save,
            instance_install_datapack_to_world,
            instance_install_datapack_to_world_bytes,
            instance_toggle_disable_project,
            instance_toggle_content_entry,
            instance_toggle_content_entries,
            instance_rollback_project,
            instance_remove_project,
            instance_remove_content_entry,
            instance_update_content_entry,
            instance_switch_content_entry_version,
            instance_restore_pack_member_default,
            instance_update_managed_modrinth_version,
            instance_repair_managed_modrinth,
            instance_run,
            instance_kill,
            instance_edit,
            instance_cache_icon,
            instance_edit_icon,
            instance_export_mrpack,
            instance_get_pack_export_candidates,
        ])
        .build()
}

#[derive(Serialize, Debug, Clone)]
pub struct Instance {
    pub id: String,
    pub path: String,
    pub install_stage: String,
    pub launcher_feature_version: String,
    pub name: String,
    pub icon_path: Option<String>,
    pub game_version: String,
    pub protocol_version: Option<u32>,
    pub loader: ModLoader,
    pub loader_version: Option<String>,
    pub loader_components: Vec<theseus::data::LoaderComponent>,
    pub groups: Vec<String>,
    pub link: Option<InstanceLink>,
    pub update_channel: ReleaseChannel,
    pub created: chrono::DateTime<chrono::Utc>,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub last_played: Option<chrono::DateTime<chrono::Utc>>,
    pub pinned_at: Option<chrono::DateTime<chrono::Utc>>,
    pub submitted_time_played: u64,
    pub recent_time_played: u64,
    pub java_path: Option<String>,
    pub extra_launch_args: Option<Vec<String>>,
    pub custom_env_vars: Option<Vec<(String, String)>>,
    pub memory: Option<MemorySettings>,
    pub force_fullscreen: Option<bool>,
    pub game_resolution: Option<WindowSize>,
    pub launch_preparation_timeout: Option<u64>,
    pub hooks: Hooks,
    pub symlink_target: Option<String>,
    pub game_dir_override: Option<String>,
    pub linked_launcher: Option<String>,
    pub linked_launcher_root: Option<String>,
    pub linked_dot_minecraft: Option<String>,
    pub linked_version_id: Option<String>,
    pub linked_version_json_path: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirectLinkInstanceRequest {
    pub name: Option<String>,
    pub launcher_type: ImportLauncherType,
    pub base_path: PathBuf,
    pub instance_folder: String,
    pub instance_path: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct InstanceRunResult {
    pub process: ProcessMetadata,
    pub gc_notice: Option<theseus::instance::GcLaunchReport>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstanceLink {
    ModrinthModpack {
        project_id: String,
        version_id: String,
    },
    // Keep this as "curseforge_modpack" (not "curse_forge_modpack") so it matches
    // the frontend InstanceLink type and the stored link_kind string.
    #[serde(rename = "curseforge_modpack")]
    CurseForgeModpack {
        project_id: String,
        version_id: String,
    },
    ServerProject {
        project_id: String,
    },
    ServerProjectModpack {
        server_project_id: String,
        content_project_id: Option<String>,
        content_version_id: String,
        project_id: Option<String>,
        version_id: Option<String>,
    },
    ImportedModpack {
        project_id: Option<String>,
        version_id: Option<String>,
        name: Option<String>,
        version_number: Option<String>,
        filename: Option<String>,
    },
    SharedInstance {
        shared_instance_id: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditInstance {
    pub name: Option<String>,

    pub game_version: Option<String>,
    pub loader: Option<ModLoader>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub loader_version: Option<Option<String>>,

    pub groups: Option<Vec<String>>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub link: Option<Option<InstanceLink>>,
    pub update_channel: Option<ReleaseChannel>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub java_path: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub extra_launch_args: Option<Option<Vec<String>>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub custom_env_vars: Option<Option<Vec<(String, String)>>>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub memory: Option<Option<MemorySettings>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub force_fullscreen: Option<Option<bool>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub game_resolution: Option<Option<WindowSize>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub launch_preparation_timeout: Option<Option<u64>>,
    pub hooks: Option<Hooks>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub game_dir_override: Option<Option<String>>,
}

impl From<InstanceMetadata> for Instance {
    fn from(metadata: InstanceMetadata) -> Self {
        Self {
            id: metadata.instance.id,
            path: metadata.instance.path,
            install_stage: metadata.instance.install_stage.as_str().to_string(),
            launcher_feature_version: metadata
                .instance
                .launcher_feature_version
                .as_str()
                .to_string(),
            name: metadata.instance.name,
            icon_path: metadata.instance.icon_path,
            game_version: metadata.applied_content_set.game_version,
            protocol_version: metadata.applied_content_set.protocol_version,
            loader: metadata.applied_content_set.loader,
            loader_version: metadata.applied_content_set.loader_version,
            loader_components: metadata.loader_components,
            groups: metadata.groups,
            link: InstanceLink::from_core(metadata.link),
            update_channel: metadata.instance.update_channel,
            created: metadata.instance.created,
            modified: metadata.instance.modified,
            last_played: metadata.instance.last_played,
            pinned_at: metadata.instance.pinned_at,
            submitted_time_played: metadata.instance.submitted_time_played,
            recent_time_played: metadata.instance.recent_time_played,
            java_path: metadata.launch_overrides.java_path,
            extra_launch_args: metadata.launch_overrides.extra_launch_args,
            custom_env_vars: metadata.launch_overrides.custom_env_vars,
            memory: metadata.launch_overrides.memory,
            force_fullscreen: metadata.launch_overrides.force_fullscreen,
            game_resolution: metadata.launch_overrides.game_resolution,
            launch_preparation_timeout: metadata
                .launch_overrides
                .launch_preparation_timeout,
            hooks: metadata.launch_overrides.hooks,
            symlink_target: metadata.instance.symlink_target,
            game_dir_override: metadata.instance.game_dir_override,
            linked_launcher: metadata.instance.linked_launcher,
            linked_launcher_root: metadata.instance.linked_launcher_root,
            linked_dot_minecraft: metadata.instance.linked_dot_minecraft,
            linked_version_id: metadata.instance.linked_version_id,
            linked_version_json_path: metadata
                .instance
                .linked_version_json_path,
        }
    }
}

impl InstanceLink {
    fn from_core(link: CoreInstanceLink) -> Option<Self> {
        match link {
            CoreInstanceLink::Unmanaged => None,
            CoreInstanceLink::ModrinthModpack {
                project_id,
                version_id,
            } => Some(Self::ModrinthModpack {
                project_id,
                version_id,
            }),
            CoreInstanceLink::CurseForgeModpack {
                project_id,
                version_id,
            } => Some(Self::CurseForgeModpack {
                project_id,
                version_id,
            }),
            CoreInstanceLink::ServerProject { project_id } => {
                Some(Self::ServerProject { project_id })
            }
            CoreInstanceLink::ServerProjectModpack {
                server_project_id,
                content_project_id,
                content_version_id,
            } => Some(Self::ServerProjectModpack {
                project_id: Some(server_project_id.clone()),
                version_id: Some(content_version_id.clone()),
                server_project_id,
                content_project_id: Some(content_project_id),
                content_version_id,
            }),
            CoreInstanceLink::ImportedModpack {
                project_id,
                version_id,
                name,
                version_number,
                filename,
            } => Some(Self::ImportedModpack {
                project_id,
                version_id,
                name,
                version_number,
                filename,
            }),
            CoreInstanceLink::SharedInstance { shared_instance_id } => {
                Some(Self::SharedInstance {
                    shared_instance_id: shared_instance_id.to_string(),
                })
            }
        }
    }

    pub(crate) fn into_core(self) -> Result<CoreInstanceLink> {
        match self {
            Self::ModrinthModpack {
                project_id,
                version_id,
            } => Ok(CoreInstanceLink::ModrinthModpack {
                project_id,
                version_id,
            }),
            Self::CurseForgeModpack {
                project_id,
                version_id,
            } => Ok(CoreInstanceLink::CurseForgeModpack {
                project_id,
                version_id,
            }),
            Self::ServerProject { project_id } => {
                Ok(CoreInstanceLink::ServerProject { project_id })
            }
            Self::ServerProjectModpack {
                server_project_id,
                content_project_id,
                content_version_id,
                ..
            } => Ok(CoreInstanceLink::ServerProjectModpack {
                server_project_id,
                content_project_id: content_project_id.unwrap_or_default(),
                content_version_id,
            }),
            Self::ImportedModpack {
                project_id,
                version_id,
                name,
                version_number,
                filename,
            } => Ok(CoreInstanceLink::ImportedModpack {
                project_id,
                version_id,
                name,
                version_number,
                filename,
            }),
            Self::SharedInstance { shared_instance_id } => {
                Ok(CoreInstanceLink::SharedInstance {
                    shared_instance_id: shared_instance_id.parse().map_err(
                        |err| {
                            theseus::Error::from(
                                theseus::ErrorKind::InputError(format!(
                                    "Invalid shared instance id: {err}"
                                )),
                            )
                        },
                    )?,
                })
            }
        }
    }
}

fn edit_to_core(edit_instance: EditInstance) -> Result<CoreEditInstance> {
    Ok(CoreEditInstance {
        install_stage: None,
        launcher_feature_version: None,
        name: edit_instance.name,
        icon_path: None,
        update_channel: edit_instance.update_channel,
        groups: edit_instance.groups,
        link: edit_instance
            .link
            .map(|link| match link {
                Some(link) => link.into_core(),
                None => Ok(CoreInstanceLink::Unmanaged),
            })
            .transpose()?,
        launch_overrides: Some(InstanceLaunchOverridesPatch {
            java_path: edit_instance.java_path,
            extra_launch_args: edit_instance.extra_launch_args,
            custom_env_vars: edit_instance.custom_env_vars,
            memory: edit_instance.memory,
            force_fullscreen: edit_instance.force_fullscreen,
            game_resolution: edit_instance.game_resolution,
            launch_preparation_timeout: edit_instance
                .launch_preparation_timeout,
            hooks: edit_instance.hooks,
        }),
        content_set_patch: Some(AppliedContentSetPatch {
            source_kind: None,
            game_version: edit_instance.game_version,
            protocol_version: Some(None),
            loader: edit_instance.loader,
            loader_version: edit_instance.loader_version,
        }),
        last_played: None,
        submitted_time_played: None,
        recent_time_played: None,
        symlink_target: None,
        game_dir_override: edit_instance.game_dir_override,
    })
}

const LOCAL_INSTANCE_ICON_MAX_DIMENSION: u32 = 256;

async fn instance_from_metadata(
    metadata: InstanceMetadata,
) -> Result<Instance> {
    let mut instance = Instance::from(metadata);
    if instance.icon_path.is_none() {
        if let Ok(Some(local_icon)) = local_instance_icon_path(
            &instance.id,
            LOCAL_INSTANCE_ICON_MAX_DIMENSION,
        )
        .await
        {
            instance.icon_path = Some(local_icon);
        }
    }
    Ok(instance)
}

#[tauri::command]
pub async fn instance_remove(instance_id: &str) -> Result<()> {
    theseus::instance::remove(instance_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn instance_create_direct_link(
    request: CreateDirectLinkInstanceRequest,
) -> Result<Instance> {
    let metadata = theseus::instance::create_with_direct_link(
        theseus::data::CreateDirectLinkInstance {
            name: request.name,
            launcher_type: request.launcher_type,
            base_path: request.base_path,
            instance_folder: request.instance_folder,
            instance_path: request.instance_path,
        },
    )
    .await?;
    instance_from_metadata(metadata).await
}

#[tauri::command]
pub async fn instance_sync_direct_links(
    roots: Vec<PathBuf>,
) -> Result<theseus::data::DirectLinkSyncReport> {
    Ok(theseus::instance::sync_direct_links(roots).await?)
}

#[tauri::command]
pub async fn instance_get(instance_id: &str) -> Result<Option<Instance>> {
    let Some(metadata) = theseus::instance::get(instance_id).await? else {
        return Ok(None);
    };
    Ok(Some(instance_from_metadata(metadata).await?))
}

#[tauri::command]
pub async fn instance_get_many(
    instance_ids: Vec<String>,
) -> Result<Vec<Instance>> {
    let ids = instance_ids.iter().map(|x| &**x).collect::<Vec<&str>>();
    let mut instances = Vec::with_capacity(ids.len());
    for metadata in theseus::instance::get_many(&ids).await? {
        instances.push(instance_from_metadata(metadata).await?);
    }
    Ok(instances)
}

#[tauri::command]
pub async fn instance_list() -> Result<Vec<Instance>> {
    let mut instances = Vec::new();
    for metadata in theseus::instance::list().await? {
        instances.push(instance_from_metadata(metadata).await?);
    }
    Ok(instances)
}

#[tauri::command]
pub async fn instance_set_pinned(
    instance_id: String,
    pinned: bool,
) -> Result<Instance> {
    let metadata = theseus::instance::set_pinned(&instance_id, pinned).await?;
    instance_from_metadata(metadata).await
}

#[tauri::command]
pub async fn instance_get_daily_playtime(
    start_date: String,
    end_date: String,
) -> Result<Vec<theseus::instance::DailyPlaytime>> {
    let start_date = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
        .map_err(|error| {
            theseus::ErrorKind::InputError(format!(
                "Invalid start date: {error}"
            ))
            .as_error()
        })?;
    let end_date =
        NaiveDate::parse_from_str(&end_date, "%Y-%m-%d").map_err(|error| {
            theseus::ErrorKind::InputError(format!("Invalid end date: {error}"))
                .as_error()
        })?;

    Ok(theseus::instance::get_daily_playtime(start_date, end_date).await?)
}

#[tauri::command]
pub async fn instance_get_daily_playtime_details(
    date: String,
) -> Result<Vec<theseus::instance::DailyPlaytimeEntry>> {
    let date =
        NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|error| {
            theseus::ErrorKind::InputError(format!("Invalid date: {error}"))
                .as_error()
        })?;

    Ok(theseus::instance::get_daily_playtime_details(date).await?)
}

#[tauri::command]
pub async fn instance_get_projects(
    instance_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> Result<DashMap<String, ContentFile>> {
    Ok(theseus::instance::get_projects(instance_id, cache_behaviour).await?)
}

#[tauri::command]
pub async fn instance_get_installed_project_ids(
    instance_id: &str,
) -> Result<Vec<String>> {
    Ok(theseus::instance::get_installed_project_ids(instance_id).await?)
}

#[tauri::command]
pub async fn instance_get_install_candidates(
    project_id: &str,
    project_type: ProjectType,
    targets: Vec<InstanceInstallTarget>,
) -> Result<Vec<InstanceInstallCandidate>> {
    Ok(theseus::instance::get_install_candidates(
        project_id,
        project_type,
        targets,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_content(
    instance_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> Result<Vec<ContentItem>> {
    instance_get_content_items(instance_id, cache_behaviour).await
}

#[tauri::command]
pub async fn instance_get_content_items(
    instance_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> Result<Vec<ContentItem>> {
    Ok(
        theseus::instance::get_content_items(instance_id, cache_behaviour)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_get_content_items_by_paths(
    instance_id: &str,
    paths: Vec<String>,
    cache_behaviour: Option<CacheBehaviour>,
) -> Result<Vec<ContentItem>> {
    Ok(theseus::instance::get_content_items_by_paths(
        instance_id,
        paths,
        cache_behaviour,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_get_content_snapshot(
    instance_id: &str,
) -> Result<theseus::data::InstanceContentSnapshot> {
    Ok(theseus::instance::get_content_snapshot(instance_id).await?)
}

#[tauri::command]
pub async fn instance_refresh_content(
    instance_id: &str,
) -> Result<theseus::data::InstanceContentSnapshot> {
    Ok(theseus::instance::refresh_content(instance_id).await?)
}

#[tauri::command]
pub async fn instance_plan_content_updates(
    instance_id: &str,
    scope: theseus::data::ContentUpdateScope,
    target: Option<&str>,
) -> Result<theseus::data::ContentUpdatePlan> {
    Ok(
        theseus::instance::plan_content_updates(instance_id, scope, target)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_apply_content_update_plan(
    plan_id: &str,
    resolutions: Vec<theseus::data::ContentUpdateResolution>,
) -> Result<theseus::data::InstanceContentSnapshot> {
    Ok(
        theseus::instance::apply_content_update_plan(plan_id, resolutions)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_plan_upgrade(
    instance_id: &str,
    target_environment: theseus::data::InstanceUpgradeEnvironment,
) -> Result<theseus::data::InstanceUpgradePlan> {
    Ok(theseus::instance::plan_instance_upgrade(
        instance_id,
        target_environment,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_get_upgrade_plan(
    plan_id: &str,
) -> Result<theseus::data::InstanceUpgradePlan> {
    Ok(theseus::instance::get_instance_upgrade_plan(plan_id).await?)
}

#[tauri::command]
pub async fn instance_update_upgrade_resolution(
    plan_id: &str,
    resolution: theseus::data::InstanceUpgradeResolution,
) -> Result<theseus::data::InstanceUpgradePlan> {
    Ok(theseus::instance::update_instance_upgrade_resolution(
        plan_id, resolution,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_update_upgrade_resolutions(
    plan_id: &str,
    resolutions: Vec<theseus::data::InstanceUpgradeResolution>,
) -> Result<theseus::data::InstanceUpgradeResolutionBatchResult> {
    Ok(theseus::instance::update_instance_upgrade_resolutions(
        plan_id,
        resolutions,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_reset_upgrade_resolution(
    plan_id: &str,
    content_id: &str,
) -> Result<theseus::data::InstanceUpgradePlan> {
    Ok(theseus::instance::reset_instance_upgrade_resolution(
        plan_id, content_id,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_select_upgrade_solution(
    plan_id: &str,
    choice: theseus::data::InstanceUpgradeSolutionChoice,
) -> Result<theseus::data::InstanceUpgradePlan> {
    Ok(
        theseus::instance::select_instance_upgrade_solution(plan_id, choice)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_resolve_custom_upgrade_solution(
    plan_id: &str,
    fixed_constraints: Vec<theseus::data::InstanceUpgradeFixedConstraint>,
) -> Result<theseus::data::InstanceUpgradePlan> {
    Ok(theseus::instance::resolve_custom_instance_upgrade_solution(
        plan_id,
        fixed_constraints,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_execute_upgrade(
    plan_id: &str,
    create_full_backup: bool,
    shared_upgrade_mode: theseus::install::SharedUpgradeMode,
    display_names: Option<theseus::install::InstanceUpgradeDisplayNames>,
) -> Result<theseus::install::InstallJobSnapshot> {
    Ok(theseus::instance::execute_instance_upgrade(
        plan_id,
        create_full_backup,
        shared_upgrade_mode,
        display_names.unwrap_or_default(),
    )
    .await?)
}

#[tauri::command]
pub async fn instance_get_post_upgrade_notice(
    instance_id: &str,
) -> Result<Option<theseus::data::InstancePostUpgradeNotice>> {
    Ok(
        theseus::instance::get_instance_post_upgrade_notice(instance_id)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_dismiss_post_upgrade_notice(
    instance_id: &str,
) -> Result<()> {
    Ok(
        theseus::instance::dismiss_instance_post_upgrade_notice(instance_id)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_get_dependencies_as_content_items(
    dependencies: Vec<Dependency>,
    cache_behaviour: Option<CacheBehaviour>,
) -> Result<Vec<ContentItem>> {
    Ok(theseus::instance::get_dependencies_as_content_items(
        dependencies,
        cache_behaviour,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_get_linked_modpack_info(
    instance_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> Result<Option<LinkedModpackInfo>> {
    Ok(
        theseus::instance::get_linked_modpack_info(
            instance_id,
            cache_behaviour,
        )
        .await?,
    )
}

#[tauri::command]
pub async fn instance_get_linked_modpack_content(
    instance_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> Result<Vec<ContentItem>> {
    Ok(theseus::instance::get_linked_modpack_content(
        instance_id,
        cache_behaviour,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_get_full_path<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<PathBuf> {
    let path = theseus::instance::get_full_path(instance_id).await?;
    crate::api::files::ensure_browsable(&app, &path);
    Ok(path)
}

#[tauri::command]
pub async fn instance_get_mod_full_path<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    instance_id: &str,
    project_path: &str,
) -> Result<PathBuf> {
    let path =
        theseus::instance::get_mod_full_path(instance_id, project_path).await?;
    if let Some(parent) = path.parent() {
        crate::api::files::ensure_browsable(&app, parent);
    }
    Ok(path)
}

#[tauri::command]
pub async fn instance_get_optimal_jre_key(
    instance_id: &str,
) -> Result<Option<JavaVersion>> {
    Ok(theseus::instance::get_optimal_jre_key(instance_id).await?)
}

#[tauri::command]
pub async fn instance_list_core_components(
    instance_id: &str,
) -> Result<Vec<theseus::data::CoreComponent>> {
    Ok(theseus::instance::list_core_components(instance_id).await?)
}

#[tauri::command]
pub async fn instance_add_core_jar_mod(
    instance_id: &str,
    source_path: PathBuf,
    target_game_version: String,
    source: Option<theseus::data::CoreComponentSource>,
) -> Result<theseus::data::CoreComponent> {
    Ok(theseus::instance::add_core_jar_mod(
        instance_id,
        source_path,
        target_game_version,
        source,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_replace_core_jar(
    instance_id: &str,
    source_path: PathBuf,
    target_game_version: String,
    source: Option<theseus::data::CoreComponentSource>,
) -> Result<theseus::data::CoreComponent> {
    Ok(theseus::instance::replace_core_jar(
        instance_id,
        source_path,
        target_game_version,
        source,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_move_core_component(
    instance_id: &str,
    component_id: &str,
    direction: i32,
) -> Result<Vec<theseus::data::CoreComponent>> {
    Ok(theseus::instance::move_core_component(
        instance_id,
        component_id,
        direction,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_set_core_component_enabled(
    instance_id: &str,
    component_id: &str,
    enabled: bool,
) -> Result<theseus::data::CoreComponent> {
    Ok(theseus::instance::set_core_component_enabled(
        instance_id,
        component_id,
        enabled,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_remove_core_component(
    instance_id: &str,
    component_id: &str,
) -> Result<()> {
    theseus::instance::remove_core_component(instance_id, component_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn instance_restore_core_component(
    instance_id: &str,
    component_id: &str,
) -> Result<theseus::data::CoreComponent> {
    Ok(
        theseus::instance::restore_core_component(instance_id, component_id)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_preview_core_jar(
    instance_id: &str,
) -> Result<Option<theseus::data::CoreJarPreview>> {
    Ok(theseus::instance::preview_core_jar(instance_id).await?)
}

#[tauri::command]
pub async fn instance_install_mcarchive_modloader(
    instance_id: &str,
    game_version: &str,
) -> Result<theseus::instance::McArchiveCoreInstallResult> {
    Ok(theseus::instance::install_mcarchive_modloader(
        instance_id,
        game_version,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_import_mcarchive_modloader(
    instance_id: &str,
    game_version: &str,
    source_path: PathBuf,
) -> Result<theseus::instance::McArchiveCoreInstallResult> {
    Ok(theseus::instance::import_mcarchive_modloader(
        instance_id,
        game_version,
        source_path,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_install_mcarchive_content(
    instance_id: &str,
    request: theseus::instance::McArchiveContentInstallRequest,
) -> Result<theseus::instance::McArchiveContentInstallResult> {
    Ok(
        theseus::instance::install_mcarchive_content(instance_id, request)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_import_mcarchive_content(
    instance_id: &str,
    request: theseus::instance::McArchiveContentInstallRequest,
    source_path: PathBuf,
) -> Result<theseus::instance::McArchiveContentInstallResult> {
    Ok(theseus::instance::import_mcarchive_content(
        instance_id,
        request,
        source_path,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_install_planet_minecraft_content(
    instance_id: &str,
    request: theseus::instance::PlanetMinecraftContentInstallRequest,
) -> Result<theseus::instance::PlanetMinecraftContentInstallResult> {
    Ok(theseus::instance::install_planet_minecraft_content(
        instance_id,
        request,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_import_planet_minecraft_content(
    instance_id: &str,
    request: theseus::instance::PlanetMinecraftContentInstallRequest,
    source_path: PathBuf,
) -> Result<theseus::instance::PlanetMinecraftContentInstallResult> {
    Ok(theseus::instance::import_planet_minecraft_content(
        instance_id,
        request,
        source_path,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_check_installed(
    instance_id: &str,
    project_id: &str,
) -> Result<bool> {
    if let Ok(projects) =
        theseus::instance::get_projects(instance_id, None).await
    {
        Ok(projects.into_iter().any(|(_, project)| {
            project
                .provider_refs
                .iter()
                .any(|reference| match reference {
                    theseus::data::ContentProviderRef::Modrinth {
                        project_id: id,
                        ..
                    } => project_id == id.as_str(),
                    theseus::data::ContentProviderRef::CurseForge {
                        project_id: id,
                        ..
                    } => project_id == format!("curseforge:{}", id.get()),
                    theseus::data::ContentProviderRef::McArchive {
                        project_id: id,
                        ..
                    } => project_id == format!("mcarchive:{id}"),
                })
        }))
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn instance_update_all(
    instance_id: &str,
) -> Result<HashMap<String, String>> {
    Ok(theseus::instance::update_all_projects(instance_id).await?)
}

#[tauri::command]
pub async fn instance_update_project(
    instance_id: &str,
    project_path: &str,
) -> Result<String> {
    Ok(
        theseus::instance::update_project(instance_id, project_path, None)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_add_project_from_version(
    instance_id: &str,
    version_id: &str,
    reason: DownloadReason,
    dependent_on_version_id: Option<String>,
) -> Result<String> {
    Ok(theseus::instance::add_project_from_version(
        instance_id,
        version_id,
        reason,
        dependent_on_version_id,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_install_project_with_dependencies(
    instance_id: &str,
    request: InstallProjectWithDependenciesRequest,
) -> Result<ResolveContentPlan> {
    Ok(theseus::instance::install_project_with_dependencies(
        instance_id,
        request,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_preview_project_with_dependencies(
    instance_id: &str,
    request: InstallProjectWithDependenciesRequest,
) -> Result<ResolveContentPlan> {
    Ok(theseus::instance::preview_project_with_dependencies(
        instance_id,
        request,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_preview_project_with_dependencies_for_target(
    request: InstallProjectWithDependenciesRequest,
    game_version: String,
    loader: ModLoader,
) -> Result<ResolveContentPlan> {
    Ok(
        theseus::instance::preview_project_with_dependencies_for_target(
            request,
            game_version,
            loader,
        )
        .await?,
    )
}

#[tauri::command]
pub async fn instance_queue_project_with_dependencies(
    instance_id: &str,
    request: InstallProjectWithDependenciesRequest,
    display_title: String,
    display_icon: Option<String>,
) -> Result<theseus::install::InstallJobSnapshot> {
    Ok(theseus::instance::queue_project_with_dependencies(
        instance_id,
        request,
        display_title,
        display_icon,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_queue_curseforge_content(
    request: theseus::curseforge::CurseForgeInstallRequest,
    display_title: String,
    display_icon: Option<String>,
) -> Result<theseus::install::InstallJobSnapshot> {
    Ok(theseus::instance::queue_curseforge_content(
        request,
        display_title,
        display_icon,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_queue_curseforge_world(
    request: theseus::curseforge::CurseForgeWorldInstallRequest,
    display_title: String,
    display_icon: Option<String>,
) -> Result<theseus::install::InstallJobSnapshot> {
    Ok(theseus::instance::queue_curseforge_world(
        request,
        display_title,
        display_icon,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_switch_project_version_with_dependencies(
    instance_id: &str,
    project_path: &str,
    version_id: &str,
) -> Result<String> {
    Ok(theseus::instance::switch_project_version_with_dependencies(
        instance_id,
        project_path,
        version_id,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_add_project_from_path(
    instance_id: &str,
    project_path: &Path,
    project_type: Option<ProjectType>,
    inner_base: Option<String>,
) -> Result<String> {
    Ok(theseus::instance::add_project_from_path(
        instance_id,
        project_path,
        project_type,
        inner_base.as_deref(),
    )
    .await?)
}

#[tauri::command]
pub async fn instance_import_world_save(
    instance_id: String,
    source_path: String,
    inner_base: Option<String>,
) -> Result<String> {
    Ok(theseus::instance::import_world_save(
        &instance_id,
        &std::path::PathBuf::from(&source_path),
        inner_base.as_deref(),
    )
    .await?)
}

#[tauri::command]
pub async fn instance_install_datapack_to_world(
    instance_id: String,
    world_path: String,
    source_path: String,
    inner_base: Option<String>,
) -> Result<String> {
    Ok(theseus::instance::install_datapack_to_world(
        &instance_id,
        &world_path,
        &std::path::PathBuf::from(&source_path),
        inner_base.as_deref(),
    )
    .await?)
}

#[tauri::command]
pub async fn instance_install_datapack_to_world_bytes(
    instance_id: String,
    world_path: String,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<String> {
    Ok(theseus::instance::install_datapack_bytes_to_world(
        &instance_id,
        &world_path,
        &file_name,
        bytes,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_toggle_disable_project(
    instance_id: &str,
    project_path: &str,
    desired_enabled: Option<bool>,
) -> Result<String> {
    Ok(theseus::instance::toggle_disable_project(
        instance_id,
        project_path,
        desired_enabled,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_toggle_content_entry(
    instance_id: &str,
    content_id: &str,
    desired_enabled: Option<bool>,
) -> Result<String> {
    Ok(theseus::instance::toggle_content_entry(
        instance_id,
        content_id,
        desired_enabled,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_toggle_content_entries(
    instance_id: &str,
    content_ids: Vec<String>,
    desired_enabled: Option<bool>,
) -> Result<Vec<theseus::instance::ContentToggleResult>> {
    Ok(theseus::instance::toggle_content_entries(
        instance_id,
        content_ids,
        desired_enabled,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_rollback_project(
    instance_id: &str,
    project_path: &str,
) -> Result<String> {
    Ok(theseus::instance::rollback_project(instance_id, project_path).await?)
}

#[tauri::command]
pub async fn instance_remove_project(
    instance_id: &str,
    project_path: &str,
) -> Result<()> {
    theseus::instance::remove_project(instance_id, project_path).await?;
    Ok(())
}

#[tauri::command]
pub async fn instance_remove_content_entry(
    instance_id: &str,
    content_id: &str,
) -> Result<()> {
    theseus::instance::remove_content_entry(instance_id, content_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn instance_update_content_entry(
    instance_id: &str,
    content_id: &str,
) -> Result<String> {
    Ok(
        theseus::instance::update_content_entry(instance_id, content_id)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_switch_content_entry_version(
    instance_id: &str,
    content_id: &str,
    version_id: &str,
) -> Result<String> {
    Ok(theseus::instance::switch_content_entry_version(
        instance_id,
        content_id,
        version_id,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_restore_pack_member_default(
    instance_id: &str,
    member_id: &str,
) -> Result<Option<String>> {
    Ok(
        theseus::instance::restore_pack_member_default(instance_id, member_id)
            .await?,
    )
}

#[tauri::command]
pub async fn instance_update_managed_modrinth_version(
    instance_id: String,
    version_id: String,
) -> Result<theseus::install::InstallJobSnapshot> {
    Ok(theseus::instance::update_managed_modrinth_version(
        &instance_id,
        &version_id,
    )
    .await?)
}

#[tauri::command]
pub async fn instance_repair_managed_modrinth(
    instance_id: &str,
) -> Result<theseus::install::InstallJobSnapshot> {
    Ok(theseus::instance::repair_managed_modrinth(instance_id).await?)
}

#[tauri::command]
pub async fn instance_export_mrpack(
    instance_id: &str,
    export_location: PathBuf,
    included_overrides: Vec<String>,
    version_id: Option<String>,
    description: Option<String>,
    name: Option<String>,
) -> Result<()> {
    theseus::instance::export_mrpack(
        instance_id,
        export_location,
        included_overrides,
        version_id,
        description,
        name,
    )
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn instance_get_pack_export_candidates(
    instance_id: &str,
) -> Result<Vec<SafeRelativeUtf8UnixPathBuf>> {
    Ok(theseus::instance::get_pack_export_candidates(instance_id).await?)
}

#[tauri::command]
pub async fn instance_run(
    instance_id: &str,
    server_address: Option<String>,
    offline_mode: bool,
    extra_launch_args: Option<Vec<String>>,
    gc_intent: Option<theseus::instance::GcLaunchIntent>,
) -> Result<InstanceRunResult> {
    let quick_play = match server_address {
        Some(addr) => QuickPlayType::Server(ServerAddress::Unresolved(addr)),
        None => QuickPlayType::None,
    };
    let (process, gc_notice) =
        theseus::instance::run_with_extra_launch_args_with_gc(
            instance_id,
            quick_play,
            offline_mode,
            extra_launch_args,
            gc_intent,
        )
        .await?;
    Ok(InstanceRunResult { process, gc_notice })
}

#[tauri::command]
pub async fn instance_kill(instance_id: &str) -> Result<()> {
    theseus::instance::kill(instance_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn instance_edit(
    instance_id: &str,
    edit_instance: EditInstance,
) -> Result<()> {
    theseus::instance::edit(instance_id, edit_to_core(edit_instance)?).await?;
    Ok(())
}

#[tauri::command]
pub async fn instance_cache_icon(
    icon_name: &str,
    bytes: Vec<u8>,
) -> Result<String> {
    Ok(theseus::instance::cache_icon(icon_name, bytes).await?)
}

#[tauri::command]
pub async fn instance_edit_icon(
    instance_id: &str,
    icon_path: Option<&Path>,
) -> Result<()> {
    theseus::instance::edit_icon(instance_id, icon_path).await?;
    Ok(())
}
