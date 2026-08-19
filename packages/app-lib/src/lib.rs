/*!
# Theseus

Theseus is a library which provides utilities for launching minecraft, creating Modrinth mod packs,
and launching Modrinth mod packs
*/
#![warn(unused_import_braces)]
#![deny(unused_must_use)]
#![recursion_limit = "256"]

#[macro_use]
mod util;

mod api;
pub mod brand;
mod error;
mod event;
pub mod install;
mod launcher;
mod logger;
pub mod mod_metadata;
mod state;
pub mod storage;
pub mod telemetry;

pub use api::*;
pub use error::*;
pub use event::{
    EventState, LoadingBar, LoadingBarType, emit::emit_loading,
    emit::init_loading,
};
pub use logger::start_logger;
pub use state::State;
pub use storage::*;
pub use util::fetch::DownloadReason;
pub use util::file_lock::{LockingProcess, get_locking_processes};
pub use util::platform::is_process_elevated;
pub use util::symlink::SymlinkCapability;

pub fn launcher_user_agent() -> String {
    brand::user_agent(env!("CARGO_PKG_VERSION"), std::env::consts::OS)
}
