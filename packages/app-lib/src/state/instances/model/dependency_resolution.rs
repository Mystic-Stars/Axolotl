use serde::{Deserialize, Serialize};

use crate::state::{ContentProvider, ContentProviderRef};

use super::ContentDependencyKind;

/// A provider-neutral, immutable selection of content dependencies. The
/// provider-specific resolver records the exact release to download here so a
/// preview and its subsequent install cannot independently select different
/// files.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResolutionPlan {
    pub id: String,
    pub instance_id: String,
    pub instance_revision: Option<u64>,
    pub target: DependencyResolutionTarget,
    pub primary: ContentProviderRef,
    #[serde(default)]
    pub primary_expected_sha1: Option<String>,
    #[serde(default)]
    pub primary_expected_size: Option<u64>,
    #[serde(default)]
    pub nodes: Vec<DependencyResolutionNode>,
    #[serde(default)]
    pub edges: Vec<DependencyResolutionEdge>,
    #[serde(default)]
    pub issues: Vec<DependencyResolutionIssue>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResolutionTarget {
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResolutionNode {
    pub content: ContentProviderRef,
    pub parent: Option<ContentProviderRef>,
    pub relation: ContentDependencyKind,
    pub source: ContentProvider,
    pub selection_reason: DependencySelectionReason,
    #[serde(default)]
    pub expected_sha1: Option<String>,
    #[serde(default)]
    pub expected_size: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResolutionEdge {
    pub parent: ContentProviderRef,
    pub child: ContentProviderRef,
    pub relation: ContentDependencyKind,
    pub evidence_provider: ContentProvider,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencySelectionReason {
    ExactVersionId,
    NativeStrictMatch,
    Sha1VerifiedModrinthFallback,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResolutionIssue {
    pub provider: ContentProvider,
    pub project_id: String,
    pub parent: Option<ContentProviderRef>,
    pub relation: Option<ContentDependencyKind>,
    pub reason: String,
}
