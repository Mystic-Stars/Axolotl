use std::{
    fmt,
    path::{Path, PathBuf},
};

use futures::stream::{FuturesUnordered, StreamExt};
use io::IOError;
use serde::{Deserialize, Serialize};

use crate::{
    install::{
        InstallPhaseDetails, InstallPhaseId, InstallProgress,
        InstallProgressReporter,
    },
    util::{
        fetch::{self, IoSemaphore},
        io,
    },
};

pub mod atlauncher;
pub mod curseforge;
pub mod gdlauncher;
mod generic;
pub mod hmcl;
mod instance_json;
pub mod mmc;
mod modrinth_app;
mod pcl;
mod pe_info;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportLauncherType {
    MultiMC,
    PrismLauncher,
    ATLauncher,
    GDLauncher,
    Curseforge,
    ModrinthApp,
    PCL2,
    PCL2CE,
    HMCL,
    Generic,
    #[serde(other)]
    Unknown,
}
// impl display
impl fmt::Display for ImportLauncherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportLauncherType::MultiMC => write!(f, "MultiMC"),
            ImportLauncherType::PrismLauncher => write!(f, "PrismLauncher"),
            ImportLauncherType::ATLauncher => write!(f, "ATLauncher"),
            ImportLauncherType::GDLauncher => write!(f, "GDLauncher"),
            ImportLauncherType::Curseforge => write!(f, "Curseforge"),
            ImportLauncherType::ModrinthApp => {
                write!(f, "Modrinth source installation")
            }
            ImportLauncherType::PCL2 => write!(f, "PCL2"),
            ImportLauncherType::PCL2CE => write!(f, "PCL2CE"),
            ImportLauncherType::HMCL => write!(f, "HMCL"),
            ImportLauncherType::Generic => write!(f, "Generic"),
            ImportLauncherType::Unknown => write!(f, "Unknown"),
        }
    }
}

// Return a list of importable instances from a launcher type and base path, by iterating through the folder and checking
pub async fn get_importable_instances(
    launcher_type: ImportLauncherType,
    base_path: PathBuf,
) -> crate::Result<Vec<String>> {
    if launcher_type == ImportLauncherType::ModrinthApp {
        return modrinth_app::get_importable_instances(base_path).await;
    }
    // Some launchers have a different folder structure for instances
    let instances_subfolder = match launcher_type {
        ImportLauncherType::GDLauncher | ImportLauncherType::ATLauncher => {
            "instances".to_string()
        }
        ImportLauncherType::Curseforge => "Instances".to_string(),
        ImportLauncherType::MultiMC => {
            mmc::get_instances_subpath(base_path.clone().join("multimc.cfg"))
                .await
                .unwrap_or_else(|| "instances".to_string())
        }
        ImportLauncherType::PrismLauncher => mmc::get_instances_subpath(
            base_path.clone().join("prismlauncher.cfg"),
        )
        .await
        .unwrap_or_else(|| "instances".to_string()),
        ImportLauncherType::ModrinthApp => unreachable!("handled above"),
        ImportLauncherType::PCL2 => {
            if !pe_info::folder_has_product(&base_path, "Plain Craft Launcher")
                || pcl::read_pcl_registry().is_none()
            {
                return Ok(Vec::new());
            }
            let mut names = Vec::new();
            for (name, path) in pcl::get_pcl_instances() {
                names.extend(
                    scan_instances_at(&PathBuf::from(path), Some(&name))
                        .await
                        .into_iter()
                        .map(|(n, _)| n),
                );
            }
            return Ok(names);
        }
        ImportLauncherType::PCL2CE => {
            if !pe_info::folder_has_product(&base_path, "Plain Craft Launcher")
                || !pcl::config_exists()
            {
                return Ok(Vec::new());
            }
            let mut names = Vec::new();
            for (name, path) in pcl::get_pclce_instances() {
                names.extend(
                    scan_instances_at(&PathBuf::from(path), Some(&name))
                        .await
                        .into_iter()
                        .map(|(n, _)| n),
                );
            }
            return Ok(names);
        }
        ImportLauncherType::HMCL => {
            if !hmcl::config_exists(&base_path) {
                return Ok(Vec::new());
            }
            let mut names = Vec::new();
            for (name, path) in hmcl::get_instances(&base_path) {
                names.extend(
                    scan_instances_at(&PathBuf::from(path), Some(&name))
                        .await
                        .into_iter()
                        .map(|(n, _)| n),
                );
            }
            return Ok(names);
        }
        ImportLauncherType::Generic => {
            return Ok(scan_instances_at(&base_path, None)
                .await
                .into_iter()
                .map(|(n, _)| n)
                .collect());
        }
        ImportLauncherType::Unknown => {
            let mut names: Vec<String> = Vec::new();
            let mut seen: std::collections::HashSet<PathBuf> =
                std::collections::HashSet::new();

            // PCL2
            if pe_info::folder_has_product(&base_path, "Plain Craft Launcher")
                && pcl::read_pcl_registry().is_some()
            {
                for (name, dir) in pcl::get_pcl_instances() {
                    for (iname, ipath) in
                        scan_instances_at(&PathBuf::from(dir), Some(&name))
                            .await
                    {
                        if seen.insert(ipath) {
                            names.push(iname);
                        }
                    }
                }
            }

            // PCL2CE
            if pe_info::folder_has_product(&base_path, "Plain Craft Launcher")
                && pcl::config_exists()
            {
                for (name, dir) in pcl::get_pclce_instances() {
                    for (iname, ipath) in
                        scan_instances_at(&PathBuf::from(dir), Some(&name))
                            .await
                    {
                        if seen.insert(ipath) {
                            names.push(iname);
                        }
                    }
                }
            }

            // HMCL
            if hmcl::config_exists(&base_path) {
                for (name, dir) in hmcl::get_instances(&base_path) {
                    for (iname, ipath) in
                        scan_instances_at(&PathBuf::from(dir), Some(&name))
                            .await
                    {
                        if seen.insert(ipath) {
                            names.push(iname);
                        }
                    }
                }
            }

            // ModrinthApp: uses its internal SQLite database; query real
            // physical profile paths for accurate dedup.
            {
                let pairs =
                    modrinth_app::get_importable_instances_with_paths(
                        base_path.clone(),
                    )
                    .await
                    .unwrap_or_default();
                for (iname, ipath) in pairs {
                    if seen.insert(ipath) {
                        names.push(iname);
                    }
                }
            }

            // Remaining launcher types: call get_importable_instances and
            // reconstruct physical paths for dedup.
            let other_types = [
                ImportLauncherType::MultiMC,
                ImportLauncherType::PrismLauncher,
                ImportLauncherType::ATLauncher,
                ImportLauncherType::GDLauncher,
                ImportLauncherType::Curseforge,
            ];
            for lt in other_types {
                if let Ok(instances) =
                    Box::pin(get_importable_instances(
                        lt,
                        base_path.clone(),
                    ))
                    .await
                {
                    let instances_folder = match lt {
                        ImportLauncherType::MultiMC => {
                            let subpath =
                                mmc::get_instances_subpath(
                                    base_path.clone().join("multimc.cfg"),
                                )
                                .await
                                .unwrap_or_else(|| {
                                    "instances".to_string()
                                });
                            base_path.join(&subpath)
                        }
                        ImportLauncherType::PrismLauncher => {
                            let subpath =
                                mmc::get_instances_subpath(
                                    base_path
                                        .clone()
                                        .join("prismlauncher.cfg"),
                                )
                                .await
                                .unwrap_or_else(|| {
                                    "instances".to_string()
                                });
                            base_path.join(&subpath)
                        }
                        ImportLauncherType::ATLauncher
                        | ImportLauncherType::GDLauncher => {
                            base_path.join("instances")
                        }
                        ImportLauncherType::Curseforge => {
                            base_path.join("Instances")
                        }
                        _ => unreachable!(),
                    };
                    for instance in instances {
                        let ipath =
                            instances_folder.join(&instance);
                        if seen.insert(ipath) {
                            names.push(instance);
                        }
                    }
                }
            }

            names.sort();
            return Ok(names);
        }
    };

    let instances_folder = base_path.join(&instances_subfolder);
    let mut instances = Vec::new();
    let mut dir = io::read_dir(&instances_folder).await.map_err(| _ | {
		crate::ErrorKind::InputError(format!(
			"Invalid {launcher_type} launcher path, could not find '{instances_subfolder}' subfolder."
		))
	})?;
    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, &instances_folder))?
    {
        let path = entry.path();
        if path.is_dir() {
            if is_valid_importable_instance(path.clone(), launcher_type).await {
                let name = path.file_name();
                if let Some(name) = name {
                    instances.push(name.to_string_lossy().to_string());
                }
            }
        }
    }
    Ok(instances)
}

async fn scan_instances_at(
    path: &Path,
    prefix: Option<&str>,
) -> Vec<(String, PathBuf)> {
    if !path.is_dir() {
        return Vec::new();
    }
    let mut instances = Vec::new();
    if instance_json::detect(path).is_some() {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "imported".to_string());
        instances.push((
            if let Some(pre) = prefix {
                format!("{pre}:{name}")
            } else {
                name
            },
            path.to_path_buf(),
        ));
    }
    let versions_dir = path.join("versions");
    if versions_dir.is_dir() {
        if let Ok(mut dir) = io::read_dir(&versions_dir).await {
            while let Ok(Some(entry)) = dir.next_entry().await {
                if entry.path().is_dir()
                    && instance_json::detect(&entry.path()).is_some()
                {
                    if let Some(name) = entry.path().file_name() {
                        let name = name.to_string_lossy().to_string();
                        let ipath = entry.path();
                        instances.push((
                            if let Some(pre) = prefix {
                                format!("{pre}:versions/{name}")
                            } else {
                                format!("versions/{name}")
                            },
                            ipath,
                        ));
                    }
                }
            }
        }
    }
    tracing::debug!(
        "scan_instances_at: path={} prefix={:?} found={}",
        path.display(),
        prefix,
        instances.len()
    );
    instances
}

fn resolve_instance_path(base_path: &Path, instance_folder: &str) -> PathBuf {
    if let Some(rest) = instance_folder.strip_prefix("versions/") {
        return base_path.join("versions").join(rest);
    }
    if base_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .as_deref()
        == Some(instance_folder)
    {
        base_path.to_path_buf()
    } else {
        base_path.join(instance_folder)
    }
}

fn split_config_name(name: &str) -> (&str, &str) {
    name.split_once(':').unwrap_or((name, ""))
}

pub(crate) async fn import_instance_with_reporter(
    instance_id: &str,
    launcher_type: ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
    reporter: InstallProgressReporter,
    symlink: bool,
) -> crate::Result<()> {
    import_instance_inner(
        instance_id,
        launcher_type,
        base_path,
        instance_folder,
        reporter,
        symlink,
    )
    .await
}

async fn import_instance_inner(
    instance_id: &str,
    launcher_type: ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
    reporter: InstallProgressReporter,
    symlink: bool,
) -> crate::Result<()> {
    tracing::debug!("Importing instance from {instance_folder} (symlink={symlink})");
    let details = InstallPhaseDetails::Import {
        launcher_type,
        instance_folder: instance_folder.clone(),
    };
    let res = match launcher_type {
        ImportLauncherType::MultiMC | ImportLauncherType::PrismLauncher => {
            mmc::import_mmc(
                base_path,       // path to base mmc folder
                instance_folder, // instance folder in mmc_base_path
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::ATLauncher => {
            atlauncher::import_atlauncher(
                base_path,       // path to atlauncher folder
                instance_folder, // instance folder in atlauncher
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::GDLauncher => {
            gdlauncher::import_gdlauncher(
                base_path.join("instances").join(instance_folder), // path to gdlauncher folder
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::Curseforge => {
            curseforge::import_curseforge(
                base_path.join("Instances").join(instance_folder), // path to curseforge folder
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::ModrinthApp => {
            modrinth_app::import_instance(
                base_path,
                instance_folder,
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::PCL2 => {
            let (config_name, rest) = split_config_name(&instance_folder);
            let game_dir = pcl::get_pcl_instance_path(config_name)
                .map(PathBuf::from)
                .unwrap_or_else(|| base_path.clone());
            let target = if rest.is_empty() { config_name } else { rest };
            let path = resolve_instance_path(&game_dir, target);
            generic::import_generic(
                path,
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::PCL2CE => {
            let (config_name, rest) = split_config_name(&instance_folder);
            let game_dir = pcl::get_pclce_instance_path(config_name)
                .map(PathBuf::from)
                .unwrap_or_else(|| base_path.clone());
            let target = if rest.is_empty() { config_name } else { rest };
            let path = resolve_instance_path(&game_dir, target);
            generic::import_generic(
                path,
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::HMCL => {
            let (config_name, rest) = split_config_name(&instance_folder);
            let game_dir = hmcl::get_instance_path(&base_path, config_name)
                .map(PathBuf::from)
                .unwrap_or_else(|| base_path.clone());
            let target = if rest.is_empty() { config_name } else { rest };
            let path = resolve_instance_path(&game_dir, target);
            generic::import_generic(
                path,
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::Generic => {
            let path = resolve_instance_path(&base_path, &instance_folder);
            generic::import_generic(
                path,
                instance_id,
                reporter.clone(),
                details.clone(),
                symlink,
            )
            .await
        }
        ImportLauncherType::Unknown => {
            let types = [
                ImportLauncherType::PCL2,
                ImportLauncherType::PCL2CE,
                ImportLauncherType::HMCL,
                ImportLauncherType::MultiMC,
                ImportLauncherType::PrismLauncher,
                ImportLauncherType::ATLauncher,
                ImportLauncherType::GDLauncher,
                ImportLauncherType::Curseforge,
                ImportLauncherType::ModrinthApp,
            ];
            let mut matched = false;
            for lt in types {
                if let Ok(instances) =
                    Box::pin(get_importable_instances(lt, base_path.clone()))
                        .await
                    && instances.contains(&instance_folder)
                {
                    matched = true;
                    Box::pin(import_instance_inner(
                        instance_id,
                        lt,
                        base_path,
                        instance_folder,
                        reporter.clone(),
                        symlink,
                    ))
                    .await?;
                    break;
                }
            }
            if !matched {
                return Err(crate::ErrorKind::InputError(
                    "Could not determine launcher type for the given path"
                        .to_string(),
                )
                .into());
            }
            return Ok(());
        }
    };

    // If import failed, delete the profile
    match res {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!("Import failed: {:?}", e);
            let _ = crate::api::instance::remove(instance_id).await;
            return Err(e);
        }
    }

    tracing::debug!("Completed import.");
    Ok(())
}

/// Returns the default path for the given launcher type
/// None if it can't be found or doesn't exist
pub fn get_default_launcher_path(
    r#type: ImportLauncherType,
) -> Option<PathBuf> {
    let path = match r#type {
        ImportLauncherType::MultiMC => {
            return find_multimc_path();
        }
        ImportLauncherType::PrismLauncher => {
            Some(dirs::data_dir()?.join("PrismLauncher"))
        }
        ImportLauncherType::ATLauncher => {
            Some(dirs::data_dir()?.join("ATLauncher"))
        }
        ImportLauncherType::GDLauncher => {
            Some(dirs::data_dir()?.join("gdlauncher_next"))
        }
        ImportLauncherType::Curseforge => {
            let home = dirs::home_dir()?;
            let primary = home.join("curseforge").join("minecraft");
            if primary.exists() {
                return Some(primary);
            }
            Some(dirs::document_dir()?.join("curseforge").join("minecraft"))
        }
        ImportLauncherType::ModrinthApp => {
            Some(dirs::data_dir()?.join("ModrinthApp"))
        }
        ImportLauncherType::PCL2 => {
            if pcl::read_pcl_registry().is_some() {
                dirs::data_dir()
            } else {
                None
            }
        }
        ImportLauncherType::PCL2CE => {
            if pcl::config_exists() {
                dirs::data_dir()
            } else {
                None
            }
        }
        ImportLauncherType::HMCL => None,
        ImportLauncherType::Generic => None,
        ImportLauncherType::Unknown => None,
    };
    let path = path?;
    if path.exists() { Some(path) } else { None }
}

/// Searches common locations for a MultiMC installation.
/// MultiMC stores data in its own application directory (not a standard data dir)
fn find_multimc_path() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Linux/macOS: ~/.local/share/multimc is the typical location
    if let Some(data_dir) = dirs::data_dir() {
        candidates.push(data_dir.join("multimc"));
        candidates.push(data_dir.join("MultiMC"));
    }

    // Windows: check common extraction locations
    #[cfg(target_os = "windows")]
    {
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join("MultiMC"));
            candidates.push(home.join("Desktop").join("MultiMC"));
            candidates.push(home.join("Downloads").join("MultiMC"));
        }
        candidates.push(PathBuf::from("C:\\MultiMC"));
        if let Some(program_files) =
            std::env::var_os("ProgramFiles").map(PathBuf::from)
        {
            candidates.push(program_files.join("MultiMC"));
        }
        if let Some(program_files_x86) =
            std::env::var_os("ProgramFiles(x86)").map(PathBuf::from)
        {
            candidates.push(program_files_x86.join("MultiMC"));
        }
    }

    // macOS: MultiMC is a .app bundle with data inside MultiMC.app/Data/
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/Applications/MultiMC.app/Data"));
        if let Some(home) = dirs::home_dir() {
            candidates.push(
                home.join("Applications").join("MultiMC.app").join("Data"),
            );
        }
    }

    candidates
        .into_iter()
        .find(|p| p.join("multimc.cfg").exists())
}

/// Checks if this PathBuf is a valid instance for the given launcher type

#[tracing::instrument]
pub async fn is_valid_importable_instance(
    instance_path: PathBuf,
    r#type: ImportLauncherType,
) -> bool {
    match r#type {
        ImportLauncherType::MultiMC | ImportLauncherType::PrismLauncher => {
            mmc::is_valid_mmc(instance_path).await
        }
        ImportLauncherType::ATLauncher => {
            atlauncher::is_valid_atlauncher(instance_path).await
        }
        ImportLauncherType::GDLauncher => {
            gdlauncher::is_valid_gdlauncher(instance_path).await
        }
        ImportLauncherType::Curseforge => {
            curseforge::is_valid_curseforge(instance_path).await
        }
        ImportLauncherType::ModrinthApp => instance_path.is_dir(),
        ImportLauncherType::PCL2
        | ImportLauncherType::PCL2CE
        | ImportLauncherType::HMCL
        | ImportLauncherType::Generic => instance_path.is_dir(),
        ImportLauncherType::Unknown => false,
    }
}

/// Caches an image file in the filesystem into the cache directory, and returns the path to the cached file.

#[tracing::instrument]
pub async fn recache_icon(
    icon_path: PathBuf,
) -> crate::Result<Option<PathBuf>> {
    let state = crate::State::get().await?;

    let bytes = tokio::fs::read(&icon_path).await;
    if let Ok(bytes) = bytes {
        let bytes = bytes::Bytes::from(bytes);
        let cache_dir = &state.directories.caches_dir();
        let semaphore = &state.io_semaphore;
        Ok(Some(
            fetch::write_cached_icon(
                &icon_path.to_string_lossy(),
                cache_dir,
                bytes,
                semaphore,
            )
            .await?,
        ))
    } else {
        // could not find icon (for instance, prism default icon, etc)
        Ok(None)
    }
}

pub(crate) async fn copy_dotminecraft_with_reporter(
    instance_id: &str,
    dotminecraft: PathBuf,
    io_semaphore: &IoSemaphore,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let instance_path =
        crate::api::instance::get_full_path(instance_id).await?;

    // Collect all files recursively
    let files = get_all_subfiles(&dotminecraft, false).await?;

    // Filter out launcher metadata files at the source root:
    // <dirname>.json (instance config), <dirname>.jar (custom jar override)
    let dirname = dotminecraft
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let skip_json = format!("{dirname}.json");
    let skip_jar = format!("{dirname}.jar");

    let files: Vec<PathBuf> = files
        .into_iter()
        .filter(|abs_path| {
            let rel = match abs_path.strip_prefix(&dotminecraft) {
                Ok(r) => r,
                Err(_) => return true,
            };
            // Only filter at root level
            if rel.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
                return true;
            }
            let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name != skip_json && name != skip_jar
        })
        .collect();

    let total = files.len() as u64;
    if total == 0 {
        reporter
            .update(
                InstallPhaseId::PreparingInstance,
                Some(InstallProgress {
                    current: 0,
                    total: 0,
                    secondary: None,
                }),
                details,
            )
            .await?;
        return Ok(());
    }

    // Build (src, dst) pairs, then copy concurrently bounded by IoSemaphore
    let pairs: Vec<(PathBuf, PathBuf)> = files
        .iter()
        .map(|src| {
            let dst = instance_path.join(
                src.strip_prefix(&dotminecraft)
                    .expect("prefix invariant from filter above"),
            );
            (src.clone(), dst)
        })
        .collect();

    let mut copy_tasks: FuturesUnordered<_> = pairs.into_iter().map(|(src, dst)| {
        async move {
            fetch::copy(&src, &dst, io_semaphore).await?;
            Ok::<_, crate::Error>(())
        }
    }).collect();

    let mut completed: u64 = 0;
    while let Some(result) = copy_tasks.next().await {
        result?;
        completed += 1;
        reporter
            .update(
                InstallPhaseId::PreparingInstance,
                Some(InstallProgress {
                    current: completed,
                    total,
                    secondary: None,
                }),
                details.clone(),
            )
            .await?;
    }

    // Final 100% report (ensures the bar fills even if reporter throttles the last update)
    reporter
        .update(
            InstallPhaseId::PreparingInstance,
            Some(InstallProgress {
                current: total,
                total,
                secondary: None,
            }),
            details,
        )
        .await?;

    Ok(())
}

pub(crate) async fn finish_import(
    instance_id: &str,
    dotminecraft: PathBuf,
    io_semaphore: &IoSemaphore,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
    symlink: bool,
) -> crate::Result<()> {
    if symlink {
        let instance_path = crate::api::instance::get_full_path(instance_id).await?;

        if instance_path.exists() {
            io::remove_dir_all(&instance_path).await?;
        }

        io::create_symlink(&dotminecraft, &instance_path).await?;

        crate::state::edit_instance(
            instance_id,
            crate::state::EditInstance {
                symlink_target: Some(Some(dotminecraft.to_string_lossy().to_string())),
                ..Default::default()
            },
            &crate::state::State::get().await?.pool,
        )
        .await?;
    } else {
        copy_dotminecraft_with_reporter(
            instance_id,
            dotminecraft,
            io_semaphore,
            reporter.clone(),
            details,
        )
        .await?;
    }

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        instance_id,
        false,
        Some(reporter),
    )
    .await?;

    Ok(())
}

/// Recursively get a list of all subfiles in src
/// uses async recursion

#[async_recursion::async_recursion]
#[tracing::instrument]
pub async fn get_all_subfiles(
    src: &Path,
    include_empty_dirs: bool,
) -> crate::Result<Vec<PathBuf>> {
    if !src.is_dir() {
        return Ok(vec![src.to_path_buf()]);
    }

    let mut files = Vec::new();
    let mut dir = io::read_dir(&src).await?;

    let mut has_files = false;
    while let Some(child) = dir
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, src))?
    {
        has_files = true;
        let src_child = child.path();
        files.append(
            &mut get_all_subfiles(&src_child, include_empty_dirs).await?,
        );
    }

    if !has_files && include_empty_dirs {
        files.push(src.to_path_buf());
    }

    Ok(files)
}


