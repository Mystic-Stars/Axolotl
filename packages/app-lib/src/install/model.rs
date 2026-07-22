use crate::api::pack::import::ImportLauncherType;
use crate::api::pack::install_from::{CreatePackInstance, CreatePackLocation};
use crate::state::{
    InstanceInstallStage, InstanceLink, InstanceMetadata, ModLoader,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub type InstallModpackPreview = CreatePackInstance;

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    #[serde(default)]
    pub display: Option<InstallJobDisplay>,
    pub rollback: Option<InstallRollbackState>,
    pub error: Option<InstallErrorView>,
    #[serde(default)]
    pub rollback_error: Option<InstallErrorView>,
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
            display: None,
            rollback: None,
            error: None,
            rollback_error: None,
        }
    }

    pub fn record_event(&mut self, kind: InstallJobEventKind) {
        self.events.push(InstallJobEvent {
            at: Utc::now(),
            kind,
        });
    }

    pub fn set_context(&mut self, context: Option<InstallErrorContext>) {
        self.context = context;
    }

    pub fn set_progress(
        &mut self,
        phase: InstallPhaseId,
        progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
    ) {
        if self.progress.phase != phase
            || matches!(&self.progress.details, InstallPhaseDetails::Empty)
                && !matches!(&details, InstallPhaseDetails::Empty)
        {
            self.record_event(InstallJobEventKind::PhaseStarted {
                phase,
                details: details.clone(),
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

    #[test]
    fn download_summary_uses_content_events_and_live_progress() {
        let mut job = job_state();
        job.record_event(InstallJobEventKind::ContentDownloadStarted {
            files: 3,
            bytes: Some(300),
        });
        job.record_event(InstallJobEventKind::ContentFileCompleted {
            path: "mods/a.jar".to_string(),
            bytes: 100,
        });
        job.set_progress(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: 2,
                total: 3,
                secondary: Some(InstallProgressSecondary {
                    current: 220,
                    total: 300,
                }),
            }),
            InstallPhaseDetails::Empty,
        );

        let summary = job.download_summary();
        assert_eq!(summary.files_completed, 2);
        assert_eq!(summary.files_total, Some(3));
        assert_eq!(summary.bytes_downloaded, 220);
        assert_eq!(summary.bytes_total, Some(300));
        assert_eq!(job.download_items().len(), 1);
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
    PhaseStarted {
        phase: InstallPhaseId,
        details: InstallPhaseDetails,
    },
    ContentDownloadStarted {
        files: u64,
        bytes: Option<u64>,
    },
    ContentFileSkipped {
        path: String,
        reason: String,
    },
    ContentFileCompleted {
        path: String,
        bytes: u64,
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
        symlink: bool,
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
        }
    }

    pub fn target(&self) -> InstallTarget {
        match self {
            Self::InstallExistingInstance { instance_id, .. }
            | Self::InstallPackToExistingInstance { instance_id, .. } => {
                InstallTarget::ExistingInstance {
                    instance_id: instance_id.clone(),
                }
            }
            _ => InstallTarget::NewInstance { instance_id: None },
        }
    }

    pub fn cleanup(&self) -> InstallCleanup {
        match self {
            Self::InstallExistingInstance { instance_id, .. }
            | Self::InstallPackToExistingInstance { instance_id, .. } => {
                InstallCleanup::RestoreExistingInstance {
                    instance_id: instance_id.clone(),
                }
            }
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
pub struct DownloadItemSnapshot {
    pub id: String,
    pub name: String,
    pub status: DownloadItemStatus,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub error: Option<String>,
    pub manual_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DownloadJobSummary {
    pub files_completed: u64,
    pub files_total: Option<u64>,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallTarget {
    NewInstance { instance_id: Option<String> },
    ExistingInstance { instance_id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallCleanup {
    DeleteNewInstance { instance_id: Option<String> },
    RestoreExistingInstance { instance_id: String },
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
    pub provider: InstallJobProvider,
    pub target: InstallTarget,
    pub phase: InstallPhaseId,
    pub progress: Option<InstallProgress>,
    pub details: InstallPhaseDetails,
    pub display: Option<InstallJobDisplay>,
    pub error: Option<InstallErrorView>,
    pub rollback_error: Option<InstallErrorView>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
    pub summary: DownloadJobSummary,
    pub items: Vec<DownloadItemSnapshot>,
}

impl InstallJobState {
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
            InstallRequest::ImportInstance { .. }
            | InstallRequest::DuplicateInstance { .. } => {
                InstallJobProvider::Local
            }
        }
    }

    pub fn download_items(&self) -> Vec<DownloadItemSnapshot> {
        self.events
            .iter()
            .filter_map(|event| match &event.kind {
                InstallJobEventKind::ContentFileCompleted { path, bytes } => {
                    Some(DownloadItemSnapshot {
                        id: path.clone(),
                        name: path.clone(),
                        status: DownloadItemStatus::Completed,
                        bytes_downloaded: *bytes,
                        bytes_total: Some(*bytes),
                        error: None,
                        manual_url: None,
                    })
                }
                InstallJobEventKind::ContentFileSkipped { path, reason } => {
                    Some(DownloadItemSnapshot {
                        id: path.clone(),
                        name: path.clone(),
                        status: DownloadItemStatus::Skipped,
                        bytes_downloaded: 0,
                        bytes_total: None,
                        error: Some(reason.clone()),
                        manual_url: None,
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn download_summary(&self) -> DownloadJobSummary {
        let mut summary = DownloadJobSummary::default();
        for event in &self.events {
            match &event.kind {
                InstallJobEventKind::ContentDownloadStarted {
                    files,
                    bytes,
                } => {
                    summary.files_total = Some(*files);
                    summary.bytes_total = *bytes;
                }
                InstallJobEventKind::ContentFileCompleted { bytes, .. } => {
                    summary.files_completed += 1;
                    summary.bytes_downloaded =
                        summary.bytes_downloaded.saturating_add(*bytes);
                }
                InstallJobEventKind::ContentFileSkipped { .. } => {
                    summary.files_completed += 1;
                }
                _ => {}
            }
        }
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
            {
                summary.bytes_downloaded = progress.current;
                summary.bytes_total = Some(progress.total);
            }
        }
        summary
    }
}
