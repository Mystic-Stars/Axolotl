use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::ContentProvider;

use super::unknown_value;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentDependencyKind {
    Required,
    Include,
}

impl ContentDependencyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Include => "include",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "required" => Ok(Self::Required),
            "include" => Ok(Self::Include),
            other => Err(unknown_value("content dependency kind", other)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentDependencyEdge {
    pub id: String,
    pub content_set_id: String,
    pub parent_entry_id: String,
    pub child_entry_id: String,
    /// Provider that supplied the dependency declaration or corroborating
    /// metadata. It is not assumed to own either endpoint.
    pub evidence_provider: ContentProvider,
    pub parent_provider: ContentProvider,
    pub child_provider: ContentProvider,
    pub dependency_kind: ContentDependencyKind,
    pub parent_project_id: String,
    pub parent_release_id: String,
    pub child_project_id: String,
    pub child_release_id: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

/// Provider identity for one end of a persisted dependency edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDependencyRef {
    pub provider: ContentProvider,
    pub project_id: String,
    pub release_id: String,
}

/// Dependency state attached to a content snapshot item.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDependencyInfo {
    pub auto_dependency: bool,
    pub required_by: Vec<ContentDependencyRef>,
    pub requires: Vec<ContentDependencyRef>,
    #[serde(default)]
    pub orphaned: bool,
}
