use crate::state::{
    InstanceInstallStage, LauncherFeatureVersion, ReleaseChannel,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct InstanceSyncedOptions {
    #[serde(default)]
    pub game_options: bool,
    pub command_history: bool,
    pub multiplayer_servers: bool,
    pub creative_hotbars: bool,
    pub screenshots: bool,
    #[serde(default)]
    pub resource_packs: bool,
    #[serde(default)]
    pub data_packs: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncedOption {
    GameOptions,
    CommandHistory,
    MultiplayerServers,
    CreativeHotbars,
    Screenshots,
    ResourcePacks,
    DataPacks,
}

impl SyncedOption {
    pub const ALL: [Self; 7] = [
        Self::GameOptions,
        Self::CommandHistory,
        Self::MultiplayerServers,
        Self::CreativeHotbars,
        Self::Screenshots,
        Self::ResourcePacks,
        Self::DataPacks,
    ];
    pub const fn is_available(self) -> bool {
        true
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GameOptions => "game_options",
            Self::CommandHistory => "command_history",
            Self::MultiplayerServers => "multiplayer_servers",
            Self::CreativeHotbars => "creative_hotbars",
            Self::Screenshots => "screenshots",
            Self::ResourcePacks => "resource_packs",
            Self::DataPacks => "data_packs",
        }
    }
}

pub type InstanceSyncedOption = SyncedOption;

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
    /// For "directly associated" instances: which external launcher manages
    /// the linked `.minecraft` (`hmcl`, `pcl2`, `pcl2_ce`, `generic`).
    #[serde(default)]
    pub linked_launcher: Option<String>,
    /// Canonical root selected for the external launcher import scan.
    #[serde(default)]
    pub linked_launcher_root: Option<String>,
    /// Absolute path of the linked `.minecraft` root the instance launches
    /// from; files are used in place, never copied or written to.
    #[serde(default)]
    pub linked_dot_minecraft: Option<String>,
    /// The actual version JSON stem used as the external version ID.
    #[serde(default)]
    pub linked_version_id: Option<String>,
    /// Canonical path of the actual local version JSON selected at creation.
    #[serde(default)]
    pub linked_version_json_path: Option<String>,
    /// Explicit game-directory layout selected for this external root. `None`
    /// preserves the launcher-specific behavior used by links created before
    /// external-root modes were configurable.
    #[serde(default)]
    pub linked_game_dir_mode: Option<String>,
    /// Optional absolute game-directory override for ordinary instances;
    /// directly associated instances resolve their game dir from the link
    /// metadata instead.
    #[serde(default)]
    pub game_dir_override: Option<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub pinned_at: Option<DateTime<Utc>>,
    pub submitted_time_played: u64,
    pub recent_time_played: u64,
}

impl Instance {
    /// Whether this instance is "directly associated" with an external
    /// launcher (HMCL/PCL): it has no profile directory of its own and its
    /// files live inside the linked `.minecraft`.
    pub fn is_direct_linked(&self) -> bool {
        self.linked_dot_minecraft
            .as_deref()
            .is_some_and(|linked| !linked.trim().is_empty())
    }
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
