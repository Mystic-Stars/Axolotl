use crate::api::Result;
use crate::api::instance::InstanceLink;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use theseus::data::ModLoader;
use theseus::install::{
    InstallJobSnapshot, InstallModpackPreview, InstallPostInstallEdit,
};
use theseus::pack::import::ImportLauncherType;
use theseus::pack::install_from::CreatePackLocation;
use uuid::Uuid;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("install")
        .invoke_handler(tauri::generate_handler![
            install_get_modpack_preview,
            install_create_instance,
            install_create_modpack_instance,
            install_import_instance,
            install_duplicate_instance,
            install_existing_instance,
            install_pack_to_existing_instance,
            install_job_list,
            install_job_get,
            install_job_retry,
            install_job_cancel,
            install_job_dismiss,
            install_job_support_details,
            download_job_list,
            download_job_get,
            download_job_retry,
            download_job_cancel,
            download_job_delete,
            download_history_clear,
            download_job_support_details,
        ])
        .build()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallCreateInstanceRequest {
    pub name: String,
    pub game_version: String,
    pub loader: ModLoader,
    pub loader_version: Option<String>,
    pub icon_path: Option<String>,
    pub link: Option<InstanceLink>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPostInstallEditRequest {
    pub name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub icon_path: Option<Option<String>>,
    pub link: Option<InstanceLink>,
}

impl InstallPostInstallEditRequest {
    fn into_core(self) -> Result<InstallPostInstallEdit> {
        Ok(InstallPostInstallEdit {
            name: self.name,
            icon_path: self.icon_path,
            link: self.link.map(|link| link.into_core()).transpose()?,
        })
    }
}

#[tauri::command]
pub async fn install_get_modpack_preview(
    location: CreatePackLocation,
) -> Result<InstallModpackPreview> {
    Ok(theseus::pack::install_from::get_instance_from_pack(location).await?)
}

#[tauri::command]
pub async fn install_create_instance(
    request: InstallCreateInstanceRequest,
) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::create_instance(
        request.name.trim().to_string(),
        request.game_version,
        request.loader,
        request.loader_version,
        request.icon_path,
        match request.link {
            Some(link) => link.into_core()?,
            None => theseus::data::InstanceLink::Unmanaged,
        },
    )
    .await?)
}

#[tauri::command]
pub async fn install_create_modpack_instance(
    location: CreatePackLocation,
    post_install_edit: Option<InstallPostInstallEditRequest>,
) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::create_modpack_instance(
        location,
        post_install_edit.map(|edit| edit.into_core()).transpose()?,
    )
    .await?)
}

#[tauri::command]
pub async fn install_import_instance(
    launcher_type: ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
    symlink: bool,
) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::import_instance(
        launcher_type,
        base_path,
        instance_folder,
        symlink,
    )
    .await?)
}

#[tauri::command]
pub async fn install_duplicate_instance(
    source_instance_id: String,
) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::duplicate_instance(source_instance_id).await?)
}

#[tauri::command]
pub async fn install_existing_instance(
    instance_id: String,
    force: bool,
) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::install_existing_instance(instance_id, force).await?)
}

#[tauri::command]
pub async fn install_pack_to_existing_instance(
    instance_id: String,
    location: CreatePackLocation,
    post_install_edit: Option<InstallPostInstallEditRequest>,
) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::install_pack_to_existing_instance(
        instance_id,
        location,
        post_install_edit.map(|edit| edit.into_core()).transpose()?,
    )
    .await?)
}

#[tauri::command]
pub async fn install_job_list(
    include_finished: bool,
) -> Result<Vec<InstallJobSnapshot>> {
    Ok(theseus::install::list_jobs(include_finished).await?)
}

#[tauri::command]
pub async fn install_job_get(job_id: Uuid) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::get_job(job_id).await?)
}

#[tauri::command]
pub async fn install_job_retry(job_id: Uuid) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::retry_job(job_id).await?)
}

#[tauri::command]
pub async fn install_job_cancel(job_id: Uuid) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::cancel_job(job_id).await?)
}

#[tauri::command]
pub async fn install_job_dismiss(job_id: Uuid) -> Result<()> {
    Ok(theseus::install::dismiss_job(job_id).await?)
}

#[tauri::command]
pub async fn install_job_support_details(job_id: Uuid) -> Result<String> {
    Ok(theseus::install::job_support_details(job_id).await?)
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJobListRequest {
    pub status: Option<String>,
    pub provider: Option<String>,
    pub query: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJobPage {
    pub jobs: Vec<InstallJobSnapshot>,
    pub next_cursor: Option<String>,
}

#[tauri::command]
pub async fn download_job_list(
    request: Option<DownloadJobListRequest>,
) -> Result<DownloadJobPage> {
    let request = request.unwrap_or_default();
    let mut jobs = theseus::install::list_jobs(true).await?;
    if let Some(status) = request.status.as_deref() {
        jobs.retain(|job| job.status.as_str() == status);
    }
    if let Some(provider) = request.provider.as_deref() {
        jobs.retain(|job| {
            format!("{:?}", job.provider)
                .to_ascii_lowercase()
                .replace('_', "")
                == provider.to_ascii_lowercase().replace('_', "")
        });
    }
    if let Some(query) = request.query.as_deref() {
        let query = query.trim().to_ascii_lowercase();
        if !query.is_empty() {
            jobs.retain(|job| {
                job.display.as_ref().is_some_and(|display| {
                    display.title.to_ascii_lowercase().contains(&query)
                }) || job.job_id.to_string().contains(&query)
            });
        }
    }
    jobs.sort_by(|a, b| b.created.cmp(&a.created));
    let offset = request
        .cursor
        .as_deref()
        .and_then(|cursor| cursor.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = request.limit.unwrap_or(100).clamp(1, 250);
    let jobs = jobs
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor =
        (jobs.len() == limit).then(|| (offset + limit).to_string());
    Ok(DownloadJobPage { jobs, next_cursor })
}

#[tauri::command]
pub async fn download_job_get(job_id: Uuid) -> Result<InstallJobSnapshot> {
    install_job_get(job_id).await
}

#[tauri::command]
pub async fn download_job_retry(job_id: Uuid) -> Result<InstallJobSnapshot> {
    Ok(theseus::install::retry_job_as_new(job_id).await?)
}

#[tauri::command]
pub async fn download_job_cancel(job_id: Uuid) -> Result<InstallJobSnapshot> {
    install_job_cancel(job_id).await
}

#[tauri::command]
pub async fn download_job_delete(job_id: Uuid) -> Result<()> {
    install_job_dismiss(job_id).await
}

#[tauri::command]
pub async fn download_history_clear() -> Result<u64> {
    Ok(theseus::install::clear_job_history().await?)
}

#[tauri::command]
pub async fn download_job_support_details(job_id: Uuid) -> Result<String> {
    install_job_support_details(job_id).await
}
