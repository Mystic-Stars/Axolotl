use crate::state::instances::commands::add_project_bytes_from_provider;
use crate::state::{
    ContentProviderRef, ContentSourceKind, McArchiveFileId, McArchiveProjectId,
    McArchiveVersionId, ProjectType, State,
};
use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McArchiveContentInstallRequest {
    pub project_id: String,
    pub project_slug: String,
    pub version_id: String,
    pub file_id: String,
    pub project_type: ProjectType,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum McArchiveContentInstallResult {
    Installed {
        relative_path: String,
    },
    ManualDownload {
        file_name: String,
        page_url: Option<String>,
        expected_sha256: Option<String>,
    },
}

#[tracing::instrument(skip(request))]
pub async fn install_mcarchive_content(
    instance_id: &str,
    request: McArchiveContentInstallRequest,
) -> crate::Result<McArchiveContentInstallResult> {
    let (project, version, file) = resolve_requested_file(&request).await?;
    if file.needs_manual_download() {
        return Ok(manual_download_result(&project, &file));
    }

    let bytes = crate::api::mcarchive::download_file(&file).await?;
    let relative_path = record_verified_file(
        instance_id,
        &request,
        project,
        version,
        file,
        bytes.into(),
    )
    .await?;
    Ok(McArchiveContentInstallResult::Installed { relative_path })
}

#[tracing::instrument(skip(request, source_path))]
pub async fn import_mcarchive_content(
    instance_id: &str,
    request: McArchiveContentInstallRequest,
    source_path: PathBuf,
) -> crate::Result<McArchiveContentInstallResult> {
    let (project, version, file) = resolve_requested_file(&request).await?;
    let expected_sha256 = file
        .sha256
        .as_deref()
        .filter(|hash| !hash.trim().is_empty());
    let Some(expected_sha256) = expected_sha256 else {
        return Ok(manual_download_result(&project, &file));
    };
    let metadata = tokio::fs::metadata(&source_path).await?;
    if !metadata.is_file() {
        return Err(crate::ErrorKind::InputError(
            "The selected MCArchive import must be a regular file".to_string(),
        )
        .into());
    }
    let source_path = crate::util::io::canonicalize(&source_path)?;
    let actual_sha256 = sha256_file(&source_path).await?;
    if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        return Err(crate::ErrorKind::InputError(format!(
            "The selected file does not match MCArchive SHA-256 for {}",
            file.name
        ))
        .into());
    }
    let bytes = tokio::fs::read(&source_path).await?;
    let relative_path = record_verified_file(
        instance_id,
        &request,
        project,
        version,
        file,
        Bytes::from(bytes),
    )
    .await?;
    Ok(McArchiveContentInstallResult::Installed { relative_path })
}

async fn resolve_requested_file(
    request: &McArchiveContentInstallRequest,
) -> crate::Result<(
    crate::mcarchive::McArchiveMod,
    crate::mcarchive::McArchiveModVersion,
    crate::mcarchive::McArchiveFile,
)> {
    if request.project_slug.trim().is_empty()
        || request.project_id.trim().is_empty()
        || request.version_id.trim().is_empty()
        || request.file_id.trim().is_empty()
    {
        return Err(crate::ErrorKind::InputError(
            "MCArchive project, version, and file identifiers are required"
                .to_string(),
        )
        .into());
    }
    let project =
        crate::api::mcarchive::get_mod_by_slug(&request.project_slug).await?;
    if project.uuid != request.project_id {
        return Err(crate::ErrorKind::InputError(
            "The selected MCArchive project no longer matches its identifier"
                .to_string(),
        )
        .into());
    }
    let version = project
        .mod_versions
        .iter()
        .find(|version| version.uuid == request.version_id)
        .cloned()
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "The selected MCArchive version is no longer available"
                    .to_string(),
            )
        })?;
    let file = version
        .files
        .iter()
        .find(|file| file.uuid == request.file_id)
        .cloned()
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "The selected MCArchive file is no longer available"
                    .to_string(),
            )
        })?;
    Ok((project, version, file))
}

async fn record_verified_file(
    instance_id: &str,
    request: &McArchiveContentInstallRequest,
    project: crate::mcarchive::McArchiveMod,
    version: crate::mcarchive::McArchiveModVersion,
    file: crate::mcarchive::McArchiveFile,
    bytes: Bytes,
) -> crate::Result<String> {
    let provider_ref = ContentProviderRef::McArchive {
        project_id: McArchiveProjectId::new(project.uuid)?,
        version_id: Some(McArchiveVersionId::new(version.uuid)?),
        file_id: Some(McArchiveFileId::new(file.uuid)?),
    };
    let state = State::get().await?;
    let relative_path = add_project_bytes_from_provider(
        instance_id,
        &file.name,
        bytes,
        None,
        request.project_type,
        ContentSourceKind::McArchive,
        &provider_ref,
        &state,
    )
    .await?;
    super::emit_content_changed(instance_id).await?;
    Ok(relative_path)
}

fn manual_download_result(
    project: &crate::mcarchive::McArchiveMod,
    file: &crate::mcarchive::McArchiveFile,
) -> McArchiveContentInstallResult {
    McArchiveContentInstallResult::ManualDownload {
        file_name: file.name.clone(),
        page_url: file
            .manual_download_url()
            .map(ToString::to_string)
            .or_else(|| project.page_url.clone()),
        expected_sha256: file.sha256.clone(),
    }
}

async fn sha256_file(path: &Path) -> crate::Result<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path).map_err(|error| {
            crate::util::io::IOError::with_path(error, &path)
        })?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 262_144];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_route_preserves_the_provider_download_page() {
        let project = crate::mcarchive::McArchiveMod {
            uuid: "project".to_string(),
            slug: "project".to_string(),
            name: "Project".to_string(),
            summary: None,
            description: None,
            page_url: Some("https://mcarchive.net/mod/project".to_string()),
            mod_versions: Vec::new(),
        };
        let file = crate::mcarchive::McArchiveFile {
            uuid: "file".to_string(),
            name: "project.jar".to_string(),
            sha256: None,
            archive_url: None,
            direct_url: None,
            redirect_url: None,
            page_url: None,
        };
        assert!(matches!(
            manual_download_result(&project, &file),
            McArchiveContentInstallResult::ManualDownload {
                page_url: Some(page_url),
                ..
            } if page_url == "https://mcarchive.net/mod/project"
        ));
    }
}
