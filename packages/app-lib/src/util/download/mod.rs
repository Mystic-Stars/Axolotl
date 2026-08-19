//! Modular download engine selection.
//!
//! The launcher can use either the native adaptive engine (HTTP/2
//! multiplexing with shared per-authority connections, falling back to
//! HTTP/1.1 single-stream) or the XMCL-compatible engine. The native engine
//! is the default; the XMCL engine remains available for users who prefer it.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

pub mod h2_download;
pub mod h2_pool;
pub mod legacy;
pub mod log;
pub(crate) mod native;
pub(crate) mod native_breaker;
pub(crate) mod native_budget;
pub(crate) mod native_reputation;
pub(crate) mod native_slow;
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

pub(crate) fn task_concurrency_limit(state: &crate::State) -> Option<usize> {
    if active_engine() == DownloadEngine::XmclCompat {
        None
    } else {
        Some(state.download_concurrency())
    }
}
