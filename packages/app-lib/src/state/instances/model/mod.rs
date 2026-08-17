#![allow(dead_code)]

mod content_entry;
pub use self::content_entry::*;

mod content_dependency;
pub use self::content_dependency::*;

mod dependency_resolution;
pub use self::dependency_resolution::*;

mod content_ownership;
pub use self::content_ownership::*;

mod content_snapshot;
pub use self::content_snapshot::*;

mod content_update_plan;
pub use self::content_update_plan::*;

mod content_provider;
pub use self::content_provider::*;

mod content_set;
pub use self::content_set::*;

mod content_set_remote_ref;
pub use self::content_set_remote_ref::*;

mod content_set_sync_state;
pub use self::content_set_sync_state::*;

mod file;
pub use self::file::*;

mod instance;
pub use self::instance::*;

mod install_candidate;
pub use self::install_candidate::*;

mod launch;
pub use self::launch::*;

mod link;
pub use self::link::*;

mod manifest;

mod update_check;
pub use self::update_check::*;

fn unknown_value(kind: &str, value: &str) -> crate::Error {
    crate::ErrorKind::InputError(format!("Unknown {kind} {value}")).into()
}
