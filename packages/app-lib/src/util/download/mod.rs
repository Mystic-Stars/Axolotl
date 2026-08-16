//! Modular download engine selection.
//!
//! The launcher can use either the legacy adaptive engine or the
//! XMCL-compatible engine. The legacy engine remains available as a
//! fallback and for users who opt out of the new engine.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

pub mod legacy;
pub mod log;
pub mod shared;
pub mod slow;
pub mod xmcl;

#[derive(
    Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize,
)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum DownloadEngine {
    #[default]
    Legacy,
    #[serde(rename = "xmcl", alias = "xmcl_compat")]
    XmclCompat,
}

impl DownloadEngine {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::XmclCompat => "xmcl",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "xmcl" | "xmcl_compat" => Self::XmclCompat,
            _ => Self::Legacy,
        }
    }
}

static ACTIVE_ENGINE: AtomicU8 = AtomicU8::new(0);

/// Returns the engine the launcher should use for new downloads.
pub fn active_engine() -> DownloadEngine {
    match ACTIVE_ENGINE.load(Ordering::Relaxed) {
        1 => DownloadEngine::XmclCompat,
        _ => DownloadEngine::Legacy,
    }
}

/// Sets the engine used by new downloads.
pub fn set_active_engine(engine: DownloadEngine) {
    ACTIVE_ENGINE.store(engine as u8, Ordering::Relaxed);
}
