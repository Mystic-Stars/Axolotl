use crate::api::Result;
use theseus::prelude::*;
use theseus::servers::ServerInfo;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("servers")
        .invoke_handler(tauri::generate_handler![
            servers_list,
            servers_get,
            servers_create,
            servers_update_settings,
            servers_set_icon,
            servers_delete,
            servers_read_file,
            servers_write_file,
            servers_download_file,
            servers_start,
            servers_send_command,
            servers_stop,
            servers_kill,
            servers_kill_port_process,
            servers_port_process,
            servers_get_log_buffer,
            servers_clear_log,
        ])
        .build()
}

#[tauri::command]
pub async fn servers_list() -> Result<Vec<ServerInfo>> {
    Ok(servers::list().await?)
}

#[tauri::command]
pub async fn servers_get(server_id: &str) -> Result<ServerInfo> {
    Ok(servers::get(server_id).await?)
}

#[tauri::command]
pub async fn servers_create(
    name: &str,
    server_type: &str,
    game_version: &str,
    loader_version: Option<String>,
    java_path: Option<String>,
    memory_mb: Option<u32>,
) -> Result<servers::ServerManifest> {
    Ok(servers::create(
        name,
        server_type,
        game_version,
        loader_version,
        java_path,
        memory_mb,
    )
    .await?)
}

#[tauri::command]
pub async fn servers_update_settings(
    server_id: &str,
    name: Option<String>,
    java_path: Option<String>,
    memory_mb: Option<u32>,
    jvm_args: Option<Vec<String>>,
) -> Result<servers::ServerManifest> {
    Ok(servers::update_settings(
        server_id, name, java_path, memory_mb, jvm_args,
    )
    .await?)
}

#[tauri::command]
pub async fn servers_set_icon(
    server_id: &str,
    icon_path: Option<String>,
) -> Result<servers::ServerManifest> {
    Ok(servers::set_icon(server_id, icon_path).await?)
}

#[tauri::command]
pub async fn servers_delete(server_id: &str) -> Result<()> {
    Ok(servers::delete(server_id).await?)
}

#[tauri::command]
pub async fn servers_read_file(server_id: &str, file: &str) -> Result<String> {
    Ok(servers::read_file(server_id, file).await?)
}

#[tauri::command]
pub async fn servers_write_file(
    server_id: &str,
    file: &str,
    contents: &str,
) -> Result<()> {
    Ok(servers::write_file(server_id, file, contents).await?)
}

#[tauri::command]
pub async fn servers_download_file(
    server_id: &str,
    url: &str,
    filename: &str,
    expected_sha1: Option<String>,
) -> Result<()> {
    Ok(servers::download_file(server_id, url, filename, expected_sha1).await?)
}

#[tauri::command]
pub async fn servers_start(
    server_id: &str,
    java_path: Option<String>,
    memory_mb: Option<u32>,
    jvm_args: Option<Vec<String>>,
) -> Result<()> {
    Ok(servers::start(server_id, java_path, memory_mb, jvm_args).await?)
}

#[tauri::command]
pub async fn servers_send_command(
    server_id: &str,
    command: &str,
) -> Result<()> {
    Ok(servers::send_command(server_id, command).await?)
}

#[tauri::command]
pub async fn servers_stop(server_id: &str) -> Result<()> {
    Ok(servers::stop(server_id).await?)
}

#[tauri::command]
pub async fn servers_kill(server_id: &str) -> Result<()> {
    Ok(servers::kill(server_id).await?)
}

#[tauri::command]
pub async fn servers_kill_port_process(port: u16) -> Result<()> {
    Ok(servers::kill_port_process(port).await?)
}

#[tauri::command]
pub async fn servers_port_process(
    port: u16,
) -> Result<Option<servers::PortProcessInfo>> {
    Ok(servers::port_process(port).await?)
}

#[tauri::command]
pub async fn servers_get_log_buffer(server_id: &str) -> Result<Vec<String>> {
    Ok(servers::get_log_buffer(server_id).await?)
}

#[tauri::command]
pub async fn servers_clear_log(server_id: &str) -> Result<()> {
    Ok(servers::clear_log(server_id).await?)
}
