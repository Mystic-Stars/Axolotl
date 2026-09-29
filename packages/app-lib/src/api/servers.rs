//! Managed dedicated Minecraft servers: manifests, downloads, and process control.
//! Each server lives in its own directory under the launcher's `servers` folder
//! and is described by an `axolotl-server.json` manifest.

mod files;
mod forge;
mod lifecycle;
mod logs;
mod manage;
mod manifest;
mod modpack;
mod ports;

pub use self::files::{download_file, read_file, write_file};
pub use self::forge::install_forge;
pub use self::lifecycle::{
    kill, resize_console, send_command, send_console_input, shutdown_all,
    start, stop,
};
pub use self::logs::{clear_log, get_log_buffer};
pub use self::manage::{create, delete, get, list, set_icon, update_settings};
pub use self::manifest::{ModpackInfo, ServerInfo, ServerManifest};
pub use self::modpack::install_modpack;
pub use self::ports::{PortProcessInfo, kill_port_process, port_process};
