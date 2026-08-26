use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreComponentKind {
    JarMod,
    ReplacementJar,
    Agent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreComponentSource {
    pub provider: String,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub file_id: Option<String>,
    pub page_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreComponent {
    pub id: String,
    pub kind: CoreComponentKind,
    pub file_name: String,
    pub relative_path: String,
    pub enabled: bool,
    pub removed: bool,
    pub order: i32,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub source: Option<CoreComponentSource>,
    pub target_game_version: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub failure_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreJarPreview {
    pub output_path: String,
    pub component_count: usize,
    pub replacement_component_id: Option<String>,
    pub entries: usize,
    pub sha1: String,
    pub sha256: String,
}
