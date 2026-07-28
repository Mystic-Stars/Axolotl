use crate::state::{ContentProvider, ReleaseChannel};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentUpdateCheck {
    pub content_entry_id: String,
    pub update_channel: ReleaseChannel,
    pub provider: Option<ContentProvider>,
    pub provider_project_id: Option<String>,
    pub provider_release_id: Option<String>,
    pub checked_at: DateTime<Utc>,
}
