//! Installer for MultiMC/Prism export zips.
//!
//! Export zips carry the same `mmc-pack.json` + `instance.cfg` layout as an
//! installed MMC instance, so the archive is extracted to a scratch directory
//! and imported through the existing MMC instance importer.

use std::path::PathBuf;

use super::archive_util;
use crate::State;
use crate::install::{
    InstallPhaseDetails, InstallPhaseId, InstallProgressReporter,
};
use crate::pack::import::ImportLauncherType;

pub(crate) async fn install_mmc_zip_with_reporter(
    instance_id: String,
    archive_path: PathBuf,
    base_folder: String,
    source_filename: Option<String>,
    reporter: InstallProgressReporter,
) -> crate::Result<()> {
    let state = State::get().await?;
    let details = InstallPhaseDetails::Import {
        launcher_type: ImportLauncherType::MultiMC,
        instance_folder: source_filename.unwrap_or_else(|| {
            archive_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "MultiMC pack".to_string())
        }),
    };
    reporter
        .update(InstallPhaseId::ExtractingOverrides, None, details.clone())
        .await?;

    let scratch = archive_util::create_import_scratch_dir(&state).await?;
    let result = async {
        archive_util::extract_archive_subdir(
            archive_path,
            base_folder,
            scratch.clone(),
        )
        .await?;
        crate::pack::import::mmc::import_mmc_instance_dir(
            scratch.clone(),
            Some(scratch.clone()),
            &instance_id,
            reporter.clone(),
            details,
            false, // zip imports don't support symlinks
        )
        .await
    }
    .await;
    if let Err(error) = tokio::fs::remove_dir_all(&scratch).await {
        tracing::warn!(
            "Failed to clean up modpack import scratch directory {}: {error}",
            scratch.display()
        );
    }
    result
}
