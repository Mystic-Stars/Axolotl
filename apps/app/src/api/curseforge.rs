use crate::api::Result;
use theseus::curseforge::{
    CurseForgeCapability, CurseForgeCategory, CurseForgeFile,
    CurseForgeFilesRequest, CurseForgeFilesResponse,
    CurseForgeFingerprintResult, CurseForgeInstallRequest,
    CurseForgeInstallResult, CurseForgeModpackInstallRequest,
    CurseForgeModpackInstallResult, CurseForgeProject,
    CurseForgeRecognitionResult, CurseForgeSearchRequest,
    UnifiedSearchResponse,
};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("curseforge")
        .invoke_handler(tauri::generate_handler![
            curseforge_capability,
            curseforge_validate_credentials,
            curseforge_search_projects,
            curseforge_get_project,
            curseforge_get_projects,
            curseforge_get_description,
            curseforge_get_files,
            curseforge_get_file,
            curseforge_get_files_many,
            curseforge_get_changelog,
            curseforge_get_download_url,
            curseforge_get_categories,
            curseforge_match_fingerprints,
            curseforge_install_file,
            curseforge_update_installed_file,
            curseforge_switch_installed_file_version,
            curseforge_recognize_instance_files,
            curseforge_install_modpack,
            curseforge_update_managed_modpack,
        ])
        .build()
}

#[tauri::command]
pub fn curseforge_capability() -> CurseForgeCapability {
    theseus::curseforge::capability()
}

#[tauri::command]
pub async fn curseforge_validate_credentials() -> Result<CurseForgeCapability> {
    Ok(theseus::curseforge::validate_credentials().await?)
}

#[tauri::command]
pub async fn curseforge_search_projects(
    request: CurseForgeSearchRequest,
) -> Result<UnifiedSearchResponse> {
    Ok(theseus::curseforge::search_projects(request).await?)
}

#[tauri::command]
pub async fn curseforge_get_project(
    project_id: u32,
) -> Result<CurseForgeProject> {
    Ok(theseus::curseforge::get_project(project_id).await?)
}

#[tauri::command]
pub async fn curseforge_get_projects(
    project_ids: Vec<u32>,
) -> Result<Vec<CurseForgeProject>> {
    Ok(theseus::curseforge::get_projects(project_ids).await?)
}

#[tauri::command]
pub async fn curseforge_get_description(project_id: u32) -> Result<String> {
    Ok(theseus::curseforge::get_description(project_id).await?)
}

#[tauri::command]
pub async fn curseforge_get_files(
    project_id: u32,
    request: CurseForgeFilesRequest,
) -> Result<CurseForgeFilesResponse> {
    Ok(theseus::curseforge::get_files(project_id, request).await?)
}

#[tauri::command]
pub async fn curseforge_get_file(
    project_id: u32,
    file_id: u32,
) -> Result<CurseForgeFile> {
    Ok(theseus::curseforge::get_file(project_id, file_id).await?)
}

#[tauri::command]
pub async fn curseforge_get_files_many(
    file_ids: Vec<u32>,
) -> Result<Vec<CurseForgeFile>> {
    Ok(theseus::curseforge::get_files_many(file_ids).await?)
}

#[tauri::command]
pub async fn curseforge_get_changelog(
    project_id: u32,
    file_id: u32,
) -> Result<String> {
    Ok(theseus::curseforge::get_changelog(project_id, file_id).await?)
}

#[tauri::command]
pub async fn curseforge_get_download_url(
    project_id: u32,
    file_id: u32,
) -> Result<Option<String>> {
    Ok(theseus::curseforge::get_download_url(project_id, file_id).await?)
}

#[tauri::command]
pub async fn curseforge_get_categories(
    class_id: Option<u32>,
) -> Result<Vec<CurseForgeCategory>> {
    Ok(theseus::curseforge::get_categories(class_id).await?)
}

#[tauri::command]
pub async fn curseforge_match_fingerprints(
    fingerprints: Vec<u64>,
) -> Result<CurseForgeFingerprintResult> {
    Ok(theseus::curseforge::match_fingerprints(fingerprints).await?)
}

#[tauri::command]
pub async fn curseforge_install_file(
    request: CurseForgeInstallRequest,
) -> Result<CurseForgeInstallResult> {
    Ok(theseus::curseforge::install_file(request).await?)
}

#[tauri::command]
pub async fn curseforge_update_installed_file(
    instance_id: String,
    relative_path: String,
) -> Result<CurseForgeInstallResult> {
    Ok(
        theseus::curseforge::update_installed_file(
            &instance_id,
            &relative_path,
        )
        .await?,
    )
}

#[tauri::command]
pub async fn curseforge_switch_installed_file_version(
    instance_id: String,
    relative_path: String,
    file_id: u32,
) -> Result<CurseForgeInstallResult> {
    Ok(
        theseus::curseforge::switch_installed_file_version(
            &instance_id,
            &relative_path,
            file_id,
        )
        .await?,
    )
}

#[tauri::command]
pub async fn curseforge_recognize_instance_files(
    instance_id: String,
) -> Result<CurseForgeRecognitionResult> {
    Ok(theseus::curseforge::recognize_instance_files(&instance_id).await?)
}

#[tauri::command]
pub async fn curseforge_install_modpack(
    request: CurseForgeModpackInstallRequest,
) -> Result<CurseForgeModpackInstallResult> {
    Ok(theseus::curseforge::install_modpack(request).await?)
}

#[tauri::command]
pub async fn curseforge_update_managed_modpack(
    instance_id: String,
    file_id: u32,
) -> Result<CurseForgeModpackInstallResult> {
    Ok(
        theseus::curseforge::update_managed_modpack(&instance_id, file_id)
            .await?,
    )
}
