use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{ContentProvider, unknown_value};
use crate::state::ProjectType;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ContentOwnershipKind {
    PackManaged,
    #[default]
    UserAdded,
    LocalDiscovered,
}

impl ContentOwnershipKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PackManaged => "pack_managed",
            Self::UserAdded => "user_added",
            Self::LocalDiscovered => "local_discovered",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "pack_managed" => Ok(Self::PackManaged),
            "user_added" => Ok(Self::UserAdded),
            "local_discovered" => Ok(Self::LocalDiscovered),
            other => Err(unknown_value("content ownership kind", other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackMemberMaterializationState {
    Present,
    PendingManual,
    Missing,
    Removed,
}

impl PackMemberMaterializationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::PendingManual => "pending_manual",
            Self::Missing => "missing",
            Self::Removed => "removed",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "present" => Ok(Self::Present),
            "pending_manual" => Ok(Self::PendingManual),
            "missing" => Ok(Self::Missing),
            "removed" => Ok(Self::Removed),
            other => {
                Err(unknown_value("pack member materialization state", other))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackMemberOverrideKind {
    None,
    Disabled,
    Removed,
    Version,
}

impl PackMemberOverrideKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Disabled => "disabled",
            Self::Removed => "removed",
            Self::Version => "version",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "none" => Ok(Self::None),
            "disabled" => Ok(Self::Disabled),
            "removed" => Ok(Self::Removed),
            "version" => Ok(Self::Version),
            other => Err(unknown_value("pack member override kind", other)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackMember {
    pub id: String,
    pub content_set_id: String,
    pub content_entry_id: Option<String>,
    pub member_key: String,
    pub project_type: ProjectType,
    pub expected_relative_path: String,
    pub provider: Option<ContentProvider>,
    pub provider_project_id: Option<String>,
    pub provider_release_id: Option<String>,
    pub required: bool,
    pub expected_sha1: Option<String>,
    pub expected_size: Option<u64>,
    pub expected_fingerprint: Option<u64>,
    pub materialization_state: PackMemberMaterializationState,
    pub override_kind: PackMemberOverrideKind,
    pub reconciled: bool,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ManualDownloadOperationKind {
    PackInstall,
    PackUpdate,
    #[default]
    ContentInstall,
    ContentUpdate,
}

impl ManualDownloadOperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PackInstall => "pack_install",
            Self::PackUpdate => "pack_update",
            Self::ContentInstall => "content_install",
            Self::ContentUpdate => "content_update",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "pack_install" => Ok(Self::PackInstall),
            "pack_update" => Ok(Self::PackUpdate),
            "content_install" => Ok(Self::ContentInstall),
            "content_update" => Ok(Self::ContentUpdate),
            other => {
                Err(unknown_value("manual download operation kind", other))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualDownloadState {
    Waiting,
    Matched,
    Imported,
    Error,
    Cancelled,
}

impl ManualDownloadState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Matched => "matched",
            Self::Imported => "imported",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "waiting" => Ok(Self::Waiting),
            "matched" => Ok(Self::Matched),
            "imported" => Ok(Self::Imported),
            "error" => Ok(Self::Error),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(unknown_value("manual download state", other)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingManualDownload {
    pub id: String,
    pub instance_id: String,
    pub pack_member_id: Option<String>,
    pub content_entry_id: Option<String>,
    pub operation_kind: ManualDownloadOperationKind,
    pub operation_target_id: Option<String>,
    pub project_type: ProjectType,
    pub provider: ContentProvider,
    pub provider_project_id: String,
    pub provider_release_id: String,
    pub file_name: String,
    pub website_url: Option<String>,
    pub target_relative_path: String,
    pub expected_sha1: Option<String>,
    pub expected_size: Option<u64>,
    pub expected_fingerprint: Option<u64>,
    pub state: ManualDownloadState,
    pub context: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_manual_download_serializes_camel_case_fields() {
        let now = Utc::now();
        let value = serde_json::to_value(PendingManualDownload {
            id: "manual-download:test".to_string(),
            instance_id: "instance:test".to_string(),
            pack_member_id: None,
            content_entry_id: None,
            operation_kind: ManualDownloadOperationKind::PackInstall,
            operation_target_id: None,
            project_type: ProjectType::Mod,
            provider: ContentProvider::CurseForge,
            provider_project_id: "123".to_string(),
            provider_release_id: "456".to_string(),
            file_name: "example.jar".to_string(),
            website_url: None,
            target_relative_path: "mods/example.jar".to_string(),
            expected_sha1: None,
            expected_size: None,
            expected_fingerprint: None,
            state: ManualDownloadState::Waiting,
            context: serde_json::Value::Null,
            created_at: now,
            modified_at: now,
        })
        .expect("pending manual download should serialize");

        assert_eq!(value["targetRelativePath"], "mods/example.jar");
        assert!(value.get("target_relative_path").is_none());
        assert_eq!(value["providerProjectId"], "123");
    }
}
