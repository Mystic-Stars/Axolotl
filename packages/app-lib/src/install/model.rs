use crate::api::curseforge::{
    CurseForgeInstallRequest, CurseForgeWorldInstallRequest,
};
use crate::api::pack::import::ImportLauncherType;
use crate::api::pack::install_from::{CreatePackInstance, CreatePackLocation};
use crate::state::{
    ContentEntry, ContentProviderRef, ContentUpdateCheck, InstanceFile,
    InstanceInstallStage, InstanceLink, InstanceMetadata, ModLoader,
    PackMember,
};
use chrono::{DateTime, Utc};
use modrinth_content_management::{ContentType, ResolutionPreferences};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

pub type InstallModpackPreview = CreatePackInstance;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallPauseReason {
    MissingRequiredContent {
        failed_files: u64,
        paths: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallContinuationState {
    InstallingPackToExistingInstance { disabled_project_ids: Vec<String> },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstallJobState {
    pub schema_version: u32,
    pub request: InstallRequest,
    pub target: InstallTarget,
    pub cleanup: InstallCleanup,
    pub progress: InstallProgressState,
    pub paths: InstallJobPaths,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<InstallErrorContext>,
    #[serde(default)]
    pub events: Vec<InstallJobEvent>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub active_downloads: HashMap<String, ActiveDownloadState>,
    #[serde(default)]
    pub display: Option<InstallJobDisplay>,
    pub rollback: Option<InstallRollbackState>,
    pub error: Option<InstallErrorView>,
    #[serde(default)]
    pub rollback_error: Option<InstallErrorView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_reason: Option<InstallPauseReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<InstallContinuationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_content: Option<MissingModpackContentState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_missing_content_paths: Vec<String>,
    /// Incrementally maintained download items, mirroring the replay of
    /// `events`. Skipped during (de)serialization because it is derivable
    /// from `events`; rebuilt lazily on first access after a load.
    #[serde(skip)]
    download_items_cache: std::sync::Mutex<Option<Vec<DownloadItemSnapshot>>>,
    #[serde(skip)]
    summary_cache: std::sync::Mutex<Option<DownloadJobSummary>>,
}

impl Clone for InstallJobState {
    fn clone(&self) -> Self {
        Self {
            schema_version: self.schema_version,
            request: self.request.clone(),
            target: self.target.clone(),
            cleanup: self.cleanup.clone(),
            progress: self.progress.clone(),
            paths: self.paths.clone(),
            context: self.context.clone(),
            events: self.events.clone(),
            active_downloads: self.active_downloads.clone(),
            display: self.display.clone(),
            rollback: self.rollback.clone(),
            error: self.error.clone(),
            rollback_error: self.rollback_error.clone(),
            pause_reason: self.pause_reason.clone(),
            continuation: self.continuation.clone(),
            missing_content: self.missing_content.clone(),
            skipped_missing_content_paths: self
                .skipped_missing_content_paths
                .clone(),
            download_items_cache: std::sync::Mutex::new(None),
            summary_cache: std::sync::Mutex::new(None),
        }
    }
}

impl InstallJobState {
    pub fn new(request: InstallRequest) -> Self {
        let target = request.target();
        let cleanup = request.cleanup();
        let kind = request.kind();
        let phase = InstallPhaseId::PreparingInstance;

        Self {
            schema_version: 1,
            request,
            target,
            cleanup,
            progress: InstallProgressState {
                phase,
                progress: None,
                details: InstallPhaseDetails::Empty,
            },
            paths: InstallJobPaths::default(),
            context: None,
            events: vec![InstallJobEvent {
                at: Utc::now(),
                kind: InstallJobEventKind::JobQueued { kind },
            }],
            active_downloads: HashMap::new(),
            display: None,
            rollback: None,
            error: None,
            rollback_error: None,
            pause_reason: None,
            continuation: None,
            missing_content: None,
            skipped_missing_content_paths: Vec::new(),
            download_items_cache: std::sync::Mutex::new(None),
            summary_cache: std::sync::Mutex::new(None),
        }
    }

    /// Invalidates the lazily built download item and summary caches.
    pub(crate) fn invalidate_download_caches(&mut self) {
        *self.download_items_cache.lock().unwrap() = None;
        *self.summary_cache.lock().unwrap() = None;
    }

    pub fn record_event(&mut self, kind: InstallJobEventKind) {
        if matches!(
            &kind,
            InstallJobEventKind::JobStarted
                | InstallJobEventKind::JobSucceeded { .. }
                | InstallJobEventKind::JobCanceled { .. }
                | InstallJobEventKind::WaitingForUser { .. }
        ) {
            self.active_downloads.clear();
        }
        self.events.push(InstallJobEvent {
            at: Utc::now(),
            kind,
        });
        self.invalidate_download_caches();
    }

    pub fn compact_transient_download_events(&mut self) {
        self.events.retain(|event| {
            !matches!(
                event.kind,
                InstallJobEventKind::DownloadRequestStarted { .. }
                    | InstallJobEventKind::DownloadRequestFinished { .. }
                    | InstallJobEventKind::DownloadRequestFailed { .. }
            )
        });
        self.invalidate_download_caches();
    }

    pub fn set_context(&mut self, context: Option<InstallErrorContext>) {
        self.context = context;
    }

    pub fn set_progress(
        &mut self,
        phase: InstallPhaseId,
        mut progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
    ) {
        let same_phase = self.progress.phase == phase;
        if !same_phase
            || matches!(&self.progress.details, InstallPhaseDetails::Empty)
                && !matches!(&details, InstallPhaseDetails::Empty)
        {
            self.record_event(InstallJobEventKind::PhaseStarted {
                phase,
                details: details.clone(),
            });
        }

        if same_phase
            && let Some(new_progress) = &mut progress
            && let Some(old_progress) = &self.progress.progress
            && let (Some(old_secondary), Some(new_secondary)) =
                (&old_progress.secondary, &new_progress.secondary)
            && old_secondary.total == new_secondary.total
        {
            new_progress.secondary = Some(InstallProgressSecondary {
                current: old_secondary.current.max(new_secondary.current),
                total: new_secondary.total,
            });
        }
        self.progress.phase = phase;
        self.progress.progress = progress;
        self.progress.details = details;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        ContentSet, ContentSetStatus, ContentSourceKind, Instance,
        InstanceLaunchOverrides, LauncherFeatureVersion, ReleaseChannel,
    };
    use chrono::TimeDelta;

    fn job_state() -> InstallJobState {
        InstallJobState::new(InstallRequest::CreateInstance {
            name: "Test".to_string(),
            game_version: "1.21.1".to_string(),
            loader: ModLoader::Vanilla,
            loader_version: None,
            icon_path: None,
            link: InstanceLink::Unmanaged,
        })
    }

    fn set_secondary_progress(
        job: &mut InstallJobState,
        phase: InstallPhaseId,
        current: u64,
        total: u64,
    ) {
        job.set_progress(
            phase,
            Some(InstallProgress {
                current: 0,
                total: 1,
                secondary: Some(InstallProgressSecondary { current, total }),
            }),
            InstallPhaseDetails::Empty,
        );
    }

    fn secondary_progress(job: &InstallJobState) -> (u64, u64) {
        let secondary = job
            .progress
            .progress
            .as_ref()
            .and_then(|progress| progress.secondary.as_ref())
            .unwrap();
        (secondary.current, secondary.total)
    }

    #[test]
    fn same_phase_same_secondary_total_keeps_progress_monotonic() {
        let mut job = job_state();
        set_secondary_progress(
            &mut job,
            InstallPhaseId::DownloadingContent,
            268,
            300,
        );
        set_secondary_progress(
            &mut job,
            InstallPhaseId::DownloadingContent,
            200,
            300,
        );

        assert_eq!(secondary_progress(&job), (268, 300));
    }

    #[test]
    fn different_phase_does_not_retain_secondary_progress() {
        let mut job = job_state();
        set_secondary_progress(
            &mut job,
            InstallPhaseId::DownloadingContent,
            268,
            300,
        );
        set_secondary_progress(
            &mut job,
            InstallPhaseId::DownloadingMinecraft,
            0,
            16,
        );

        assert_eq!(secondary_progress(&job), (0, 16));
    }

    #[test]
    fn same_phase_different_secondary_total_starts_new_counter() {
        let mut job = job_state();
        set_secondary_progress(
            &mut job,
            InstallPhaseId::DownloadingContent,
            268,
            300,
        );
        set_secondary_progress(
            &mut job,
            InstallPhaseId::DownloadingContent,
            0,
            16,
        );

        assert_eq!(secondary_progress(&job), (0, 16));
    }

    #[test]
    fn legacy_rollback_state_defaults_missing_content_revision() {
        let now = Utc::now();
        let instance_id = "instance".to_string();
        let content_set_id = "content-set".to_string();
        let mut job =
            InstallJobState::new(InstallRequest::InstallExistingInstance {
                instance_id: instance_id.clone(),
                force: false,
            });
        job.rollback = Some(InstallRollbackState {
            instance: InstanceMetadata {
                instance: Instance {
                    id: instance_id.clone(),
                    path: "instance".to_string(),
                    applied_content_set_id: Some(content_set_id.clone()),
                    install_stage: InstanceInstallStage::Installed,
                    launcher_feature_version:
                        LauncherFeatureVersion::MOST_RECENT,
                    update_channel: ReleaseChannel::Release,
                    name: "Test".to_string(),
                    icon_path: None,
                    symlink_target: None,
                    created: now,
                    modified: now,
                    last_played: None,
                    pinned_at: None,
                    submitted_time_played: 0,
                    recent_time_played: 0,
                },
                applied_content_set: ContentSet {
                    id: content_set_id,
                    instance_id: instance_id.clone(),
                    name: "Test".to_string(),
                    source_kind: ContentSourceKind::Local,
                    status: ContentSetStatus::Available,
                    game_version: "1.21.1".to_string(),
                    protocol_version: None,
                    loader: ModLoader::Vanilla,
                    loader_version: None,
                    revision: 7,
                    created: now,
                    modified: now,
                },
                link: InstanceLink::Unmanaged,
                groups: Vec::new(),
                launch_overrides: InstanceLaunchOverrides::empty(instance_id),
            },
            install_stage: InstanceInstallStage::Installed,
            content: None,
        });

        let mut legacy = serde_json::to_value(job).unwrap();
        legacy["rollback"]["instance"]["applied_content_set"]
            .as_object_mut()
            .unwrap()
            .remove("revision");

        let restored: InstallJobState = serde_json::from_value(legacy).unwrap();

        assert_eq!(
            restored
                .rollback
                .unwrap()
                .instance
                .applied_content_set
                .revision,
            0
        );
    }

    #[test]
    fn download_summary_uses_content_events_and_live_progress() {
        let mut job = job_state();
        job.record_event(InstallJobEventKind::ContentDownloadStarted {
            files: 3,
            bytes: Some(300),
        });
        job.record_event(InstallJobEventKind::ContentFileDownloadAttempt {
            path: "mods/a.jar".to_string(),
            bytes_total: Some(100),
            attempt: 2,
            max_attempts: 3,
        });
        job.record_event(InstallJobEventKind::DownloadRequestStarted {
            path: "mods/a.jar".to_string(),
            name: "a.jar".to_string(),
            url: "https://mod.mcimirror.top/data/a.jar".to_string(),
            source: "mcim".to_string(),
            bytes_total: Some(100),
            attempt: 1,
            max_attempts: 4,
        });
        job.record_event(InstallJobEventKind::ContentFileCompleted {
            path: "mods/a.jar".to_string(),
            bytes: 100,
        });
        job.record_event(InstallJobEventKind::ContentFileDownloadAttempt {
            path: "mods/manual.jar".to_string(),
            bytes_total: Some(120),
            attempt: 3,
            max_attempts: 3,
        });
        job.record_event(InstallJobEventKind::ContentFileSkipped {
            path: "mods/manual.jar".to_string(),
            reason: "manual download required".to_string(),
            project_id: Some("123".to_string()),
            version_id: Some("456".to_string()),
            manual_url: Some(
                "https://www.curseforge.com/minecraft/mc-mods/example"
                    .to_string(),
            ),
        });
        job.record_event(InstallJobEventKind::ContentFileFailed {
            path: "mods/failed.jar".to_string(),
            reason: "network request failed".to_string(),
            project_id: Some("789".to_string()),
            version_id: Some("987".to_string()),
        });
        job.set_progress(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: 3,
                total: 3,
                secondary: Some(InstallProgressSecondary {
                    current: 220,
                    total: 300,
                }),
            }),
            InstallPhaseDetails::Empty,
        );

        let summary = job.download_summary();
        assert_eq!(summary.files_completed, 3);
        assert_eq!(summary.files_total, Some(3));
        assert_eq!(summary.bytes_downloaded, 220);
        assert_eq!(summary.bytes_total, Some(300));
        let items = job.download_items();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].status, DownloadItemStatus::Completed);
        assert_eq!(items[0].attempt, Some(1));
        assert_eq!(items[0].max_attempts, Some(4));
        assert_eq!(
            items[0].request_url.as_deref(),
            Some("https://mod.mcimirror.top/data/a.jar")
        );
        assert_eq!(items[0].source.as_deref(), Some("mcim"));
        assert_eq!(items[1].status, DownloadItemStatus::Skipped);
        assert_eq!(items[1].attempt, Some(3));
        assert_eq!(items[1].max_attempts, Some(3));
        assert_eq!(items[1].project_id.as_deref(), Some("123"));
        assert_eq!(items[1].version_id.as_deref(), Some("456"));
        assert!(items[1].manual_url.is_some());
        assert_eq!(items[2].status, DownloadItemStatus::Failed);
        assert_eq!(items[2].project_id.as_deref(), Some("789"));
        assert_eq!(items[2].version_id.as_deref(), Some("987"));
        assert!(items[2].manual_url.is_none());
    }

    #[test]
    fn required_content_items_are_queued_before_download_requests_start() {
        let mut job = job_state();
        job.record_event(InstallJobEventKind::ContentDownloadStarted {
            files: 2,
            bytes: Some(300),
        });
        job.record_event(InstallJobEventKind::ContentFileQueued {
            path: "mods/a.jar".to_string(),
            bytes_total: Some(100),
            max_attempts: 4,
        });
        job.record_event(InstallJobEventKind::ContentFileQueued {
            path: "mods/b.jar".to_string(),
            bytes_total: Some(200),
            max_attempts: 6,
        });

        let items = job.download_items();
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|item| item.status == DownloadItemStatus::Queued)
        );
        assert!(items.iter().all(|item| item.attempt == Some(0)));
        assert_eq!(items[0].max_attempts, Some(4));
        assert_eq!(items[1].bytes_total, Some(200));
        assert!(items.iter().all(|item| item.request_url.is_none()));
        assert!(items.iter().all(|item| item.source.is_none()));
    }

    #[test]
    fn recovery_validation_mode_is_derived_from_typed_resume_history() {
        let mut job =
            InstallJobState::new(InstallRequest::CreateModpackInstance {
                location: CreatePackLocation::FromFile {
                    path: PathBuf::from("test.mrpack"),
                },
                post_install_edit: None,
            });
        job.missing_content = Some(MissingModpackContentState::default());
        job.set_progress(
            InstallPhaseId::DownloadingContent,
            None,
            InstallPhaseDetails::Empty,
        );
        assert_eq!(
            job.execution_mode(InstallJobStatus::Running),
            InstallJobExecutionMode::Normal
        );

        job.record_event(InstallJobEventKind::WaitingForUser {
            reason: InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                paths: vec!["mods/missing.jar".to_string()],
            },
        });
        assert_eq!(
            job.execution_mode(InstallJobStatus::WaitingForUser),
            InstallJobExecutionMode::Normal
        );
        job.record_event(InstallJobEventKind::JobQueued {
            kind: InstallJobKind::CreateModpackInstance,
        });
        assert_eq!(
            job.execution_mode(InstallJobStatus::Queued),
            InstallJobExecutionMode::RecoveryValidation
        );
        assert_eq!(
            job.execution_mode(InstallJobStatus::Running),
            InstallJobExecutionMode::RecoveryValidation
        );

        job.set_progress(
            InstallPhaseId::ExtractingOverrides,
            None,
            InstallPhaseDetails::Empty,
        );
        assert_eq!(
            job.execution_mode(InstallJobStatus::Running),
            InstallJobExecutionMode::Normal
        );
    }

    #[test]
    fn resumed_existing_file_moves_from_queued_to_verifying() {
        let mut job =
            InstallJobState::new(InstallRequest::CreateModpackInstance {
                location: CreatePackLocation::FromFile {
                    path: PathBuf::from("test.mrpack"),
                },
                post_install_edit: None,
            });
        let path = "mods/missing.jar".to_string();
        job.missing_content = Some(MissingModpackContentState::default());
        job.set_progress(
            InstallPhaseId::DownloadingContent,
            None,
            InstallPhaseDetails::Empty,
        );
        job.record_event(InstallJobEventKind::ContentFileQueued {
            path: path.clone(),
            bytes_total: Some(100),
            max_attempts: 4,
        });
        job.record_event(InstallJobEventKind::WaitingForUser {
            reason: InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                paths: vec![path.clone()],
            },
        });
        job.record_event(InstallJobEventKind::ContentFileRecovered {
            path: path.clone(),
            bytes: 100,
        });
        job.record_event(InstallJobEventKind::JobQueued {
            kind: InstallJobKind::CreateModpackInstance,
        });
        job.record_event(InstallJobEventKind::ContentFileQueued {
            path: path.clone(),
            bytes_total: Some(100),
            max_attempts: 4,
        });
        assert_eq!(job.download_items()[0].status, DownloadItemStatus::Queued);

        job.record_event(InstallJobEventKind::ContentFileVerificationStarted {
            path,
        });
        assert_eq!(
            job.execution_mode(InstallJobStatus::Running),
            InstallJobExecutionMode::RecoveryValidation
        );
        assert_eq!(
            job.download_items()[0].status,
            DownloadItemStatus::Verifying
        );
    }

    #[test]
    fn all_successful_required_content_items_are_completed() {
        let mut job = job_state();
        for (path, bytes) in [("mods/a.jar", 100), ("mods/b.jar", 200)] {
            job.record_event(InstallJobEventKind::ContentFileQueued {
                path: path.to_string(),
                bytes_total: Some(bytes),
                max_attempts: 4,
            });
            job.record_event(InstallJobEventKind::ContentFileCompleted {
                path: path.to_string(),
                bytes,
            });
        }

        let items = job.download_items();
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|item| item.status == DownloadItemStatus::Completed)
        );
    }

    #[test]
    fn skipped_required_content_item_is_not_failed() {
        let mut job = job_state();
        job.record_event(InstallJobEventKind::ContentFileQueued {
            path: "mods/client-unsupported.jar".to_string(),
            bytes_total: Some(100),
            max_attempts: 2,
        });
        job.record_event(InstallJobEventKind::ContentFileSkipped {
            path: "mods/client-unsupported.jar".to_string(),
            reason: "unsupported on client".to_string(),
            project_id: None,
            version_id: None,
            manual_url: None,
        });

        let items = job.download_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, DownloadItemStatus::Skipped);
        assert_ne!(items[0].status, DownloadItemStatus::Failed);
    }

    #[test]
    fn multiple_required_content_failures_have_individual_failed_items() {
        let mut job = job_state();
        for path in ["mods/a.jar", "mods/b.jar", "mods/c.jar"] {
            job.record_event(InstallJobEventKind::ContentFileQueued {
                path: path.to_string(),
                bytes_total: Some(100),
                max_attempts: 4,
            });
        }
        for path in ["mods/a.jar", "mods/c.jar"] {
            job.record_event(InstallJobEventKind::ContentFileFailed {
                path: path.to_string(),
                reason: "download failed".to_string(),
                project_id: None,
                version_id: None,
            });
        }
        job.record_event(InstallJobEventKind::ContentFileCompleted {
            path: "mods/b.jar".to_string(),
            bytes: 100,
        });

        let items = job.download_items();
        assert_eq!(items.len(), 3);
        assert_eq!(
            items
                .iter()
                .filter(|item| item.status == DownloadItemStatus::Failed)
                .count(),
            2
        );
        assert_eq!(items[1].status, DownloadItemStatus::Completed);
    }

    #[test]
    fn request_events_create_items_for_java_and_minecraft_downloads() {
        let mut job = job_state();
        job.record_event(InstallJobEventKind::DownloadRequestStarted {
            path: "java/runtime-manifest.json".to_string(),
            name: "runtime-manifest.json".to_string(),
            url: "https://piston-meta.mojang.com/v1/runtime.json".to_string(),
            source: "official".to_string(),
            bytes_total: Some(1024),
            attempt: 1,
            max_attempts: 2,
        });
        job.record_event(InstallJobEventKind::DownloadRequestFinished {
            path: "java/runtime-manifest.json".to_string(),
            bytes: 1024,
        });

        let items = job.download_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "runtime-manifest.json");
        assert_eq!(items[0].status, DownloadItemStatus::Completed);
        assert_eq!(items[0].bytes_downloaded, 1024);
        assert_eq!(items[0].bytes_total, Some(1024));
        assert_eq!(items[0].source.as_deref(), Some("official"));

        job.compact_transient_download_events();
        assert!(job.events.iter().all(|event| !matches!(
            event.kind,
            InstallJobEventKind::DownloadRequestStarted { .. }
                | InstallJobEventKind::DownloadRequestFinished { .. }
                | InstallJobEventKind::DownloadRequestFailed { .. }
        )));
    }

    #[test]
    fn active_downloads_expose_live_bytes_and_hide_stale_eta() {
        let mut job = job_state();
        job.set_progress(
            InstallPhaseId::DownloadingMinecraft,
            Some(InstallProgress {
                current: 400,
                total: 1_000,
                secondary: None,
            }),
            InstallPhaseDetails::Empty,
        );
        job.active_downloads.insert(
            "client.jar".to_string(),
            ActiveDownloadState {
                name: "client.jar".to_string(),
                url: "https://piston-data.mojang.com/client.jar".to_string(),
                source: "official".to_string(),
                bytes_downloaded: 400,
                bytes_total: Some(1_000),
                attempt: 1,
                max_attempts: 3,
                status: DownloadItemStatus::Downloading,
                last_reported_bytes: 400,
                last_progress_at: Utc::now(),
                speed_bytes_per_second: Some(200),
                speed_sample_started_at: Utc::now(),
                speed_sample_started_bytes: 400,
            },
        );

        let items = job.download_items();
        assert_eq!(items[0].bytes_downloaded, 400);
        assert_eq!(items[0].status, DownloadItemStatus::Downloading);
        let summary = job.download_summary();
        assert_eq!(summary.speed_bytes_per_second, Some(200));
        assert_eq!(summary.eta_seconds, Some(3));

        job.active_downloads
            .get_mut("client.jar")
            .unwrap()
            .last_progress_at = Utc::now() - TimeDelta::seconds(4);
        let stalled = job.download_summary();
        assert_eq!(stalled.speed_bytes_per_second, None);
        assert_eq!(stalled.eta_seconds, None);
        job.record_event(InstallJobEventKind::JobCanceled {
            phase: InstallPhaseId::DownloadingMinecraft,
        });
        assert!(job.active_downloads.is_empty());
    }

    #[test]
    fn canceling_a_job_clears_active_download_requests() {
        let mut job = job_state();
        job.record_event(InstallJobEventKind::DownloadRequestStarted {
            path: "mods/a.jar".to_string(),
            name: "a.jar".to_string(),
            url: "https://cdn.modrinth.com/data/a.jar".to_string(),
            source: "official".to_string(),
            bytes_total: Some(1024),
            attempt: 1,
            max_attempts: 2,
        });
        job.record_event(InstallJobEventKind::JobCanceled {
            phase: InstallPhaseId::DownloadingContent,
        });

        let items = job.download_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, DownloadItemStatus::Canceled);
    }

    #[test]
    fn minecraft_download_progress_includes_byte_details() {
        let mut job = job_state();
        job.set_progress(
            InstallPhaseId::DownloadingMinecraft,
            Some(InstallProgress {
                current: 125,
                total: 500,
                secondary: None,
            }),
            InstallPhaseDetails::Empty,
        );

        let summary = job.download_summary();
        assert_eq!(summary.bytes_downloaded, 125);
        assert_eq!(summary.bytes_total, Some(500));
    }

    #[test]
    fn download_summary_tracks_metrics_and_java_progress() {
        let mut job = job_state();
        job.set_progress(
            InstallPhaseId::PreparingJava,
            Some(InstallProgress {
                current: 1_024,
                total: 2_048,
                secondary: None,
            }),
            InstallPhaseDetails::Java {
                major_version: 21,
                step: InstallJavaStep::Downloading,
            },
        );
        job.events.last_mut().unwrap().at = Utc::now() - TimeDelta::seconds(2);
        job.record_event(InstallJobEventKind::DownloadMetrics {
            source: "bmclapi".to_string(),
            fallback_count: u64::MAX,
        });
        job.record_event(InstallJobEventKind::DownloadMetrics {
            source: "official".to_string(),
            fallback_count: 1,
        });

        let summary = job.download_summary();
        assert_eq!(summary.bytes_downloaded, 1_024);
        assert_eq!(summary.bytes_total, Some(2_048));
        assert_eq!(summary.source.as_deref(), Some("official"));
        assert_eq!(summary.fallback_count, u64::MAX);
        assert_eq!(summary.speed_bytes_per_second, None);
        assert_eq!(summary.eta_seconds, None);
    }

    #[test]
    fn curseforge_instance_jobs_use_the_curseforge_provider() {
        let job = InstallJobState::new(InstallRequest::CreateInstance {
            name: "CurseForge pack".to_string(),
            game_version: "1.20.1".to_string(),
            loader: ModLoader::Forge,
            loader_version: Some("latest".to_string()),
            icon_path: None,
            link: InstanceLink::CurseForgeModpack {
                project_id: "123".to_string(),
                version_id: "456".to_string(),
            },
        });

        assert_eq!(job.provider(), InstallJobProvider::CurseForge);
    }

    #[test]
    fn curseforge_managed_modpack_updates_are_tracked_as_pack_install_jobs() {
        let job = InstallJobState::new(
            InstallRequest::UpdateManagedCurseForgeModpack {
                instance_id: "instance".to_string(),
                file_id: 456,
            },
        );

        assert_eq!(job.provider(), InstallJobProvider::CurseForge);
        assert_eq!(
            job.request.kind(),
            InstallJobKind::InstallPackToExistingInstance
        );
        assert!(job.request.completes_instance_install_stage());
        assert!(matches!(
            job.request.target(),
            InstallTarget::ExistingInstance { .. }
        ));
        assert!(matches!(
            job.request.cleanup(),
            InstallCleanup::RestoreExistingInstance { .. }
        ));
    }

    #[test]
    fn deleted_instance_is_exposed_by_download_snapshot_state() {
        let mut job = job_state();
        assert!(!job.instance_deleted());
        job.record_event(InstallJobEventKind::TargetInstanceDeleted {
            instance_id: "deleted-instance".to_string(),
        });
        assert!(job.instance_deleted());
    }

    #[test]
    fn canceling_and_waiting_statuses_round_trip() {
        for status in [
            InstallJobStatus::Canceling,
            InstallJobStatus::WaitingForUser,
        ] {
            assert_eq!(
                InstallJobStatus::from_stored_str(status.as_str()),
                status
            );
            assert!(!status.is_finished());
        }
    }

    #[test]
    fn waiting_job_state_round_trip_preserves_pause_and_file_items() {
        let mut job = job_state();
        for path in ["mods/complete.jar", "mods/missing.jar"] {
            job.record_event(InstallJobEventKind::ContentFileQueued {
                path: path.to_string(),
                bytes_total: Some(100),
                max_attempts: 4,
            });
        }
        job.record_event(InstallJobEventKind::ContentFileCompleted {
            path: "mods/complete.jar".to_string(),
            bytes: 100,
        });
        job.record_event(InstallJobEventKind::ContentFileFailed {
            path: "mods/missing.jar".to_string(),
            reason: "network failure".to_string(),
            project_id: None,
            version_id: None,
        });
        let reason = InstallPauseReason::MissingRequiredContent {
            failed_files: 1,
            paths: vec!["mods/missing.jar".to_string()],
        };
        job.pause_reason = Some(reason.clone());
        job.continuation =
            Some(InstallContinuationState::InstallingPackToExistingInstance {
                disabled_project_ids: vec!["project-a".to_string()],
            });
        job.record_event(InstallJobEventKind::WaitingForUser {
            reason: reason.clone(),
        });

        let restored: InstallJobState =
            serde_json::from_str(&serde_json::to_string(&job).unwrap())
                .unwrap();
        let items = restored.download_items();

        assert_eq!(restored.pause_reason, Some(reason));
        assert_eq!(restored.continuation, job.continuation);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].status, DownloadItemStatus::Completed);
        assert_eq!(items[1].status, DownloadItemStatus::Failed);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallJobEvent {
    pub at: DateTime<Utc>,
    pub kind: InstallJobEventKind,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallInterruptReason {
    AppClosed,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallJobEventKind {
    JobQueued {
        kind: InstallJobKind,
    },
    JobStarted,
    JobSucceeded {
        instance_id: Option<String>,
    },
    JobCanceled {
        phase: InstallPhaseId,
    },
    WaitingForUser {
        reason: InstallPauseReason,
    },
    PhaseStarted {
        phase: InstallPhaseId,
        details: InstallPhaseDetails,
    },
    ContentDownloadStarted {
        files: u64,
        bytes: Option<u64>,
    },
    ContentFileQueued {
        path: String,
        bytes_total: Option<u64>,
        max_attempts: u32,
    },
    ContentFileBrowserOptions {
        path: String,
        urls: Vec<String>,
    },
    ContentFileVerificationStarted {
        path: String,
    },
    ContentFileWritingStarted {
        path: String,
    },
    ContentFileRecovered {
        path: String,
        bytes: u64,
    },
    ContentFileDownloadAttempt {
        path: String,
        bytes_total: Option<u64>,
        attempt: u32,
        max_attempts: u32,
    },
    DownloadRequestStarted {
        path: String,
        name: String,
        url: String,
        source: String,
        bytes_total: Option<u64>,
        attempt: u32,
        max_attempts: u32,
    },
    DownloadRequestFinished {
        path: String,
        bytes: u64,
    },
    DownloadRequestFailed {
        path: String,
    },
    ContentFileSkipped {
        path: String,
        reason: String,
        #[serde(default)]
        project_id: Option<String>,
        #[serde(default)]
        version_id: Option<String>,
        #[serde(default)]
        manual_url: Option<String>,
    },
    ContentFileFailed {
        path: String,
        reason: String,
        #[serde(default)]
        project_id: Option<String>,
        #[serde(default)]
        version_id: Option<String>,
    },
    ContentFileCompleted {
        path: String,
        bytes: u64,
    },
    DownloadMetrics {
        source: String,
        fallback_count: u64,
    },
    TargetInstanceDeleted {
        instance_id: String,
    },
    Interrupted {
        reason: InstallInterruptReason,
        phase: InstallPhaseId,
    },
    Failed {
        phase: InstallPhaseId,
        code: String,
        message: String,
    },
    RollbackStarted {
        cleanup: InstallCleanup,
    },
    RollbackCompleted,
    RollbackFailed {
        message: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallRequest {
    CreateInstance {
        name: String,
        game_version: String,
        loader: ModLoader,
        loader_version: Option<String>,
        icon_path: Option<String>,
        link: InstanceLink,
    },
    CreateModpackInstance {
        location: CreatePackLocation,
        #[serde(default)]
        post_install_edit: Option<InstallPostInstallEdit>,
    },
    ImportInstance {
        launcher_type: ImportLauncherType,
        base_path: PathBuf,
        instance_folder: String,
        #[serde(default)]
        instance_path: Option<String>,
        #[serde(default)]
        symlink: bool,
        #[serde(default)]
        game_version: Option<String>,
        #[serde(default)]
        loader: Option<ModLoader>,
        #[serde(default)]
        loader_version: Option<String>,
    },
    DuplicateInstance {
        source_instance_id: String,
    },
    InstallExistingInstance {
        instance_id: String,
        force: bool,
    },
    InstallPackToExistingInstance {
        instance_id: String,
        location: CreatePackLocation,
        #[serde(default)]
        post_install_edit: Option<InstallPostInstallEdit>,
    },
    UpdateManagedCurseForgeModpack {
        instance_id: String,
        file_id: u32,
    },
    InstallContent {
        instance_id: String,
        project_id: String,
        version_id: Option<String>,
        content_type: ContentType,
        #[serde(default)]
        selected: ResolutionPreferences,
        #[serde(default)]
        excluded_project_ids: Vec<String>,
        display_title: String,
        #[serde(default)]
        display_icon: Option<String>,
    },
    InstallCurseForgeContent {
        request: CurseForgeInstallRequest,
        display_title: String,
        #[serde(default)]
        display_icon: Option<String>,
    },
    InstallCurseForgeWorld {
        request: CurseForgeWorldInstallRequest,
        display_title: String,
        #[serde(default)]
        display_icon: Option<String>,
    },
    DownloadJava {
        vendor: String,
        version: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstallPostInstallEdit {
    pub name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub icon_path: Option<Option<String>>,
    pub link: Option<InstanceLink>,
}

impl InstallRequest {
    pub(crate) fn completes_instance_install_stage(&self) -> bool {
        match self {
            Self::CreateInstance { .. }
            | Self::CreateModpackInstance { .. }
            | Self::ImportInstance { .. }
            | Self::DuplicateInstance { .. }
            | Self::InstallExistingInstance { .. }
            | Self::InstallPackToExistingInstance { .. }
            | Self::UpdateManagedCurseForgeModpack { .. } => true,
            Self::InstallContent { .. }
            | Self::InstallCurseForgeContent { .. }
            | Self::InstallCurseForgeWorld { .. }
            | Self::DownloadJava { .. } => false,
        }
    }

    pub fn kind(&self) -> InstallJobKind {
        match self {
            Self::CreateInstance { .. } => InstallJobKind::CreateInstance,
            Self::CreateModpackInstance { .. } => {
                InstallJobKind::CreateModpackInstance
            }
            Self::ImportInstance { .. } => InstallJobKind::ImportInstance,
            Self::DuplicateInstance { .. } => InstallJobKind::DuplicateInstance,
            Self::InstallExistingInstance { .. } => {
                InstallJobKind::InstallExistingInstance
            }
            Self::InstallPackToExistingInstance { .. } => {
                InstallJobKind::InstallPackToExistingInstance
            }
            Self::UpdateManagedCurseForgeModpack { .. } => {
                InstallJobKind::InstallPackToExistingInstance
            }
            Self::InstallContent { .. } => InstallJobKind::InstallContent,
            Self::InstallCurseForgeContent { .. } => {
                InstallJobKind::InstallContent
            }
            Self::InstallCurseForgeWorld { .. } => {
                InstallJobKind::InstallContent
            }
            Self::DownloadJava { .. } => InstallJobKind::DownloadJava,
        }
    }

    pub fn target(&self) -> InstallTarget {
        match self {
            Self::InstallExistingInstance { instance_id, .. }
            | Self::InstallPackToExistingInstance { instance_id, .. }
            | Self::UpdateManagedCurseForgeModpack { instance_id, .. }
            | Self::InstallContent { instance_id, .. } => {
                InstallTarget::ExistingInstance {
                    instance_id: instance_id.clone(),
                }
            }
            Self::InstallCurseForgeContent { request, .. } => {
                InstallTarget::ExistingInstance {
                    instance_id: request.instance_id.clone(),
                }
            }
            Self::InstallCurseForgeWorld { request, .. } => {
                InstallTarget::ExistingInstance {
                    instance_id: request.instance_id.clone(),
                }
            }
            _ => InstallTarget::NewInstance { instance_id: None },
        }
    }

    pub fn cleanup(&self) -> InstallCleanup {
        match self {
            Self::InstallExistingInstance { instance_id, .. }
            | Self::InstallPackToExistingInstance { instance_id, .. }
            | Self::UpdateManagedCurseForgeModpack { instance_id, .. } => {
                InstallCleanup::RestoreExistingInstance {
                    instance_id: instance_id.clone(),
                }
            }
            Self::InstallContent { .. } => InstallCleanup::None,
            Self::InstallCurseForgeContent { .. } => InstallCleanup::None,
            Self::InstallCurseForgeWorld { .. } => InstallCleanup::None,
            _ => InstallCleanup::DeleteNewInstance { instance_id: None },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJobKind {
    CreateInstance,
    CreateModpackInstance,
    ImportInstance,
    DuplicateInstance,
    InstallExistingInstance,
    InstallPackToExistingInstance,
    InstallContent,
    DownloadJava,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJobProvider {
    Modrinth,
    CurseForge,
    Minecraft,
    Java,
    Application,
    Local,
}

impl InstallJobProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::CurseForge => "curse_forge",
            Self::Minecraft => "minecraft",
            Self::Java => "java",
            Self::Application => "application",
            Self::Local => "local",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadItemStatus {
    Queued,
    Downloading,
    Verifying,
    Writing,
    WaitingForUser,
    Completed,
    Skipped,
    Failed,
    Canceled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActiveDownloadState {
    pub name: String,
    pub url: String,
    pub source: String,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub attempt: u32,
    pub max_attempts: u32,
    pub status: DownloadItemStatus,
    #[serde(default)]
    pub last_reported_bytes: u64,
    pub last_progress_at: DateTime<Utc>,
    #[serde(default)]
    pub speed_bytes_per_second: Option<u64>,
    #[serde(default = "Utc::now")]
    pub speed_sample_started_at: DateTime<Utc>,
    #[serde(default)]
    pub speed_sample_started_bytes: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DownloadItemSnapshot {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    pub status: DownloadItemStatus,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    #[serde(default)]
    pub attempt: Option<u32>,
    #[serde(default)]
    pub max_attempts: Option<u32>,
    pub error: Option<String>,
    pub manual_url: Option<String>,
    pub request_url: Option<String>,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DownloadJobSummary {
    pub files_completed: u64,
    pub files_total: Option<u64>,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub speed_bytes_per_second: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub source: Option<String>,
    pub fallback_count: u64,
}

impl InstallJobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CreateInstance => "create_instance",
            Self::CreateModpackInstance => "create_modpack_instance",
            Self::ImportInstance => "import_instance",
            Self::DuplicateInstance => "duplicate_instance",
            Self::InstallExistingInstance => "install_existing_instance",
            Self::InstallPackToExistingInstance => {
                "install_pack_to_existing_instance"
            }
            Self::InstallContent => "install_content",
            Self::DownloadJava => "download_java",
        }
    }

    pub fn from_stored_str(value: &str) -> Self {
        match value {
            "create_modpack_instance" => Self::CreateModpackInstance,
            "import_instance" => Self::ImportInstance,
            "duplicate_instance" => Self::DuplicateInstance,
            "install_existing_instance" => Self::InstallExistingInstance,
            "install_pack_to_existing_instance" => {
                Self::InstallPackToExistingInstance
            }
            "install_content" => Self::InstallContent,
            "download_java" => Self::DownloadJava,
            _ => Self::CreateInstance,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJobStatus {
    Queued,
    Running,
    Canceling,
    WaitingForUser,
    Succeeded,
    Failed,
    Interrupted,
    Canceled,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJobExecutionMode {
    Normal,
    RecoveryValidation,
}

impl InstallJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Canceling => "canceling",
            Self::WaitingForUser => "waiting_for_user",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
            Self::Canceled => "canceled",
        }
    }

    pub fn from_stored_str(value: &str) -> Self {
        match value {
            "running" => Self::Running,
            "canceling" => Self::Canceling,
            "waiting_for_user" => Self::WaitingForUser,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "interrupted" => Self::Interrupted,
            "canceled" => Self::Canceled,
            _ => Self::Queued,
        }
    }

    pub fn is_finished(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Interrupted | Self::Canceled
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallTarget {
    NewInstance { instance_id: Option<String> },
    ExistingInstance { instance_id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallCleanup {
    DeleteNewInstance { instance_id: Option<String> },
    RestoreExistingInstance { instance_id: String },
    None,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallProgressState {
    pub phase: InstallPhaseId,
    pub progress: Option<InstallProgress>,
    pub details: InstallPhaseDetails,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhaseId {
    PreparingInstance,
    ResolvingPack,
    DownloadingPackFile,
    ReadingPackManifest,
    DownloadingContent,
    ExtractingOverrides,
    ResolvingMinecraft,
    ResolvingLoader,
    PreparingJava,
    DownloadingMinecraft,
    RunningLoaderProcessors,
    Finalizing,
    RollingBack,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallProgress {
    pub current: u64,
    pub total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<InstallProgressSecondary>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallProgressSecondary {
    pub current: u64,
    pub total: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJavaStep {
    Resolving,
    FetchingMetadata,
    Downloading,
    Extracting,
    Validating,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallPhaseDetails {
    Empty,
    Instance {
        name: String,
    },
    Minecraft {
        game_version: String,
        loader: ModLoader,
    },
    Java {
        major_version: u32,
        step: InstallJavaStep,
    },
    Modpack {
        project_id: Option<String>,
        version_id: Option<String>,
        title: Option<String>,
    },
    Import {
        launcher_type: ImportLauncherType,
        instance_folder: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstallJobPaths {
    pub staging_dir: Option<PathBuf>,
    pub final_instance_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Clone, Debug, bon::Builder)]
#[builder(start_fn = new)]
pub struct InstallErrorContext {
    #[builder(start_fn, into)]
    pub operation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub target_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub entry_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub cache_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub sqlite_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub expected_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub minecraft_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub loader: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub arch: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallJobDisplay {
    pub title: String,
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallRollbackState {
    pub instance: InstanceMetadata,
    pub install_stage: InstanceInstallStage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<InstallContentRollbackSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallContentRollbackStage {
    Planned,
    Ready,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallContentRollbackSnapshot {
    pub staging_id: String,
    pub stage: InstallContentRollbackStage,
    pub files: Vec<InstallRollbackFile>,
    pub entries: Vec<InstallRollbackContentEntry>,
    pub pack_members: Vec<PackMember>,
    #[serde(default)]
    pub replacement_paths: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallRollbackFile {
    pub relative_path: String,
    pub staged_name: String,
    pub existed: bool,
    pub physical_sha1: Option<String>,
    pub physical_size: Option<u64>,
    pub instance_file: Option<InstanceFile>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallRollbackContentEntry {
    pub entry: ContentEntry,
    pub provider_refs: Vec<InstallRollbackProviderRef>,
    pub update_check: Option<ContentUpdateCheck>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallRollbackProviderRef {
    pub provider_ref: ContentProviderRef,
    pub origin: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MissingModpackContentState {
    pub files: Vec<MissingModpackFileState>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MissingModpackFileState {
    pub item_id: String,
    pub manifest_path: String,
    pub target_path: String,
    pub expected_size: u64,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub download_urls: Vec<String>,
    pub browser_urls: Vec<String>,
    #[serde(default)]
    pub validate_as_jar: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallErrorView {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<InstallPhaseId>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<InstallApiErrorDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<InstallErrorContext>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallApiErrorDetails {
    pub error: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

impl InstallErrorView {
    pub fn from_error(
        code: &str,
        phase: InstallPhaseId,
        error: &crate::Error,
        context: Option<InstallErrorContext>,
    ) -> Self {
        Self {
            code: code.to_string(),
            phase: Some(phase),
            message: error.to_string(),
            api: match error.raw.as_ref() {
                crate::ErrorKind::LabrinthError(error) => {
                    Some(InstallApiErrorDetails {
                        error: error.error.clone(),
                        status: error.status,
                        method: error.method.clone(),
                        url: error.url.clone(),
                        route: error.route.clone(),
                    })
                }
                crate::ErrorKind::HttpError {
                    status,
                    method,
                    url,
                } => Some(InstallApiErrorDetails {
                    error: "http_error".to_string(),
                    status: Some(*status),
                    method: Some(method.clone()),
                    url: Some(url.clone()),
                    route: None,
                }),
                _ => None,
            },
            context,
        }
    }

    pub fn from_message(
        code: &str,
        phase: InstallPhaseId,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_string(),
            phase: Some(phase),
            message: message.into(),
            api: None,
            context: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallJobSnapshot {
    pub job_id: Uuid,
    pub instance_id: Option<String>,
    pub instance_deleted: bool,
    pub kind: InstallJobKind,
    pub status: InstallJobStatus,
    pub execution_mode: InstallJobExecutionMode,
    pub provider: InstallJobProvider,
    pub target: InstallTarget,
    pub phase: InstallPhaseId,
    pub progress: Option<InstallProgress>,
    pub details: InstallPhaseDetails,
    pub display: Option<InstallJobDisplay>,
    pub error: Option<InstallErrorView>,
    pub rollback_error: Option<InstallErrorView>,
    pub pause_reason: Option<InstallPauseReason>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
    pub summary: DownloadJobSummary,
    pub items: Vec<DownloadItemSnapshot>,
}

impl InstallJobState {
    pub fn execution_mode(
        &self,
        status: InstallJobStatus,
    ) -> InstallJobExecutionMode {
        if !matches!(
            status,
            InstallJobStatus::Queued | InstallJobStatus::Running
        ) || self.progress.phase != InstallPhaseId::DownloadingContent
            || self.missing_content.is_none()
            || !matches!(
                &self.request,
                InstallRequest::CreateModpackInstance { .. }
                    | InstallRequest::InstallPackToExistingInstance { .. }
            )
        {
            return InstallJobExecutionMode::Normal;
        }

        let last_missing_pause = self.events.iter().rposition(|event| {
            matches!(
                &event.kind,
                InstallJobEventKind::WaitingForUser {
                    reason: InstallPauseReason::MissingRequiredContent { .. }
                }
            )
        });
        let last_queued = self.events.iter().rposition(|event| {
            matches!(&event.kind, InstallJobEventKind::JobQueued { .. })
        });
        if last_missing_pause
            .zip(last_queued)
            .is_some_and(|(paused, queued)| queued > paused)
        {
            InstallJobExecutionMode::RecoveryValidation
        } else {
            InstallJobExecutionMode::Normal
        }
    }

    pub fn instance_deleted(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.kind,
                InstallJobEventKind::TargetInstanceDeleted { .. }
            )
        })
    }

    pub fn provider(&self) -> InstallJobProvider {
        match &self.request {
            InstallRequest::CreateModpackInstance { location, .. }
            | InstallRequest::InstallPackToExistingInstance {
                location, ..
            } => match location {
                CreatePackLocation::FromVersionId { .. } => {
                    InstallJobProvider::Modrinth
                }
                CreatePackLocation::FromFile { .. } => {
                    InstallJobProvider::Local
                }
            },
            InstallRequest::CreateInstance { link, .. } => match link {
                InstanceLink::CurseForgeModpack { .. } => {
                    InstallJobProvider::CurseForge
                }
                _ => InstallJobProvider::Minecraft,
            },
            InstallRequest::InstallExistingInstance { .. } => {
                InstallJobProvider::Minecraft
            }
            InstallRequest::InstallContent { .. } => {
                InstallJobProvider::Modrinth
            }
            InstallRequest::InstallCurseForgeContent { .. }
            | InstallRequest::InstallCurseForgeWorld { .. }
            | InstallRequest::UpdateManagedCurseForgeModpack { .. } => {
                InstallJobProvider::CurseForge
            }
            InstallRequest::DownloadJava { .. } => InstallJobProvider::Java,
            InstallRequest::ImportInstance { .. }
            | InstallRequest::DuplicateInstance { .. } => {
                InstallJobProvider::Local
            }
        }
    }

    /// Replays `events` into download item snapshots. This is the expensive
    /// part of `download_items` and its result is cached; the cheap
    /// `active_downloads` overlay is applied live by the caller.
    fn download_items_from_events(&self) -> Vec<DownloadItemSnapshot> {
        let mut items = Vec::<DownloadItemSnapshot>::new();
        let mut indices = HashMap::<String, usize>::new();
        for event in &self.events {
            match &event.kind {
                InstallJobEventKind::ContentFileQueued {
                    path,
                    bytes_total,
                    max_attempts,
                } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Queued;
                        item.bytes_downloaded = 0;
                        item.bytes_total = *bytes_total;
                        item.attempt = Some(0);
                        item.max_attempts = Some(*max_attempts);
                        item.error = None;
                        item.request_url = None;
                        item.source = None;
                    } else {
                        let index = items.len();
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: path.clone(),
                            project_id: None,
                            version_id: None,
                            status: DownloadItemStatus::Queued,
                            bytes_downloaded: 0,
                            bytes_total: *bytes_total,
                            attempt: Some(0),
                            max_attempts: Some(*max_attempts),
                            error: None,
                            manual_url: None,
                            request_url: None,
                            source: None,
                        });
                        indices.insert(path.clone(), index);
                    }
                }
                InstallJobEventKind::ContentFileBrowserOptions {
                    path,
                    urls,
                } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.manual_url = urls.first().cloned();
                    }
                }
                InstallJobEventKind::ContentFileVerificationStarted {
                    path,
                } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Verifying;
                        item.error = None;
                    }
                }
                InstallJobEventKind::ContentFileWritingStarted { path } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Writing;
                        item.error = None;
                    }
                }
                InstallJobEventKind::ContentFileRecovered { path, bytes } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Completed;
                        item.bytes_downloaded = *bytes;
                        item.bytes_total = item.bytes_total.or(Some(*bytes));
                        item.error = None;
                    }
                }
                InstallJobEventKind::ContentFileDownloadAttempt {
                    path,
                    bytes_total,
                    attempt,
                    max_attempts,
                } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Queued;
                        item.bytes_downloaded = 0;
                        item.bytes_total = *bytes_total;
                        item.attempt = Some(*attempt);
                        item.max_attempts = Some(*max_attempts);
                        item.error = None;
                        item.request_url = None;
                        item.source = None;
                    } else {
                        let index = items.len();
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: path.clone(),
                            project_id: None,
                            version_id: None,
                            status: DownloadItemStatus::Queued,
                            bytes_downloaded: 0,
                            bytes_total: *bytes_total,
                            attempt: Some(*attempt),
                            max_attempts: Some(*max_attempts),
                            error: None,
                            manual_url: None,
                            request_url: None,
                            source: None,
                        });
                        indices.insert(path.clone(), index);
                    }
                }
                InstallJobEventKind::DownloadRequestStarted {
                    path,
                    name,
                    url,
                    source,
                    bytes_total,
                    attempt,
                    max_attempts,
                } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Downloading;
                        item.bytes_total = item.bytes_total.or(*bytes_total);
                        item.attempt = Some(*attempt);
                        item.max_attempts = Some(*max_attempts);
                        item.error = None;
                        item.request_url = Some(url.clone());
                        item.source = Some(source.clone());
                    } else {
                        let index = items.len();
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: name.clone(),
                            project_id: None,
                            version_id: None,
                            status: DownloadItemStatus::Downloading,
                            bytes_downloaded: 0,
                            bytes_total: *bytes_total,
                            attempt: Some(*attempt),
                            max_attempts: Some(*max_attempts),
                            error: None,
                            manual_url: None,
                            request_url: Some(url.clone()),
                            source: Some(source.clone()),
                        });
                        indices.insert(path.clone(), index);
                    }
                }
                InstallJobEventKind::DownloadRequestFinished {
                    path,
                    bytes,
                } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Completed;
                        item.bytes_downloaded = *bytes;
                        item.bytes_total = item.bytes_total.or(Some(*bytes));
                    }
                }
                InstallJobEventKind::DownloadRequestFailed { path } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Failed;
                    }
                }
                InstallJobEventKind::ContentFileCompleted { path, bytes } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Completed;
                        item.bytes_downloaded = *bytes;
                        item.bytes_total = Some(*bytes);
                        item.error = None;
                    } else {
                        let index = items.len();
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: path.clone(),
                            project_id: None,
                            version_id: None,
                            status: DownloadItemStatus::Completed,
                            bytes_downloaded: *bytes,
                            bytes_total: Some(*bytes),
                            attempt: None,
                            max_attempts: None,
                            error: None,
                            manual_url: None,
                            request_url: None,
                            source: None,
                        });
                        indices.insert(path.clone(), index);
                    }
                }
                InstallJobEventKind::ContentFileSkipped {
                    path,
                    reason,
                    project_id,
                    version_id,
                    manual_url,
                } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Skipped;
                        item.bytes_downloaded = 0;
                        item.project_id = project_id.clone();
                        item.version_id = version_id.clone();
                        item.error = Some(reason.clone());
                        item.manual_url = manual_url.clone();
                    } else {
                        let index = items.len();
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: path.clone(),
                            project_id: project_id.clone(),
                            version_id: version_id.clone(),
                            status: DownloadItemStatus::Skipped,
                            bytes_downloaded: 0,
                            bytes_total: None,
                            attempt: None,
                            max_attempts: None,
                            error: Some(reason.clone()),
                            manual_url: manual_url.clone(),
                            request_url: None,
                            source: None,
                        });
                        indices.insert(path.clone(), index);
                    }
                }
                InstallJobEventKind::ContentFileFailed {
                    path,
                    reason,
                    project_id,
                    version_id,
                } => {
                    if let Some(item) = indices
                        .get(path)
                        .and_then(|&index| items.get_mut(index))
                    {
                        item.status = DownloadItemStatus::Failed;
                        item.bytes_downloaded = 0;
                        item.project_id = project_id.clone();
                        item.version_id = version_id.clone();
                        item.error = Some(reason.clone());
                    } else {
                        let index = items.len();
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: path.clone(),
                            project_id: project_id.clone(),
                            version_id: version_id.clone(),
                            status: DownloadItemStatus::Failed,
                            bytes_downloaded: 0,
                            bytes_total: None,
                            attempt: None,
                            max_attempts: None,
                            error: Some(reason.clone()),
                            manual_url: None,
                            request_url: None,
                            source: None,
                        });
                        indices.insert(path.clone(), index);
                    }
                }
                InstallJobEventKind::JobCanceled { .. } => {
                    for item in &mut items {
                        if matches!(
                            item.status,
                            DownloadItemStatus::Queued
                                | DownloadItemStatus::Downloading
                                | DownloadItemStatus::Verifying
                                | DownloadItemStatus::Writing
                        ) {
                            item.status = DownloadItemStatus::Canceled;
                        }
                    }
                }
                _ => {}
            }
        }
        items
    }

    /// Applies the live `active_downloads` overlay on top of an event-replay
    /// snapshot, producing the final item list.
    fn overlay_active_downloads(
        &self,
        mut items: Vec<DownloadItemSnapshot>,
    ) -> Vec<DownloadItemSnapshot> {
        let mut indices = HashMap::<String, usize>::new();
        for (index, item) in items.iter().enumerate() {
            indices.insert(item.id.clone(), index);
        }
        let terminal = self.events.iter().rev().any(|event| {
            matches!(
                event.kind,
                InstallJobEventKind::JobSucceeded { .. }
                    | InstallJobEventKind::JobCanceled { .. }
            )
        });
        if !terminal {
            for (id, active) in &self.active_downloads {
                if let Some(item) =
                    indices.get(id).and_then(|&index| items.get_mut(index))
                {
                    item.name = active.name.clone();
                    item.status = active.status;
                    item.bytes_downloaded = active.bytes_downloaded;
                    item.bytes_total = active.bytes_total;
                    item.attempt = Some(active.attempt);
                    item.max_attempts = Some(active.max_attempts);
                    item.error = None;
                    item.request_url = Some(active.url.clone());
                    item.source = Some(active.source.clone());
                } else {
                    items.push(DownloadItemSnapshot {
                        id: id.clone(),
                        name: active.name.clone(),
                        project_id: None,
                        version_id: None,
                        status: active.status,
                        bytes_downloaded: active.bytes_downloaded,
                        bytes_total: active.bytes_total,
                        attempt: Some(active.attempt),
                        max_attempts: Some(active.max_attempts),
                        error: None,
                        manual_url: None,
                        request_url: Some(active.url.clone()),
                        source: Some(active.source.clone()),
                    });
                }
            }
        }
        items
    }

    pub fn download_items(&self) -> Vec<DownloadItemSnapshot> {
        let cached = self
            .download_items_cache
            .lock()
            .expect("download items cache lock poisoned")
            .clone();
        let from_events = match cached {
            Some(items) => items,
            None => {
                let items = self.download_items_from_events();
                *self
                    .download_items_cache
                    .lock()
                    .expect("download items cache lock poisoned") =
                    Some(items.clone());
                items
            }
        };
        self.overlay_active_downloads(from_events)
    }

    /// Replays `events` into the event-derived summary base (settled files,
    /// byte totals, source, fallback count). This is the expensive part of
    /// `download_summary` and its result is cached; live progress and speed
    /// are applied by the caller.
    fn summary_from_events(&self) -> DownloadJobSummary {
        let mut summary = DownloadJobSummary::default();
        let mut settled_content = HashMap::<String, u64>::new();
        for event in &self.events {
            match &event.kind {
                InstallJobEventKind::ContentDownloadStarted {
                    files,
                    bytes,
                } => {
                    summary.files_total = Some(*files);
                    summary.bytes_total = *bytes;
                }

                InstallJobEventKind::ContentFileCompleted { path, bytes }
                | InstallJobEventKind::ContentFileRecovered { path, bytes } => {
                    // 同一个文件在暂停/恢复后可能再次产生 settled 事件。
                    // 按 path 保存最终状态，避免重复累计。
                    settled_content.insert(path.clone(), *bytes);
                }

                InstallJobEventKind::ContentFileSkipped { path, .. }
                | InstallJobEventKind::ContentFileFailed { path, .. } => {
                    // Failed / Skipped 算作已处理，但不贡献完成字节。
                    // 如果之后恢复成功，同一个 path 会被 Recovered 覆盖成真实大小。
                    settled_content.insert(path.clone(), 0);
                }

                InstallJobEventKind::DownloadMetrics {
                    source,
                    fallback_count,
                } => {
                    summary.source = Some(source.clone());
                    summary.fallback_count =
                        summary.fallback_count.saturating_add(*fallback_count);
                }

                _ => {}
            }
        }
        summary.files_completed = settled_content.len() as u64;
        summary.bytes_downloaded = settled_content
            .values()
            .copied()
            .fold(0_u64, u64::saturating_add);
        summary
    }

    pub fn download_summary(&self) -> DownloadJobSummary {
        let cached = self
            .summary_cache
            .lock()
            .expect("download summary cache lock poisoned")
            .clone();
        let mut summary = match cached {
            Some(summary) => summary,
            None => {
                let summary = self.summary_from_events();
                *self
                    .summary_cache
                    .lock()
                    .expect("download summary cache lock poisoned") =
                    Some(summary.clone());
                summary
            }
        };
        if let Some(progress) = &self.progress.progress {
            if self.progress.phase == InstallPhaseId::DownloadingContent {
                summary.files_completed = progress.current;
                summary.files_total = Some(progress.total);

                if let Some(bytes) = &progress.secondary {
                    summary.bytes_downloaded = bytes.current;
                    summary.bytes_total = Some(bytes.total);
                }
            } else if self.progress.phase
                == InstallPhaseId::DownloadingMinecraft
                || self.progress.phase == InstallPhaseId::DownloadingPackFile
                || matches!(
                    &self.progress.details,
                    InstallPhaseDetails::Java {
                        step: InstallJavaStep::Downloading,
                        ..
                    }
                )
            {
                summary.bytes_downloaded = progress.current;
                summary.bytes_total = Some(progress.total);
            }
        }
        let actively_downloading = matches!(
            self.progress.phase,
            InstallPhaseId::DownloadingPackFile
                | InstallPhaseId::DownloadingContent
                | InstallPhaseId::DownloadingMinecraft
        ) || matches!(
            &self.progress.details,
            InstallPhaseDetails::Java {
                step: InstallJavaStep::Downloading,
                ..
            }
        );
        let active_speed = self
            .active_downloads
            .values()
            .filter(|download| {
                Utc::now()
                    .signed_duration_since(download.last_progress_at)
                    .num_milliseconds()
                    < 3_000
            })
            .filter_map(|download| download.speed_bytes_per_second)
            .fold(0_u64, u64::saturating_add);
        if actively_downloading && active_speed > 0 {
            summary.speed_bytes_per_second = Some(active_speed);
            summary.eta_seconds = summary.bytes_total.and_then(|total| {
                total
                    .saturating_sub(summary.bytes_downloaded)
                    .checked_add(active_speed - 1)
                    .and_then(|remaining| remaining.checked_div(active_speed))
            });
        }
        summary
    }
}
