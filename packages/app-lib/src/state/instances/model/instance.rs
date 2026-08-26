use crate::state::{
    InstanceInstallStage, LauncherFeatureVersion, ReleaseChannel,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub path: String,
    pub applied_content_set_id: Option<String>,
    pub install_stage: InstanceInstallStage,
    pub launcher_feature_version: LauncherFeatureVersion,
    pub update_channel: ReleaseChannel,
    pub name: String,
    pub icon_path: Option<String>,
    pub symlink_target: Option<String>,
    #[serde(default)]
    pub game_dir_override: Option<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub pinned_at: Option<DateTime<Utc>>,
    pub submitted_time_played: u64,
    pub recent_time_played: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct DailyPlaytime {
    pub date: String,
    pub played_seconds: u64,
    pub session_count: u64,
    pub top_instance_name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DailyPlaytimeEntry {
    pub instance_id: String,
    pub instance_name: String,
    pub played_seconds: u64,
    pub session_count: u64,
}

pub(crate) fn playtime_to_storage(
    value: u64,
    column: &str,
) -> crate::Result<i64> {
    i64::try_from(value).map_err(|_| {
        crate::ErrorKind::InputError(format!(
            "Expected {column} to fit in SQLite INTEGER"
        ))
        .into()
    })
}
