use serde::{Deserialize, Serialize};

use super::InstanceUpgradeIssueCode;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancePostUpgradeWarning {
    pub code: InstanceUpgradeIssueCode,
    pub content_id: Option<String>,
    pub relative_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancePostUpgradeNotice {
    pub instance_id: String,
    pub upgrade_job_id: String,
    pub target_game_version: String,
    pub consecutive_clean_launches: u8,
    pub warnings: Vec<InstancePostUpgradeWarning>,
}
