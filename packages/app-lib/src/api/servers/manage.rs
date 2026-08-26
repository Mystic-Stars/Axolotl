//! Listing, creation, and settings management for servers.

use chrono::Utc;
use uuid::Uuid;

use crate::state::remove_log_buffer;
use crate::util::io::{self, IOError};
use crate::{ErrorKind, Result, State};

use super::lifecycle::is_running;
use super::manifest::{
    ServerInfo, ServerManifest, build_server_info, read_manifest,
    sanitize_folder_name, server_path, type_default_jar_name, write_manifest,
};

pub async fn list() -> Result<Vec<ServerInfo>> {
    let state = State::get().await?;
    let servers_dir = state.directories.servers_dir();
    if !servers_dir.exists() {
        return Ok(Vec::new());
    }

    let mut servers = Vec::new();
    let mut entries = tokio::fs::read_dir(&servers_dir)
        .await
        .map_err(|e| IOError::with_path(e, &servers_dir))?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, &servers_dir))?
    {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(manifest) = read_manifest(&path).await else {
            continue;
        };
        servers.push(build_server_info(&manifest, &path).await);
    }
    servers.sort_by(|a, b| {
        a.manifest
            .name
            .to_lowercase()
            .cmp(&b.manifest.name.to_lowercase())
    });
    Ok(servers)
}

pub async fn get(server_id: &str) -> Result<ServerInfo> {
    let path = server_path(server_id).await?;
    let manifest = read_manifest(&path).await?;
    Ok(build_server_info(&manifest, &path).await)
}

pub async fn create(
    name: &str,
    server_type: &str,
    game_version: &str,
    loader_version: Option<String>,
    java_path: Option<String>,
    memory_mb: Option<u32>,
) -> Result<ServerManifest> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ErrorKind::InputError(
            "Server name cannot be empty".to_string(),
        )
        .as_error());
    }

    let state = State::get().await?;
    let id = Uuid::new_v4().to_string();
    let dir_name = format!("{}-{}", sanitize_folder_name(name), &id[..8]);
    let dir = state.directories.servers_dir().join(&dir_name);
    io::create_dir_all(&dir).await?;

    let manifest = ServerManifest {
        id: dir_name,
        name: name.to_string(),
        server_type: server_type.to_string(),
        game_version: game_version.to_string(),
        loader_version,
        jar_name: type_default_jar_name(server_type),
        java_path,
        memory_mb,
        icon_path: None,
        jvm_args: Vec::new(),
        created_at: Utc::now(),
        last_started_at: None,
        last_exit_crashed: false,
    };
    write_manifest(&dir, &manifest).await?;
    Ok(manifest)
}

/// Sets or clears the server icon. `None` resets to the default icon.
pub async fn set_icon(
    server_id: &str,
    icon_path: Option<String>,
) -> Result<ServerManifest> {
    let path = server_path(server_id).await?;
    let mut manifest = read_manifest(&path).await?;
    manifest.icon_path = icon_path;
    write_manifest(&path, &manifest).await?;
    Ok(manifest)
}

pub async fn update_settings(
    server_id: &str,
    name: Option<String>,
    java_path: Option<String>,
    memory_mb: Option<u32>,
    jvm_args: Option<Vec<String>>,
) -> Result<ServerManifest> {
    let path = server_path(server_id).await?;
    let mut manifest = read_manifest(&path).await?;
    if let Some(name) = name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(ErrorKind::InputError(
                "Server name cannot be empty".to_string(),
            )
            .as_error());
        }
        manifest.name = name;
    }
    if let Some(java_path) = java_path {
        manifest.java_path = if java_path.is_empty() {
            None
        } else {
            Some(java_path)
        };
    }
    if let Some(memory_mb) = memory_mb {
        manifest.memory_mb = Some(memory_mb);
    }
    if let Some(jvm_args) = jvm_args {
        manifest.jvm_args = jvm_args;
    }
    write_manifest(&path, &manifest).await?;
    Ok(manifest)
}

pub async fn delete(server_id: &str) -> Result<()> {
    if is_running(server_id) {
        return Err(ErrorKind::InputError(
            "Stop the server before deleting it".to_string(),
        )
        .as_error());
    }
    let path = server_path(server_id).await?;
    remove_log_buffer(server_id);
    tokio::fs::remove_dir_all(&path)
        .await
        .map_err(|e| IOError::with_path(e, &path))?;
    Ok(())
}
