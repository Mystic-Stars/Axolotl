//! Server manifests and per-server metadata: the `axolotl-server.json`
//! document, derived display info, and path helpers shared by the other
//! `servers` submodules.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::State;
use crate::util::io;
use crate::{ErrorKind, Result};

use super::lifecycle::is_running;

const MANIFEST_FILE: &str = "axolotl-server.json";
const DEFAULT_JAR_NAME: &str = "server.jar";
/// Executable launcher jar downloaded from Fabric Meta; must match the
/// filename used by the frontend's `resolveServerJar('fabric')`.
const FABRIC_SERVER_JAR_NAME: &str = "fabric-server.jar";

pub(super) fn type_default_jar_name(server_type: &str) -> Option<String> {
    match server_type {
        "fabric" => Some(FABRIC_SERVER_JAR_NAME.to_string()),
        _ => None,
    }
}

/// Resolves the jar a server launches with: the manifest override, then the
/// server type default, then the generic default.
pub(super) fn resolve_jar_name(manifest: &ServerManifest) -> String {
    manifest
        .jar_name
        .clone()
        .or_else(|| type_default_jar_name(&manifest.server_type))
        .unwrap_or_else(|| DEFAULT_JAR_NAME.to_string())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerManifest {
    pub id: String,
    pub name: String,
    pub server_type: String,
    pub game_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jar_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jvm_args: Vec<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_exit_crashed: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct ServerInfo {
    #[serde(flatten)]
    pub manifest: ServerManifest,
    pub path: String,
    pub running: bool,
    pub eula_exists: bool,
    pub eula_accepted: bool,
    pub port: Option<u16>,
}

pub(super) async fn server_path(server_id: &str) -> Result<PathBuf> {
    if server_id.contains(['/', '\\'])
        || server_id.contains("..")
        || server_id.is_empty()
    {
        return Err(ErrorKind::InputError(format!(
            "Invalid server id: {server_id}"
        ))
        .as_error());
    }
    let state = State::get().await?;
    let path = state.directories.server_dir(server_id);
    if !path.is_dir() {
        return Err(ErrorKind::InputError(format!(
            "Unknown server: {server_id}"
        ))
        .as_error());
    }
    Ok(path)
}

pub(super) async fn read_manifest(dir: &Path) -> Result<ServerManifest> {
    let bytes = io::read(dir.join(MANIFEST_FILE)).await?;
    serde_json::from_slice(&bytes).map_err(|e| {
        ErrorKind::FSError(format!("Failed to parse server manifest: {e}"))
            .as_error()
    })
}

pub(super) async fn write_manifest(
    dir: &Path,
    manifest: &ServerManifest,
) -> Result<()> {
    let contents = serde_json::to_string_pretty(manifest)?;
    io::write(dir.join(MANIFEST_FILE), contents).await?;
    Ok(())
}

pub(super) fn sanitize_folder_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "server".to_string()
    } else {
        trimmed.chars().take(32).collect()
    }
}

pub(super) async fn build_server_info(
    manifest: &ServerManifest,
    path: &Path,
) -> ServerInfo {
    let eula_text = tokio::fs::read_to_string(path.join("eula.txt"))
        .await
        .unwrap_or_default();
    let eula_exists = !eula_text.is_empty();
    let eula_accepted = eula_text
        .lines()
        .find_map(|line| line.split_once('='))
        .filter(|(key, _)| key.trim() == "eula")
        .is_some_and(|(_, value)| value.trim().eq_ignore_ascii_case("true"));
    let port = tokio::fs::read_to_string(path.join("server.properties"))
        .await
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                let (key, value) = line.split_once('=')?;
                (key.trim() == "server-port")
                    .then(|| value.trim().parse::<u16>().ok())?
            })
        });
    ServerInfo {
        manifest: manifest.clone(),
        path: path.to_string_lossy().into_owned(),
        running: is_running(&manifest.id),
        eula_exists,
        eula_accepted,
        port,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_folder_name_replaces_unsafe_characters() {
        assert_eq!(sanitize_folder_name("My Server!"), "My-Server");
        assert_eq!(sanitize_folder_name("../etc"), "etc");
        assert_eq!(sanitize_folder_name("///"), "server");
    }

    #[test]
    fn server_manifest_round_trips() {
        let manifest = ServerManifest {
            id: "test-12345678".to_string(),
            name: "Test".to_string(),
            server_type: "vanilla".to_string(),
            game_version: "1.21.4".to_string(),
            loader_version: None,
            jar_name: None,
            java_path: None,
            memory_mb: Some(2048),
            icon_path: None,
            jvm_args: Vec::new(),
            created_at: Utc::now(),
            last_started_at: None,
            last_exit_crashed: false,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: ServerManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, manifest.id);
        assert_eq!(parsed.name, manifest.name);
    }

    #[test]
    fn resolves_jar_name_from_type_then_manifest() {
        let mut manifest = ServerManifest {
            id: "test-12345678".to_string(),
            name: "Test".to_string(),
            server_type: "fabric".to_string(),
            game_version: "26.2".to_string(),
            loader_version: Some("0.19.3".to_string()),
            jar_name: None,
            java_path: None,
            memory_mb: None,
            icon_path: None,
            jvm_args: Vec::new(),
            created_at: Utc::now(),
            last_started_at: None,
            last_exit_crashed: false,
        };
        assert_eq!(resolve_jar_name(&manifest), FABRIC_SERVER_JAR_NAME);

        manifest.server_type = "vanilla".to_string();
        assert_eq!(resolve_jar_name(&manifest), DEFAULT_JAR_NAME);

        manifest.jar_name = Some("custom.jar".to_string());
        assert_eq!(resolve_jar_name(&manifest), "custom.jar");
    }
}
