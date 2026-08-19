use super::events::{InstallProgressReporter, emit_install_job};
use super::model::{
    InstallCleanup, InstallContinuationState, InstallErrorContext,
    InstallErrorView, InstallJavaStep, InstallJobDisplay, InstallJobEventKind,
    InstallJobSnapshot, InstallJobState, InstallJobStatus, InstallPauseReason,
    InstallPhaseDetails, InstallPhaseId, InstallPostInstallEdit,
    InstallProgress, InstallRequest, InstallRollbackState, InstallTarget,
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
    ContentProviderRef, InstanceInstallStage, InstanceLink, ModLoader, State,
};
use crate::util::fetch::DownloadReason;
use std::collections::HashSet;
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
    start(InstallRequest::CreateInstance {
        name,
        game_version,
        loader,
        loader_version,
        icon_path,
        link,
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
            Ok(()) => {
                job.state
                    .record_event(InstallJobEventKind::RollbackCompleted);
                true
            }
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
        store::insert(id, &job_state, InstallJobStatus::Queued, &state).await?;
    emit_install_job(&record.snapshot()).await?;
    spawn_job(id);
    Ok(record.snapshot())
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
            icon_path,
            link,
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
                job_state.request = InstallRequest::CreateInstance {
                    name: name.clone(),
                    game_version: game_version.clone(),
                    loader,
                    loader_version: loader_version.clone(),
                    icon_path: icon_path.clone(),
                    link: link.clone(),
                };
            }
            let metadata = crate::api::instance::create(
                name,
                game_version,
                loader,
                loader_version,
                icon_path,
                link,
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
            )
            .await?;
            set_display(
                job_state,
                created.instance.name,
                created.instance.icon_path,
            );
            set_instance_id(job_state, created.instance.id);
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
    job_state = live_reporter.current_state().await?;

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
            job_state.progress.phase = InstallPhaseId::Finalizing;
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
            if let Some(instance_id) = instance_id
                && let Err(error) =
                    emit_instance(&instance_id, InstancePayloadType::Edited)
                        .await
            {
                tracing::warn!(
                    job_id = %job_id,
                    instance_id,
                    error = %error,
                    "Install job succeeded, but its final instance event could not be emitted"
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
                Ok(()) => {
                    job_state
                        .record_event(InstallJobEventKind::RollbackCompleted);
                    true
                }
            };
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
            Ok(()) => {
                job_state.record_event(InstallJobEventKind::RollbackCompleted);
                true
            }
        };
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
            icon_path: _,
            link,
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
                        game_version,
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
                Some(reporter),
                crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
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
            PreparingInstance | Finalizing => "instance_error",
            ResolvingPack | DownloadingPackFile | ReadingPackManifest => {
                "pack_error"
            }
            DownloadingContent => "content_error",
            ExtractingOverrides => "path_error",
            PreparingJava => "java_error",
            DownloadingMinecraft => "instance_error",
            RollingBack => "rollback_error",
            ResolvingMinecraft | ResolvingLoader | RunningLoaderProcessors => {
                "launcher_error"
            }
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
}
