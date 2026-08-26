use super::events::{InstallProgressReporter, emit_install_job};
use super::model::{
    InstallCleanup, InstallContinuationState, InstallErrorContext,
    InstallErrorView, InstallJavaStep, InstallJobDisplay, InstallJobEventKind,
    InstallJobSnapshot, InstallJobState, InstallJobStatus, InstallPauseReason,
    InstallPhaseDetails, InstallPhaseId, InstallPostInstallEdit,
    InstallProgress, InstallRequest, InstallRollbackState, InstallTarget,
    InstanceUpgradeCompatibilityWarning, InstanceUpgradeDisplayNames,
    InstanceUpgradeExecution, InstanceUpgradeExternalChange,
    InstanceUpgradeExternalChangeKind, InstanceUpgradeResult,
    InstanceUpgradeWatchBaseline, SharedUpgradeMode,
};
use super::{diagnostics, recovery, store};
use crate::ErrorKind;
use crate::api::pack::install_from::{
    CreatePackLocation, generate_pack_from_file,
    generate_pack_from_version_id_with_reporter, get_instance_from_pack,
};
use crate::api::pack::install_mrpack::{
    MrpackInstallOutcome, install_zipped_mrpack_files_with_reporter,
    related_file_paths,
};
use crate::event::InstancePayloadType;
use crate::event::emit::emit_instance;
use crate::state::{
    ContentProvider, ContentProviderRef, InstanceInstallStage, InstanceLink,
    InstanceUpgradeAction, InstanceUpgradeDependencyChangeKind,
    LoaderComponent, LoaderComponentKind, LoaderComponentRole, ModLoader,
    State,
};
use crate::util::fetch::DownloadReason;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use uuid::Uuid;

enum InstallExecutionOutcome<T> {
    Completed(T),
    WaitingForUser(InstallPauseReason),
}

pub async fn create_instance(
    name: String,
    game_version: String,
    loader: ModLoader,
    loader_version: Option<String>,
    icon_path: Option<String>,
    link: InstanceLink,
) -> crate::Result<InstallJobSnapshot> {
    create_instance_with_adjuncts(
        name,
        game_version,
        loader,
        loader_version,
        Vec::new(),
        icon_path,
        link,
        None,
    )
    .await
}

pub async fn create_instance_with_adjuncts(
    name: String,
    game_version: String,
    loader: ModLoader,
    loader_version: Option<String>,
    adjuncts: Vec<crate::state::LoaderComponent>,
    icon_path: Option<String>,
    link: InstanceLink,
    game_dir_override: Option<String>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::CreateInstance {
        name,
        game_version,
        loader,
        loader_version,
        adjuncts,
        icon_path,
        link,
        game_dir_override,
    })
    .await
}

pub async fn create_modpack_instance(
    location: CreatePackLocation,
    post_install_edit: Option<InstallPostInstallEdit>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::CreateModpackInstance {
        location,
        post_install_edit,
    })
    .await
}

pub async fn import_instance(
    launcher_type: crate::api::pack::import::ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
    symlink: bool,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::ImportInstance {
        launcher_type,
        base_path,
        instance_folder,
        instance_path: None,
        symlink,
        game_version: None,
        loader: None,
        loader_version: None,
        game_dir_override: None,
    })
    .await
}

/// Like [`import_instance`] but with a pre-resolved filesystem path.
/// Used by the frontend when the path is already known from scanning,
/// avoiding redundant config/registry re-resolution.
pub async fn import_instance_with_path(
    launcher_type: crate::api::pack::import::ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
    instance_path: Option<String>,
    symlink: bool,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::ImportInstance {
        launcher_type,
        base_path,
        instance_folder,
        instance_path,
        symlink,
        game_version: None,
        loader: None,
        loader_version: None,
        game_dir_override: None,
    })
    .await
}

pub async fn import_instance_with_plan(
    launcher_type: crate::api::pack::import::ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
    instance_path: Option<String>,
    symlink: bool,
    game_version: Option<String>,
    loader: Option<crate::state::ModLoader>,
    loader_version: Option<String>,
    game_dir_override: Option<String>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::ImportInstance {
        launcher_type,
        base_path,
        instance_folder,
        instance_path,
        symlink,
        game_version,
        loader,
        loader_version,
        game_dir_override,
    })
    .await
}

pub async fn duplicate_instance(
    source_instance_id: String,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::DuplicateInstance { source_instance_id }).await
}

pub async fn install_existing_instance(
    instance_id: String,
    force: bool,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallExistingInstance { instance_id, force }).await
}

pub async fn upgrade_unmanaged_instance(
    instance_id: String,
    plan_id: String,
    execution: InstanceUpgradeExecution,
    create_full_backup: bool,
    shared_upgrade_mode: SharedUpgradeMode,
    display_names: InstanceUpgradeDisplayNames,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::UpgradeUnmanagedInstance {
        instance_id,
        plan_id,
        execution,
        create_full_backup,
        shared_upgrade_mode,
        display_names,
    })
    .await
}

pub async fn install_content(
    instance_id: String,
    project_id: String,
    version_id: Option<String>,
    content_type: modrinth_content_management::ContentType,
    selected: modrinth_content_management::ResolutionPreferences,
    excluded_project_ids: Vec<String>,
    display_title: String,
    display_icon: Option<String>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallContent {
        instance_id,
        project_id,
        version_id,
        content_type,
        selected,
        excluded_project_ids,
        display_title,
        display_icon,
    })
    .await
}

pub async fn install_curseforge_content(
    request: crate::api::curseforge::CurseForgeInstallRequest,
    display_title: String,
    display_icon: Option<String>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallCurseForgeContent {
        request,
        display_title,
        display_icon,
    })
    .await
}

pub async fn install_curseforge_world(
    request: crate::api::curseforge::CurseForgeWorldInstallRequest,
    display_title: String,
    display_icon: Option<String>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallCurseForgeWorld {
        request,
        display_title,
        display_icon,
    })
    .await
}

pub async fn download_java(
    vendor: String,
    version: u32,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::DownloadJava { vendor, version }).await
}

pub async fn install_pack_to_existing_instance(
    instance_id: String,
    location: CreatePackLocation,
    post_install_edit: Option<InstallPostInstallEdit>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallPackToExistingInstance {
        instance_id,
        location,
        post_install_edit,
    })
    .await
}

pub async fn update_managed_curseforge_modpack(
    instance_id: String,
    file_id: u32,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::UpdateManagedCurseForgeModpack {
        instance_id,
        file_id,
    })
    .await
}

pub async fn list_jobs(
    include_finished: bool,
) -> crate::Result<Vec<InstallJobSnapshot>> {
    let state = State::get().await?;
    Ok(store::list(include_finished, &state)
        .await?
        .into_iter()
        .map(|job| job.snapshot())
        .collect())
}

pub async fn get_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    Ok(store::get_required(job_id, &state).await?.snapshot())
}

pub async fn job_support_details(job_id: Uuid) -> crate::Result<String> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;
    diagnostics::build_job_support_details(&job, &state).await
}

pub async fn retry_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let mut job = store::get_required(job_id, &state).await?;

    if !matches!(
        job.status,
        InstallJobStatus::Failed | InstallJobStatus::Interrupted
    ) {
        return Err(crate::ErrorKind::InputError(
            "Only failed or interrupted install jobs can be retried"
                .to_string(),
        )
        .into());
    }

    job.state.target = job.state.request.target();
    job.state.cleanup = job.state.request.cleanup();
    job.state.rollback = None;
    job.state.error = None;
    job.state.rollback_error = None;
    job.state.pause_reason = None;
    job.state.continuation = None;
    job.state.context = None;
    job.state.progress.phase = InstallPhaseId::PreparingInstance;
    job.state.progress.progress = None;
    job.state.progress.details = InstallPhaseDetails::Empty;
    job.state.progress.parallel = None;
    prepare_initial_instance(&mut job.state, &state).await?;
    job.state.record_event(InstallJobEventKind::JobQueued {
        kind: job.state.request.kind(),
    });

    let record = store::update_status(
        job_id,
        InstallJobStatus::Queued,
        &job.state,
        &state,
    )
    .await?;
    emit_install_job(&record.snapshot()).await?;
    spawn_job(job_id);

    // The spawned job may already have progressed (or finished) by the time
    // the command returns; hand the caller the freshest stored state.
    Ok(store::get_required(job_id, &state).await?.snapshot())
}

pub async fn repair_cache_and_retry_job(
    job_id: Uuid,
) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let initial_job = store::get_required(job_id, &state).await?;
    let _ = validated_cache_repair_types(&initial_job)?;

    let operation_lock = state
        .install_job_operation_locks
        .entry(job_id)
        .or_default()
        .clone();
    let mut operation = operation_lock.lock().await;
    if operation.cache_repair_started {
        return Ok(store::get_required(job_id, &state).await?.snapshot());
    }

    let job = store::get_required(job_id, &state).await?;
    let cache_types = validated_cache_repair_types(&job)?;
    operation.cache_repair_started = true;

    if let Err(error) =
        crate::state::CachedEntry::purge_cache_types(&cache_types, &state.pool)
            .await
    {
        operation.cache_repair_started = false;
        return Err(crate::ErrorKind::OtherError(format!(
            "Project cache cleanup failed; retry was not started: {error}"
        ))
        .into());
    }

    retry_job(job_id).await.map_err(|error| {
        crate::ErrorKind::OtherError(format!(
            "Project cache was cleared, but retry could not be started: {error}"
        ))
        .into()
    })
}

fn validated_cache_repair_types(
    job: &store::InstallJobRecord,
) -> crate::Result<Vec<crate::state::CacheValueType>> {
    validated_cache_repair_types_for(job.status, job.state.error.as_ref())
}

fn validated_cache_repair_types_for(
    status: InstallJobStatus,
    error: Option<&InstallErrorView>,
) -> crate::Result<Vec<crate::state::CacheValueType>> {
    if !matches!(
        status,
        InstallJobStatus::Failed | InstallJobStatus::Interrupted
    ) {
        return Err(crate::ErrorKind::InputError(
            "Only failed or interrupted install jobs can repair cache"
                .to_string(),
        )
        .into());
    }
    let error = error.ok_or_else(|| {
        crate::ErrorKind::InputError(
            "Install job has no cache repair error".to_string(),
        )
    })?;
    if error.code != "cache_repair_required" {
        return Err(crate::ErrorKind::InputError(
            "Install job does not require cache repair".to_string(),
        )
        .into());
    }
    let cache_types = error
        .context
        .as_ref()
        .map(|context| context.cache_types.as_slice())
        .unwrap_or_default();
    if cache_types.is_empty() {
        return Err(crate::ErrorKind::InputError(
            "Install job has no repairable cache types".to_string(),
        )
        .into());
    }

    let mut validated = Vec::new();
    for cache_type in cache_types {
        let cache_type =
            crate::state::CacheValueType::from_repairable_str(cache_type)
                .ok_or_else(|| {
                    crate::ErrorKind::InputError(format!(
                        "Cache type is not repairable: {cache_type}"
                    ))
                })?;
        if !validated.contains(&cache_type) {
            validated.push(cache_type);
        }
    }
    Ok(validated)
}

pub async fn resume_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;
    if job.status != InstallJobStatus::WaitingForUser {
        return Err(crate::ErrorKind::InputError(
            "Only install jobs waiting for user action can be resumed"
                .to_string(),
        )
        .into());
    }

    queue_waiting_job(job_id, job.state, &state).await
}

pub async fn skip_missing_content_and_resume_job(
    job_id: Uuid,
) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;
    if job.status != InstallJobStatus::WaitingForUser {
        return Err(crate::ErrorKind::InputError(
            "Only install jobs waiting for user action can skip missing content"
                .to_string(),
        )
        .into());
    }
    if matches!(
        job.state.request,
        InstallRequest::UpdateManagedCurseForgeModpack { .. }
    ) {
        return Err(crate::ErrorKind::InputError(
            "CurseForge modpack version updates cannot skip required manual downloads"
                .to_string(),
        )
        .into());
    }

    let mut current_missing_paths = job
        .snapshot()
        .items
        .into_iter()
        .filter(|item| {
            item.status == super::model::DownloadItemStatus::Failed
                || (item.status == super::model::DownloadItemStatus::Skipped
                    && item.manual_url.is_some())
        })
        .map(|item| item.id)
        .collect::<Vec<_>>();
    let mut job_state = job.state;
    let InstallPauseReason::MissingRequiredContent { paths, .. } =
        job_state.pause_reason.as_ref().ok_or_else(|| {
            crate::ErrorKind::InputError(
                "Install job has no missing content to skip".to_string(),
            )
        })?;
    if current_missing_paths.is_empty() {
        current_missing_paths = paths.clone();
    }
    if current_missing_paths.is_empty() {
        return Err(crate::ErrorKind::InputError(
            "Install job has no missing content to skip".to_string(),
        )
        .into());
    }
    job_state
        .skipped_missing_content_paths
        .extend(current_missing_paths);
    job_state.skipped_missing_content_paths.sort_unstable();
    job_state.skipped_missing_content_paths.dedup();

    queue_waiting_job(job_id, job_state, &state).await
}

async fn queue_waiting_job(
    job_id: Uuid,
    mut job_state: InstallJobState,
    state: &State,
) -> crate::Result<InstallJobSnapshot> {
    prepare_resumed_job(&mut job_state);
    let Some(record) = store::update_status_if(
        job_id,
        InstallJobStatus::WaitingForUser,
        InstallJobStatus::Queued,
        &job_state,
        &state,
    )
    .await?
    else {
        return Err(crate::ErrorKind::InputError(
            "Install job is no longer waiting for user action".to_string(),
        )
        .into());
    };
    InstallProgressReporter::reset_job(job_id);
    emit_install_job(&record.snapshot()).await?;
    spawn_job(job_id);
    Ok(store::get_required(job_id, &state).await?.snapshot())
}

fn prepare_resumed_job(job_state: &mut InstallJobState) {
    job_state.pause_reason = None;
    job_state.error = None;
    job_state.rollback_error = None;
    job_state.context = None;
    job_state.active_downloads.clear();
    job_state.record_event(InstallJobEventKind::JobQueued {
        kind: job_state.request.kind(),
    });
}

pub async fn retry_job_as_new(
    job_id: Uuid,
) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;
    if !matches!(
        job.status,
        InstallJobStatus::Failed
            | InstallJobStatus::Interrupted
            | InstallJobStatus::Canceled
    ) {
        return Err(crate::ErrorKind::InputError(
            "Only failed, interrupted, or canceled downloads can be retried"
                .to_string(),
        )
        .into());
    }
    let new_job = start(job.state.request).await?;
    // The spawned job may already have progressed (or finished) by the time
    // the command returns; hand the caller the freshest stored state.
    Ok(store::get_required(new_job.job_id, &state)
        .await?
        .snapshot())
}

pub async fn cancel_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let mut job = loop {
        let mut job = store::get_required(job_id, &state).await?;
        match job.status {
            InstallJobStatus::Running => {
                let Some(record) = store::update_status_if(
                    job_id,
                    InstallJobStatus::Running,
                    InstallJobStatus::Canceling,
                    &job.state,
                    &state,
                )
                .await?
                else {
                    continue;
                };
                if let Some(token) =
                    state.install_job_cancellations.get(&job_id)
                {
                    token.cancel();
                }
                emit_install_job(&record.snapshot()).await?;
                return Ok(record.snapshot());
            }
            InstallJobStatus::Canceling => return Ok(job.snapshot()),
            InstallJobStatus::Queued | InstallJobStatus::WaitingForUser => {
                let expected = job.status;
                begin_canceling_job(&mut job.state);
                let Some(record) = store::update_status_if(
                    job_id,
                    expected,
                    InstallJobStatus::Canceling,
                    &job.state,
                    &state,
                )
                .await?
                else {
                    continue;
                };
                emit_install_job(&record.snapshot()).await?;
                break record;
            }
            _ => {
                return Err(crate::ErrorKind::InputError(
                    "Only queued, running, or waiting install jobs can be canceled"
                        .to_string(),
                )
                .into());
            }
        }
    };

    let cleanup_succeeded =
        match recovery::apply_cleanup(&mut job.state, &state).await {
            Ok(()) => true,
            Err(error) => {
                job.state.rollback_error = Some(InstallErrorView::from_error(
                    "rollback_error",
                    InstallPhaseId::RollingBack,
                    &error,
                    None,
                ));
                job.state.record_event(InstallJobEventKind::RollbackFailed {
                    message: error.to_string(),
                });
                false
            }
        };
    recovery::finalize_rollback_state(&mut job.state, cleanup_succeeded);
    if cleanup_succeeded {
        clear_deleted_new_instance_id(&mut job.state);
    }
    let record = store::update_status(
        job_id,
        InstallJobStatus::Canceled,
        &job.state,
        &state,
    )
    .await?;
    emit_install_job(&record.snapshot()).await?;

    Ok(record.snapshot())
}

fn begin_canceling_job(job_state: &mut InstallJobState) {
    let canceled_phase = job_state.progress.phase;
    job_state.error = Some(InstallErrorView::from_message(
        "canceled",
        canceled_phase,
        "Install was canceled",
    ));
    job_state.pause_reason = None;
    job_state.record_event(InstallJobEventKind::JobCanceled {
        phase: canceled_phase,
    });
    job_state.progress.phase = InstallPhaseId::RollingBack;
    job_state.progress.progress = None;
    job_state.progress.details = InstallPhaseDetails::Empty;
    job_state.progress.parallel = None;
    job_state.record_event(InstallJobEventKind::RollbackStarted {
        cleanup: job_state.cleanup.clone(),
    });
}

pub async fn dismiss_job(job_id: Uuid) -> crate::Result<()> {
    let state = State::get().await?;
    store::dismiss(job_id, &state).await
}

pub async fn clear_job_history() -> crate::Result<u64> {
    let state = State::get().await?;
    store::clear_finished(&state).await
}

async fn start(request: InstallRequest) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let id = Uuid::new_v4();
    let mut job_state = InstallJobState::new(request);
    prepare_initial_instance(&mut job_state, &state).await?;
    let record =
        match store::insert(id, &job_state, InstallJobStatus::Queued, &state)
            .await
        {
            Ok(record) => record,
            Err(error) => {
                return Err(cleanup_failed_initial_install(
                    &mut job_state,
                    &state,
                    error,
                )
                .await);
            }
        };
    emit_install_job(&record.snapshot()).await?;
    spawn_job(id);
    Ok(record.snapshot())
}

async fn cleanup_failed_initial_install(
    job_state: &mut InstallJobState,
    state: &State,
    error: crate::Error,
) -> crate::Error {
    match recovery::apply_cleanup(job_state, state).await {
        Ok(()) => error,
        Err(cleanup_error) => crate::ErrorKind::OtherError(format!(
            "Install initialization failed: {error}; cleanup also failed: {cleanup_error}"
        ))
        .into(),
    }
}

async fn prepare_initial_instance(
    job_state: &mut InstallJobState,
    state: &State,
) -> crate::Result<()> {
    match job_state.request.clone() {
        InstallRequest::CreateInstance {
            name,
            mut game_version,
            mut loader,
            mut loader_version,
            mut adjuncts,
            icon_path,
            link,
            game_dir_override,
        } => {
            if let InstanceLink::CurseForgeModpack {
                project_id,
                version_id,
            } = &link
            {
                let project_id = project_id.parse::<u32>().map_err(|_| {
                    ErrorKind::InputError(
                        "CurseForge project ID is invalid".to_string(),
                    )
                })?;
                let file_id = version_id.parse::<u32>().map_err(|_| {
                    ErrorKind::InputError(
                        "CurseForge file ID is invalid".to_string(),
                    )
                })?;
                let target = crate::api::curseforge::get_modpack_target(
                    project_id, file_id,
                )
                .await?;
                game_version = target.game_version;
                loader = target.loader;
                loader_version = target.loader_version;
                adjuncts.clear();
                job_state.request = InstallRequest::CreateInstance {
                    name: name.clone(),
                    game_version: game_version.clone(),
                    loader,
                    loader_version: loader_version.clone(),
                    adjuncts: Vec::new(),
                    icon_path: icon_path.clone(),
                    link: link.clone(),
                    game_dir_override: game_dir_override.clone(),
                };
            }
            resolve_required_adjuncts(
                &game_version,
                loader,
                &mut adjuncts,
                state,
            )
            .await?;
            job_state.request = InstallRequest::CreateInstance {
                name: name.clone(),
                game_version: game_version.clone(),
                loader,
                loader_version: loader_version.clone(),
                adjuncts: adjuncts.clone(),
                icon_path: icon_path.clone(),
                link: link.clone(),
                game_dir_override: game_dir_override.clone(),
            };
            let metadata = crate::api::instance::create(
                name,
                game_version,
                loader,
                loader_version,
                icon_path,
                link,
                None,
                game_dir_override,
            )
            .await?;
            if !adjuncts.is_empty() {
                let mut components = metadata.loader_components.clone();
                for adjunct in &mut adjuncts {
                    adjunct.instance_id = metadata.instance.id.clone();
                    adjunct.role = crate::state::LoaderComponentRole::Adjunct;
                }
                components.extend(adjuncts);
                validate_loader_components(&components)?;
                crate::state::instances::commands::replace_instance_loader_components(
					&metadata.instance.id,
					&components,
					&state.pool,
				)
				.await?;
            }
            set_display(
                job_state,
                metadata.instance.name,
                metadata.instance.icon_path,
            );
            set_instance_id(job_state, metadata.instance.id);
        }
        InstallRequest::CreateModpackInstance {
            location,
            post_install_edit,
        } => {
            let preview = get_instance_from_pack(location).await?;
            let name = post_install_edit
                .as_ref()
                .and_then(|edit| edit.name.clone())
                .unwrap_or_else(|| preview.name.clone());
            let icon_path = match post_install_edit
                .as_ref()
                .and_then(|edit| edit.icon_path.as_ref())
            {
                Some(icon_path) => icon_path.clone(),
                None => preview
                    .icon
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string())
                    .or_else(|| preview.icon_url.clone()),
            };
            let link = post_install_edit
                .as_ref()
                .and_then(|edit| edit.link.clone())
                .or_else(|| preview.link.clone())
                .unwrap_or(InstanceLink::Unmanaged);
            let metadata = crate::api::instance::create(
                name,
                preview.game_version,
                preview.modloader,
                preview.loader_version,
                icon_path,
                link,
                None,
                None,
            )
            .await?;
            set_display(
                job_state,
                metadata.instance.name,
                metadata.instance.icon_path,
            );
            set_instance_id(job_state, metadata.instance.id);
        }
        InstallRequest::ImportInstance {
            instance_folder,
            symlink: _,
            base_path: _,
            game_dir_override,
            ..
        } => {
            let metadata = crate::api::instance::create(
                instance_folder,
                "unknown".to_string(),
                ModLoader::Vanilla,
                None,
                None,
                InstanceLink::Unmanaged,
                None,
                game_dir_override,
            )
            .await?;
            set_display(
                job_state,
                metadata.instance.name,
                metadata.instance.icon_path,
            );
            set_instance_id(job_state, metadata.instance.id);
        }
        InstallRequest::DuplicateInstance { source_instance_id } => {
            let metadata =
                crate::state::get_instance(&source_instance_id, &state.pool)
                    .await?
                    .ok_or_else(|| {
                        crate::ErrorKind::InputError(
                            "Unknown instance".to_string(),
                        )
                    })?;
            let created = crate::api::instance::create(
                metadata.instance.name,
                metadata.applied_content_set.game_version,
                metadata.applied_content_set.loader,
                metadata.applied_content_set.loader_version,
                metadata.instance.icon_path,
                metadata.link,
                None,
                None,
            )
            .await?;
            set_display(
                job_state,
                created.instance.name,
                created.instance.icon_path,
            );
            set_instance_id(job_state, created.instance.id);
        }
        InstallRequest::UpgradeUnmanagedInstance {
            instance_id,
            shared_upgrade_mode,
            display_names,
            ..
        } => {
            let metadata =
                crate::state::get_instance(&instance_id, &state.pool)
                    .await?
                    .ok_or_else(|| {
                        crate::ErrorKind::InputError(
                            "Unknown upgrade source instance".to_string(),
                        )
                    })?;
            set_display(
                job_state,
                metadata.instance.name.clone(),
                metadata.instance.icon_path.clone(),
            );
            match shared_upgrade_mode {
                SharedUpgradeMode::Direct => {
                    prepare_existing_rollback(job_state, state, &instance_id)
                        .await?;
                }
                SharedUpgradeMode::CopyAndUpgrade => {
                    let created = crate::api::instance::create(
                        display_names.copy.unwrap_or_else(|| {
                            format!(
                                "{} (Upgraded Copy)",
                                metadata.instance.name
                            )
                        }),
                        metadata.applied_content_set.game_version.clone(),
                        metadata.applied_content_set.loader,
                        metadata.applied_content_set.loader_version.clone(),
                        metadata.instance.icon_path.clone(),
                        InstanceLink::Unmanaged,
                        None,
                        None,
                    )
                    .await?;
                    set_instance_id(job_state, created.instance.id.clone());
                    if let Err(error) = clone_instance_loader_components(
                        &metadata.loader_components,
                        &created.instance.id,
                        state,
                    )
                    .await
                    {
                        return Err(cleanup_failed_initial_install(
                            job_state, state, error,
                        )
                        .await);
                    }
                }
            }
        }
        InstallRequest::InstallExistingInstance { instance_id, .. }
        | InstallRequest::InstallPackToExistingInstance {
            instance_id, ..
        }
        | InstallRequest::UpdateManagedCurseForgeModpack {
            instance_id, ..
        } => {
            prepare_existing_rollback(job_state, state, &instance_id).await?;
        }
        InstallRequest::InstallContent {
            instance_id,
            display_title,
            display_icon,
            ..
        } => {
            crate::state::get_instance(&instance_id, &state.pool)
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError(format!(
                        "Unknown instance {instance_id}"
                    ))
                })?;
            set_display(job_state, display_title, display_icon);
        }
        InstallRequest::InstallCurseForgeContent {
            request,
            display_title,
            display_icon,
        } => {
            crate::state::get_instance(&request.instance_id, &state.pool)
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError(format!(
                        "Unknown instance {}",
                        request.instance_id
                    ))
                })?;
            set_display(job_state, display_title, display_icon);
        }
        InstallRequest::InstallCurseForgeWorld {
            request,
            display_title,
            display_icon,
        } => {
            crate::state::get_instance(&request.instance_id, &state.pool)
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError(format!(
                        "Unknown instance {}",
                        request.instance_id
                    ))
                })?;
            set_display(job_state, display_title, display_icon);
        }
        InstallRequest::DownloadJava { vendor, version } => {
            set_display(job_state, format!("Java {version} ({vendor})"), None);
        }
    }

    Ok(())
}

fn spawn_job(job_id: Uuid) {
    tokio::spawn(async move {
        if let Err(error) = run_job(job_id).await {
            tracing::error!("Install job {job_id} failed: {error}");
        }
    });
}

fn begin_failed_job_rollback(
    job_state: &mut InstallJobState,
    error: &crate::Error,
) {
    let failed_phase = job_state.progress.phase;
    let error_view =
        install_error_view(failed_phase, error, job_state.context.clone());
    job_state.record_event(InstallJobEventKind::Failed {
        phase: failed_phase,
        code: error_view.code.clone(),
        message: error_view.message.clone(),
    });
    job_state.error = Some(error_view);
    job_state.progress.phase = InstallPhaseId::RollingBack;
    job_state.progress.progress = None;
    job_state.progress.details = InstallPhaseDetails::Empty;
    job_state.progress.parallel = None;
    job_state.record_event(InstallJobEventKind::RollbackStarted {
        cleanup: job_state.cleanup.clone(),
    });
}

fn latest_failure_phase(
    execution_state: &InstallJobState,
    reporter_state: &InstallJobState,
) -> InstallPhaseId {
    let latest_phase = |state: &InstallJobState| {
        state
            .events
            .iter()
            .rev()
            .find_map(|event| match &event.kind {
                InstallJobEventKind::PhaseStarted { phase, .. } => {
                    Some((event.at, *phase))
                }
                _ => None,
            })
    };
    match (latest_phase(execution_state), latest_phase(reporter_state)) {
        (Some(execution), Some(reporter)) if reporter.0 > execution.0 => {
            reporter.1
        }
        (Some(execution), _) => execution.1,
        (None, Some(reporter)) => reporter.1,
        (None, None) => reporter_state.progress.phase,
    }
}

fn begin_waiting_for_user(
    job_state: &mut InstallJobState,
    reason: InstallPauseReason,
) {
    job_state.pause_reason = Some(reason.clone());
    job_state.error = None;
    job_state.rollback_error = None;
    job_state.context = None;
    job_state.progress.parallel = None;
    job_state.record_event(InstallJobEventKind::WaitingForUser { reason });
}

async fn run_job(job_id: Uuid) -> crate::Result<()> {
    let state = State::get().await?;
    let mut job = store::get_required(job_id, &state).await?;

    if job.status != InstallJobStatus::Queued {
        return Ok(());
    }

    let _install_permit = state.install_job_semaphore.acquire().await?;
    job = store::get_required(job_id, &state).await?;

    if job.status != InstallJobStatus::Queued {
        return Ok(());
    }

    let mut job_state = job.state.clone();
    job_state.record_event(InstallJobEventKind::JobStarted);
    let Some(record) = store::update_status_if(
        job_id,
        InstallJobStatus::Queued,
        InstallJobStatus::Running,
        &job_state,
        &state,
    )
    .await?
    else {
        return Ok(());
    };
    let cancellation = tokio_util::sync::CancellationToken::new();
    state
        .install_job_cancellations
        .insert(job_id, cancellation.clone());
    emit_install_job(&record.snapshot()).await?;
    if store::get_required(job_id, &state).await?.status
        == InstallJobStatus::Canceling
    {
        cancellation.cancel();
    }
    let live_reporter = InstallProgressReporter::new(job_id, job_state.clone());

    enum RunResult {
        Completed(crate::Result<InstallExecutionOutcome<Option<String>>>),
        Canceled,
    }

    let result = tokio::select! {
        biased;
        _ = cancellation.cancelled() => RunResult::Canceled,
        result = run_request(job_id, &mut job_state, &state) => RunResult::Completed(result),
    };
    state.install_job_cancellations.remove(&job_id);
    let execution_state = job_state;
    let reporter_state = live_reporter.current_state().await?;
    let failure_phase = latest_failure_phase(&execution_state, &reporter_state);
    job_state = reporter_state;

    match result {
        RunResult::Completed(Ok(InstallExecutionOutcome::Completed(
            instance_id,
        ))) => {
            if let Some(instance_id) = instance_id.as_ref() {
                set_instance_id(&mut job_state, instance_id.clone());
            }
            if cancellation.is_cancelled() {
                finish_canceled_job(job_id, &mut job_state, &state).await?;
                return Ok(());
            }
            job_state.record_event(InstallJobEventKind::JobSucceeded {
                instance_id: current_instance_id(&job_state),
            });
            job_state.progress.phase = if matches!(
                job_state.request,
                InstallRequest::UpgradeUnmanagedInstance { .. }
            ) {
                InstallPhaseId::Completed
            } else {
                InstallPhaseId::Finalizing
            };
            job_state.progress.progress = None;
            job_state.progress.details = InstallPhaseDetails::Empty;
            job_state.progress.parallel = None;
            job_state.error = None;
            job_state.rollback_error = None;
            job_state.pause_reason = None;
            job_state.continuation = None;
            job_state.missing_content = None;
            job_state.skipped_missing_content_paths.clear();
            job_state.context = None;
            let mut completed_state = job_state.clone();
            completed_state.rollback = None;
            let Some(record) =
                store::complete_running_job(job_id, &completed_state, &state)
                    .await?
            else {
                if store::get_required(job_id, &state).await?.status
                    == InstallJobStatus::Canceling
                {
                    finish_canceled_job(job_id, &mut job_state, &state).await?;
                }
                return Ok(());
            };
            if let Some(instance_id) = instance_id.as_ref()
                && let Err(error) =
                    emit_instance(instance_id, InstancePayloadType::Edited)
                        .await
            {
                tracing::warn!(
                    job_id = %job_id,
                    instance_id,
                    error = %error,
                    "Install job succeeded, but its final instance event could not be emitted"
                );
            }
            if let Err(error) =
                recovery::discard_content_rollback(&mut job_state, &state).await
            {
                tracing::warn!(
                    job_id = %job_id,
                    error = %error,
                    "Install job succeeded, but rollback staging could not be discarded"
                );
            }
            if let Err(error) = emit_install_job(&record.snapshot()).await {
                tracing::warn!(
                    job_id = %job_id,
                    error = %error,
                    "Install job succeeded, but its final event could not be emitted"
                );
            }
        }
        RunResult::Completed(Ok(InstallExecutionOutcome::WaitingForUser(
            reason,
        ))) => {
            let mut waiting_state = job_state.clone();
            begin_waiting_for_user(&mut waiting_state, reason);
            let Some(record) = store::update_status_if(
                job_id,
                InstallJobStatus::Running,
                InstallJobStatus::WaitingForUser,
                &waiting_state,
                &state,
            )
            .await?
            else {
                if store::get_required(job_id, &state).await?.status
                    == InstallJobStatus::Canceling
                {
                    finish_canceled_job(job_id, &mut job_state, &state).await?;
                }
                return Ok(());
            };
            emit_install_job(&record.snapshot()).await?;
        }
        RunResult::Canceled => {
            finish_canceled_job(job_id, &mut job_state, &state).await?;
        }
        RunResult::Completed(Err(error)) => {
            job_state.progress.phase = failure_phase;
            begin_failed_job_rollback(&mut job_state, &error);
            let cleanup_succeeded = match recovery::apply_cleanup(
                &mut job_state,
                &state,
            )
            .await
            {
                Err(rollback_error) => {
                    tracing::error!(
                        "Error rolling back failed install job {job_id}: {rollback_error}"
                    );
                    job_state.rollback_error = Some(install_error_view(
                        InstallPhaseId::RollingBack,
                        &rollback_error,
                        None,
                    ));
                    job_state.record_event(
                        InstallJobEventKind::RollbackFailed {
                            message: rollback_error.to_string(),
                        },
                    );
                    false
                }
                Ok(()) => true,
            };
            recovery::finalize_rollback_state(
                &mut job_state,
                cleanup_succeeded,
            );
            if cleanup_succeeded {
                clear_deleted_new_instance_id(&mut job_state);
            }
            let record = store::update_status(
                job_id,
                InstallJobStatus::Failed,
                &job_state,
                &state,
            )
            .await?;
            emit_install_job(&record.snapshot()).await?;
            return Err(error);
        }
    }

    Ok(())
}

async fn finish_canceled_job(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    state: &State,
) -> crate::Result<()> {
    let canceled_phase = job_state.progress.phase;
    job_state.error = Some(InstallErrorView::from_message(
        "canceled",
        canceled_phase,
        "Install was canceled",
    ));
    job_state.pause_reason = None;
    job_state.record_event(InstallJobEventKind::JobCanceled {
        phase: canceled_phase,
    });
    job_state.progress.phase = InstallPhaseId::RollingBack;
    job_state.progress.progress = None;
    job_state.progress.details = InstallPhaseDetails::Empty;
    job_state.record_event(InstallJobEventKind::RollbackStarted {
        cleanup: job_state.cleanup.clone(),
    });
    let cleanup_succeeded =
        match recovery::apply_cleanup(job_state, state).await {
            Err(rollback_error) => {
                job_state.rollback_error = Some(install_error_view(
                    InstallPhaseId::RollingBack,
                    &rollback_error,
                    None,
                ));
                job_state.record_event(InstallJobEventKind::RollbackFailed {
                    message: rollback_error.to_string(),
                });
                false
            }
            Ok(()) => true,
        };
    recovery::finalize_rollback_state(job_state, cleanup_succeeded);
    if cleanup_succeeded {
        clear_deleted_new_instance_id(job_state);
    }
    let record = store::update_status(
        job_id,
        InstallJobStatus::Canceled,
        job_state,
        state,
    )
    .await?;
    emit_install_job(&record.snapshot()).await
}

async fn run_request(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    state: &State,
) -> crate::Result<InstallExecutionOutcome<Option<String>>> {
    match job_state.request.clone() {
        InstallRequest::CreateInstance {
            name,
            game_version,
            loader,
            loader_version: _,
            adjuncts,
            icon_path: _,
            link,
            game_dir_override: _,
        } => {
            let Some(instance_id) = current_instance_id(job_state) else {
                return Err(crate::ErrorKind::InputError(
                    "Install job is missing its instance id".to_string(),
                )
                .into());
            };
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::PreparingInstance,
                InstallPhaseDetails::Instance { name: name.clone() },
            )
            .await?;
            let reporter =
                InstallProgressReporter::new(job_id, job_state.clone());
            if let InstanceLink::CurseForgeModpack {
                project_id,
                version_id,
            } = link
            {
                let project_id = project_id.parse::<u32>().map_err(|_| {
                    ErrorKind::InputError(
                        "CurseForge project ID is invalid".to_string(),
                    )
                })?;
                let file_id = version_id.parse::<u32>().map_err(|_| {
                    ErrorKind::InputError(
                        "CurseForge file ID is invalid".to_string(),
                    )
                })?;
                crate::state::instances::commands::set_instance_install_stage(
                    &instance_id,
                    InstanceInstallStage::PackInstalling,
                    &state.pool,
                )
                .await?;
                emit_instance(&instance_id, InstancePayloadType::Edited)
                    .await?;
                let result = crate::api::curseforge::install_modpack_with_reporter(
                    crate::api::curseforge::CurseForgeModpackInstallRequest {
                        instance_id: instance_id.clone(),
                        project_id,
                        file_id,
                        install_optional: false,
                        allow_target_change: false,
                    },
                    Some(reporter.clone()),
                )
                .await?;
                if let Some(reason) = curseforge_manual_download_pause(
                    &result,
                    &job_state.skipped_missing_content_paths,
                ) {
                    return Ok(InstallExecutionOutcome::WaitingForUser(reason));
                }
            }
            reporter
                .update(
                    InstallPhaseId::DownloadingMinecraft,
                    None,
                    InstallPhaseDetails::Minecraft {
                        game_version: game_version.clone(),
                        loader,
                    },
                )
                .await?;
            let context =
                crate::state::instances::commands::get_instance_launch_context(
                    &instance_id,
                    &state.pool,
                )
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError("Unknown instance".to_string())
                })?;
            crate::launcher::install_minecraft_with_reporter(
                &context,
                false,
                Some(reporter.clone()),
                crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
            )
            .await?;
            install_adjunct_components(
                state,
                &instance_id,
                &adjuncts,
                &game_version,
                loader,
                reporter.cancellation_token(),
            )
            .await?;
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::CreateModpackInstance {
            location,
            post_install_edit,
        } => {
            let Some(instance_id) = current_instance_id(job_state) else {
                return Err(crate::ErrorKind::InputError(
                    "Install job is missing its instance id".to_string(),
                )
                .into());
            };
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::ResolvingPack,
                modpack_details(&location),
            )
            .await?;
            if let InstallExecutionOutcome::WaitingForUser(reason) =
                install_pack(
                    job_id,
                    job_state,
                    location,
                    instance_id.clone(),
                    DownloadReason::Modpack,
                )
                .await?
            {
                return Ok(InstallExecutionOutcome::WaitingForUser(reason));
            }
            apply_post_install_edit(&instance_id, post_install_edit).await?;
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::ImportInstance {
            launcher_type,
            base_path,
            instance_folder,
            instance_path,
            symlink,
            game_version,
            loader,
            loader_version,
            game_dir_override: _,
        } => {
            tracing::debug!(
                "InstallRequest::ImportInstance: launcher_type={launcher_type} base_path={} instance_folder={instance_folder} symlink={symlink}",
                base_path.display()
            );
            let Some(instance_id) = current_instance_id(job_state) else {
                return Err(crate::ErrorKind::InputError(
                    "Install job is missing its instance id".to_string(),
                )
                .into());
            };
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::PreparingInstance,
                InstallPhaseDetails::Import {
                    launcher_type,
                    instance_folder: instance_folder.clone(),
                },
            )
            .await?;
            crate::api::pack::import::import_instance_with_reporter(
                &instance_id,
                launcher_type,
                base_path,
                instance_folder,
                instance_path,
                crate::api::pack::import::ImportOverrides {
                    game_version,
                    loader,
                    loader_version,
                },
                // TODO(B2): apply overrides to launcher-specific importers
                // (MultiMC/Prism/ATLauncher/GDLauncher/Curseforge/ModrinthApp);
                // generic/PCL/HMCL/Axolotl paths already consume them.
                InstallProgressReporter::new(job_id, job_state.clone()),
                symlink,
            )
            .await?;
            let context =
                crate::state::instances::commands::get_instance_launch_context(
                    &instance_id,
                    &state.pool,
                )
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError("Unknown instance".to_string())
                })?;
            crate::launcher::install_minecraft_with_reporter(
                &context,
                false,
                Some(InstallProgressReporter::new(job_id, job_state.clone())),
                crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
            )
            .await?;
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::DuplicateInstance { source_instance_id } => {
            let Some(instance_id) = current_instance_id(job_state) else {
                return Err(crate::ErrorKind::InputError(
                    "Install job is missing its instance id".to_string(),
                )
                .into());
            };
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::PreparingInstance,
                InstallPhaseDetails::Empty,
            )
            .await?;
            let state = State::get().await?;
            crate::api::pack::import::copy_dotminecraft_with_reporter(
                &instance_id,
                crate::api::instance::get_full_path(&source_instance_id)
                    .await?,
                &state.io_semaphore,
                InstallProgressReporter::new(job_id, job_state.clone()),
                InstallPhaseDetails::Empty,
            )
            .await?;
            let context =
                crate::state::instances::commands::get_instance_launch_context(
                    &instance_id,
                    &state.pool,
                )
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError("Unknown instance".to_string())
                })?;
            crate::launcher::install_minecraft_with_reporter(
                &context,
                false,
                Some(InstallProgressReporter::new(job_id, job_state.clone())),
                crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
            )
            .await?;
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::UpgradeUnmanagedInstance {
            instance_id: source_instance_id,
            plan_id,
            execution,
            create_full_backup,
            shared_upgrade_mode,
            display_names,
        } => {
            let target_instance_id = current_instance_id(job_state)
                .ok_or_else(|| {
                    crate::ErrorKind::InputError(
                        "Upgrade job is missing its target instance id"
                            .to_string(),
                    )
                })?;
            run_instance_upgrade(
                job_id,
                job_state,
                state,
                &source_instance_id,
                &target_instance_id,
                &plan_id,
                execution,
                create_full_backup,
                shared_upgrade_mode,
                display_names,
            )
            .await?;
            Ok(InstallExecutionOutcome::Completed(Some(target_instance_id)))
        }
        InstallRequest::InstallExistingInstance { instance_id, force } => {
            prepare_existing_rollback(job_state, state, &instance_id).await?;
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingMinecraft,
                InstallPhaseDetails::Empty,
            )
            .await?;
            let context =
                crate::state::instances::commands::get_instance_launch_context(
                    &instance_id,
                    &state.pool,
                )
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError("Unknown instance".to_string())
                })?;
            crate::launcher::install_minecraft_with_reporter(
                &context,
                force,
                Some(InstallProgressReporter::new(job_id, job_state.clone())),
                crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
            )
            .await?;
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::InstallPackToExistingInstance {
            instance_id,
            location,
            post_install_edit,
        } => {
            prepare_existing_rollback(job_state, state, &instance_id).await?;
            let disabled_project_ids = match job_state.continuation.clone() {
                Some(InstallContinuationState::InstallingPackToExistingInstance {
                    disabled_project_ids,
                }) => disabled_project_ids.into_iter().collect(),
                None => {
                    let disabled_project_ids = remove_existing_pack_content(
                        job_id,
                        job_state,
                        state,
                        &instance_id,
                    )
                    .await?;
                    let mut persisted_ids = disabled_project_ids
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>();
                    persisted_ids.sort_unstable();
                    let continuation = InstallContinuationState::InstallingPackToExistingInstance {
                        disabled_project_ids: persisted_ids,
                    };
                    job_state.continuation = Some(continuation.clone());
                    InstallProgressReporter::new(job_id, job_state.clone())
                        .set_continuation(Some(continuation))
                        .await?;
                    disabled_project_ids
                }
            };
            if let InstallExecutionOutcome::WaitingForUser(reason) =
                install_pack(
                    job_id,
                    job_state,
                    location,
                    instance_id.clone(),
                    DownloadReason::Modpack,
                )
                .await?
            {
                return Ok(InstallExecutionOutcome::WaitingForUser(reason));
            }
            restore_disabled_projects(
                &instance_id,
                disabled_project_ids,
                state,
            )
            .await?;
            job_state.continuation = None;
            InstallProgressReporter::new(job_id, job_state.clone())
                .set_continuation(None)
                .await?;
            apply_post_install_edit(&instance_id, post_install_edit).await?;
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::InstallContent {
            instance_id,
            project_id,
            version_id,
            content_type,
            selected,
            excluded_project_ids,
            display_title: _,
            display_icon: _,
        } => {
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingContent,
                InstallPhaseDetails::Empty,
            )
            .await?;
            let plan = crate::state::instances::commands::resolve_install_plan(
                &instance_id,
                crate::state::instances::commands::InstanceInstallProjectRequest {
                    project_id: project_id.clone(),
                    version_id,
                    content_type,
                    selected,
                    excluded_project_ids,
                    force_project_ids: Vec::new(),
                },
                state,
            )
            .await?;
            let total = (plan.dependencies.len() + 1) as u64;
            let reporter =
                InstallProgressReporter::new(job_id, job_state.clone());
            reporter
                .update(
                    InstallPhaseId::DownloadingContent,
                    Some(InstallProgress {
                        current: 0,
                        total,
                        secondary: None,
                    }),
                    InstallPhaseDetails::Empty,
                )
                .await?;
            crate::state::instances::commands::install_resolved_content_plan_with_reporter(
                &instance_id,
                &plan,
                Some(reporter.clone()),
                state,
            )
            .await?;
            reporter
                .update(
                    InstallPhaseId::DownloadingContent,
                    Some(InstallProgress {
                        current: total,
                        total,
                        secondary: None,
                    }),
                    InstallPhaseDetails::Empty,
                )
                .await?;
            crate::api::instance::emit_content_changed(&instance_id).await?;
            let dependency_project_ids = plan
                .dependencies
                .iter()
                .map(|dependency| dependency.project_id.clone())
                .collect::<Vec<_>>();
            emit_instance(
                &instance_id,
                InstancePayloadType::ContentInstallFinished {
                    project_ids: std::iter::once(project_id.clone())
                        .chain(dependency_project_ids.iter().cloned())
                        .collect(),
                    dependency_project_ids,
                },
            )
            .await?;
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::InstallCurseForgeContent {
            request,
            display_title: _,
            display_icon: _,
        } => {
            let instance_id = request.instance_id.clone();
            let primary_project_id = request.project_id;
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingContent,
                InstallPhaseDetails::Empty,
            )
            .await?;
            let reporter =
                InstallProgressReporter::new(job_id, job_state.clone());
            let result = crate::api::curseforge::install_file_with_reporter(
                request, reporter,
            )
            .await?;
            crate::api::instance::emit_content_changed(&instance_id).await?;
            let dependency_project_ids = result
                .installed
                .iter()
                .filter(|installed| installed.dependency)
                .map(|installed| format!("curseforge:{}", installed.project_id))
                .collect::<Vec<_>>();
            emit_instance(
                &instance_id,
                InstancePayloadType::ContentInstallFinished {
                    project_ids: std::iter::once(format!(
                        "curseforge:{primary_project_id}"
                    ))
                    .chain(dependency_project_ids.iter().cloned())
                    .collect(),
                    dependency_project_ids,
                },
            )
            .await?;
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::InstallCurseForgeWorld {
            request,
            display_title: _,
            display_icon: _,
        } => {
            let instance_id = request.instance_id.clone();
            if curseforge_world_was_imported_manually(job_state, &request) {
                return Ok(InstallExecutionOutcome::Completed(Some(
                    instance_id,
                )));
            }
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingContent,
                InstallPhaseDetails::Empty,
            )
            .await?;
            let reporter =
                InstallProgressReporter::new(job_id, job_state.clone());
            let result = crate::api::curseforge::install_world_with_reporter(
                request.clone(),
                reporter.clone(),
            )
            .await?;
            if let Some(manual_download) = result.manual_download {
                let path = format!("saves/{}", manual_download.file_name);
                let manual_url = manual_download.website_url.clone().or_else(|| {
					Some(format!(
						"https://www.curseforge.com/minecraft/worlds/{}/download/{}",
						manual_download.project_slug, manual_download.file_id
					))
				});
                reporter
                    .record_events(vec![
                        InstallJobEventKind::ContentFileSkipped {
                            path: path.clone(),
                            reason: "CurseForge requires a manual download"
                                .to_string(),
                            project_id: Some(
                                manual_download.project_id.to_string(),
                            ),
                            version_id: Some(
                                manual_download.file_id.to_string(),
                            ),
                            manual_url,
                        },
                    ])
                    .await?;
                return Ok(InstallExecutionOutcome::WaitingForUser(
                    InstallPauseReason::MissingRequiredContent {
                        failed_files: 1,
                        paths: vec![path],
                    },
                ));
            }
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::UpdateManagedCurseForgeModpack {
            instance_id,
            file_id,
        } => {
            prepare_existing_rollback(job_state, state, &instance_id).await?;
            crate::state::instances::commands::set_instance_install_stage(
                &instance_id,
                InstanceInstallStage::PackInstalling,
                &state.pool,
            )
            .await?;
            emit_instance(&instance_id, InstancePayloadType::Edited).await?;
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingContent,
                InstallPhaseDetails::Empty,
            )
            .await?;
            let reporter =
                InstallProgressReporter::new(job_id, job_state.clone());
            let result =
                crate::api::curseforge::update_managed_modpack_with_reporter(
                    &instance_id,
                    file_id,
                    Some(reporter.clone()),
                )
                .await?;
            if !result.content.failed_downloads.is_empty() {
                return Err(ErrorKind::NetworkError(format!(
                    "{} CurseForge files could not be downloaded automatically",
                    result.content.failed_downloads.len()
                ))
                .into());
            }
            if let Some(reason) = curseforge_manual_download_pause(
                &result,
                &job_state.skipped_missing_content_paths,
            ) {
                return Ok(InstallExecutionOutcome::WaitingForUser(reason));
            }
            Ok(InstallExecutionOutcome::Completed(Some(instance_id)))
        }
        InstallRequest::DownloadJava { vendor, version } => {
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::PreparingJava,
                InstallPhaseDetails::Java {
                    major_version: version,
                    step: InstallJavaStep::FetchingMetadata,
                },
            )
            .await?;
            let reporter =
                InstallProgressReporter::new(job_id, job_state.clone());
            let path = crate::api::jre::download_java_from_feed_with_reporter(
                &vendor, version, reporter,
            )
            .await?;
            let _ = path;
            Ok(InstallExecutionOutcome::Completed(None))
        }
    }
}

async fn apply_post_install_edit(
    instance_id: &str,
    edit: Option<InstallPostInstallEdit>,
) -> crate::Result<()> {
    let Some(edit) = edit else {
        return Ok(());
    };

    if edit.name.is_none() && edit.icon_path.is_none() && edit.link.is_none() {
        return Ok(());
    }

    crate::api::instance::edit(
        instance_id,
        crate::state::instances::commands::EditInstance {
            name: edit.name,
            icon_path: edit.icon_path,
            link: edit.link,
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

enum StagedUpgradeDownload {
    Modrinth(crate::state::instances::commands::DownloadedProjectVersion),
    CurseForge(crate::api::curseforge::StagedCurseForgeUpgrade),
}

struct StagedUpgradeMutation {
    existing_path: Option<String>,
    target_path: String,
    ownership: crate::state::instances::ContentOwnershipKind,
    auto_dependency: bool,
    enabled: bool,
    download: StagedUpgradeDownload,
}

struct UpgradeStagingRequest {
    index: usize,
    provider: ContentProvider,
    project_id: String,
    release_id: String,
    auto_dependency: bool,
    enabled: bool,
    existing_path: Option<String>,
    ownership: crate::state::instances::ContentOwnershipKind,
    project_type: Option<crate::state::ProjectType>,
}

struct AppliedUpgradeContent {
    skipped: Vec<String>,
    launcher_expected_files:
        HashMap<String, Option<crate::state::InstanceUpgradeSourceFile>>,
}

#[allow(clippy::too_many_arguments)]
async fn run_instance_upgrade(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    state: &State,
    source_instance_id: &str,
    target_instance_id: &str,
    plan_id: &str,
    execution: InstanceUpgradeExecution,
    create_full_backup: bool,
    shared_upgrade_mode: SharedUpgradeMode,
    display_names: InstanceUpgradeDisplayNames,
) -> crate::Result<()> {
    let compatibility_warning_details =
        upgrade_compatibility_warning_details(&execution);
    let (upgrade_source_files, upgrade_watch) = if shared_upgrade_mode
        == SharedUpgradeMode::CopyAndUpgrade
    {
        update_progress(
            job_id,
            job_state,
            state,
            InstallPhaseId::CreatingBackup,
            InstallPhaseDetails::Empty,
        )
        .await?;
        copy_physical_instance_contents(
            job_id,
            job_state,
            state,
            source_instance_id,
            target_instance_id,
        )
        .await?;
        let files = crate::state::instances::commands::scan_instance_upgrade_source_files(
                target_instance_id,
                state,
            )
            .await?;
        let watch = state
            .file_watcher
            .track_upgrade_source(
                target_instance_id,
                files.iter().map(|file| file.relative_path.clone()),
            )
            .await
            .map(|snapshot| InstanceUpgradeWatchBaseline {
                epoch: snapshot.epoch,
                generation: snapshot.generation,
                dirty_paths: snapshot.dirty_paths.into_iter().collect(),
            });
        (files, watch)
    } else {
        (
            execution.source_files.clone(),
            execution.source_watch.clone(),
        )
    };

    let backup_instance_id = if should_create_upgrade_backup(
        create_full_backup,
        shared_upgrade_mode,
    ) {
        update_progress(
            job_id,
            job_state,
            state,
            InstallPhaseId::CreatingBackup,
            InstallPhaseDetails::Empty,
        )
        .await?;
        Some(
            create_upgrade_backup(
                job_id,
                job_state,
                state,
                source_instance_id,
                display_names.backup.as_deref(),
            )
            .await?,
        )
    } else {
        None
    };
    InstallProgressReporter::new(job_id, job_state.clone())
        .set_upgrade_result(InstanceUpgradeResult {
            plan_id: plan_id.to_string(),
            source_instance_id: source_instance_id.to_string(),
            target_instance_id: target_instance_id.to_string(),
            backup_instance_id: backup_instance_id.clone(),
            source_environment: Some(execution.source_environment.clone()),
            target_environment: Some(execution.target_environment.clone()),
            solution: execution.solution.clone(),
            compatibility_warnings: execution.warnings.clone(),
            compatibility_warning_details: compatibility_warning_details
                .clone(),
            external_changes: Vec::new(),
            skipped_due_to_external_conflict: Vec::new(),
        })
        .await?;

    update_progress(
        job_id,
        job_state,
        state,
        InstallPhaseId::StagingContent,
        InstallPhaseDetails::Empty,
    )
    .await?;
    update_progress(
        job_id,
        job_state,
        state,
        InstallPhaseId::DownloadingContent,
        InstallPhaseDetails::Empty,
    )
    .await?;
    let staged = stage_upgrade_content(
        target_instance_id,
        &execution,
        Some(InstallProgressReporter::new(job_id, job_state.clone())),
        state,
    )
    .await?;

    let mut external_changes = collect_upgrade_external_changes(
        target_instance_id,
        &upgrade_source_files,
        upgrade_watch.as_ref(),
        state,
    )
    .await?;
    for relative_path in
        collect_unsafe_upgrade_paths(target_instance_id).await?
    {
        if !external_changes
            .iter()
            .any(|change| change.relative_path == relative_path)
        {
            external_changes.push(InstanceUpgradeExternalChange {
                relative_path,
                kind: InstanceUpgradeExternalChangeKind::Modified,
            });
        }
    }
    let mut external_paths = external_changes
        .iter()
        .map(|change| change.relative_path.clone())
        .collect::<HashSet<_>>();
    for mutation in &staged {
        if let Some(path) = mutation.existing_path.as_deref()
            && source_file_changed(
                path,
                &upgrade_source_files,
                target_instance_id,
            )
            .await?
            && external_paths.insert(path.to_string())
        {
            external_changes.push(InstanceUpgradeExternalChange {
                relative_path: path.to_string(),
                kind: InstanceUpgradeExternalChangeKind::Modified,
            });
        }
    }
    if shared_upgrade_mode == SharedUpgradeMode::Direct {
        let replacement_paths = staged
            .iter()
            .map(|mutation| mutation.target_path.clone())
            .collect::<Vec<_>>();
        recovery::prepare_existing_upgrade_content_rollback(
            job_id,
            job_state,
            state,
            replacement_paths,
        )
        .await?;
        recovery::restore_upgrade_db_baseline(job_state, state).await?;
    }

    update_progress(
        job_id,
        job_state,
        state,
        InstallPhaseId::ApplyingContent,
        InstallPhaseDetails::Empty,
    )
    .await?;
    let applied = apply_upgrade_content(
        target_instance_id,
        staged,
        &execution,
        &external_paths,
        &upgrade_source_files,
        state,
    )
    .await?;
    if !applied.skipped.is_empty() {
        InstallProgressReporter::new(job_id, job_state.clone())
            .record_events(
                applied
                    .skipped
                    .iter()
                    .map(|relative_path| {
                        InstallJobEventKind::UpgradeItemSkipped {
                            relative_path: relative_path.clone(),
                            reason: "external_conflict".to_string(),
                        }
                    })
                    .collect(),
            )
            .await?;
    }

    update_progress(
        job_id,
        job_state,
        state,
        InstallPhaseId::UpdatingLoader,
        InstallPhaseDetails::Minecraft {
            game_version: execution.target_environment.game_version.clone(),
            loader: execution.target_environment.mod_loader,
        },
    )
    .await?;
    let upgraded_target_name = display_names
        .should_auto_rename
        .then(|| default_upgrade_instance_name(&execution.target_environment))
        .or(display_names.upgraded_target.clone());
    crate::state::edit_instance(
        target_instance_id,
        crate::state::EditInstance {
            name: upgraded_target_name,
            content_set_patch: Some(crate::state::AppliedContentSetPatch {
                source_kind: Some(
                    crate::state::instances::ContentSourceKind::Local,
                ),
                game_version: Some(
                    execution.target_environment.game_version.clone(),
                ),
                loader: Some(execution.target_environment.mod_loader),
                loader_version: Some(
                    execution.target_environment.mod_loader_version.clone(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        },
        &state.pool,
    )
    .await?;
    update_progress(
        job_id,
        job_state,
        state,
        InstallPhaseId::DownloadingMinecraft,
        InstallPhaseDetails::Empty,
    )
    .await?;
    let context =
        crate::state::instances::commands::get_instance_launch_context(
            target_instance_id,
            &state.pool,
        )
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown upgrade target".to_string())
        })?;
    crate::launcher::install_minecraft_with_reporter(
        &context,
        false,
        Some(InstallProgressReporter::new(job_id, job_state.clone())),
        crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
    )
    .await?;

    update_progress(
        job_id,
        job_state,
        state,
        InstallPhaseId::Verifying,
        InstallPhaseDetails::Empty,
    )
    .await?;
    crate::state::instances::commands::get_content_snapshot(
        target_instance_id,
        true,
        state,
    )
    .await?;
    let final_files =
        crate::state::instances::commands::scan_instance_upgrade_source_files(
            target_instance_id,
            state,
        )
        .await?;
    merge_upgrade_external_changes(
        &mut external_changes,
        final_upgrade_external_changes(
            &upgrade_source_files,
            &final_files,
            &applied.launcher_expected_files,
        ),
    );
    if !external_changes.is_empty() {
        InstallProgressReporter::new(job_id, job_state.clone())
            .record_events(
                external_changes
                    .iter()
                    .map(|change| InstallJobEventKind::UpgradeExternalChange {
                        relative_path: change.relative_path.clone(),
                        kind: change.kind,
                    })
                    .collect(),
            )
            .await?;
    }
    InstallProgressReporter::new(job_id, job_state.clone())
        .set_upgrade_result(InstanceUpgradeResult {
            plan_id: plan_id.to_string(),
            source_instance_id: source_instance_id.to_string(),
            target_instance_id: target_instance_id.to_string(),
            backup_instance_id,
            source_environment: Some(execution.source_environment),
            target_environment: Some(execution.target_environment),
            solution: execution.solution,
            compatibility_warnings: execution.warnings,
            compatibility_warning_details: compatibility_warning_details
                .clone(),
            external_changes,
            skipped_due_to_external_conflict: applied.skipped,
        })
        .await?;
    Ok(())
}

fn default_upgrade_instance_name(
    environment: &crate::state::InstanceUpgradeEnvironment,
) -> String {
    let loader = match environment.mod_loader {
        ModLoader::Vanilla => "Vanilla",
        ModLoader::Forge => "Forge",
        ModLoader::Fabric => "Fabric",
        ModLoader::Quilt => "Quilt",
        ModLoader::NeoForge => "NeoForge",
        ModLoader::OptiFine => "OptiFine",
        ModLoader::Cleanroom => "Cleanroom",
        ModLoader::LiteLoader => "LiteLoader",
        ModLoader::LegacyFabric => "Legacy Fabric",
        ModLoader::Babric => "Babric",
    };
    let loader_version = environment
        .mod_loader_version
        .as_deref()
        .filter(|version| !matches!(*version, "latest" | "stable"))
        .map(|version| format!(" {version}"))
        .unwrap_or_default();
    format!("{}-{loader}{loader_version}", environment.game_version)
}

fn upgrade_compatibility_warning_details(
    execution: &InstanceUpgradeExecution,
) -> Vec<InstanceUpgradeCompatibilityWarning> {
    let physical_details = execution
        .items
        .iter()
        .filter_map(|item| {
            let code = physical_upgrade_warning_code(execution, item)?;
            execution
                .warnings
                .iter()
                .any(|warning| warning.code == code)
                .then(|| InstanceUpgradeCompatibilityWarning {
                    code,
                    relative_path: Some(item.relative_path.clone()),
                    content_id: Some(item.content_id.clone()),
                    provider: item.provider,
                    project_id: item.project_id.clone(),
                    conflicting_project_id: None,
                })
        })
        .collect::<Vec<_>>();
    let mut details = execution
        .warnings
        .iter()
        .filter_map(|warning| {
            let item = warning
                .content_id
                .as_ref()
                .and_then(|content_id| {
                    execution
                        .items
                        .iter()
                        .find(|item| item.content_id == *content_id)
                })
                .or_else(|| {
                    let mut matches = execution.items.iter().filter(|item| {
                        item.provider == warning.provider
                            && item.project_id == warning.project_id
                    });
                    let item = matches.next()?;
                    matches.next().is_none().then_some(item)
                });
            if item.is_none()
                && physical_details
                    .iter()
                    .any(|detail| detail.code == warning.code)
            {
                return None;
            }
            Some(InstanceUpgradeCompatibilityWarning {
                code: warning.code,
                relative_path: item.map(|item| item.relative_path.clone()),
                content_id: item
                    .map(|item| item.content_id.clone())
                    .or_else(|| warning.content_id.clone()),
                provider: warning.provider,
                project_id: warning.project_id.clone(),
                conflicting_project_id: warning.conflicting_project_id.clone(),
            })
        })
        .collect::<Vec<_>>();
    for detail in physical_details {
        if !details.iter().any(|existing| {
            existing.code == detail.code
                && existing.content_id == detail.content_id
                && existing.relative_path == detail.relative_path
        }) {
            details.push(detail);
        }
    }
    details
}

fn physical_upgrade_warning_code(
    execution: &InstanceUpgradeExecution,
    item: &crate::state::InstanceUpgradeItem,
) -> Option<crate::state::InstanceUpgradeIssueCode> {
    use crate::state::{
        InstanceUpgradeAction, InstanceUpgradeIssueCode,
        InstanceUpgradeItemStatus,
    };

    let action = execution
        .solution
        .selections
        .iter()
        .find(|selection| selection.content_id == item.content_id)
        .map(|selection| selection.action)
        .unwrap_or(item.resolution.action);
    match item.status {
        InstanceUpgradeItemStatus::Unidentified => {
            Some(InstanceUpgradeIssueCode::Unidentified)
        }
        InstanceUpgradeItemStatus::UnsupportedContentType => {
            Some(InstanceUpgradeIssueCode::UnsupportedContentType)
        }
        InstanceUpgradeItemStatus::NoCompatibleRelease
        | InstanceUpgradeItemStatus::UpgradeAvailable
            if action == InstanceUpgradeAction::Keep =>
        {
            Some(InstanceUpgradeIssueCode::KeepIncompatible)
        }
        InstanceUpgradeItemStatus::NoCompatibleShaderRuntime
            if action == InstanceUpgradeAction::Keep =>
        {
            Some(InstanceUpgradeIssueCode::NoCompatibleShaderRuntime)
        }
        InstanceUpgradeItemStatus::ShaderRuntimeMissing
            if action == InstanceUpgradeAction::Keep =>
        {
            Some(InstanceUpgradeIssueCode::ShaderRuntimeMissing)
        }
        InstanceUpgradeItemStatus::ShaderRuntimeUnknown
            if action == InstanceUpgradeAction::Keep =>
        {
            Some(InstanceUpgradeIssueCode::ShaderRuntimeUnknown)
        }
        _ => None,
    }
}

async fn copy_physical_instance_contents(
    job_id: Uuid,
    job_state: &InstallJobState,
    state: &State,
    source_instance_id: &str,
    target_instance_id: &str,
) -> crate::Result<()> {
    let source_path = crate::util::io::canonicalize(
        &crate::api::instance::get_full_path(source_instance_id).await?,
    )?;
    crate::api::pack::import::copy_dotminecraft_with_reporter(
        target_instance_id,
        source_path,
        &state.io_semaphore,
        InstallProgressReporter::new(job_id, job_state.clone()),
        InstallPhaseDetails::Empty,
    )
    .await?;
    crate::state::sync_content_files(target_instance_id, state).await?;
    Ok(())
}

async fn create_upgrade_backup(
    job_id: Uuid,
    job_state: &InstallJobState,
    state: &State,
    source_instance_id: &str,
    backup_name: Option<&str>,
) -> crate::Result<String> {
    let source = crate::state::get_instance(source_instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown upgrade source".to_string())
        })?;
    let backup = crate::api::instance::create(
        backup_name.map(str::to_string).unwrap_or_else(|| {
            format!("{} (Upgrade Backup)", source.instance.name)
        }),
        source.applied_content_set.game_version.clone(),
        source.applied_content_set.loader,
        source.applied_content_set.loader_version.clone(),
        source.instance.icon_path.clone(),
        InstanceLink::Unmanaged,
        None,
        None,
    )
    .await?;
    let backup_result = async {
        clone_instance_loader_components(
            &source.loader_components,
            &backup.instance.id,
            state,
        )
        .await?;
        copy_physical_instance_contents(
            job_id,
            job_state,
            state,
            source_instance_id,
            &backup.instance.id,
        )
        .await?;
        clone_upgrade_backup_content_metadata(
            source_instance_id,
            &source.applied_content_set.id,
            &backup.instance.id,
            &backup.applied_content_set.id,
            state,
        )
        .await?;
        crate::state::instances::commands::set_instance_install_stage(
            &backup.instance.id,
            InstanceInstallStage::Installed,
            &state.pool,
        )
        .await
    }
    .await;
    if let Err(error) = backup_result {
        return match crate::state::remove_instance(&backup.instance.id, state)
            .await
        {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(crate::ErrorKind::OtherError(format!(
                "Upgrade backup creation failed: {error}; cleanup also failed: {cleanup_error}"
            ))
            .into()),
        };
    }
    Ok(backup.instance.id)
}

async fn clone_upgrade_backup_content_metadata(
    source_instance_id: &str,
    source_content_set_id: &str,
    target_instance_id: &str,
    target_content_set_id: &str,
    state: &State,
) -> crate::Result<()> {
    use crate::state::instances::adapters::sqlite::content_rows;

    let source_files =
        content_rows::get_instance_files(source_instance_id, &state.pool)
            .await?;
    let target_files =
        content_rows::get_instance_files(target_instance_id, &state.pool)
            .await?;
    let source_entries =
        content_rows::get_content_entries(source_content_set_id, &state.pool)
            .await?;
    let source_edges = content_rows::get_content_dependency_edges(
        source_content_set_id,
        &state.pool,
    )
    .await?;
    let dependency_backfilled =
        content_rows::get_dependency_backfilled_entry_ids(
            source_content_set_id,
            &state.pool,
        )
        .await?;
    let source_paths = source_files
        .iter()
        .map(|file| (file.id.as_str(), file.relative_path.as_str()))
        .collect::<HashMap<_, _>>();
    let target_file_ids = target_files
        .iter()
        .map(|file| (file.relative_path.as_str(), file.id.as_str()))
        .collect::<HashMap<_, _>>();
    let mut provider_refs = HashMap::new();
    for entry in &source_entries {
        provider_refs.insert(
            entry.id.as_str(),
            content_rows::get_content_provider_refs_with_origin(
                &entry.id,
                &state.pool,
            )
            .await?,
        );
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "DELETE FROM instance_content_dependencies WHERE content_set_id = ?",
    )
    .bind(target_content_set_id)
    .execute(&mut *tx)
    .await?;
    let mut entry_ids = HashMap::new();
    for source_entry in &source_entries {
        let target_file_id = match source_entry.file_id.as_deref() {
            Some(source_file_id) => {
                let relative_path = source_paths.get(source_file_id).ok_or_else(
                    || {
                        crate::ErrorKind::FSError(format!(
                            "Upgrade backup source content entry {} has no file",
                            source_entry.id
                        ))
                    },
                )?;
                Some(*target_file_ids.get(relative_path).ok_or_else(|| {
                    crate::ErrorKind::FSError(format!(
                        "Upgrade backup is missing copied content file {relative_path}"
                    ))
                })?)
            }
            None => None,
        };
        let target_entry =
            content_rows::upsert_content_entry_from_parts_in_transaction(
                content_rows::UpsertContentEntry {
                    instance_id: target_instance_id,
                    content_set_id: target_content_set_id,
                    file_id: target_file_id,
                    project_type: source_entry.project_type,
                    source_kind: source_entry.source_kind,
                    ownership_kind: source_entry.ownership_kind,
                    auto_dependency: source_entry.auto_dependency,
                    server_requirement: source_entry.server_requirement,
                    client_requirement: source_entry.client_requirement,
                    enabled: source_entry.enabled,
                },
                &mut tx,
            )
            .await?;
        sqlx::query(
            "DELETE FROM instance_content_provider_refs WHERE content_entry_id = ?",
        )
        .bind(&target_entry.id)
        .execute(&mut *tx)
        .await?;
        for (provider_ref, origin) in &provider_refs[source_entry.id.as_str()] {
            content_rows::upsert_content_provider_ref_in_transaction(
                &target_entry.id,
                provider_ref,
                *origin,
                &mut tx,
            )
            .await?;
        }
        if dependency_backfilled.contains(&source_entry.id) {
            content_rows::set_content_entry_dependency_backfilled_in_transaction(
                &target_entry.id,
                &mut tx,
            )
            .await?;
        }
        entry_ids.insert(source_entry.id.as_str(), target_entry.id);
    }
    for source_edge in source_edges {
        let parent_entry_id = entry_ids
            .get(source_edge.parent_entry_id.as_str())
            .ok_or_else(|| {
                crate::ErrorKind::FSError(format!(
                    "Upgrade backup cannot map dependency parent {}",
                    source_edge.parent_entry_id
                ))
            })?;
        let child_entry_id = entry_ids
            .get(source_edge.child_entry_id.as_str())
            .ok_or_else(|| {
                crate::ErrorKind::FSError(format!(
                    "Upgrade backup cannot map dependency child {}",
                    source_edge.child_entry_id
                ))
            })?;
        let now = chrono::Utc::now();
        content_rows::upsert_content_dependency_edge_in_transaction(
            &crate::state::instances::ContentDependencyEdge {
                id: format!("content-dependency:{}", Uuid::new_v4()),
                content_set_id: target_content_set_id.to_string(),
                parent_entry_id: parent_entry_id.clone(),
                child_entry_id: child_entry_id.clone(),
                created_at: now,
                modified_at: now,
                ..source_edge
            },
            &mut tx,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn clone_instance_loader_components(
    components: &[LoaderComponent],
    target_instance_id: &str,
    state: &State,
) -> crate::Result<()> {
    let components = components
        .iter()
        .cloned()
        .map(|mut component| {
            component.instance_id = target_instance_id.to_string();
            component
        })
        .collect::<Vec<_>>();
    crate::state::instances::commands::replace_instance_loader_components(
        target_instance_id,
        &components,
        &state.pool,
    )
    .await
}

async fn stage_upgrade_content(
    instance_id: &str,
    execution: &InstanceUpgradeExecution,
    reporter: Option<InstallProgressReporter>,
    state: &State,
) -> crate::Result<Vec<StagedUpgradeMutation>> {
    let metadata = crate::state::get_instance(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown upgrade target".to_string())
        })?;
    let entries = crate::state::instances::adapters::sqlite::content_rows::get_content_entries(
        &metadata.applied_content_set.id,
        &state.pool,
    )
    .await?;
    let files = crate::state::instances::adapters::sqlite::content_rows::get_instance_files(
        instance_id,
        &state.pool,
    )
    .await?;
    let files_by_id = files
        .iter()
        .map(|file| (file.id.as_str(), file.relative_path.as_str()))
        .collect::<HashMap<_, _>>();
    let entries_by_path = entries
        .iter()
        .filter_map(|entry| {
            Some((
                files_by_id.get(entry.file_id.as_deref()?)?.to_string(),
                entry,
            ))
        })
        .collect::<HashMap<_, _>>();
    let item_paths = execution
        .items
        .iter()
        .map(|item| (item.content_id.as_str(), item.relative_path.as_str()))
        .collect::<HashMap<_, _>>();
    let mut requests = Vec::<(
        Option<String>,
        ContentProvider,
        String,
        String,
        bool,
        bool,
    )>::new();
    for selection in &execution.solution.selections {
        if selection.action != InstanceUpgradeAction::Upgrade {
            continue;
        }
        let Some(provider) = selection.provider else {
            continue;
        };
        let project_id = selection.project_id.clone().ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Upgrade selection {} has no project id",
                selection.content_id
            ))
        })?;
        let target_release_id =
            selection.target_release_id.clone().ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Upgrade selection {} has no target release",
                    selection.content_id
                ))
            })?;
        requests.push((
            Some(selection.content_id.clone()),
            provider,
            project_id,
            target_release_id,
            false,
            selection.enabled,
        ));
    }
    for change in &execution.solution.dependency_changes {
        if !matches!(
            change.kind,
            InstanceUpgradeDependencyChangeKind::Add
                | InstanceUpgradeDependencyChangeKind::Upgrade
        ) {
            continue;
        }
        requests.push((
            change.existing_content_id.clone(),
            change.provider,
            change.project_id.clone(),
            change.target_release_id.clone().ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Dependency {} has no target release",
                    change.project_id
                ))
            })?,
            true,
            change.enabled,
        ));
    }

    let mut seen = HashSet::new();
    requests.retain(|(content_id, provider, project_id, release_id, ..)| {
        seen.insert(format!(
            "{}:{}:{}:{}",
            content_id.as_deref().unwrap_or("new"),
            provider.as_str(),
            project_id,
            release_id
        ))
    });
    let contexts = requests
        .into_iter()
        .enumerate()
        .map(
            |(
                index,
                (
                    content_id,
                    provider,
                    project_id,
                    release_id,
                    auto_dependency,
                    enabled,
                ),
            )| {
                let source_path = content_id
                    .as_deref()
                    .and_then(|content_id| item_paths.get(content_id).copied());
                let existing_entry = content_id
                    .as_deref()
                    .and_then(|content_id| {
                        entries.iter().find(|entry| entry.id == content_id)
                    })
                    .or_else(|| {
                        source_path.and_then(|path| entries_by_path.get(path).copied())
                    });
                let existing_path = existing_entry
                    .and_then(|entry| entry.file_id.as_deref())
                    .and_then(|file_id| files_by_id.get(file_id).copied())
                    .map(ToString::to_string)
                    .or_else(|| source_path.map(ToString::to_string));
                let ownership = existing_entry
                    .map(|entry| entry.ownership_kind)
                    .unwrap_or(
                        crate::state::instances::ContentOwnershipKind::UserAdded,
                    );
                let project_type = existing_entry
                    .map(|entry| entry.project_type)
                    .or_else(|| {
                        source_path.and_then(
                            crate::state::instances::adapters::filesystem::project_type_from_relative_path,
                        )
                    });
                UpgradeStagingRequest {
                    index,
                    provider,
                    project_id,
                    release_id,
                    auto_dependency,
                    enabled,
                    existing_path,
                    ownership,
                    project_type,
                }
            },
        )
        .collect::<Vec<_>>();
    if let Some(reporter) = reporter.as_ref() {
        reporter
            .record_events(vec![InstallJobEventKind::ContentDownloadStarted {
                files: contexts.len() as u64,
                bytes: None,
            }])
            .await?;
    }
    let mut downloads = contexts
        .into_iter()
        .map(|context| {
            let reporter = reporter.clone();
            async move {
                let index = context.index;
                let mutation = stage_one_upgrade_request(
                    instance_id,
                    context,
                    reporter,
                    state,
                )
                .await?;
                Ok::<_, crate::Error>((index, mutation))
            }
        })
        .collect::<FuturesUnordered<_>>();
    collect_ordered_upgrade_staging(&mut downloads).await
}

async fn collect_ordered_upgrade_staging<F, T>(
    downloads: &mut FuturesUnordered<F>,
) -> crate::Result<Vec<T>>
where
    F: Future<Output = crate::Result<(usize, T)>>,
{
    let mut staged = Vec::with_capacity(downloads.len());
    while let Some(result) = downloads.next().await {
        staged.push(result?);
    }
    staged.sort_by_key(|(index, _)| *index);
    Ok(staged.into_iter().map(|(_, mutation)| mutation).collect())
}

async fn stage_one_upgrade_request(
    instance_id: &str,
    context: UpgradeStagingRequest,
    reporter: Option<InstallProgressReporter>,
    state: &State,
) -> crate::Result<StagedUpgradeMutation> {
    let download = match context.provider {
            ContentProvider::Modrinth => StagedUpgradeDownload::Modrinth(
                match reporter.as_ref() {
                    Some(reporter) => crate::state::instances::commands::download_project_version_with_reporter(
                        instance_id,
                        &context.release_id,
                        if context.auto_dependency {
                            DownloadReason::Dependency
                        } else {
                            DownloadReason::Update
                        },
                        None,
                        reporter.clone(),
                        state,
                    )
                    .await?,
                    None => crate::state::instances::commands::download_project_version(
                        instance_id,
                        &context.release_id,
                        if context.auto_dependency {
                            DownloadReason::Dependency
                        } else {
                            DownloadReason::Update
                        },
                        None,
                        state,
                    )
                    .await?,
                },
            ),
            ContentProvider::CurseForge => {
                let project_id = context.project_id.parse::<u32>().map_err(|_| {
                    crate::ErrorKind::InputError(
                        "CurseForge project id is invalid".to_string(),
                    )
                })?;
                let file_id = context.release_id.parse::<u32>().map_err(|_| {
                    crate::ErrorKind::InputError(
                        "CurseForge file id is invalid".to_string(),
                    )
                })?;
                StagedUpgradeDownload::CurseForge(
                    crate::api::curseforge::stage_curseforge_upgrade_file(
                        project_id,
                        file_id,
                        context.project_type,
                        reporter.as_ref(),
                    )
                    .await?,
                )
            }
            ContentProvider::McArchive => {
                return Err(crate::ErrorKind::InputError(
                    "MCArchive content cannot be downloaded by upgrade execution"
                        .to_string(),
                )
                .into());
            }
            ContentProvider::Local => {
                return Err(crate::ErrorKind::InputError(
                    "Local-only content cannot be downloaded by upgrade execution"
                        .to_string(),
                )
                .into());
            }
        };
    let target_path =
        context
            .existing_path
            .clone()
            .unwrap_or_else(|| match &download {
                StagedUpgradeDownload::Modrinth(download) => format!(
                    "{}/{}",
                    download.project_type.get_folder(),
                    download.file_name
                ),
                StagedUpgradeDownload::CurseForge(download) => format!(
                    "{}/{}",
                    download.project_type.get_folder(),
                    download.file.file_name
                ),
            });
    let download_size = match &download {
        StagedUpgradeDownload::Modrinth(download) => download.size,
        StagedUpgradeDownload::CurseForge(download) => {
            download.file.file_length
        }
    };
    if let Some(reporter) = reporter {
        reporter
            .record_events(vec![InstallJobEventKind::ContentFileCompleted {
                path: target_path.clone(),
                bytes: download_size,
            }])
            .await?;
    }
    Ok(StagedUpgradeMutation {
        existing_path: context.existing_path,
        target_path,
        ownership: context.ownership,
        auto_dependency: context.auto_dependency,
        enabled: context.enabled,
        download,
    })
}

async fn apply_upgrade_content(
    instance_id: &str,
    staged: Vec<StagedUpgradeMutation>,
    execution: &InstanceUpgradeExecution,
    external_paths: &HashSet<String>,
    source_files: &[crate::state::InstanceUpgradeSourceFile],
    state: &State,
) -> crate::Result<AppliedUpgradeContent> {
    let mut skipped = Vec::new();
    let mut launcher_expected_files = HashMap::new();
    #[cfg(debug_assertions)]
    let fail_after_mutations = injected_upgrade_failure_after_mutations();
    #[cfg(debug_assertions)]
    tracing::warn!(
        raw_env = ?std::env::var("AXOLOTL_TEST_UPGRADE_FAIL_AFTER_MUTATIONS"),
        parsed = ?fail_after_mutations,
        "T11 upgrade fault injection enabled"
    );
    #[cfg(not(debug_assertions))]
    tracing::warn!(
        "T11 upgrade fault injection unavailable: debug_assertions=false"
    );
    #[cfg(debug_assertions)]
    let pause_after_mutations = injected_upgrade_pause_after_mutations();
    #[cfg(debug_assertions)]
    tracing::warn!(
        raw_env = ?std::env::var("AXOLOTL_TEST_UPGRADE_PAUSE_AFTER_MUTATIONS"),
        parsed = ?pause_after_mutations,
        "T12 upgrade crash-test pause configured"
    );
    #[cfg(debug_assertions)]
    let mut completed_mutations = 0_usize;
    for mutation in staged {
        let changed_after_staging = match mutation.existing_path.as_deref() {
            Some(path) => {
                source_file_changed(path, source_files, instance_id).await?
            }
            None => false,
        };
        if upgrade_mutation_conflicts(&mutation, external_paths)
            || changed_after_staging
        {
            skipped.push(mutation.target_path);
            continue;
        }
        let relative_path = match mutation.download {
            StagedUpgradeDownload::Modrinth(download) => {
                crate::state::instances::commands::apply_downloaded_project_version_at_path(
                    instance_id,
                    &mutation.target_path,
                    download,
                    crate::state::instances::ContentSourceKind::Local,
                    mutation.ownership,
                    state,
                )
                .await?
            }
            StagedUpgradeDownload::CurseForge(download) => {
                crate::api::curseforge::apply_staged_curseforge_upgrade_file(
                    instance_id,
                    download,
                    mutation.ownership,
                    &mutation.target_path,
                )
                .await?
            }
        };
        let scope = crate::state::instances::commands::resolve_content_scope(
            instance_id,
            None,
            state,
        )
        .await?;
        let mut final_relative_path = relative_path.clone();
        if let Some(entry) = crate::state::instances::adapters::sqlite::content_rows::get_content_entry_by_relative_path(
            &scope.content_set_id,
            &relative_path,
            &state.pool,
        )
        .await?
        {
            if mutation.auto_dependency {
                crate::state::instances::adapters::sqlite::content_rows::set_content_entry_auto_dependency(
                    &entry.id,
                    true,
                    &state.pool,
                )
                .await?;
            }
            if entry.enabled != mutation.enabled {
                let toggled = crate::state::instances::commands::toggle_content_entries(
                    instance_id,
                    &[entry.id],
                    Some(mutation.enabled),
                    state,
                )
                .await?;
                if let Some(toggled) = toggled.first() {
                    final_relative_path = toggled.path.clone();
                }
            }
        }
        record_launcher_expected_file(
            instance_id,
            &relative_path,
            &final_relative_path,
            &mut launcher_expected_files,
        )
        .await?;
        #[cfg(debug_assertions)]
        {
            completed_mutations += 1;
            if fail_after_mutations == Some(completed_mutations) {
                return Err(crate::ErrorKind::InputError(format!(
                    "Injected unmanaged instance upgrade failure after {completed_mutations} mutation(s)"
                ))
                .into());
            }
        }
        #[cfg(debug_assertions)]
        if pause_after_mutations == Some(completed_mutations) {
            tracing::warn!(
                "T12 upgrade crash-test pause after {completed_mutations} mutation(s); terminate process now"
            );
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    let item_paths = execution
        .items
        .iter()
        .map(|item| (item.content_id.as_str(), item.relative_path.as_str()))
        .collect::<HashMap<_, _>>();
    for item in &execution.items {
        let path = item.relative_path.as_str();
        if external_paths.contains(path)
            || source_file_changed(path, source_files, instance_id).await?
        {
            continue;
        }
        let (_, desired) = execution.final_physical_decision(item);
        let final_path =
            set_upgrade_path_enabled(instance_id, path, desired, state).await?;
        record_launcher_expected_file(
            instance_id,
            path,
            &final_path,
            &mut launcher_expected_files,
        )
        .await?;
    }
    for change in &execution.solution.dependency_changes {
        let Some(content_id) = change.existing_content_id.as_deref() else {
            continue;
        };
        let Some(path) = item_paths.get(content_id).copied() else {
            continue;
        };
        if external_paths.contains(path)
            || source_file_changed(path, source_files, instance_id).await?
        {
            if change.kind == InstanceUpgradeDependencyChangeKind::Remove
                && !skipped.iter().any(|skipped_path| skipped_path == path)
            {
                skipped.push(path.to_string());
            }
            continue;
        }
        match change.kind {
            InstanceUpgradeDependencyChangeKind::Remove => {
                crate::state::instances::commands::remove_project(
                    instance_id,
                    path,
                    state,
                )
                .await?;
                launcher_expected_files.insert(path.to_string(), None);
            }
            InstanceUpgradeDependencyChangeKind::Keep => {
                let final_path = set_upgrade_path_enabled(
                    instance_id,
                    path,
                    change.enabled,
                    state,
                )
                .await?;
                record_launcher_expected_file(
                    instance_id,
                    path,
                    &final_path,
                    &mut launcher_expected_files,
                )
                .await?;
            }
            InstanceUpgradeDependencyChangeKind::Add
            | InstanceUpgradeDependencyChangeKind::Upgrade => {}
        }
    }
    Ok(AppliedUpgradeContent {
        skipped,
        launcher_expected_files,
    })
}

#[cfg(debug_assertions)]
fn injected_upgrade_failure_after_mutations() -> Option<usize> {
    std::env::var("AXOLOTL_TEST_UPGRADE_FAIL_AFTER_MUTATIONS")
        .ok()
        .and_then(|value| debug_mutation_count(&value))
}

#[cfg(debug_assertions)]
fn injected_upgrade_pause_after_mutations() -> Option<usize> {
    std::env::var("AXOLOTL_TEST_UPGRADE_PAUSE_AFTER_MUTATIONS")
        .ok()
        .and_then(|value| debug_mutation_count(&value))
}

#[cfg(debug_assertions)]
fn debug_mutation_count(value: &str) -> Option<usize> {
    value.trim().parse().ok().filter(|count| *count > 0)
}

async fn set_upgrade_path_enabled(
    instance_id: &str,
    relative_path: &str,
    enabled: bool,
    state: &State,
) -> crate::Result<String> {
    let scope = crate::state::instances::commands::resolve_content_scope(
        instance_id,
        None,
        state,
    )
    .await?;
    if let Some(entry) = crate::state::instances::adapters::sqlite::content_rows::get_content_entry_by_relative_path(
        &scope.content_set_id,
        relative_path,
        &state.pool,
    )
    .await?
    {
        if entry.enabled != enabled {
            let toggled = crate::state::instances::commands::toggle_content_entries(
                instance_id,
                &[entry.id],
                Some(enabled),
                state,
            )
            .await?;
            if let Some(toggled) = toggled.first() {
                return Ok(toggled.path.clone());
            }
        }
        return Ok(relative_path.to_string());
    }
    crate::state::instances::commands::toggle_disable_project(
        instance_id,
        relative_path,
        Some(enabled),
        state,
    )
    .await
}

async fn collect_upgrade_external_changes(
    instance_id: &str,
    source_files: &[crate::state::InstanceUpgradeSourceFile],
    baseline: Option<&super::model::InstanceUpgradeWatchBaseline>,
    state: &State,
) -> crate::Result<Vec<InstanceUpgradeExternalChange>> {
    let current = state.file_watcher.content_watch_snapshot(instance_id).await;
    let requires_full_scan = match (baseline, current.as_ref()) {
        (Some(baseline), Some(current)) => {
            baseline.epoch != current.epoch
                || current.generation > baseline.generation
        }
        _ => true,
    };
    if requires_full_scan {
        let current_files = crate::state::instances::commands::scan_instance_upgrade_source_files(
            instance_id,
            state,
        )
        .await?;
        return Ok(diff_upgrade_source_files(source_files, &current_files));
    }
    let dirty_paths = match (baseline, current.as_ref()) {
        (Some(baseline), Some(current)) if baseline.epoch == current.epoch => {
            let baseline_paths = baseline
                .dirty_paths
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            current
                .dirty_paths
                .iter()
                .filter(|path| !baseline_paths.contains(path.as_str()))
                .cloned()
                .collect::<Vec<_>>()
        }
        _ => Vec::new(),
    };
    let source_by_path = source_files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<HashMap<_, _>>();
    let base = crate::api::instance::get_full_path(instance_id).await?;
    let mut changes = Vec::new();
    for relative_path in dirty_paths {
        let path = match recovery::checked_instance_path(&base, &relative_path)
        {
            Ok(path) => path,
            Err(_) => {
                changes.push(InstanceUpgradeExternalChange {
                    relative_path,
                    kind: InstanceUpgradeExternalChangeKind::Modified,
                });
                continue;
            }
        };
        let exists = tokio::fs::symlink_metadata(&path).await.is_ok();
        let kind = classify_upgrade_external_change(
            source_by_path.contains_key(relative_path.as_str()),
            exists,
        );
        changes.push(InstanceUpgradeExternalChange {
            relative_path,
            kind,
        });
    }
    Ok(changes)
}

fn diff_upgrade_source_files(
    source_files: &[crate::state::InstanceUpgradeSourceFile],
    current_files: &[crate::state::InstanceUpgradeSourceFile],
) -> Vec<InstanceUpgradeExternalChange> {
    let source = source_files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<HashMap<_, _>>();
    let current = current_files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<HashMap<_, _>>();
    let mut paths = source
        .keys()
        .chain(current.keys())
        .copied()
        .collect::<Vec<_>>();
    paths.sort_unstable();
    paths.dedup();
    paths
        .into_iter()
        .filter_map(|path| match (source.get(path), current.get(path)) {
            (None, Some(_)) => Some(InstanceUpgradeExternalChange {
                relative_path: path.to_string(),
                kind: InstanceUpgradeExternalChangeKind::Added,
            }),
            (Some(_), None) => Some(InstanceUpgradeExternalChange {
                relative_path: path.to_string(),
                kind: InstanceUpgradeExternalChangeKind::Removed,
            }),
            (Some(source), Some(current))
                if source.sha1 != current.sha1
                    || source.size != current.size
                    || source.enabled != current.enabled =>
            {
                Some(InstanceUpgradeExternalChange {
                    relative_path: path.to_string(),
                    kind: InstanceUpgradeExternalChangeKind::Modified,
                })
            }
            _ => None,
        })
        .collect()
}

async fn collect_unsafe_upgrade_paths(
    instance_id: &str,
) -> crate::Result<Vec<String>> {
    let base = crate::api::instance::get_full_path(instance_id).await?;
    let resolved_base = crate::util::io::canonicalize(&base)?;
    let mut unsafe_paths = Vec::new();
    for path in
        crate::api::pack::import::get_all_subfiles(&resolved_base, false)
            .await?
    {
        let metadata = tokio::fs::symlink_metadata(&path).await?;
        if !crate::util::io::is_symlink_or_reparse(&metadata) {
            continue;
        }
        let Ok(relative) = path.strip_prefix(&resolved_base) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        if matches!(
            relative.split('/').next(),
            Some("mods" | "resourcepacks" | "shaderpacks" | "datapacks")
        ) {
            unsafe_paths.push(relative);
        }
    }
    unsafe_paths.sort_unstable();
    unsafe_paths.dedup();
    Ok(unsafe_paths)
}

async fn source_file_changed(
    relative_path: &str,
    source_files: &[crate::state::InstanceUpgradeSourceFile],
    instance_id: &str,
) -> crate::Result<bool> {
    let Some(expected) = source_files
        .iter()
        .find(|file| file.relative_path == relative_path)
    else {
        return Ok(false);
    };
    let base = crate::api::instance::get_full_path(instance_id).await?;
    let path = match recovery::checked_instance_path(&base, relative_path) {
        Ok(path) => path,
        Err(_) => return Ok(true),
    };
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(true);
        }
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file()
        || crate::util::io::is_symlink_or_reparse(&metadata)
        || metadata.len() != expected.size
    {
        return Ok(true);
    }
    let (_, sha1) = crate::util::fetch::sha1_file_async(&path).await?;
    Ok(sha1 != expected.sha1)
}

async fn record_launcher_expected_file(
    instance_id: &str,
    original_path: &str,
    final_path: &str,
    expected: &mut HashMap<
        String,
        Option<crate::state::InstanceUpgradeSourceFile>,
    >,
) -> crate::Result<()> {
    if original_path != final_path {
        expected.insert(original_path.to_string(), None);
    }
    expected.insert(
        final_path.to_string(),
        current_upgrade_source_file(instance_id, final_path).await?,
    );
    Ok(())
}

async fn current_upgrade_source_file(
    instance_id: &str,
    relative_path: &str,
) -> crate::Result<Option<crate::state::InstanceUpgradeSourceFile>> {
    let base = crate::api::instance::get_full_path(instance_id).await?;
    let path = recovery::checked_instance_path(&base, relative_path)?;
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || crate::util::io::is_symlink_or_reparse(&metadata)
    {
        return Ok(None);
    }
    let (_, sha1) = crate::util::fetch::sha1_file_async(&path).await?;
    Ok(Some(crate::state::InstanceUpgradeSourceFile {
        relative_path: relative_path.to_string(),
        sha1,
        size: metadata.len(),
        enabled: !relative_path.ends_with(".disabled"),
    }))
}

fn final_upgrade_external_changes(
    source_files: &[crate::state::InstanceUpgradeSourceFile],
    current_files: &[crate::state::InstanceUpgradeSourceFile],
    launcher_expected_files: &HashMap<
        String,
        Option<crate::state::InstanceUpgradeSourceFile>,
    >,
) -> Vec<InstanceUpgradeExternalChange> {
    diff_upgrade_source_files(source_files, current_files)
        .into_iter()
        .filter(|change| {
            let Some(expected) =
                launcher_expected_files.get(&change.relative_path)
            else {
                return true;
            };
            let current = current_files
                .iter()
                .find(|file| file.relative_path == change.relative_path);
            match (expected.as_ref(), current) {
                (None, None) => false,
                (Some(expected), Some(current)) => expected != current,
                _ => true,
            }
        })
        .collect()
}

fn merge_upgrade_external_changes(
    changes: &mut Vec<InstanceUpgradeExternalChange>,
    additional: Vec<InstanceUpgradeExternalChange>,
) {
    for change in additional {
        if let Some(existing) = changes
            .iter_mut()
            .find(|existing| existing.relative_path == change.relative_path)
        {
            existing.kind = change.kind;
        } else {
            changes.push(change);
        }
    }
    changes.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
}

fn should_create_upgrade_backup(
    requested: bool,
    mode: SharedUpgradeMode,
) -> bool {
    requested && mode == SharedUpgradeMode::Direct
}

fn upgrade_mutation_conflicts(
    mutation: &StagedUpgradeMutation,
    external_paths: &HashSet<String>,
) -> bool {
    external_paths.contains(&mutation.target_path)
        || mutation
            .existing_path
            .as_ref()
            .is_some_and(|path| external_paths.contains(path))
}

fn classify_upgrade_external_change(
    existed_at_start: bool,
    exists_now: bool,
) -> InstanceUpgradeExternalChangeKind {
    match (existed_at_start, exists_now) {
        (false, true) => InstanceUpgradeExternalChangeKind::Added,
        (true, false) => InstanceUpgradeExternalChangeKind::Removed,
        _ => InstanceUpgradeExternalChangeKind::Modified,
    }
}

async fn remove_existing_pack_content(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    state: &State,
    instance_id: &str,
) -> crate::Result<HashSet<String>> {
    let metadata = crate::state::instances::commands::get_instance_metadata(
        instance_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;
    let (project_id, version_id) = match &metadata.link {
        InstanceLink::ModrinthModpack {
            project_id,
            version_id,
        } => (project_id.clone(), version_id.clone()),
        InstanceLink::ServerProjectModpack {
            content_project_id,
            content_version_id,
            ..
        } => (content_project_id.clone(), content_version_id.clone()),
        InstanceLink::ImportedModpack { .. } => {
            recovery::prepare_existing_content_rollback(
                job_id,
                job_state,
                state,
                Vec::new(),
            )
            .await?;
            return Ok(HashSet::new());
        }
        _ => return Ok(HashSet::new()),
    };

    let disabled_project_ids =
        crate::state::instances::commands::list_project_files(
            instance_id,
            state,
        )
        .await?
        .into_iter()
        .filter_map(|file| {
            (!file.enabled).then(|| {
                file.provider_refs
                    .iter()
                    .find_map(|provider| match provider {
                        ContentProviderRef::Modrinth { project_id, .. } => {
                            Some(project_id.to_string())
                        }
                        ContentProviderRef::CurseForge { .. } => None,
                        ContentProviderRef::McArchive { .. } => None,
                    })
            })?
        })
        .collect::<HashSet<_>>();
    let reporter = InstallProgressReporter::new(job_id, job_state.clone());
    let old_pack = generate_pack_from_version_id_with_reporter(
        project_id.clone(),
        version_id.clone(),
        metadata.instance.name.clone(),
        None,
        instance_id.to_string(),
        DownloadReason::Update,
        reporter,
    )
    .await?;

    let related_paths = related_file_paths(&old_pack.file).await?;
    recovery::prepare_existing_content_rollback(
        job_id,
        job_state,
        state,
        related_paths,
    )
    .await?;

    Ok(disabled_project_ids)
}

async fn restore_disabled_projects(
    instance_id: &str,
    disabled_project_ids: HashSet<String>,
    state: &State,
) -> crate::Result<()> {
    if disabled_project_ids.is_empty() {
        return Ok(());
    }

    for file in crate::state::instances::commands::list_project_files(
        instance_id,
        state,
    )
    .await?
    {
        let is_disabled_modrinth_project = file.provider_refs.iter().any(
            |provider| {
                matches!(
                    provider,
                    ContentProviderRef::Modrinth { project_id, .. }
                        if disabled_project_ids.contains(&project_id.to_string())
                )
            },
        );
        if file.enabled && is_disabled_modrinth_project {
            crate::state::instances::commands::toggle_disable_project(
                instance_id,
                &file.relative_path,
                Some(false),
                state,
            )
            .await?;
        }
    }

    Ok(())
}

async fn install_pack(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    location: CreatePackLocation,
    instance_id: String,
    reason: DownloadReason,
) -> crate::Result<InstallExecutionOutcome<()>> {
    let reporter = InstallProgressReporter::new(job_id, job_state.clone());
    reporter
        .update(
            InstallPhaseId::DownloadingPackFile,
            None,
            modpack_details(&location),
        )
        .await?;

    let create_pack = match location {
        CreatePackLocation::FromVersionId {
            project_id,
            version_id,
            title,
            icon_url,
        } => {
            reporter
                .set_context(
                    InstallErrorContext::new("download modpack file")
                        .project_id(project_id.clone())
                        .version_id(version_id.clone())
                        .build(),
                )
                .await?;
            generate_pack_from_version_id_with_reporter(
                project_id,
                version_id,
                title,
                icon_url,
                instance_id.clone(),
                reason,
                reporter.clone(),
            )
            .await?
        }
        CreatePackLocation::FromFile { path } => {
            reporter
                .set_context(
                    InstallErrorContext::new("read local modpack file")
                        .source_path(path.display().to_string())
                        .build(),
                )
                .await?;
            match crate::api::pack::detect::detect_local_pack(&path).await {
                Ok(detected) => {
                    if detected.format
                        != crate::api::pack::detect::LocalPackFormat::Mrpack
                    {
                        // Non-mrpack format — dispatch to format-specific
                        // installer via install_local_pack_file.
                        return install_local_pack_file(
                            detected,
                            path,
                            instance_id,
                            reporter,
                        )
                        .await;
                    }
                    // Mrpack — fall through to standard mrpack install.
                    generate_pack_from_file(path, instance_id.clone()).await?
                }
                Err(detect_error) => {
                    // No format recognised — try recursive extraction
                    // (3-level deep search for sub-archives, bundled
                    // packs, etc.) before giving up.
                    tracing::debug!(
                        "Local pack format detection failed, trying recursive extraction: {detect_error}"
                    );
                    return install_local_pack_file_recursive(
                        path,
                        instance_id,
                        reporter,
                        0,
                        3,
                    )
                    .await;
                }
            }
        }
    };

    let outcome = install_zipped_mrpack_files_with_reporter(
        create_pack,
        false,
        reason,
        reporter,
    )
    .await?;
    Ok(match outcome {
        MrpackInstallOutcome::Completed(_) => {
            InstallExecutionOutcome::Completed(())
        }
        MrpackInstallOutcome::WaitingForUser(reason) => {
            InstallExecutionOutcome::WaitingForUser(reason)
        }
    })
}

/// Recursively tries to detect and install a modpack, up to max_depth levels.
#[async_recursion::async_recursion]
async fn install_local_pack_file_recursive(
    path: PathBuf,
    instance_id: String,
    reporter: InstallProgressReporter,
    current_depth: usize,
    max_depth: usize,
) -> crate::Result<InstallExecutionOutcome<()>> {
    // First try standard detection - this will already include our InstanceFolder fallback
    if let Ok(detected) =
        crate::api::pack::detect::detect_local_pack(&path).await
    {
        // If it's a standard format (including InstanceFolder), just install it
        return install_local_pack_file(detected, path, instance_id, reporter)
            .await;
    }

    // If standard detection failed and we're not at max depth, try to look for
    // sub-compressed files to extract and check
    if current_depth < max_depth {
        // Report progress: scanning/extracting phase (high-latency operation)
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "archive".to_string());
        reporter
            .update(
                InstallPhaseId::ResolvingPack,
                None,
                InstallPhaseDetails::Modpack {
                    project_id: None,
                    version_id: None,
                    title: Some(format!("Scanning {filename} (level {current_depth}/{max_depth})")),
                },
            )
            .await?;

        let state = State::get().await?;
        let scratch =
            crate::api::pack::archive_util::create_import_scratch_dir(&state)
                .await?;

        // Extract the entire archive to check for sub-packs
        // First, let's list all entries to find potential sub-compressed files
        let file = std::fs::File::open(&path)?;
        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(_) => {
                // Not a valid zip, can't proceed further
                let _ = tokio::fs::remove_dir_all(&scratch).await;
                return Err(crate::ErrorKind::InputError(
                    "Unrecognized modpack format: no known pack manifest was found in the archive".to_string()
                ).into());
            }
        };

        let mut sub_archive_paths = Vec::new();

        // Collect all potential sub-archive files
        for i in 0..archive.len() {
            let lower_name = {
                let entry = archive
                    .by_index_raw(i)
                    .map_err(|e| ErrorKind::OtherError(e.to_string()))?;
                let name = crate::api::pack::detect::decode_zip_entry_name(
                    entry.name_raw(),
                );
                name.to_lowercase()
            }; // entry dropped here, releasing the mutable borrow on archive

            // Check if it looks like a compressed file
            if lower_name.ends_with(".zip") || lower_name.ends_with(".mrpack") {
                // Extract this sub-archive
                reporter
                    .update(
                        InstallPhaseId::ExtractingOverrides,
                        Some(InstallProgress {
                            current: sub_archive_paths.len() as u64 + 1,
                            total: archive.len() as u64,
                            secondary: None,
                        }),
                        InstallPhaseDetails::Modpack {
                            project_id: None,
                            version_id: None,
                            title: Some(format!("Extracting {filename}")),
                        },
                    )
                    .await?;

                let mut entry = archive
                    .by_index(i)
                    .map_err(|e| ErrorKind::OtherError(e.to_string()))?;
                let sub_path = scratch.join(entry.mangled_name());

                if let Some(parent) = sub_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut out = std::fs::File::create(&sub_path)?;
                std::io::copy(&mut entry, &mut out)?;

                sub_archive_paths.push(sub_path);
            }
        }

        // Try to install each sub-archive recursively
        for sub_path in sub_archive_paths {
            let result = install_local_pack_file_recursive(
                sub_path,
                instance_id.clone(),
                reporter.clone(),
                current_depth + 1,
                max_depth,
            )
            .await;

            if result.is_ok() {
                // Success! Clean up and return
                let _ = tokio::fs::remove_dir_all(&scratch).await;
                return result;
            }
        }

        // Clean up scratch directory
        let _ = tokio::fs::remove_dir_all(&scratch).await;
    }

    // If all else fails, return error
    Err(ErrorKind::InputError(
        "Unrecognized modpack format: no known pack manifest was found in the archive"
            .to_string(),
    ).into())
}

/// Dispatches a local non-mrpack modpack file to its format-specific
/// installer, based on the detected pack format.
#[async_recursion::async_recursion]
async fn install_local_pack_file(
    detected: crate::api::pack::detect::DetectedLocalPack,
    path: PathBuf,
    instance_id: String,
    reporter: InstallProgressReporter,
) -> crate::Result<InstallExecutionOutcome<()>> {
    use crate::api::pack::detect::LocalPackFormat;

    let source_filename = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string());
    match detected.format {
        LocalPackFormat::Mrpack => {
            let create_pack =
                generate_pack_from_file(path, instance_id.clone()).await?;
            return Ok(
                match install_zipped_mrpack_files_with_reporter(
                    create_pack,
                    false,
                    DownloadReason::Modpack,
                    reporter,
                )
                .await?
                {
                    MrpackInstallOutcome::Completed(_) => {
                        InstallExecutionOutcome::Completed(())
                    }
                    MrpackInstallOutcome::WaitingForUser(reason) => {
                        InstallExecutionOutcome::WaitingForUser(reason)
                    }
                },
            );
        }
        LocalPackFormat::CurseForge => {
            let skipped_missing_content_paths = reporter
                .current_state()
                .await?
                .skipped_missing_content_paths;
            let result = crate::api::curseforge::install_modpack_from_local_archive_with_reporter(
                instance_id,
                path,
                detected.base_folder,
                source_filename,
                false,
                reporter,
                crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
            )
            .await?;
            if let Some(reason) = curseforge_manual_download_pause(
                &result,
                &skipped_missing_content_paths,
            ) {
                return Ok(InstallExecutionOutcome::WaitingForUser(reason));
            }
        }
        LocalPackFormat::Mcbbs => {
            crate::api::pack::install_mcbbs::install_mcbbs_pack_with_reporter(
                instance_id,
                path,
                detected.base_folder,
                source_filename,
                reporter,
            )
            .await?;
        }
        LocalPackFormat::Hmcl => {
            crate::api::pack::install_hmcl::install_hmcl_pack_with_reporter(
                instance_id,
                path,
                detected.base_folder,
                source_filename,
                reporter,
            )
            .await?;
        }
        LocalPackFormat::MmcExport => {
            crate::api::pack::install_mmc_zip::install_mmc_zip_with_reporter(
                instance_id,
                path,
                detected.base_folder,
                source_filename,
                reporter,
            )
            .await?;
        }
        LocalPackFormat::LauncherBundled => {
            let inner_entry = detected.inner_pack_entry.ok_or_else(|| {
                ErrorKind::InputError(
                    "Launcher bundle is missing its inner modpack file"
                        .to_string(),
                )
            })?;
            let state = State::get().await?;
            let scratch =
                crate::api::pack::archive_util::create_import_scratch_dir(
                    &state,
                )
                .await?;
            let inner_name = inner_entry
                .rsplit('/')
                .next()
                .unwrap_or("modpack.zip")
                .to_string();
            let inner_path = scratch.join(&inner_name);
            crate::api::pack::archive_util::extract_archive_entry_to_file(
                path,
                inner_entry,
                inner_path.clone(),
            )
            .await?;

            // Use our recursive function to install the inner pack
            let result = install_local_pack_file_recursive(
                inner_path,
                instance_id,
                reporter,
                1, // already one level deep
                3,
            )
            .await;

            // Clean up temporary directory
            if let Err(error) = tokio::fs::remove_dir_all(&scratch).await {
                tracing::warn!(
                    "Failed to clean up modpack import scratch directory {}: {error}",
                    scratch.display()
                );
            }

            return result;
        }
        LocalPackFormat::PlainArchive => {
            let version_id = detected.plain_version_id.ok_or_else(|| {
                ErrorKind::InputError(
                    "Could not locate the instance version in the archive"
                        .to_string(),
                )
            })?;
            crate::api::pack::install_plain_archive::install_plain_archive_with_reporter(
                instance_id,
                path,
                detected.base_folder,
                version_id,
                source_filename,
                reporter,
            )
            .await?;
        }
        LocalPackFormat::InstanceFolder => {
            // Extract the base folder contents to a temporary directory
            let state = State::get().await?;
            let scratch =
                crate::api::pack::archive_util::create_import_scratch_dir(
                    &state,
                )
                .await?;

            // Extract the instance folder contents
            crate::api::pack::archive_util::extract_archive_subdir(
                path,
                detected.base_folder,
                scratch.clone(),
            )
            .await?;

            // Now import it as a generic instance
            let details = InstallPhaseDetails::Modpack {
                project_id: None,
                version_id: None,
                title: source_filename.clone(),
            };

            crate::api::pack::import::generic::import_generic(
                scratch,
                &instance_id,
                reporter,
                details,
                false,
                &crate::api::pack::import::ImportOverrides::default(),
                None, // Not compatible mode
            )
            .await?;
        }
    }
    Ok(InstallExecutionOutcome::Completed(()))
}

fn curseforge_manual_download_pause(
    result: &crate::api::curseforge::CurseForgeModpackInstallResult,
    skipped_missing_content_paths: &[String],
) -> Option<InstallPauseReason> {
    let missing_downloads = result
        .content
        .manual_downloads
        .iter()
        .filter(|download| {
            !skipped_missing_content_paths.contains(&download.file_name)
        })
        .collect::<Vec<_>>();
    if missing_downloads.is_empty() {
        return None;
    }
    Some(InstallPauseReason::MissingRequiredContent {
        failed_files: missing_downloads.len() as u64,
        paths: missing_downloads
            .iter()
            .map(|download| download.file_name.clone())
            .collect(),
    })
}

fn curseforge_world_was_imported_manually(
    job_state: &InstallJobState,
    request: &crate::api::curseforge::CurseForgeWorldInstallRequest,
) -> bool {
    let project_id = request.project_id.to_string();
    let file_id = request.file_id.to_string();
    job_state.download_items().iter().any(|item| {
        item.status == super::model::DownloadItemStatus::Completed
            && item.project_id.as_deref() == Some(project_id.as_str())
            && item.version_id.as_deref() == Some(file_id.as_str())
    })
}

async fn prepare_existing_rollback(
    job_state: &mut InstallJobState,
    state: &State,
    instance_id: &str,
) -> crate::Result<()> {
    if job_state.rollback.is_some() {
        return Ok(());
    }

    let instance = crate::state::get_instance(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Unknown instance {instance_id}"
            ))
        })?;
    let install_stage = instance.instance.install_stage;
    set_display(
        job_state,
        instance.instance.name.clone(),
        instance.instance.icon_path.clone(),
    );
    job_state.rollback = Some(InstallRollbackState {
        instance,
        install_stage,
        content: None,
    });
    job_state.cleanup = InstallCleanup::RestoreExistingInstance {
        instance_id: instance_id.to_string(),
    };

    crate::state::instances::commands::set_instance_install_stage(
        instance_id,
        InstanceInstallStage::MinecraftInstalling,
        &state.pool,
    )
    .await?;
    emit_instance(instance_id, InstancePayloadType::Edited).await?;

    Ok(())
}

async fn update_progress(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    state: &State,
    phase: InstallPhaseId,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    job_state.set_progress(phase, None, details);
    let record = store::update_state(job_id, job_state, state).await?;
    emit_install_job(&record.snapshot()).await?;
    Ok(())
}

fn set_instance_id(job_state: &mut InstallJobState, instance_id: String) {
    job_state.target = match &job_state.target {
        InstallTarget::ExistingInstance { .. } => {
            InstallTarget::ExistingInstance {
                instance_id: instance_id.clone(),
            }
        }
        InstallTarget::NewInstance { .. } => InstallTarget::NewInstance {
            instance_id: Some(instance_id.clone()),
        },
    };
    job_state.cleanup = match &job_state.cleanup {
        InstallCleanup::RestoreExistingInstance { .. } => {
            InstallCleanup::RestoreExistingInstance { instance_id }
        }
        InstallCleanup::DeleteNewInstance { .. } => {
            InstallCleanup::DeleteNewInstance {
                instance_id: Some(instance_id),
            }
        }
        InstallCleanup::None => InstallCleanup::None,
    };
}

fn clear_deleted_new_instance_id(job_state: &mut InstallJobState) {
    if matches!(job_state.cleanup, InstallCleanup::DeleteNewInstance { .. }) {
        job_state.target = InstallTarget::NewInstance { instance_id: None };
        job_state.cleanup =
            InstallCleanup::DeleteNewInstance { instance_id: None };
    }
}

fn set_display(
    job_state: &mut InstallJobState,
    title: String,
    icon: Option<String>,
) {
    job_state.display = Some(InstallJobDisplay { title, icon });
}

fn install_error_view(
    phase: InstallPhaseId,
    error: &crate::Error,
    context: Option<InstallErrorContext>,
) -> InstallErrorView {
    let context = match error.raw.as_ref() {
        ErrorKind::CacheReadError {
            cache_type,
            sqlite_code,
            ..
        } => {
            let mut context = context.unwrap_or_else(|| {
                InstallErrorContext::new("read project metadata cache").build()
            });
            context.cache_types = vec![cache_type.clone()];
            context.sqlite_code = sqlite_code.clone();
            Some(context)
        }
        _ => context,
    };
    InstallErrorView::from_error(
        install_error_code(phase, error),
        phase,
        error,
        context,
    )
}

fn install_error_code(
    phase: InstallPhaseId,
    error: &crate::Error,
) -> &'static str {
    use InstallPhaseId::*;

    match error.raw.as_ref() {
        ErrorKind::CacheReadError { .. } => "cache_repair_required",
        ErrorKind::InputError(msg)
            if msg.starts_with("Unrecognized modpack format")
                && matches!(phase, ResolvingPack) =>
        {
            "unrecognized_format"
        }
        ErrorKind::InputError(_) => match phase {
            PreparingInstance | CreatingBackup | Finalizing | Completed => {
                "instance_error"
            }
            ResolvingPack | DownloadingPackFile | ReadingPackManifest => {
                "pack_error"
            }
            DownloadingContent | StagingContent | ApplyingContent => {
                "content_error"
            }
            ExtractingOverrides => "path_error",
            PreparingJava => "java_error",
            DownloadingMinecraft => "instance_error",
            RollingBack => "rollback_error",
            ResolvingMinecraft
            | ResolvingLoader
            | RunningLoaderProcessors
            | UpdatingLoader
            | Verifying => "launcher_error",
        },
        ErrorKind::LauncherError(_) => match phase {
            RunningLoaderProcessors => "processor_error",
            PreparingJava => "java_error",
            ResolvingLoader => "loader_error",
            _ => "launcher_error",
        },
        ErrorKind::JREError(_) => "java_error",
        ErrorKind::NoValueFor(_) | ErrorKind::MetadataError(_) => match phase {
            ResolvingLoader => "loader_error",
            PreparingJava => "java_error",
            _ => "metadata_error",
        },
        ErrorKind::FetchError(_)
        | ErrorKind::NetworkError(_)
        | ErrorKind::HttpError { .. }
        | ErrorKind::ApiIsDownError(_) => "network_error",
        ErrorKind::Any(_)
            if matches!(
                phase,
                DownloadingPackFile
                    | DownloadingContent
                    | ResolvingMinecraft
                    | ResolvingLoader
                    | PreparingJava
                    | DownloadingMinecraft
            ) =>
        {
            "network_error"
        }
        ErrorKind::LabrinthError(_) => "api_error",
        ErrorKind::HashError(_, _) => "hash_error",
        ErrorKind::ZipError(_) => "archive_error",
        ErrorKind::DeserializationError(_) | ErrorKind::StripPrefixError(_) => {
            "path_error"
        }
        ErrorKind::FSError(_)
        | ErrorKind::IOError(_)
        | ErrorKind::StdIOError(_)
        | ErrorKind::UTFError(_) => "filesystem_error",
        ErrorKind::INIError(_) | ErrorKind::JSONError(_) => "parse_error",
        ErrorKind::Sqlx(_) | ErrorKind::SqlxMigrate(_) => "database_error",
        ErrorKind::JoinError(_)
        | ErrorKind::RecvError(_)
        | ErrorKind::AcquireError(_)
        | ErrorKind::EventError(_) => "internal_error",
        ErrorKind::OtherError(_) | ErrorKind::Any(_) => "internal_error",
        _ => "unknown_error",
    }
}

fn current_instance_id(job_state: &InstallJobState) -> Option<String> {
    match &job_state.target {
        InstallTarget::NewInstance { instance_id } => instance_id.clone(),
        InstallTarget::ExistingInstance { instance_id } => {
            Some(instance_id.clone())
        }
    }
}

pub(crate) const OPTIFABRIC_CURSEFORGE_PROJECT_ID: u32 = 322_385;

async fn resolve_required_adjuncts(
    game_version: &str,
    loader: ModLoader,
    adjuncts: &mut Vec<LoaderComponent>,
    _state: &State,
) -> crate::Result<()> {
    for adjunct in adjuncts.iter() {
        match adjunct.kind {
            LoaderComponentKind::OptiFine
                if !matches!(
                    loader,
                    ModLoader::Forge
                        | ModLoader::NeoForge
                        | ModLoader::Fabric
                        | ModLoader::LegacyFabric
                ) =>
            {
                return Err(ErrorKind::InputError(format!(
                    "OptiFine is not supported with {}",
                    loader.as_str()
                ))
                .into());
            }
            LoaderComponentKind::LiteLoader if loader != ModLoader::Forge => {
                return Err(ErrorKind::InputError(format!(
                    "LiteLoader is not supported with {}",
                    loader.as_str()
                ))
                .into());
            }
            _ => {}
        }
    }
    for adjunct in adjuncts.iter_mut() {
        adjunct.role = LoaderComponentRole::Adjunct;
        adjunct.instance_id.clear();
        match adjunct.kind {
            LoaderComponentKind::OptiFine => {
                let resolved =
					crate::launcher::optifine::resolve_loader_version(
						game_version,
						adjunct.version.as_deref(),
					)
					.await?
					.ok_or_else(|| {
						ErrorKind::InputError(format!(
							"No OptiFine version supports Minecraft {game_version}"
						))
					})?;
                adjunct.version = Some(resolved.id);
            }
            LoaderComponentKind::LiteLoader => {
                let resolved =
					crate::launcher::get_loader_version_from_profile(
						game_version,
						ModLoader::LiteLoader,
						adjunct.version.as_deref(),
					)
					.await?
					.ok_or_else(|| {
						ErrorKind::InputError(format!(
							"No LiteLoader version supports Minecraft {game_version}"
						))
					})?;
                adjunct.version = Some(resolved.id);
            }
            _ => {}
        }
    }
    if adjuncts
        .iter()
        .any(|component| component.kind == LoaderComponentKind::OptiFine)
        && matches!(loader, ModLoader::Fabric | ModLoader::LegacyFabric)
        && !adjuncts
            .iter()
            .any(|component| component.kind == LoaderComponentKind::OptiFabric)
    {
        let version_id = resolve_optifabric_version(game_version).await?;
        adjuncts.push(LoaderComponent {
            instance_id: String::new(),
            kind: LoaderComponentKind::OptiFabric,
            version: Some(version_id),
            role: LoaderComponentRole::Adjunct,
            provider_metadata: Some(serde_json::json!({
                "projectId": OPTIFABRIC_CURSEFORGE_PROJECT_ID,
                "provider": "curseforge"
            })),
        });
    }
    let mut components =
        vec![LoaderComponent::new_primary(String::new(), loader, None)];
    components.extend(adjuncts.iter().cloned());
    validate_loader_components(&components)
}

pub(crate) fn validate_loader_components(
    components: &[LoaderComponent],
) -> crate::Result<()> {
    let primary = components
        .iter()
        .find(|component| component.role == LoaderComponentRole::Primary)
        .ok_or_else(|| {
            ErrorKind::InputError(
                "Loader selection has no primary loader".to_string(),
            )
        })?;
    let has = |kind| {
        components.iter().any(|component| {
            component.role == LoaderComponentRole::Adjunct
                && component.kind == kind
        })
    };
    if has(LoaderComponentKind::OptiFine) {
        match primary.kind {
            LoaderComponentKind::Vanilla => {}
            LoaderComponentKind::Forge | LoaderComponentKind::NeoForge => {}
            LoaderComponentKind::Fabric | LoaderComponentKind::LegacyFabric
                if has(LoaderComponentKind::OptiFabric) => {}
            _ => {
                return Err(ErrorKind::InputError(format!(
                    "OptiFine is not supported with {}",
                    primary.kind.as_str()
                ))
                .into());
            }
        }
    }
    if has(LoaderComponentKind::OptiFabric)
        && !has(LoaderComponentKind::OptiFine)
    {
        return Err(ErrorKind::InputError(
            "OptiFabric can only be installed with OptiFine".to_string(),
        )
        .into());
    }
    if has(LoaderComponentKind::OptiFabric)
        && !matches!(
            primary.kind,
            LoaderComponentKind::Fabric | LoaderComponentKind::LegacyFabric
        )
    {
        return Err(ErrorKind::InputError(format!(
            "OptiFabric is not supported with {}",
            primary.kind.as_str()
        ))
        .into());
    }
    if has(LoaderComponentKind::LiteLoader)
        && !matches!(
            primary.kind,
            LoaderComponentKind::Vanilla | LoaderComponentKind::Forge
        )
    {
        return Err(ErrorKind::InputError(format!(
            "LiteLoader is not supported with {}",
            primary.kind.as_str()
        ))
        .into());
    }
    if components.iter().any(|component| {
        component.role == LoaderComponentRole::Adjunct
            && !matches!(
                component.kind,
                LoaderComponentKind::OptiFine
                    | LoaderComponentKind::LiteLoader
                    | LoaderComponentKind::OptiFabric
            )
    }) {
        return Err(ErrorKind::InputError(
            "Only OptiFine, LiteLoader, and OptiFabric can be adjunct loaders"
                .to_string(),
        )
        .into());
    }
    Ok(())
}

pub(crate) async fn resolve_optifabric_version(
    game_version: &str,
) -> crate::Result<String> {
    let files = crate::api::curseforge::get_files(
        OPTIFABRIC_CURSEFORGE_PROJECT_ID,
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
    select_optifabric_file_id(&files, game_version)
        .map(|file_id| file_id.to_string())
        .ok_or_else(|| {
            ErrorKind::InputError(format!(
                "OptiFine requires OptiFabric, but no OptiFabric version supports Minecraft {game_version}"
            ))
            .into()
        })
}

fn select_optifabric_file_id(
    files: &[crate::api::curseforge::CurseForgeFile],
    game_version: &str,
) -> Option<u32> {
    files
        .iter()
        .find(|file| {
            file.is_available
                && file
                    .game_versions
                    .iter()
                    .any(|version| version == game_version)
        })
        .map(|file| file.id)
}

pub(crate) async fn install_optifabric_file(
    instance_id: &str,
    game_version: &str,
    version: &str,
) -> crate::Result<String> {
    let file_id = version.parse::<u32>().map_err(|_| {
        ErrorKind::InputError(
            "OptiFabric CurseForge file ID is invalid".to_string(),
        )
    })?;
    let file = crate::api::curseforge::get_file(
        OPTIFABRIC_CURSEFORGE_PROJECT_ID,
        file_id,
    )
    .await?;
    if file.mod_id != OPTIFABRIC_CURSEFORGE_PROJECT_ID
        || !file.is_available
        || !file
            .game_versions
            .iter()
            .any(|version| version == game_version)
    {
        return Err(ErrorKind::InputError(format!(
            "OptiFabric file {file_id} does not support Minecraft {game_version}"
        ))
        .into());
    }

    let result = crate::api::curseforge::install_file(
        crate::api::curseforge::CurseForgeInstallRequest {
            instance_id: instance_id.to_string(),
            project_id: OPTIFABRIC_CURSEFORGE_PROJECT_ID,
            file_id,
            project_type: "mod".to_string(),
            ownership_kind: crate::state::instances::ContentOwnershipKind::UserAdded,
            manual_operation_kind:
                crate::state::instances::ManualDownloadOperationKind::ContentInstall,
            game_version: Some(game_version.to_string()),
            mod_loader_type: Some(4),
            world_name: None,
            install_dependencies: false,
            excluded_dependency_project_ids: Vec::new(),
            force_dependency_project_ids: Vec::new(),
            dependency_plan_id: None,
        },
    )
    .await?;
    if !result.manual_downloads.is_empty() {
        return Err(ErrorKind::InputError(
            "OptiFabric requires a manual CurseForge download".to_string(),
        )
        .into());
    }
    if let Some(failure) = result.failed_downloads.first() {
        return Err(ErrorKind::InputError(format!(
            "Failed to install OptiFabric: {}",
            failure.reason
        ))
        .into());
    }
    if !result.installed.iter().any(|installed| {
        !installed.dependency
            && installed.project_id == OPTIFABRIC_CURSEFORGE_PROJECT_ID
            && installed.file_id == file_id
    }) {
        return Err(ErrorKind::InputError(
            "OptiFabric was not installed".to_string(),
        )
        .into());
    }
    Ok(file_id.to_string())
}

async fn install_adjunct_components(
    state: &State,
    instance_id: &str,
    adjuncts: &[LoaderComponent],
    game_version: &str,
    loader: ModLoader,
    cancellation: tokio_util::sync::CancellationToken,
) -> crate::Result<()> {
    if adjuncts.is_empty() {
        return Ok(());
    }
    let metadata = crate::api::instance::get(instance_id)
        .await?
        .ok_or_else(|| ErrorKind::InputError("Unknown instance".to_string()))?;
    let instance_path = state.directories.instance_game_dir(&metadata.instance);
    let mut components = metadata.loader_components.clone();

    for adjunct in adjuncts {
        match adjunct.kind {
            LoaderComponentKind::OptiFine => {
                let version = crate::launcher::optifine::resolve_loader_version(
					game_version,
					adjunct.version.as_deref(),
				)
				.await?
				.ok_or_else(|| {
					ErrorKind::InputError(format!(
						"No OptiFine version supports Minecraft {game_version}"
					))
				})?;
                crate::api::pack::install_mcbbs::install_optifine_mod(
                    state,
                    instance_id,
                    cancellation.clone(),
                    game_version,
                    &version.id,
                    &instance_path,
                )
                .await?;
                set_component_version(
                    &mut components,
                    LoaderComponentKind::OptiFine,
                    version.id,
                );
            }
            LoaderComponentKind::OptiFabric => {
                let version_id = match &adjunct.version {
                    Some(version) => version.clone(),
                    None => resolve_optifabric_version(game_version).await?,
                };
                let version_id = install_optifabric_file(
                    instance_id,
                    game_version,
                    &version_id,
                )
                .await?;
                set_component_version(
                    &mut components,
                    LoaderComponentKind::OptiFabric,
                    version_id,
                );
            }
            LoaderComponentKind::LiteLoader => {
                let version = install_liteloader_adjunct(
                    state,
                    &metadata,
                    game_version,
                    loader,
                    adjunct.version.as_deref(),
                )
                .await?;
                set_component_version(
                    &mut components,
                    LoaderComponentKind::LiteLoader,
                    version,
                );
            }
            _ => {}
        }
    }
    crate::state::instances::commands::replace_instance_loader_components(
        instance_id,
        &components,
        &state.pool,
    )
    .await
}

fn set_component_version(
    components: &mut [LoaderComponent],
    kind: LoaderComponentKind,
    version: String,
) {
    if let Some(component) = components
        .iter_mut()
        .find(|component| component.kind == kind)
    {
        component.version = Some(version);
    }
}

pub(crate) async fn install_liteloader_adjunct(
    state: &State,
    metadata: &crate::state::InstanceMetadata,
    game_version: &str,
    primary_loader: ModLoader,
    requested_version: Option<&str>,
) -> crate::Result<String> {
    let version = crate::launcher::get_loader_version_from_profile(
        game_version,
        ModLoader::LiteLoader,
        requested_version,
    )
    .await?
    .ok_or_else(|| {
        ErrorKind::InputError(format!(
            "No LiteLoader version supports Minecraft {game_version}"
        ))
    })?;
    install_liteloader_adjunct_resolved(
        state,
        metadata,
        game_version,
        primary_loader,
        &version,
    )
    .await
}

pub(crate) async fn install_liteloader_adjunct_resolved(
    state: &State,
    metadata: &crate::state::InstanceMetadata,
    game_version: &str,
    primary_loader: ModLoader,
    version: &daedalus::modded::LoaderVersion,
) -> crate::Result<String> {
    let partial = crate::api::loader_metadata::resolve_loader_profile(
        state,
        game_version,
        version,
    )
    .await?;
    let primary_version = metadata
        .applied_content_set
        .loader_version
        .as_deref()
        .ok_or_else(|| {
            ErrorKind::InputError(format!(
                "{} adjunct installation requires a pinned primary version",
                primary_loader.as_str()
            ))
        })?;
    let version_id = format!("{game_version}-{primary_version}");
    let path = state
        .directories
        .version_dir(&version_id)
        .join(format!("{version_id}.json"));
    let bytes = crate::util::io::read(&path).await?;
    let primary: daedalus::minecraft::VersionInfo =
        serde_json::from_slice(&bytes)?;
    let mut merged = daedalus::modded::merge_partial_version(partial, primary);
    merged.id.clone_from(&version_id);
    crate::launcher::download::download_libraries(
        state,
        None,
        &merged.libraries,
        &version_id,
        None,
        0.0,
        std::env::consts::ARCH,
        false,
        false,
        None,
    )
    .await?;
    crate::util::io::write(&path, serde_json::to_vec(&merged)?).await?;
    Ok(version.id.clone())
}

fn modpack_details(location: &CreatePackLocation) -> InstallPhaseDetails {
    match location {
        CreatePackLocation::FromVersionId {
            project_id,
            version_id,
            title,
            ..
        } => InstallPhaseDetails::Modpack {
            project_id: Some(project_id.clone()),
            version_id: Some(version_id.clone()),
            title: Some(title.clone()),
        },
        CreatePackLocation::FromFile { .. } => InstallPhaseDetails::Modpack {
            project_id: None,
            version_id: None,
            title: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn physical_test_item(
        content_id: &str,
        relative_path: &str,
        status: crate::state::InstanceUpgradeItemStatus,
        action: InstanceUpgradeAction,
        current_enabled: bool,
    ) -> crate::state::InstanceUpgradeItem {
        crate::state::InstanceUpgradeItem {
            content_id: content_id.to_string(),
            relative_path: relative_path.to_string(),
            project_type: crate::state::ProjectType::Mod,
            provider: None,
            project_id: None,
            current_release_id: None,
            current_enabled,
            auto_dependency: false,
            status,
            resolution: crate::state::InstanceUpgradeResolution {
                content_id: content_id.to_string(),
                action,
                allow_prerelease: false,
                confirmed_prerelease_dependencies: Vec::new(),
            },
            candidate_release_ids: Vec::new(),
        }
    }

    fn physical_test_execution(
        items: Vec<crate::state::InstanceUpgradeItem>,
        selections: Vec<crate::state::InstanceUpgradeSelection>,
    ) -> InstanceUpgradeExecution {
        let environment = crate::state::InstanceUpgradeEnvironment {
            game_version: "1.21.9".to_string(),
            mod_loader: ModLoader::Fabric,
            mod_loader_version: Some("0.18.5".to_string()),
            shader_runtime: crate::state::ShaderRuntime::Iris,
        };
        InstanceUpgradeExecution {
            source_revision: 1,
            source_files: Vec::new(),
            source_environment: environment.clone(),
            target_environment: environment,
            items,
            solution: crate::state::InstanceUpgradeSolution {
                kind: crate::state::InstanceUpgradeSolutionKind::Custom,
                selections,
                dependency_changes: Vec::new(),
                warnings: Vec::new(),
            },
            warnings: Vec::new(),
            source_watch: None,
        }
    }

    #[test]
    fn final_physical_decision_uses_solution_then_item_resolution() {
        let solver_item = physical_test_item(
            "solver",
            "mods/solver.jar",
            crate::state::InstanceUpgradeItemStatus::UpgradeAvailable,
            InstanceUpgradeAction::Disable,
            true,
        );
        let local_disable = physical_test_item(
            "local-disable",
            "mods/local-disable.jar",
            crate::state::InstanceUpgradeItemStatus::Unidentified,
            InstanceUpgradeAction::Disable,
            true,
        );
        let local_keep_disabled = physical_test_item(
            "local-keep",
            "mods/local-keep.jar.disabled",
            crate::state::InstanceUpgradeItemStatus::Unidentified,
            InstanceUpgradeAction::Keep,
            false,
        );
        let execution = physical_test_execution(
            vec![
                solver_item.clone(),
                local_disable.clone(),
                local_keep_disabled.clone(),
            ],
            vec![crate::state::InstanceUpgradeSelection {
                content_id: "solver".to_string(),
                provider: Some(ContentProvider::Modrinth),
                project_id: Some("project".to_string()),
                current_release_id: Some("old".to_string()),
                target_release_id: None,
                action: InstanceUpgradeAction::Keep,
                enabled: true,
            }],
        );

        assert_eq!(
            execution.final_physical_decision(&solver_item),
            (InstanceUpgradeAction::Keep, true)
        );
        assert_eq!(
            execution.final_physical_decision(&local_disable),
            (InstanceUpgradeAction::Disable, false)
        );
        assert_eq!(
            execution.final_physical_decision(&local_keep_disabled),
            (InstanceUpgradeAction::Keep, false)
        );
    }

    #[test]
    fn generated_upgrade_name_uses_exact_resolved_loader_version() {
        assert_eq!(
            default_upgrade_instance_name(
                &crate::state::InstanceUpgradeEnvironment {
                    game_version: "1.21.9".to_string(),
                    mod_loader: ModLoader::Fabric,
                    mod_loader_version: Some("0.18.5".to_string()),
                    shader_runtime: crate::state::ShaderRuntime::Iris,
                },
            ),
            "1.21.9-Fabric 0.18.5"
        );
    }

    #[test]
    fn compatibility_warning_details_recover_unique_exact_content_identity() {
        let environment = crate::state::InstanceUpgradeEnvironment {
            game_version: "1.21.9".to_string(),
            mod_loader: ModLoader::Fabric,
            mod_loader_version: Some("0.18.5".to_string()),
            shader_runtime: crate::state::ShaderRuntime::Iris,
        };
        let execution = InstanceUpgradeExecution {
            source_revision: 1,
            source_files: Vec::new(),
            source_environment: environment.clone(),
            target_environment: environment,
            items: vec![crate::state::InstanceUpgradeItem {
                content_id: "content".to_string(),
                relative_path: "resourcepacks/foo.zip".to_string(),
                project_type: crate::state::ProjectType::ResourcePack,
                provider: Some(crate::state::ContentProvider::Modrinth),
                project_id: Some("project".to_string()),
                current_release_id: Some("release".to_string()),
                current_enabled: true,
                auto_dependency: false,
                status:
                    crate::state::InstanceUpgradeItemStatus::NoCompatibleRelease,
                resolution: crate::state::InstanceUpgradeResolution {
                    content_id: "content".to_string(),
                    action: crate::state::InstanceUpgradeAction::Keep,
                    allow_prerelease: false,
                    confirmed_prerelease_dependencies: Vec::new(),
                },
                candidate_release_ids: Vec::new(),
            }],
            solution: crate::state::InstanceUpgradeSolution {
                kind: crate::state::InstanceUpgradeSolutionKind::Custom,
                selections: Vec::new(),
                dependency_changes: Vec::new(),
                warnings: Vec::new(),
            },
            warnings: vec![crate::state::InstanceUpgradeIssue {
                code: crate::state::InstanceUpgradeIssueCode::KeepIncompatible,
                message: "preserved".to_string(),
                content_id: None,
                provider: Some(crate::state::ContentProvider::Modrinth),
                project_id: Some("project".to_string()),
                conflicting_project_id: None,
                dependency_requirements: Vec::new(),
            }],
            source_watch: None,
        };

        let details = upgrade_compatibility_warning_details(&execution);
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].content_id.as_deref(), Some("content"));
        assert_eq!(
            details[0].relative_path.as_deref(),
            Some("resourcepacks/foo.zip")
        );
    }

    #[test]
    fn local_path_only_preserve_warnings_create_physical_notice_details() {
        let environment = crate::state::InstanceUpgradeEnvironment {
            game_version: "1.21.9".to_string(),
            mod_loader: ModLoader::Fabric,
            mod_loader_version: Some("0.18.5".to_string()),
            shader_runtime: crate::state::ShaderRuntime::Iris,
        };
        let item = |content_id: &str, relative_path: &str| {
            crate::state::InstanceUpgradeItem {
                content_id: content_id.to_string(),
                relative_path: relative_path.to_string(),
                project_type: crate::state::ProjectType::ResourcePack,
                provider: None,
                project_id: None,
                current_release_id: None,
                current_enabled: true,
                auto_dependency: false,
                status:
                    crate::state::InstanceUpgradeItemStatus::NoCompatibleRelease,
                resolution: crate::state::InstanceUpgradeResolution {
                    content_id: content_id.to_string(),
                    action: crate::state::InstanceUpgradeAction::Keep,
                    allow_prerelease: false,
                    confirmed_prerelease_dependencies: Vec::new(),
                },
                candidate_release_ids: Vec::new(),
            }
        };
        let execution = InstanceUpgradeExecution {
            source_revision: 1,
            source_files: Vec::new(),
            source_environment: environment.clone(),
            target_environment: environment,
            items: vec![
                item("resource-pack", "resourcepacks/foo.zip"),
                item("shader-pack", "shaderpacks/bar.zip"),
            ],
            solution: crate::state::InstanceUpgradeSolution {
                kind: crate::state::InstanceUpgradeSolutionKind::Custom,
                selections: Vec::new(),
                dependency_changes: Vec::new(),
                warnings: Vec::new(),
            },
            warnings: vec![crate::state::InstanceUpgradeIssue {
                code: crate::state::InstanceUpgradeIssueCode::KeepIncompatible,
                message: "local content preserved".to_string(),
                content_id: None,
                provider: None,
                project_id: None,
                conflicting_project_id: None,
                dependency_requirements: Vec::new(),
            }],
            source_watch: None,
        };

        let details = upgrade_compatibility_warning_details(&execution);
        assert_eq!(details.len(), 2);
        assert!(details.iter().any(|detail| {
            detail.relative_path.as_deref() == Some("resourcepacks/foo.zip")
                && detail.provider.is_none()
                && detail.project_id.is_none()
        }));
        assert_eq!(
            details
                .iter()
                .filter(|detail| detail.relative_path.is_some())
                .count(),
            2
        );
    }

    fn upgrade_job_state() -> InstallJobState {
        let environment = crate::state::InstanceUpgradeEnvironment {
            game_version: "1.21.1".to_string(),
            mod_loader: ModLoader::Fabric,
            mod_loader_version: Some("0.16.0".to_string()),
            shader_runtime: crate::state::ShaderRuntime::Iris,
        };
        InstallJobState::new(InstallRequest::UpgradeUnmanagedInstance {
            instance_id: "instance".to_string(),
            plan_id: "plan".to_string(),
            execution: InstanceUpgradeExecution {
                source_revision: 1,
                source_files: Vec::new(),
                source_environment: environment.clone(),
                target_environment: environment,
                items: Vec::new(),
                solution: crate::state::InstanceUpgradeSolution {
                    kind: crate::state::InstanceUpgradeSolutionKind::Custom,
                    selections: Vec::new(),
                    dependency_changes: Vec::new(),
                    warnings: Vec::new(),
                },
                warnings: Vec::new(),
                source_watch: None,
            },
            create_full_backup: false,
            shared_upgrade_mode: SharedUpgradeMode::Direct,
            display_names: InstanceUpgradeDisplayNames::default(),
        })
    }

    fn components(
        primary: ModLoader,
        adjuncts: &[LoaderComponentKind],
    ) -> Vec<LoaderComponent> {
        std::iter::once(LoaderComponent::new_primary("", primary, None))
            .chain(adjuncts.iter().map(|kind| LoaderComponent {
                instance_id: String::new(),
                kind: *kind,
                version: None,
                role: LoaderComponentRole::Adjunct,
                provider_metadata: None,
            }))
            .collect()
    }

    fn curseforge_file(
        id: u32,
        is_available: bool,
        game_versions: &[&str],
    ) -> crate::api::curseforge::CurseForgeFile {
        crate::api::curseforge::CurseForgeFile {
            id,
            game_id: 432,
            mod_id: OPTIFABRIC_CURSEFORGE_PROJECT_ID,
            is_available,
            display_name: String::new(),
            file_name: String::new(),
            release_type: 1,
            file_status: 4,
            hashes: Vec::new(),
            file_date: String::new(),
            file_length: 0,
            download_count: 0,
            file_size_on_disk: None,
            download_url: None,
            game_versions: game_versions
                .iter()
                .map(ToString::to_string)
                .collect(),
            sortable_game_versions: Vec::new(),
            dependencies: Vec::new(),
            expose_as_alternative: None,
            parent_project_file_id: None,
            alternate_file_id: None,
            is_server_pack: None,
            server_pack_file_id: None,
            is_early_access_content: None,
            early_access_end_date: None,
            file_fingerprint: 0,
            modules: Vec::new(),
        }
    }

    #[test]
    fn loader_component_preflight_accepts_verified_combinations() {
        for components in [
            components(ModLoader::Vanilla, &[LoaderComponentKind::OptiFine]),
            components(ModLoader::Vanilla, &[LoaderComponentKind::LiteLoader]),
            components(ModLoader::Forge, &[LoaderComponentKind::OptiFine]),
            components(ModLoader::NeoForge, &[LoaderComponentKind::OptiFine]),
            components(ModLoader::Forge, &[LoaderComponentKind::LiteLoader]),
            components(
                ModLoader::Fabric,
                &[
                    LoaderComponentKind::OptiFine,
                    LoaderComponentKind::OptiFabric,
                ],
            ),
        ] {
            validate_loader_components(&components).unwrap();
        }
    }

    #[test]
    fn loader_component_preflight_rejects_unverified_combinations() {
        for components in [
            components(ModLoader::Quilt, &[LoaderComponentKind::OptiFine]),
            components(ModLoader::Cleanroom, &[LoaderComponentKind::OptiFine]),
            components(
                ModLoader::LegacyFabric,
                &[LoaderComponentKind::LiteLoader],
            ),
            components(ModLoader::Fabric, &[LoaderComponentKind::LiteLoader]),
            components(ModLoader::Fabric, &[LoaderComponentKind::OptiFine]),
            components(
                ModLoader::Forge,
                &[
                    LoaderComponentKind::OptiFine,
                    LoaderComponentKind::OptiFabric,
                ],
            ),
        ] {
            assert!(validate_loader_components(&components).is_err());
        }
    }

    #[test]
    fn optifabric_selection_requires_an_available_exact_game_version() {
        let files = vec![
            curseforge_file(1, true, &["1.19.2"]),
            curseforge_file(2, false, &["1.20.1"]),
            curseforge_file(3, true, &["1.20.1"]),
        ];

        assert_eq!(select_optifabric_file_id(&files, "1.20.1"), Some(3));
        assert_eq!(select_optifabric_file_id(&files, "1.20.2"), None);
    }

    #[test]
    fn stalled_downloads_are_reported_as_network_errors() {
        let error: crate::Error = crate::ErrorKind::NetworkError(
            "no data received for 60 seconds".to_string(),
        )
        .into();

        assert_eq!(
            install_error_code(InstallPhaseId::DownloadingMinecraft, &error),
            "network_error"
        );
    }

    #[test]
    fn cache_read_errors_have_repair_context_but_generic_sqlx_does_not() {
        let cache_error: crate::Error = crate::ErrorKind::CacheReadError {
            cache_type: "curseforge_project".to_string(),
            message: "malformed cache row".to_string(),
            sqlite_code: Some("11".to_string()),
        }
        .into();
        let view = install_error_view(
            InstallPhaseId::ResolvingPack,
            &cache_error,
            None,
        );
        assert_eq!(view.code, "cache_repair_required");
        let context = view.context.unwrap();
        assert_eq!(context.cache_types, vec!["curseforge_project"]);
        assert_eq!(context.sqlite_code.as_deref(), Some("11"));

        let database_error: crate::Error =
            crate::ErrorKind::Sqlx(sqlx::Error::RowNotFound).into();
        assert_eq!(
            install_error_code(InstallPhaseId::ResolvingPack, &database_error),
            "database_error"
        );
    }

    #[test]
    fn cache_repair_validation_rejects_old_or_unknown_context() {
        let old_context = InstallErrorContext::new("read cache").build();
        let old_error = InstallErrorView::from_message(
            "cache_repair_required",
            InstallPhaseId::ResolvingPack,
            "cache failed",
        );
        assert!(
            validated_cache_repair_types_for(
                InstallJobStatus::Failed,
                Some(&InstallErrorView {
                    context: Some(old_context),
                    ..old_error.clone()
                }),
            )
            .is_err()
        );

        let mut unknown_context =
            InstallErrorContext::new("read cache").build();
        unknown_context.cache_types = vec!["install_jobs".to_string()];
        assert!(
            validated_cache_repair_types_for(
                InstallJobStatus::Failed,
                Some(&InstallErrorView {
                    context: Some(unknown_context),
                    ..old_error
                }),
            )
            .is_err()
        );
    }

    #[test]
    fn cache_repair_validation_accepts_only_whitelisted_terminal_jobs() {
        let mut context = InstallErrorContext::new("read cache").build();
        context.cache_types = vec![
            "curseforge_project".to_string(),
            "curseforge_project".to_string(),
        ];
        let error = InstallErrorView {
            code: "cache_repair_required".to_string(),
            phase: Some(InstallPhaseId::ResolvingPack),
            message: "cache failed".to_string(),
            api: None,
            context: Some(context),
        };
        assert_eq!(
            validated_cache_repair_types_for(
                InstallJobStatus::Interrupted,
                Some(&error),
            )
            .unwrap(),
            vec![crate::state::CacheValueType::CurseForgeProject]
        );
        assert!(
            validated_cache_repair_types_for(
                InstallJobStatus::Running,
                Some(&error),
            )
            .is_err()
        );
    }

    #[test]
    fn missing_required_content_pauses_without_starting_rollback() {
        let mut job_state =
            InstallJobState::new(InstallRequest::DownloadJava {
                vendor: "test".to_string(),
                version: 21,
            });
        job_state.progress.phase = InstallPhaseId::DownloadingContent;
        job_state.cleanup = InstallCleanup::DeleteNewInstance {
            instance_id: Some("same-instance".to_string()),
        };
        let cleanup = job_state.cleanup.clone();
        let reason = InstallPauseReason::MissingRequiredContent {
            failed_files: 2,
            paths: vec!["mods/a.jar".to_string(), "mods/b.jar".to_string()],
        };

        begin_waiting_for_user(&mut job_state, reason.clone());

        assert_eq!(
            job_state.progress.phase,
            InstallPhaseId::DownloadingContent
        );
        assert_eq!(job_state.pause_reason, Some(reason));
        assert_eq!(job_state.cleanup, cleanup);
        assert!(job_state.error.is_none());
        assert!(job_state.rollback.is_none());
        assert!(job_state.events.iter().any(|event| matches!(
            &event.kind,
            InstallJobEventKind::WaitingForUser { .. }
        )));
        assert!(!job_state.events.iter().any(|event| matches!(
            &event.kind,
            InstallJobEventKind::RollbackStarted { .. }
        )));
    }

    #[test]
    fn curseforge_manual_downloads_create_a_recoverable_pause() {
        let manual_download =
            crate::api::curseforge::CurseForgeManualDownload {
                project_id: 123,
                file_id: 456,
                file_name: "mods/manual.jar".to_string(),
                ownership_kind:
                    crate::state::instances::ContentOwnershipKind::PackManaged,
                operation_kind: crate::state::instances::ManualDownloadOperationKind::PackInstall,
                website_url: Some(
                    "https://www.curseforge.com/minecraft/mc-mods/example/download/456"
                        .to_string(),
                ),
                project_type: "mod".to_string(),
                project_slug: "example".to_string(),
                target_folder: "mods".to_string(),
                hashes: Vec::new(),
                file_length: 12,
                file_fingerprint: 34,
            };
        let result = crate::api::curseforge::CurseForgeModpackInstallResult {
            content: crate::api::curseforge::CurseForgeInstallResult {
                manual_downloads: vec![manual_download],
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            curseforge_manual_download_pause(&result, &[]),
            Some(InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                paths: vec!["mods/manual.jar".to_string()],
            })
        );
        assert_eq!(
            curseforge_manual_download_pause(
                &result,
                &["mods/manual.jar".to_string()],
            ),
            None
        );
    }

    #[test]
    fn recovered_manual_world_download_does_not_run_again() {
        let request = crate::api::curseforge::CurseForgeWorldInstallRequest {
            instance_id: "instance".to_string(),
            project_id: 123,
            file_id: 456,
        };
        let mut job_state =
            InstallJobState::new(InstallRequest::InstallCurseForgeWorld {
                request: request.clone(),
                display_title: "World".to_string(),
                display_icon: None,
            });
        job_state.record_event(InstallJobEventKind::ContentFileSkipped {
            path: "saves/world.zip".to_string(),
            reason: "manual download required".to_string(),
            project_id: Some(request.project_id.to_string()),
            version_id: Some(request.file_id.to_string()),
            manual_url: Some("https://www.curseforge.com/download".to_string()),
        });
        job_state.record_event(InstallJobEventKind::ContentFileRecovered {
            path: "saves/world.zip".to_string(),
            bytes: 42,
        });

        assert!(curseforge_world_was_imported_manually(&job_state, &request));
    }

    #[test]
    fn resume_preserves_instance_cleanup_and_existing_pack_checkpoint() {
        let mut job_state =
            InstallJobState::new(InstallRequest::DownloadJava {
                vendor: "test".to_string(),
                version: 21,
            });
        job_state.target = InstallTarget::NewInstance {
            instance_id: Some("same-instance".to_string()),
        };
        job_state.cleanup = InstallCleanup::DeleteNewInstance {
            instance_id: Some("same-instance".to_string()),
        };
        job_state.continuation =
            Some(InstallContinuationState::InstallingPackToExistingInstance {
                disabled_project_ids: vec!["project-a".to_string()],
            });
        job_state.pause_reason =
            Some(InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                paths: vec!["mods/a.jar".to_string()],
            });
        let target = job_state.target.clone();
        let cleanup = job_state.cleanup.clone();
        let continuation = job_state.continuation.clone();

        prepare_resumed_job(&mut job_state);

        assert_eq!(job_state.target, target);
        assert_eq!(job_state.cleanup, cleanup);
        assert_eq!(job_state.continuation, continuation);
        assert!(job_state.pause_reason.is_none());
        assert!(job_state.events.iter().any(|event| matches!(
            &event.kind,
            InstallJobEventKind::JobQueued { .. }
        )));
    }

    #[test]
    fn resumed_job_can_pause_again_without_rollback() {
        let mut job_state =
            InstallJobState::new(InstallRequest::DownloadJava {
                vendor: "test".to_string(),
                version: 21,
            });
        job_state.pause_reason =
            Some(InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                paths: vec!["mods/first.jar".to_string()],
            });
        prepare_resumed_job(&mut job_state);
        begin_waiting_for_user(
            &mut job_state,
            InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                paths: vec!["mods/still-missing.jar".to_string()],
            },
        );

        assert!(matches!(
            job_state.pause_reason,
            Some(InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                ..
            })
        ));
        assert!(!job_state.events.iter().any(|event| matches!(
            &event.kind,
            InstallJobEventKind::RollbackStarted { .. }
        )));
    }

    #[test]
    fn canceling_waiting_jobs_keeps_the_original_cleanup_plan() {
        for cleanup in [
            InstallCleanup::DeleteNewInstance {
                instance_id: Some("new-instance".to_string()),
            },
            InstallCleanup::RestoreExistingInstance {
                instance_id: "existing-instance".to_string(),
            },
        ] {
            let mut job_state =
                InstallJobState::new(InstallRequest::DownloadJava {
                    vendor: "test".to_string(),
                    version: 21,
                });
            job_state.cleanup = cleanup.clone();
            job_state.pause_reason =
                Some(InstallPauseReason::MissingRequiredContent {
                    failed_files: 1,
                    paths: vec!["mods/missing.jar".to_string()],
                });

            begin_canceling_job(&mut job_state);

            assert_eq!(job_state.cleanup, cleanup);
            assert!(job_state.pause_reason.is_none());
            assert_eq!(job_state.progress.phase, InstallPhaseId::RollingBack);
            assert!(job_state.events.iter().any(|event| matches!(
                &event.kind,
                InstallJobEventKind::RollbackStarted {
                    cleanup: event_cleanup,
                } if event_cleanup == &cleanup
            )));
        }
    }

    #[test]
    fn manifest_and_override_errors_remain_fatal() {
        for (phase, message) in [
            (
                InstallPhaseId::ReadingPackManifest,
                "No pack manifest found in mrpack",
            ),
            (InstallPhaseId::ExtractingOverrides, "Invalid override path"),
        ] {
            let mut job_state =
                InstallJobState::new(InstallRequest::DownloadJava {
                    vendor: "test".to_string(),
                    version: 21,
                });
            job_state.progress.phase = phase;
            let error: crate::Error =
                crate::ErrorKind::InputError(message.to_string()).into();

            begin_failed_job_rollback(&mut job_state, &error);

            assert!(job_state.pause_reason.is_none());
            assert_eq!(job_state.progress.phase, InstallPhaseId::RollingBack);
            assert!(job_state.events.iter().any(|event| matches!(
                &event.kind,
                InstallJobEventKind::Failed {
                    phase: failed_phase,
                    ..
                } if *failed_phase == phase
            )));
            assert!(job_state.events.iter().any(|event| matches!(
                &event.kind,
                InstallJobEventKind::RollbackStarted { .. }
            )));
        }
    }

    #[test]
    fn upgrade_failure_preserves_applying_phase_after_successful_rollback() {
        let mut reporter_state = upgrade_job_state();
        reporter_state.set_progress(
            InstallPhaseId::StagingContent,
            None,
            InstallPhaseDetails::Empty,
        );
        let mut execution_state = reporter_state.clone();
        execution_state.set_progress(
            InstallPhaseId::ApplyingContent,
            None,
            InstallPhaseDetails::Empty,
        );
        let failed_phase =
            latest_failure_phase(&execution_state, &reporter_state);
        let mut terminal_state = reporter_state;
        terminal_state.progress.phase = failed_phase;
        let error: crate::Error = crate::ErrorKind::InputError(
            "Injected unmanaged instance upgrade failure after 1 mutation(s)"
                .to_string(),
        )
        .into();

        begin_failed_job_rollback(&mut terminal_state, &error);
        recovery::finalize_rollback_state(&mut terminal_state, true);
        let status = InstallJobStatus::Failed;

        assert_eq!(status, InstallJobStatus::Failed);
        assert_eq!(terminal_state.progress.phase, InstallPhaseId::Finalizing);
        assert_eq!(
            terminal_state.error.as_ref().and_then(|error| error.phase),
            Some(InstallPhaseId::ApplyingContent)
        );
        assert!(terminal_state.rollback_error.is_none());
    }

    #[test]
    fn interrupted_upgrade_uses_terminal_phase_after_successful_recovery() {
        let mut job_state = upgrade_job_state();
        job_state.progress.phase = InstallPhaseId::RollingBack;
        job_state.error = Some(InstallErrorView::from_message(
            "app_closed",
            InstallPhaseId::ApplyingContent,
            "App closed while install was running",
        ));

        recovery::finalize_rollback_state(&mut job_state, true);
        let status = InstallJobStatus::Interrupted;

        assert_eq!(status, InstallJobStatus::Interrupted);
        assert_eq!(job_state.progress.phase, InstallPhaseId::Finalizing);
        assert!(job_state.rollback_error.is_none());
    }

    #[test]
    fn rollback_failure_keeps_recovery_phase_and_error() {
        let mut job_state = upgrade_job_state();
        job_state.progress.phase = InstallPhaseId::RollingBack;
        job_state.rollback_error = Some(InstallErrorView::from_message(
            "rollback_error",
            InstallPhaseId::RollingBack,
            "rollback failed",
        ));
        job_state.record_event(InstallJobEventKind::RollbackFailed {
            message: "rollback failed".to_string(),
        });

        recovery::finalize_rollback_state(&mut job_state, false);

        assert_eq!(job_state.progress.phase, InstallPhaseId::RollingBack);
        assert_eq!(
            job_state
                .rollback_error
                .as_ref()
                .and_then(|error| error.phase),
            Some(InstallPhaseId::RollingBack)
        );
        assert!(job_state.events.iter().any(|event| matches!(
            event.kind,
            InstallJobEventKind::RollbackFailed { .. }
        )));
        assert!(!job_state.events.iter().any(|event| matches!(
            event.kind,
            InstallJobEventKind::RollbackCompleted
        )));
    }

    #[test]
    fn instance_upgrade_direct_backup_honors_enabled_option() {
        assert!(should_create_upgrade_backup(
            true,
            SharedUpgradeMode::Direct
        ));
    }

    #[test]
    fn instance_upgrade_direct_backup_can_be_disabled() {
        assert!(!should_create_upgrade_backup(
            false,
            SharedUpgradeMode::Direct
        ));
    }

    #[test]
    fn instance_upgrade_copy_never_creates_second_backup() {
        assert!(!should_create_upgrade_backup(
            true,
            SharedUpgradeMode::CopyAndUpgrade
        ));
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn upgrade_backup_clones_authoritative_content_metadata() {
        crate::event::EventState::init().await.unwrap();
        let root = tempfile::tempdir().unwrap().keep();
        let state = State::init_for_test(root.to_string_lossy().to_string())
            .await
            .unwrap();
        let source = crate::api::instance::create(
            "T13 Source".to_string(),
            "1.21.8".to_string(),
            ModLoader::Fabric,
            Some("0.17.2".to_string()),
            None,
            InstanceLink::Unmanaged,
            None,
            None,
        )
        .await
        .unwrap();
        let source_id = source.instance.id.clone();
        let source_base = state
            .directories
            .instances_dir()
            .join(&source.instance.path);
        crate::util::io::create_dir_all(source_base.join("mods"))
            .await
            .unwrap();
        let content = [
            (
                "mods/sodium.jar",
                b"sodium".as_slice(),
                "AANobbMI",
                "7pwil2dy",
                crate::state::instances::ContentOwnershipKind::UserAdded,
            ),
            (
                "mods/lithium.jar",
                b"lithium".as_slice(),
                "gvQqBUqZ",
                "qxIL7Kb8",
                crate::state::instances::ContentOwnershipKind::PackManaged,
            ),
        ];
        for (relative_path, bytes, project_id, release_id, ownership) in content
        {
            let path = source_base.join(relative_path);
            crate::util::io::write(&path, bytes).await.unwrap();
            let (_, sha1) =
                crate::util::fetch::sha1_file_async(&path).await.unwrap();
            let provider_ref = ContentProviderRef::Modrinth {
                project_id: crate::state::ModrinthProjectId::new(project_id)
                    .unwrap(),
                version_id: Some(
                    crate::state::ModrinthVersionId::new(release_id).unwrap(),
                ),
            };
            crate::state::record_project_file_atomic(
                &source_id,
                relative_path,
                &sha1,
                bytes.len() as u64,
                crate::state::ProjectType::Mod,
                crate::state::instances::ContentSourceKind::Local,
                ownership,
                Some(&provider_ref),
                true,
                None,
                &state,
            )
            .await
            .unwrap();
        }
        let source_entries = crate::state::instances::adapters::sqlite::content_rows::get_content_entries(
            &source.applied_content_set.id,
            &state.pool,
        )
        .await
        .unwrap();
        let source_files = crate::state::instances::adapters::sqlite::content_rows::get_instance_files(
            &source_id,
            &state.pool,
        )
        .await
        .unwrap();
        let paths_by_file = source_files
            .iter()
            .map(|file| (file.id.as_str(), file.relative_path.as_str()))
            .collect::<HashMap<_, _>>();
        let entry_by_path = source_entries
            .iter()
            .map(|entry| {
                (paths_by_file[entry.file_id.as_deref().unwrap()], entry)
            })
            .collect::<HashMap<_, _>>();
        let sodium = entry_by_path["mods/sodium.jar"];
        let lithium = entry_by_path["mods/lithium.jar"];
        crate::state::instances::adapters::sqlite::content_rows::upsert_content_provider_ref(
            &sodium.id,
            &ContentProviderRef::CurseForge {
                project_id: crate::state::CurseForgeProjectId::new(394468)
                    .unwrap(),
                file_id: Some(
                    crate::state::CurseForgeFileId::new(6853381).unwrap(),
                ),
            },
            false,
            &state.pool,
        )
        .await
        .unwrap();
        crate::state::instances::adapters::sqlite::content_rows::set_content_entry_auto_dependency(
            &lithium.id,
            true,
            &state.pool,
        )
        .await
        .unwrap();
        crate::state::instances::commands::toggle_content_entries(
            &source_id,
            std::slice::from_ref(&lithium.id),
            Some(false),
            &state,
        )
        .await
        .unwrap();
        let mut tx = state.pool.begin().await.unwrap();
        let now = chrono::Utc::now();
        crate::state::instances::adapters::sqlite::content_rows::upsert_content_dependency_edge_in_transaction(
            &crate::state::instances::ContentDependencyEdge {
                id: format!("content-dependency:{}", Uuid::new_v4()),
                content_set_id: source.applied_content_set.id.clone(),
                parent_entry_id: sodium.id.clone(),
                child_entry_id: lithium.id.clone(),
                evidence_provider: ContentProvider::Modrinth,
                parent_provider: ContentProvider::Modrinth,
                child_provider: ContentProvider::Modrinth,
                dependency_kind: crate::state::instances::ContentDependencyKind::Required,
                parent_project_id: "AANobbMI".to_string(),
                parent_release_id: "7pwil2dy".to_string(),
                child_project_id: "gvQqBUqZ".to_string(),
                child_release_id: "qxIL7Kb8".to_string(),
                created_at: now,
                modified_at: now,
            },
            &mut tx,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let job_id = Uuid::new_v4();
        let mut job_state = upgrade_job_state();
        job_state.target = InstallTarget::ExistingInstance {
            instance_id: source_id.clone(),
        };
        job_state.cleanup = InstallCleanup::RestoreExistingInstance {
            instance_id: source_id.clone(),
        };
        store::insert(job_id, &job_state, InstallJobStatus::Running, &state)
            .await
            .unwrap();

        let backup_id =
            create_upgrade_backup(job_id, &job_state, &state, &source_id, None)
                .await
                .unwrap();
        let snapshot = crate::state::instances::commands::get_content_snapshot(
            &backup_id, false, &state,
        )
        .await
        .unwrap();
        let sodium_snapshot = snapshot
            .items
            .iter()
            .find(|item| {
                item.provider_project_id.as_deref() == Some("AANobbMI")
            })
            .unwrap();
        let lithium_snapshot = snapshot
            .items
            .iter()
            .find(|item| {
                item.provider_project_id.as_deref() == Some("gvQqBUqZ")
            })
            .unwrap();
        assert_eq!(
            sodium_snapshot.provider_release_id.as_deref(),
            Some("7pwil2dy")
        );
        assert_eq!(
            lithium_snapshot.provider_release_id.as_deref(),
            Some("qxIL7Kb8")
        );
        assert_eq!(
            sodium_snapshot.ownership_kind,
            crate::state::instances::ContentOwnershipKind::UserAdded
        );
        assert_eq!(
            lithium_snapshot.ownership_kind,
            crate::state::instances::ContentOwnershipKind::PackManaged
        );
        assert!(
            lithium_snapshot
                .dependency
                .as_ref()
                .unwrap()
                .auto_dependency
        );

        let backup = crate::state::get_instance(&backup_id, &state.pool)
            .await
            .unwrap()
            .unwrap();
        let backup_entries = crate::state::instances::adapters::sqlite::content_rows::get_content_entries(
            &backup.applied_content_set.id,
            &state.pool,
        )
        .await
        .unwrap();
        let backup_files = crate::state::instances::adapters::sqlite::content_rows::get_instance_files(
            &backup_id,
            &state.pool,
        )
        .await
        .unwrap();
        let backup_paths = backup_files
            .iter()
            .map(|file| (file.id.as_str(), file.relative_path.as_str()))
            .collect::<HashMap<_, _>>();
        let backup_by_project = backup_entries
            .iter()
            .map(|entry| {
                let path = backup_paths[entry.file_id.as_deref().unwrap()];
                (path, entry)
            })
            .collect::<HashMap<_, _>>();
        let backup_sodium = backup_by_project["mods/sodium.jar"];
        let backup_lithium = backup_by_project["mods/lithium.jar.disabled"];
        assert!(!backup_lithium.enabled);
        let refs = crate::state::instances::adapters::sqlite::content_rows::get_content_provider_refs_with_origin(
            &backup_sodium.id,
            &state.pool,
        )
        .await
        .unwrap();
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().any(|(provider_ref, origin)| {
            *origin
                && matches!(
                    provider_ref,
                    ContentProviderRef::Modrinth { project_id, version_id }
                        if project_id.to_string() == "AANobbMI"
                            && version_id.as_ref().map(ToString::to_string).as_deref()
                                == Some("7pwil2dy")
                )
        }));
        assert!(refs.iter().any(|(provider_ref, origin)| {
            !origin
                && matches!(
                    provider_ref,
                    ContentProviderRef::CurseForge {
                        project_id,
                        file_id,
                    } if project_id.get() == 394468
                        && file_id.as_ref().is_some_and(|id| id.get() == 6853381)
                )
        }));
        let edges = crate::state::instances::adapters::sqlite::content_rows::get_content_dependency_edges(
            &backup.applied_content_set.id,
            &state.pool,
        )
        .await
        .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].parent_entry_id, backup_sodium.id);
        assert_eq!(edges[0].child_entry_id, backup_lithium.id);
        assert_eq!(edges[0].parent_release_id, "7pwil2dy");
        assert_eq!(edges[0].child_release_id, "qxIL7Kb8");
    }

    #[cfg(not(feature = "tauri"))]
    #[tokio::test]
    async fn upgrade_applies_solver_and_non_solver_physical_actions() {
        crate::event::EventState::init().await.unwrap();
        let state = if State::initialized() {
            State::get().await.unwrap()
        } else {
            let root = tempfile::tempdir().unwrap().keep();
            State::init_for_test(root.to_string_lossy().to_string())
                .await
                .unwrap()
        };
        let instance = crate::api::instance::create(
            format!("Upgrade physical actions {}", Uuid::new_v4()),
            "1.21.8".to_string(),
            ModLoader::Fabric,
            Some("0.17.2".to_string()),
            None,
            InstanceLink::Unmanaged,
            None,
            None,
        )
        .await
        .unwrap();
        let instance_id = instance.instance.id.clone();
        let base = state
            .directories
            .instances_dir()
            .join(&instance.instance.path);
        crate::util::io::create_dir_all(base.join("mods"))
            .await
            .unwrap();

        let recognized = [
            (
                "mods/solver-upgrade.jar",
                b"old-upgrade".as_slice(),
                "AANobbMI",
                "7pwil2dy",
            ),
            (
                "mods/solver-keep.jar",
                b"old-keep".as_slice(),
                "gvQqBUqZ",
                "qxIL7Kb8",
            ),
            (
                "mods/solver-disable.jar",
                b"old-disable".as_slice(),
                "mOgUt4GM",
                "oldRelease",
            ),
        ];
        for (relative_path, bytes, project_id, release_id) in recognized {
            let path = base.join(relative_path);
            crate::util::io::write(&path, bytes).await.unwrap();
            let (_, sha1) =
                crate::util::fetch::sha1_file_async(&path).await.unwrap();
            crate::state::record_project_file_atomic(
                &instance_id,
                relative_path,
                &sha1,
                bytes.len() as u64,
                crate::state::ProjectType::Mod,
                crate::state::instances::ContentSourceKind::Local,
                crate::state::instances::ContentOwnershipKind::UserAdded,
                Some(&ContentProviderRef::Modrinth {
                    project_id: crate::state::ModrinthProjectId::new(
                        project_id,
                    )
                    .unwrap(),
                    version_id: Some(
                        crate::state::ModrinthVersionId::new(release_id)
                            .unwrap(),
                    ),
                }),
                true,
                None,
                &state,
            )
            .await
            .unwrap();
        }
        for (relative_path, bytes) in [
            ("mods/local-keep.jar.disabled", b"local-keep".as_slice()),
            ("mods/local-disable.jar", b"local-disable".as_slice()),
            (
                "mods/unsupported-disable.jar",
                b"unsupported-disable".as_slice(),
            ),
            (
                "mods/external-disable.jar",
                b"externally-modified".as_slice(),
            ),
        ] {
            crate::util::io::write(&base.join(relative_path), bytes)
                .await
                .unwrap();
        }

        let entries = crate::state::instances::adapters::sqlite::content_rows::get_content_entries(
            &instance.applied_content_set.id,
            &state.pool,
        )
        .await
        .unwrap();
        let files = crate::state::instances::adapters::sqlite::content_rows::get_instance_files(
            &instance_id,
            &state.pool,
        )
        .await
        .unwrap();
        let paths = files
            .iter()
            .map(|file| (file.id.as_str(), file.relative_path.as_str()))
            .collect::<HashMap<_, _>>();
        let content_ids = entries
            .iter()
            .map(|entry| {
                (
                    paths[entry.file_id.as_deref().unwrap()].to_string(),
                    entry.id.clone(),
                )
            })
            .collect::<HashMap<_, _>>();
        let upgrade_id = content_ids["mods/solver-upgrade.jar"].clone();
        let keep_id = content_ids["mods/solver-keep.jar"].clone();
        let disable_id = content_ids["mods/solver-disable.jar"].clone();

        let items = vec![
            physical_test_item(
                &upgrade_id,
                "mods/solver-upgrade.jar",
                crate::state::InstanceUpgradeItemStatus::UpgradeAvailable,
                InstanceUpgradeAction::Upgrade,
                true,
            ),
            physical_test_item(
                &keep_id,
                "mods/solver-keep.jar",
                crate::state::InstanceUpgradeItemStatus::NoCompatibleRelease,
                InstanceUpgradeAction::Keep,
                true,
            ),
            physical_test_item(
                &disable_id,
                "mods/solver-disable.jar",
                crate::state::InstanceUpgradeItemStatus::NoCompatibleRelease,
                InstanceUpgradeAction::Disable,
                true,
            ),
            physical_test_item(
                "local-keep",
                "mods/local-keep.jar.disabled",
                crate::state::InstanceUpgradeItemStatus::Unidentified,
                InstanceUpgradeAction::Keep,
                false,
            ),
            physical_test_item(
                "local-disable",
                "mods/local-disable.jar",
                crate::state::InstanceUpgradeItemStatus::Unidentified,
                InstanceUpgradeAction::Disable,
                true,
            ),
            physical_test_item(
                "unsupported-disable",
                "mods/unsupported-disable.jar",
                crate::state::InstanceUpgradeItemStatus::UnsupportedContentType,
                InstanceUpgradeAction::Disable,
                true,
            ),
            physical_test_item(
                "external-disable",
                "mods/external-disable.jar",
                crate::state::InstanceUpgradeItemStatus::Unidentified,
                InstanceUpgradeAction::Disable,
                true,
            ),
        ];
        let selections = vec![
            crate::state::InstanceUpgradeSelection {
                content_id: upgrade_id,
                provider: Some(ContentProvider::Modrinth),
                project_id: Some("AANobbMI".to_string()),
                current_release_id: Some("7pwil2dy".to_string()),
                target_release_id: Some("vf7UgZpC".to_string()),
                action: InstanceUpgradeAction::Upgrade,
                enabled: true,
            },
            crate::state::InstanceUpgradeSelection {
                content_id: keep_id,
                provider: Some(ContentProvider::Modrinth),
                project_id: Some("gvQqBUqZ".to_string()),
                current_release_id: Some("qxIL7Kb8".to_string()),
                target_release_id: None,
                action: InstanceUpgradeAction::Keep,
                enabled: true,
            },
            crate::state::InstanceUpgradeSelection {
                content_id: disable_id,
                provider: Some(ContentProvider::Modrinth),
                project_id: Some("mOgUt4GM".to_string()),
                current_release_id: Some("oldRelease".to_string()),
                target_release_id: None,
                action: InstanceUpgradeAction::Disable,
                enabled: false,
            },
        ];
        let execution = physical_test_execution(items, selections);
        let staged_path = state
            .directories
            .caches_dir()
            .join(format!("upgrade-test-{}.jar", Uuid::new_v4()));
        crate::util::io::create_dir_all(
            staged_path.parent().expect("staged path has parent"),
        )
        .await
        .unwrap();
        crate::util::io::write(&staged_path, b"target-upgrade")
            .await
            .unwrap();
        let (_, staged_sha1) =
            crate::util::fetch::sha1_file_async(&staged_path)
                .await
                .unwrap();
        let staged = vec![StagedUpgradeMutation {
            existing_path: Some("mods/solver-upgrade.jar".to_string()),
            target_path: "mods/solver-upgrade.jar".to_string(),
            ownership: crate::state::instances::ContentOwnershipKind::UserAdded,
            auto_dependency: false,
            enabled: true,
            download: StagedUpgradeDownload::Modrinth(
                crate::state::instances::commands::DownloadedProjectVersion {
                    file_name: "solver-upgrade.jar".to_string(),
                    path: staged_path,
                    sha1: staged_sha1,
                    size: b"target-upgrade".len() as u64,
                    project_type: crate::state::ProjectType::Mod,
                    project_id: "AANobbMI".to_string(),
                    version_id: "vf7UgZpC".to_string(),
                },
            ),
        }];

        apply_upgrade_content(
            &instance_id,
            staged,
            &execution,
            &HashSet::from(["mods/external-disable.jar".to_string()]),
            &[],
            &state,
        )
        .await
        .unwrap();

        assert_eq!(
            tokio::fs::read(base.join("mods/solver-upgrade.jar"))
                .await
                .unwrap(),
            b"target-upgrade"
        );
        assert_eq!(
            tokio::fs::read(base.join("mods/solver-keep.jar"))
                .await
                .unwrap(),
            b"old-keep"
        );
        assert!(base.join("mods/solver-disable.jar.disabled").exists());
        assert!(base.join("mods/local-keep.jar.disabled").exists());
        assert!(base.join("mods/local-disable.jar.disabled").exists());
        assert!(base.join("mods/unsupported-disable.jar.disabled").exists());
        assert!(base.join("mods/external-disable.jar").exists());
        assert!(!base.join("mods/external-disable.jar.disabled").exists());
        assert_eq!(
            tokio::fs::read(base.join("mods/external-disable.jar"))
                .await
                .unwrap(),
            b"externally-modified"
        );
    }

    #[test]
    fn instance_upgrade_external_add_is_classified() {
        assert_eq!(
            classify_upgrade_external_change(false, true),
            InstanceUpgradeExternalChangeKind::Added
        );
    }

    #[test]
    fn instance_upgrade_external_remove_is_classified() {
        assert_eq!(
            classify_upgrade_external_change(true, false),
            InstanceUpgradeExternalChangeKind::Removed
        );
    }

    #[test]
    fn instance_upgrade_external_modify_is_classified() {
        assert_eq!(
            classify_upgrade_external_change(true, true),
            InstanceUpgradeExternalChangeKind::Modified
        );
    }

    #[test]
    fn instance_upgrade_full_scan_detects_added_file() {
        let changes = diff_upgrade_source_files(
            &[],
            &[upgrade_source_file("mods/new.jar", "new", true)],
        );
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, InstanceUpgradeExternalChangeKind::Added);
    }

    #[test]
    fn instance_upgrade_full_scan_detects_removed_file() {
        let changes = diff_upgrade_source_files(
            &[upgrade_source_file("mods/old.jar", "old", true)],
            &[],
        );
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, InstanceUpgradeExternalChangeKind::Removed);
    }

    #[test]
    fn instance_upgrade_full_scan_detects_enabled_state_change() {
        let changes = diff_upgrade_source_files(
            &[upgrade_source_file("mods/mod.jar", "same", true)],
            &[upgrade_source_file("mods/mod.jar", "same", false)],
        );
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].kind,
            InstanceUpgradeExternalChangeKind::Modified
        );
    }

    #[test]
    fn instance_upgrade_reports_external_edit_after_launcher_mutation() {
        let source = upgrade_source_file("mods/lithium.jar", "old", true);
        let expected = upgrade_source_file("mods/lithium.jar", "target", true);
        let current = upgrade_source_file("mods/lithium.jar", "user", true);
        let changes = final_upgrade_external_changes(
            &[source],
            &[current],
            &HashMap::from([("mods/lithium.jar".to_string(), Some(expected))]),
        );

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].relative_path, "mods/lithium.jar");
        assert_eq!(
            changes[0].kind,
            InstanceUpgradeExternalChangeKind::Modified
        );
    }

    #[test]
    fn instance_upgrade_reports_external_edit_of_skipped_mutation() {
        let changes = final_upgrade_external_changes(
            &[upgrade_source_file("mods/sodium.jar", "old", true)],
            &[upgrade_source_file("mods/sodium.jar", "user", true)],
            &HashMap::new(),
        );

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].relative_path, "mods/sodium.jar");
    }

    #[test]
    fn instance_upgrade_reports_external_add_but_not_launcher_write() {
        let changes = final_upgrade_external_changes(
            &[],
            &[
                upgrade_source_file("mods/dependency.jar", "target", true),
                upgrade_source_file(
                    "mods/t16-external-added.jar",
                    "user",
                    true,
                ),
            ],
            &HashMap::from([(
                "mods/dependency.jar".to_string(),
                Some(upgrade_source_file(
                    "mods/dependency.jar",
                    "target",
                    true,
                )),
            )]),
        );

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].relative_path, "mods/t16-external-added.jar");
        assert_eq!(changes[0].kind, InstanceUpgradeExternalChangeKind::Added);
    }

    #[test]
    fn instance_upgrade_external_changes_coalesce_with_skipped_conflicts() {
        let mut changes = vec![InstanceUpgradeExternalChange {
            relative_path: "mods/sodium.jar".to_string(),
            kind: InstanceUpgradeExternalChangeKind::Modified,
        }];
        merge_upgrade_external_changes(
            &mut changes,
            vec![
                InstanceUpgradeExternalChange {
                    relative_path: "mods/sodium.jar".to_string(),
                    kind: InstanceUpgradeExternalChangeKind::Modified,
                },
                InstanceUpgradeExternalChange {
                    relative_path: "mods/t16-external-added.jar".to_string(),
                    kind: InstanceUpgradeExternalChangeKind::Added,
                },
            ],
        );

        assert_eq!(changes.len(), 2);
        assert!(changes.iter().any(|change| {
            change.relative_path == "mods/sodium.jar"
                && change.kind == InstanceUpgradeExternalChangeKind::Modified
        }));
    }

    #[test]
    fn instance_upgrade_external_target_conflict_skips_mutation() {
        let mutation =
            test_upgrade_mutation(Some("mods/old.jar"), "mods/new.jar");
        assert!(upgrade_mutation_conflicts(
            &mutation,
            &HashSet::from(["mods/new.jar".to_string()])
        ));
    }

    #[test]
    fn instance_upgrade_external_delete_conflict_skips_mutation() {
        let mutation =
            test_upgrade_mutation(Some("mods/old.jar"), "mods/new.jar");
        assert!(upgrade_mutation_conflicts(
            &mutation,
            &HashSet::from(["mods/old.jar".to_string()])
        ));
    }

    #[test]
    fn instance_upgrade_unrelated_external_change_does_not_skip_mutation() {
        let mutation =
            test_upgrade_mutation(Some("mods/old.jar"), "mods/new.jar");
        assert!(!upgrade_mutation_conflicts(
            &mutation,
            &HashSet::from(["mods/user.jar".to_string()])
        ));
    }

    fn test_upgrade_mutation(
        existing_path: Option<&str>,
        target_path: &str,
    ) -> StagedUpgradeMutation {
        StagedUpgradeMutation {
            existing_path: existing_path.map(ToString::to_string),
            target_path: target_path.to_string(),
            ownership: crate::state::instances::ContentOwnershipKind::UserAdded,
            auto_dependency: false,
            enabled: true,
            download: StagedUpgradeDownload::Modrinth(
                crate::state::instances::commands::DownloadedProjectVersion {
                    file_name: "new.jar".to_string(),
                    path: PathBuf::from("new.jar"),
                    sha1: "sha1".to_string(),
                    size: 1,
                    project_type: crate::state::ProjectType::Mod,
                    project_id: "project".to_string(),
                    version_id: "version".to_string(),
                },
            ),
        }
    }

    #[test]
    fn instance_upgrade_staging_populates_persisted_download_summary() {
        let staged = (0..27)
            .map(|index| {
                test_upgrade_mutation(
                    Some(&format!("mods/old-{index}.jar")),
                    &format!("mods/target-{index}.jar"),
                )
            })
            .collect::<Vec<_>>();
        let mut job = InstallJobState::new(InstallRequest::DownloadJava {
            vendor: "test".to_string(),
            version: 21,
        });
        job.record_event(InstallJobEventKind::ContentDownloadStarted {
            files: staged.len() as u64,
            bytes: Some(staged.len() as u64),
        });
        for mutation in staged {
            job.record_event(InstallJobEventKind::ContentFileCompleted {
                path: mutation.target_path,
                bytes: 1,
            });
        }

        let persisted = serde_json::to_string(&job).unwrap();
        let restored: InstallJobState =
            serde_json::from_str(&persisted).unwrap();
        let summary = restored.download_summary();
        assert_eq!(summary.files_completed, 27);
        assert_eq!(summary.files_total, Some(27));
        assert_eq!(summary.bytes_downloaded, 27);
        assert_eq!(summary.bytes_total, Some(27));
    }

    #[tokio::test]
    async fn upgrade_staging_scheduler_enters_requests_concurrently() {
        use std::sync::Arc;
        use tokio::sync::Barrier;

        let barrier = Arc::new(Barrier::new(2));
        let mut downloads = (0..2)
            .map(|index| {
                let barrier = barrier.clone();
                async move {
                    barrier.wait().await;
                    Ok::<_, crate::Error>((index, index))
                }
            })
            .collect::<FuturesUnordered<_>>();

        assert_eq!(
            collect_ordered_upgrade_staging(&mut downloads)
                .await
                .unwrap(),
            vec![0, 1]
        );
    }

    #[tokio::test]
    async fn upgrade_staging_scheduler_restores_request_order() {
        let mut downloads = [2_usize, 0, 1]
            .into_iter()
            .map(|index| async move {
                Ok::<_, crate::Error>((index, format!("mutation-{index}")))
            })
            .collect::<FuturesUnordered<_>>();

        assert_eq!(
            collect_ordered_upgrade_staging(&mut downloads)
                .await
                .unwrap(),
            vec!["mutation-0", "mutation-1", "mutation-2"]
        );
    }

    #[tokio::test]
    async fn upgrade_staging_scheduler_returns_first_error() {
        let mut downloads = [
            Ok((0, "first")),
            Err(crate::ErrorKind::InputError("failed".into()).into()),
        ]
        .into_iter()
        .map(std::future::ready)
        .collect::<FuturesUnordered<_>>();

        assert!(
            collect_ordered_upgrade_staging(&mut downloads)
                .await
                .is_err()
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn instance_upgrade_debug_hook_ignores_invalid_and_zero_counts() {
        assert_eq!(debug_mutation_count(""), None);
        assert_eq!(debug_mutation_count("invalid"), None);
        assert_eq!(debug_mutation_count("0"), None);
        assert_eq!(debug_mutation_count(" 2 "), Some(2));
    }

    fn upgrade_source_file(
        relative_path: &str,
        sha1: &str,
        enabled: bool,
    ) -> crate::state::InstanceUpgradeSourceFile {
        crate::state::InstanceUpgradeSourceFile {
            relative_path: relative_path.to_string(),
            sha1: sha1.to_string(),
            size: 1,
            enabled,
        }
    }
}
