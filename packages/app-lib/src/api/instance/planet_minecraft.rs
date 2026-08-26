use crate::api::planet_minecraft::PlanetMinecraftInstallRoute;
use crate::state::instances::commands::add_project_bytes;
use crate::state::{ContentSourceKind, ProjectType, State};
use bytes::Bytes;
use std::path::PathBuf;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanetMinecraftContentInstallRequest {
    pub project_id: String,
    pub version_id: String,
    pub project_type: ProjectType,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PlanetMinecraftContentInstallResult {
    Installed {
        relative_path: String,
    },
    ManualDownload {
        page_url: String,
        file_name: Option<String>,
    },
}

pub async fn install_planet_minecraft_content(
    instance_id: &str,
    request: PlanetMinecraftContentInstallRequest,
) -> crate::Result<PlanetMinecraftContentInstallResult> {
    let project =
        crate::api::planet_minecraft::get_project(&request.project_id).await?;
    let version = project
        .versions
        .iter()
        .find(|version| version.id == request.version_id)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "The selected Planet Minecraft release is no longer available"
                    .to_string(),
            )
        })?;
    match version.download.install_route() {
        PlanetMinecraftInstallRoute::Manual {
            page_url,
            file_name,
        } => Ok(PlanetMinecraftContentInstallResult::ManualDownload {
            page_url,
            file_name,
        }),
        PlanetMinecraftInstallRoute::Automatic {
            direct_url,
            sha256,
            file_name,
        } => {
            let bytes = crate::api::planet_minecraft::download_verified_file(
                &direct_url,
                &sha256,
            )
            .await?;
            let name =
                file_name.unwrap_or_else(|| format!("{}.jar", version.name));
            let state = State::get().await?;
            let relative_path = add_project_bytes(
                instance_id,
                &name,
                Bytes::from(bytes),
                None,
                Some(request.project_type),
                ContentSourceKind::Local,
                &state,
            )
            .await?;
            super::emit_content_changed(instance_id).await?;
            Ok(
                PlanetMinecraftContentInstallResult::Installed {
                    relative_path,
                },
            )
        }
    }
}

pub async fn import_planet_minecraft_content(
    instance_id: &str,
    request: PlanetMinecraftContentInstallRequest,
    source_path: PathBuf,
) -> crate::Result<PlanetMinecraftContentInstallResult> {
    let project =
        crate::api::planet_minecraft::get_project(&request.project_id).await?;
    let version = project
        .versions
        .iter()
        .find(|version| version.id == request.version_id)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "The selected Planet Minecraft release is no longer available"
                    .to_string(),
            )
        })?;
    let PlanetMinecraftInstallRoute::Manual {
        file_name,
        page_url: _,
    } = version.download.install_route()
    else {
        return Err(crate::ErrorKind::InputError(
				"This Planet Minecraft release has a verified direct download; use automatic installation"
					.to_string(),
			)
			.into());
    };
    let metadata = tokio::fs::metadata(&source_path).await?;
    if !metadata.is_file() {
        return Err(crate::ErrorKind::InputError(
            "The selected Planet Minecraft import must be a regular file"
                .to_string(),
        )
        .into());
    }
    let bytes = tokio::fs::read(&source_path).await?;
    let state = State::get().await?;
    let relative_path = add_project_bytes(
        instance_id,
        file_name.as_deref().unwrap_or("planet-minecraft.jar"),
        Bytes::from(bytes),
        None,
        Some(request.project_type),
        ContentSourceKind::Local,
        &state,
    )
    .await?;
    super::emit_content_changed(instance_id).await?;
    Ok(PlanetMinecraftContentInstallResult::Installed { relative_path })
}
