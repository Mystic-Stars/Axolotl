use std::{collections::HashMap, path::PathBuf};

use super::instance_json;
use crate::{
    State,
    install::{InstallPhaseDetails, InstallProgressReporter},
    pack::{
        import::finish_import,
        install_from::{self, CreatePackDescription, PackDependency},
    },
};

pub async fn import_generic(
    instance_folder: PathBuf,
    instance_id: &str,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let name = instance_folder
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "imported".to_string());

    let dotminecraft = instance_folder.join(".minecraft");
    let dotminecraft = if dotminecraft.is_dir() {
        tracing::debug!(
            "import_generic: using .minecraft subdir at {}",
            dotminecraft.display()
        );
        dotminecraft
    } else {
        tracing::debug!(
            "import_generic: using folder directly at {}",
            instance_folder.display()
        );
        instance_folder
    };

    let info = instance_json::detect(&dotminecraft).ok_or_else(|| {
		crate::ErrorKind::InputError(
			"Could not detect Minecraft version. Make sure the folder contains a valid version JSON.".into(),
		)
	})?;

    let description = CreatePackDescription {
        icon: None,
        override_title: Some(name),
        project_id: None,
        version_id: None,
        instance_id: instance_id.to_string(),
        source_filename: None,
    };

    let mut dependencies =
        HashMap::from([(PackDependency::Minecraft, info.vanilla_name)]);
    if let Some(ref loader) = info.loader {
        let dep = match loader.as_str() {
            "forge" => Some(PackDependency::Forge),
            "neoforge" => Some(PackDependency::NeoForge),
            "fabric" => Some(PackDependency::FabricLoader),
            "quilt" => Some(PackDependency::QuiltLoader),
            _ => None,
        };
        if let (Some(dep), Some(version)) = (dep, info.loader_version.clone()) {
            dependencies.insert(dep, version);
        }
    }

    install_from::set_instance_information(
        instance_id.to_string(),
        &description,
        "Imported from folder",
        None,
        &dependencies,
        false,
    )
    .await?;

    let state = State::get().await?;
    finish_import(
        instance_id,
        dotminecraft,
        &state.io_semaphore,
        reporter,
        details,
    )
    .await
}
