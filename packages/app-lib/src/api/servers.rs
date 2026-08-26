//! Managed dedicated Minecraft servers: manifests, downloads, and process control.
//! Each server lives in its own directory under the launcher's `servers` folder
//! and is described by an `axolotl-server.json` manifest.

mod files;
mod lifecycle;
mod logs;
mod manage;
mod manifest;
mod ports;

pub use self::files::{download_file, read_file, write_file};
pub use self::lifecycle::{kill, send_command, start, stop};
pub use self::logs::{clear_log, get_log_buffer};
pub use self::manage::{create, delete, get, list, set_icon, update_settings};
pub use self::manifest::{ServerInfo, ServerManifest};
pub use self::ports::{PortProcessInfo, kill_port_process, port_process};
