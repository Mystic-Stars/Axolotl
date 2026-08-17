use crate::event::emit::{emit_instance, emit_loading, init_loading};
use crate::event::{InstancePayloadType, LoadingBarType};
use crate::state::instances::adapters::sqlite::{content_rows, instance_rows};
use crate::state::instances::{
    ContentOwnershipKind, PackMemberMaterializationState,
    PackMemberOverrideKind,
};
use crate::state::{ContentProvider, ContentSourceKind, ProjectType, State};
use crate::util::fetch;
use modrinth_content_management::{
    ContentType, ResolutionPreferences, ResolveContentPlan,
};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct InstallProjectWithDependenciesRequest {
    pub project_id: String,
    pub version_id: Option<String>,
    pub content_type: ContentType,
    #[serde(default)]
    pub selected: ResolutionPreferences,
    #[serde(default)]
    pub excluded_project_ids: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentToggleResult {
    pub content_id: String,
    pub path: String,
    pub enabled: bool,
}

#[tracing::instrument]
pub async fn update_all_projects(
    instance_id: &str,
) -> crate::Result<HashMap<String, String>> {
    let state = State::get().await?;
    let instance = get_instance_display_info(instance_id, &state).await?;
    let loading_bar = init_loading(
        LoadingBarType::InstanceUpdate {
            instance_id: instance.id.clone(),
            instance_name: instance.name.clone(),
        },
        100.0,
        "Updating instance",
    )
    .await?;
    let map = crate::state::instances::commands::update_all_projects(
        instance_id,
        &state,
    )
    .await?;
    emit_loading(&loading_bar, 100.0, Some("Updated instance"))?;
    emit_content_changed(&instance.id).await?;

    Ok(map)
}

#[tracing::instrument]
pub async fn update_project(
    instance_id: &str,
    project_path: &str,
    skip_send_event: Option<bool>,
) -> crate::Result<String> {
    let state = State::get().await?;
    let path = crate::state::instances::commands::update_project(
        instance_id,
        project_path,
        &state,
    )
    .await?;

    if !skip_send_event.unwrap_or(false) {
        emit_content_changed(instance_id).await?;
    }

    Ok(path)
}

#[tracing::instrument]
pub async fn add_project_from_version(
    instance_id: &str,
    version_id: &str,
    reason: fetch::DownloadReason,
    dependent_on_version_id: Option<String>,
) -> crate::Result<String> {
    let state = State::get().await?;
    let project_path =
        crate::state::instances::commands::add_project_from_version(
            instance_id,
            version_id,
            reason,
            dependent_on_version_id,
            crate::state::ContentSourceKind::Local,
            crate::state::instances::ContentOwnershipKind::UserAdded,
            &state,
        )
        .await?;
    emit_content_changed(instance_id).await?;

    Ok(project_path)
}

#[tracing::instrument]
pub async fn install_project_with_dependencies(
    instance_id: &str,
    request: InstallProjectWithDependenciesRequest,
) -> crate::Result<ResolveContentPlan> {
    let state = State::get().await?;
    let metadata = super::get::get(instance_id).await?.ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;
    let plan = crate::state::instances::commands::resolve_install_plan(
        instance_id,
        crate::state::instances::commands::InstanceInstallProjectRequest {
            project_id: request.project_id,
            version_id: request.version_id,
            content_type: request.content_type,
            selected: request.selected,
            excluded_project_ids: request.excluded_project_ids,
        },
        &state,
    )
    .await?;

    let instance_id = metadata.instance.id;
    let project_ids = plan_project_ids(&plan);
    let install_plan = plan.clone();
    tokio::spawn(async move {
        match crate::state::instances::commands::install_resolved_content_plan(
            &instance_id,
            &install_plan,
            &state,
        )
        .await
        {
            Ok(_paths) => {
                if let Err(error) = emit_instance(
                    &instance_id,
                    InstancePayloadType::ContentInstallFinished {
                        project_ids: project_ids.clone(),
                        dependency_project_ids: plan_project_ids(&install_plan)
                            .into_iter()
                            .skip(1)
                            .collect(),
                    },
                )
                .await
                {
                    tracing::error!(
                        "Failed to emit content install finished event: {error}"
                    );
                }
                if let Err(error) = emit_content_changed(&instance_id).await {
                    tracing::error!(
                        "Failed to emit instance edited event after content install: {error}"
                    );
                }
            }
            Err(error) => {
                if let Err(emit_error) = emit_instance(
                    &instance_id,
                    InstancePayloadType::ContentInstallFailed {
                        project_ids,
                        message: error.to_string(),
                    },
                )
                .await
                {
                    tracing::error!(
                        "Failed to emit content install failed event: {emit_error}"
                    );
                }
            }
        }
    });

    Ok(plan)
}

#[tracing::instrument]
pub async fn preview_project_with_dependencies(
    instance_id: &str,
    request: InstallProjectWithDependenciesRequest,
) -> crate::Result<ResolveContentPlan> {
    let state = State::get().await?;
    super::get::get(instance_id).await?.ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;
    crate::state::instances::commands::resolve_install_plan(
        instance_id,
        crate::state::instances::commands::InstanceInstallProjectRequest {
            project_id: request.project_id,
            version_id: request.version_id,
            content_type: request.content_type,
            selected: request.selected,
            excluded_project_ids: request.excluded_project_ids,
        },
        &state,
    )
    .await
}

#[tracing::instrument]
pub async fn preview_project_with_dependencies_for_target(
    request: InstallProjectWithDependenciesRequest,
    game_version: String,
    loader: crate::state::ModLoader,
) -> crate::Result<ResolveContentPlan> {
    let state = State::get().await?;
    crate::state::instances::commands::resolve_install_plan_for_target(
        crate::state::instances::commands::InstanceInstallProjectRequest {
            project_id: request.project_id,
            version_id: request.version_id,
            content_type: request.content_type,
            selected: request.selected,
            excluded_project_ids: request.excluded_project_ids,
        },
        game_version,
        loader,
        &state,
    )
    .await
}

#[tracing::instrument]
pub async fn queue_project_with_dependencies(
    instance_id: &str,
    request: InstallProjectWithDependenciesRequest,
    display_title: String,
    display_icon: Option<String>,
) -> crate::Result<crate::install::InstallJobSnapshot> {
    crate::install::install_content(
        instance_id.to_string(),
        request.project_id,
        request.version_id,
        request.content_type,
        request.selected,
        request.excluded_project_ids,
        display_title,
        display_icon,
    )
    .await
}

#[tracing::instrument]
pub async fn queue_curseforge_content(
    request: crate::api::curseforge::CurseForgeInstallRequest,
    display_title: String,
    display_icon: Option<String>,
) -> crate::Result<crate::install::InstallJobSnapshot> {
    crate::install::install_curseforge_content(
        request,
        display_title,
        display_icon,
    )
    .await
}

#[tracing::instrument]
pub async fn queue_curseforge_world(
    request: crate::api::curseforge::CurseForgeWorldInstallRequest,
    display_title: String,
    display_icon: Option<String>,
) -> crate::Result<crate::install::InstallJobSnapshot> {
    crate::install::install_curseforge_world(
        request,
        display_title,
        display_icon,
    )
    .await
}

fn plan_project_ids(plan: &ResolveContentPlan) -> Vec<String> {
    let mut project_ids = Vec::with_capacity(plan.dependencies.len() + 1);
    project_ids.push(plan.primary.project_id.clone());
    project_ids.extend(
        plan.dependencies
            .iter()
            .map(|dependency| dependency.project_id.clone()),
    );
    project_ids
}

#[tracing::instrument]
pub async fn switch_project_version_with_dependencies(
    instance_id: &str,
    project_path: &str,
    version_id: &str,
) -> crate::Result<String> {
    let state = State::get().await?;
    let metadata = super::get::get(instance_id).await?.ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;
    let path =
        crate::state::instances::commands::switch_project_version_with_dependencies(
            instance_id,
            project_path,
            version_id,
            &state,
        )
        .await?;
    emit_content_changed(&metadata.instance.id).await?;

    Ok(path)
}

#[tracing::instrument]
pub async fn add_project_from_path(
    instance_id: &str,
    path: &Path,
    project_type: Option<ProjectType>,
    inner_base: Option<&str>,
) -> crate::Result<String> {
    if let Some(relative_path) =
        crate::api::curseforge::import_pending_manual_download_from_path(
            instance_id,
            path,
        )
        .await?
    {
        emit_content_changed(instance_id).await?;
        return Ok(relative_path);
    }

    let state = State::get().await?;
    let result = crate::state::instances::commands::add_project_from_path(
        instance_id,
        path,
        project_type,
        inner_base,
        &state,
    )
    .await;

    if result.is_ok() {
        emit_instance(instance_id, InstancePayloadType::Synced).await?;
    }

    result
}

#[tracing::instrument]
pub async fn import_world_save(
    instance_id: &str,
    source_path: &Path,
    inner_base: Option<&str>,
) -> crate::Result<String> {
    let state = State::get().await?;
    crate::state::instances::commands::import_world_save(
        &state,
        instance_id,
        source_path,
        inner_base,
    )
    .await
}

pub async fn install_datapack_to_world(
    instance_id: &str,
    world_path: &str,
    source_path: &Path,
    inner_base: Option<&str>,
) -> crate::Result<String> {
    let state = State::get().await?;
    crate::state::instances::commands::install_datapack_to_world(
        instance_id,
        world_path,
        source_path,
        inner_base,
        &state,
    )
    .await
}

pub async fn install_datapack_bytes_to_world(
    instance_id: &str,
    world_path: &str,
    file_name: &str,
    bytes: Vec<u8>,
) -> crate::Result<String> {
    let state = State::get().await?;
    crate::state::instances::commands::install_datapack_bytes_to_world(
        instance_id,
        world_path,
        file_name,
        &bytes,
        &state,
    )
    .await
}

#[tracing::instrument]
pub async fn toggle_disable_project(
    instance_id: &str,
    project: &str,
    desired_enabled: Option<bool>,
) -> crate::Result<String> {
    let state = State::get().await?;
    let res = crate::state::instances::commands::toggle_disable_project(
        instance_id,
        project,
        desired_enabled,
        &state,
    )
    .await?;
    emit_content_changed(instance_id).await?;

    Ok(res)
}

#[tracing::instrument]
pub async fn rollback_project(
    instance_id: &str,
    project_path: &str,
) -> crate::Result<String> {
    let state = State::get().await?;
    let res = crate::state::instances::commands::rollback_project(
        instance_id,
        project_path,
        &state,
    )
    .await?;
    emit_content_changed(instance_id).await?;

    Ok(res)
}

#[tracing::instrument]
pub async fn remove_project(
    instance_id: &str,
    project: &str,
) -> crate::Result<()> {
    let state = State::get().await?;
    crate::state::instances::commands::remove_project(
        instance_id,
        project,
        &state,
    )
    .await?;
    emit_content_changed(instance_id).await?;

    Ok(())
}

#[tracing::instrument]
pub async fn toggle_content_entry(
    instance_id: &str,
    content_id: &str,
    desired_enabled: Option<bool>,
) -> crate::Result<String> {
    let mut results = toggle_content_entries(
        instance_id,
        vec![content_id.to_string()],
        desired_enabled,
    )
    .await?;
    results.pop().map(|result| result.path).ok_or_else(|| {
        crate::ErrorKind::InputError(
            "The selected content no longer exists".to_string(),
        )
        .into()
    })
}

#[tracing::instrument]
pub async fn toggle_content_entries(
    instance_id: &str,
    content_ids: Vec<String>,
    desired_enabled: Option<bool>,
) -> crate::Result<Vec<ContentToggleResult>> {
    let state = State::get().await?;
    let results = crate::state::instances::commands::toggle_content_entries(
        instance_id,
        &content_ids,
        desired_enabled,
        &state,
    )
    .await?
    .into_iter()
    .map(|result| ContentToggleResult {
        content_id: result.content_id,
        path: result.path,
        enabled: result.enabled,
    })
    .collect::<Vec<_>>();
    if !results.is_empty() {
        emit_content_changed(instance_id).await?;
    }
    Ok(results)
}

#[tracing::instrument]
pub async fn remove_content_entry(
    instance_id: &str,
    content_id: &str,
) -> crate::Result<()> {
    let target = content_mutation_target(instance_id, content_id).await?;
    let path = target.relative_path.ok_or_else(|| {
        crate::ErrorKind::InputError(
            "The selected content is not present on disk".to_string(),
        )
    })?;
    remove_project(instance_id, &path).await
}

#[tracing::instrument]
pub async fn update_content_entry(
    instance_id: &str,
    content_id: &str,
) -> crate::Result<String> {
    let target = content_mutation_target(instance_id, content_id).await?;
    let path = target.relative_path.ok_or_else(|| {
        crate::ErrorKind::InputError(
            "The selected content is not present on disk".to_string(),
        )
    })?;
    match target.provider {
        Some(ContentProvider::CurseForge) => {
            let result = crate::api::curseforge::update_installed_file(
                instance_id,
                &path,
            )
            .await?;
            let updated_path = result
                .installed
                .iter()
                .find(|file| !file.dependency)
                .map_or(path, |file| file.relative_path.clone());
            emit_content_changed(instance_id).await?;
            Ok(updated_path)
        }
        _ => update_project(instance_id, &path, None).await,
    }
}

#[tracing::instrument]
pub async fn switch_content_entry_version(
    instance_id: &str,
    content_id: &str,
    version_id: &str,
) -> crate::Result<String> {
    let target = content_mutation_target(instance_id, content_id).await?;
    let path = target.relative_path.ok_or_else(|| {
        crate::ErrorKind::InputError(
            "The selected content is not present on disk".to_string(),
        )
    })?;
    match target.provider {
        Some(ContentProvider::CurseForge) => {
            let file_id = version_id.parse::<u32>().map_err(|_| {
                crate::ErrorKind::InputError(
                    "The selected CurseForge file ID is invalid".to_string(),
                )
            })?;
            let result = crate::api::curseforge::switch_installed_file_version(
                instance_id,
                &path,
                file_id,
            )
            .await?;
            let updated_path = result
                .installed
                .iter()
                .find(|file| !file.dependency)
                .map_or(path, |file| file.relative_path.clone());
            emit_content_changed(instance_id).await?;
            Ok(updated_path)
        }
        _ => {
            switch_project_version_with_dependencies(
                instance_id,
                &path,
                version_id,
            )
            .await
        }
    }
}

#[tracing::instrument]
pub async fn restore_pack_member_default(
    instance_id: &str,
    member_id: &str,
) -> crate::Result<Option<String>> {
    let state = State::get().await?;
    let target = content_rows::get_content_mutation_target(
        instance_id,
        member_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(
            "The selected pack member no longer exists".to_string(),
        )
    })?;
    if target.member_id.as_deref() != Some(member_id) {
        return Err(crate::ErrorKind::InputError(
            "Restore requires a pack member ID".to_string(),
        )
        .into());
    }

    let member = content_rows::get_pack_members(
        &content_rows::get_applied_content_set(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(
                    "Instance has no applied content set".to_string(),
                )
            })?
            .id,
        &state.pool,
    )
    .await?
    .into_iter()
    .find(|member| member.id == member_id)
    .ok_or_else(|| {
        crate::ErrorKind::InputError(
            "The selected pack member no longer exists".to_string(),
        )
    })?;

    if member.override_kind == PackMemberOverrideKind::None
        && member.materialization_state
            == PackMemberMaterializationState::Present
    {
        return Ok(target.relative_path);
    }
    if member.override_kind == PackMemberOverrideKind::Disabled
        && let Some(path) = target.relative_path
    {
        return toggle_disable_project(instance_id, &path, Some(true))
            .await
            .map(Some);
    }

    let project_id =
        member.provider_project_id.as_deref().ok_or_else(|| {
            crate::ErrorKind::InputError(
                "This pack member has no provider project to restore from"
                    .to_string(),
            )
        })?;
    let release_id =
        member.provider_release_id.as_deref().ok_or_else(|| {
            crate::ErrorKind::InputError(
                "This pack member has no original version to restore"
                    .to_string(),
            )
        })?;
    let old_path = target.relative_path;
    let (restored_path, pending_manual) = match member.provider {
        Some(ContentProvider::Modrinth) => (
            Some(
                crate::state::instances::commands::add_project_from_version(
                    instance_id,
                    release_id,
                    fetch::DownloadReason::Update,
                    None,
                    ContentSourceKind::ModrinthModpack,
                    ContentOwnershipKind::PackManaged,
                    &state,
                )
                .await?,
            ),
            false,
        ),
        Some(ContentProvider::CurseForge) => {
            let content_set =
                content_rows::get_applied_content_set(instance_id, &state.pool)
                    .await?
                    .ok_or_else(|| {
                        crate::ErrorKind::InputError(
                            "Instance has no applied content set".to_string(),
                        )
                    })?;
            let result = crate::api::curseforge::install_file(
				crate::api::curseforge::CurseForgeInstallRequest {
					instance_id: instance_id.to_string(),
					project_id: project_id.parse().map_err(|_| {
						crate::ErrorKind::InputError(
							"Stored CurseForge project ID is invalid".to_string(),
						)
					})?,
					file_id: release_id.parse().map_err(|_| {
						crate::ErrorKind::InputError(
							"Stored CurseForge file ID is invalid".to_string(),
						)
					})?,
					project_type: member.project_type.get_name().to_string(),
					ownership_kind: ContentOwnershipKind::PackManaged,
					manual_operation_kind: crate::state::instances::ManualDownloadOperationKind::ContentUpdate,
					game_version: Some(content_set.game_version),
					mod_loader_type: curseforge_loader_type(content_set.loader),
					world_name: None,
					install_dependencies: false,
					excluded_dependency_project_ids: Vec::new(),
					dependency_plan_id: None,
				},
			)
			.await?;
            let pending_manual = !result.manual_downloads.is_empty();
            let failure_reason = result
                .failed_downloads
                .first()
                .map(|failure| failure.reason.clone());
            let restored_path = result
                .installed
                .into_iter()
                .find(|file| !file.dependency)
                .map(|file| file.relative_path);
            if restored_path.is_none() && !pending_manual {
                return Err(crate::ErrorKind::OtherError(
                    failure_reason.unwrap_or_else(|| {
                        "CurseForge did not return the restored file"
                            .to_string()
                    }),
                )
                .into());
            }
            (restored_path, pending_manual)
        }
        Some(ContentProvider::Local) => {
            return Err(crate::ErrorKind::InputError(
                "This pack member has no managed provider".to_string(),
            )
            .into());
        }
        None => {
            return Err(crate::ErrorKind::InputError(
                "This pack member has no managed provider".to_string(),
            )
            .into());
        }
    };

    if let (Some(old_path), Some(new_path)) =
        (old_path.as_deref(), restored_path.as_deref())
        && old_path != new_path
        && crate::state::instances::commands::archive_project_file(
            instance_id,
            old_path,
            new_path,
            &state,
        )
        .await?
        .is_none()
    {
        crate::state::instances::commands::remove_project(
            instance_id,
            old_path,
            &state,
        )
        .await?;
    }

    let content_set =
        content_rows::get_applied_content_set(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(
                    "Instance has no applied content set".to_string(),
                )
            })?;
    let _instance_lock = state.lock_instance_content(instance_id).await;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    content_rows::set_pack_member_override_in_transaction(
        member_id,
        if pending_manual {
            PackMemberMaterializationState::PendingManual
        } else {
            PackMemberMaterializationState::Present
        },
        PackMemberOverrideKind::None,
        &mut tx,
    )
    .await?;
    content_rows::bump_content_set_revision_in_transaction(
        &content_set.id,
        &mut tx,
    )
    .await?;
    tx.commit().await?;
    emit_content_changed(instance_id).await?;
    Ok(restored_path)
}

pub(crate) async fn emit_content_changed(
    instance_id: &str,
) -> crate::Result<()> {
    let state = State::get().await?;
    let content_set =
        content_rows::get_applied_content_set(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(
                    "Instance has no applied content set".to_string(),
                )
            })?;
    emit_instance(
        instance_id,
        InstancePayloadType::ContentChanged {
            revision: content_set.revision,
        },
    )
    .await
}

async fn content_mutation_target(
    instance_id: &str,
    content_id: &str,
) -> crate::Result<content_rows::ContentMutationTarget> {
    let state = State::get().await?;
    content_rows::get_content_mutation_target(
        instance_id,
        content_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(
            "The selected content no longer exists".to_string(),
        )
        .into()
    })
}

fn curseforge_loader_type(loader: crate::state::ModLoader) -> Option<u32> {
    match loader {
        crate::state::ModLoader::Forge => Some(1),
        crate::state::ModLoader::Fabric => Some(4),
        crate::state::ModLoader::Quilt => Some(5),
        crate::state::ModLoader::NeoForge => Some(6),
        _ => None,
    }
}

#[tracing::instrument]
pub async fn update_managed_modrinth_version(
    instance_id: &str,
    version_id: &str,
) -> crate::Result<crate::install::InstallJobSnapshot> {
    let state = State::get().await?;
    let metadata = crate::state::instances::commands::get_instance_metadata(
        instance_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;

    let post_install_edit = match &metadata.link {
        crate::state::InstanceLink::ServerProjectModpack {
            server_project_id,
            content_project_id,
            ..
        } => Some(crate::install::InstallPostInstallEdit {
            name: Some(metadata.instance.name.clone()),
            icon_path: Some(metadata.instance.icon_path.clone()),
            link: Some(crate::state::InstanceLink::ServerProjectModpack {
                server_project_id: server_project_id.clone(),
                content_project_id: content_project_id.clone(),
                content_version_id: version_id.to_string(),
            }),
        }),
        _ => None,
    };

    let project_id = match &metadata.link {
        crate::state::InstanceLink::ModrinthModpack { project_id, .. } => {
            project_id.clone()
        }
        crate::state::InstanceLink::ServerProjectModpack {
            content_project_id,
            ..
        } => content_project_id.clone(),
        _ => {
            return Err(unmanaged_pack_error(&metadata.instance.id).into());
        }
    };

    crate::install::install_pack_to_existing_instance(
        metadata.instance.id,
        crate::api::pack::install_from::CreatePackLocation::FromVersionId {
            project_id,
            version_id: version_id.to_string(),
            title: metadata.instance.name.clone(),
            icon_url: None,
        },
        post_install_edit,
    )
    .await
}

#[tracing::instrument]
pub async fn repair_managed_modrinth(
    instance_id: &str,
) -> crate::Result<crate::install::InstallJobSnapshot> {
    let state = State::get().await?;
    let metadata = crate::state::instances::commands::get_instance_metadata(
        instance_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;

    let post_install_edit = match &metadata.link {
        crate::state::InstanceLink::ServerProjectModpack { .. } => {
            Some(crate::install::InstallPostInstallEdit {
                name: Some(metadata.instance.name.clone()),
                icon_path: Some(metadata.instance.icon_path.clone()),
                link: Some(metadata.link.clone()),
            })
        }
        _ => None,
    };

    let (project_id, version_id) = match &metadata.link {
        crate::state::InstanceLink::ModrinthModpack {
            project_id,
            version_id,
        } => (project_id.clone(), version_id.clone()),
        crate::state::InstanceLink::ServerProjectModpack {
            content_project_id,
            content_version_id,
            ..
        } => (content_project_id.clone(), content_version_id.clone()),
        _ => {
            return Err(unmanaged_pack_error(&metadata.instance.id).into());
        }
    };

    crate::install::install_pack_to_existing_instance(
        metadata.instance.id,
        crate::api::pack::install_from::CreatePackLocation::FromVersionId {
            project_id,
            version_id,
            title: metadata.instance.name.clone(),
            icon_url: None,
        },
        post_install_edit,
    )
    .await
}

fn unmanaged_pack_error(instance_id: &str) -> crate::ErrorKind {
    crate::ErrorKind::InputError(format!(
        "Instance {instance_id} is not a managed Modrinth pack, or has been disconnected."
    ))
}

async fn get_instance_display_info(
    instance_id: &str,
    state: &State,
) -> crate::Result<instance_rows::InstanceDisplayInfo> {
    instance_rows::get_instance_display_info(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string()).into()
        })
}
