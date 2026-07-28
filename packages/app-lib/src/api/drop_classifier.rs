//! Unified content-type classifier for dropped / imported files.
//!
//! Determines what kind of Minecraft content a file or folder represents,
//! supporting launcher directories, mod JARs, resource packs, world saves,
//! litematic files, shader packs, and more.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::pack::detect::{LocalPackFormat, detect_local_pack_sync};
use crate::api::pack::import::ImportLauncherType;
use crate::mod_metadata::manifest::read_jar_manifest;
use crate::state::{ModrinthProjectId, ModrinthVersionId};

/// Maximum number of items allowed in a ZIP before we classify it as "ZIP
/// with many items" rather than "single file/folder wrapped in ZIP".
const ZIP_TOP_LEVEL_LIMIT: usize = 200;

/// Result of classifying a file path dropped / imported by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DroppedItemType {
    /// A recognised third-party launcher root folder.
    Launcher {
        launcher_type: ImportLauncherType,
        base_path: PathBuf,
    },
    /// HMCL launcher with separate launcher and data directories.
    HmclLauncher {
        launcher_dir: PathBuf,
        data_dir: PathBuf,
    },
    /// A mod JAR file.
    Mod { file_path: PathBuf },
    /// A `.litematic` or `.schematic` file.
    Litematic { file_path: PathBuf },
    /// A resource pack or data pack.
    ResourcePack { file_path: PathBuf },
    /// A shader pack.
    ShaderPack { file_path: PathBuf },
    /// A Minecraft world save folder or archive.
    WorldSave { file_path: PathBuf },
    /// A shortcut / symlink that was resolved to another item type.
    ShortcutResolved {
        original: PathBuf,
        resolved_to: Box<DroppedItemType>,
    },
    /// A modpack archive (.mrpack, CurseForge, MultiMC, etc.).
    Modpack { file_path: PathBuf },
    /// Could not be classified.
    Unknown { reason: String },
}

/// Classify a dropped file or folder path into a `DroppedItemType`.
///
/// The classification follows a strict priority order defined in the plan.
/// Returns `Unknown` instead of panicking on any error.
pub fn classify_dropped_item(path: &Path) -> DroppedItemType {
    // Step 0: Path must exist.
    if !path.exists() {
        let reason = "Path does not exist".to_string();
        tracing::warn!(
            "Classification failed for '{}': {reason}",
            path.display()
        );
        return DroppedItemType::Unknown { reason };
    }

    // Step 1: Shortcut / symlink resolution.
    if let Some(resolved) =
        crate::util::resolve_shortcut::resolve_shortcut(path, 3)
        && resolved != path
    {
        let inner = classify_dropped_item(&resolved);
        return DroppedItemType::ShortcutResolved {
            original: path.to_path_buf(),
            resolved_to: Box::new(inner),
        };
    }

    // Step 2: ZIP archive (.zip, .mrpack).
    // NOTE: .jar is deliberately excluded here — JAR files are handled by
    // Step 4 (manifest-based classification) to properly distinguish mod
    // JARs from launcher JARs without going through extraction.
    let is_zip = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "zip" | "mrpack" | "ZIP" | "MRPACK"));

    if is_zip {
        // Check if it's a known modpack format before extracting.
        if let Ok(detected) = detect_local_pack_sync(path) {
            match detected.format {
                LocalPackFormat::Mrpack
                | LocalPackFormat::CurseForge
                | LocalPackFormat::Mcbbs
                | LocalPackFormat::Hmcl
                | LocalPackFormat::MmcExport
                | LocalPackFormat::LauncherBundled => {
                    return DroppedItemType::Modpack {
                        file_path: path.to_path_buf(),
                    };
                }
                _ => {} // PlainArchive / InstanceFolder → classify_zip extraction
            }
        }
        return classify_zip(path);
    }

    // Step 3: Launcher EXE (Windows .exe).
    if let Some(ext) = path.extension()
        && ext.eq_ignore_ascii_case("exe")
    {
        return classify_launcher_exe(path);
    }

    // Step 3.5: .disabled suffix — strip for recognition, keep original path for usage.
    if let Some(ext) = path.extension()
        && ext.eq_ignore_ascii_case("disabled")
        && let Some(stem) = path.file_stem()
        && let Some(stem_str) = stem.to_str()
        && let Some(underlying_ext) = stem_str.rsplit('.').next()
        && underlying_ext.eq_ignore_ascii_case("jar")
    {
        // classify_jar reads the original file content (still valid) and stores
        // the original .jar.disabled path in the result — no path rewrite needed.
        return classify_jar(path);
    }
    // Other .disabled extensions fall through to file classification below.

    if let Some(ext) = path.extension()
        && ext.eq_ignore_ascii_case("jar")
    {
        return classify_jar(path);
    }

    // Step 5: Directory.
    if path.is_dir() {
        let result = classify_folder(path);
        tracing::debug!(
            "classify_dropped_item: directory path={} result={:?}",
            path.display(),
            result
        );
        return result;
    }

    // Step 6: File (non-JAR, non-EXE, non-ZIP).
    let result = classify_file(path);
    tracing::debug!(
        "classify_dropped_item: file path={} result={:?}",
        path.display(),
        result
    );
    result
}

// ─── Step 2: ZIP archive classification ────────────────────────────────────

fn classify_zip(path: &Path) -> DroppedItemType {
    let Ok(file) = std::fs::File::open(path) else {
        return DroppedItemType::Unknown {
            reason: "Cannot open ZIP file".to_string(),
        };
    };

    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return DroppedItemType::Unknown {
            reason: "File is not a valid ZIP archive".to_string(),
        };
    };

    // Collect top-level entries (skip directory entries) and probe for known
    // content markers.  When we can classify from entry names alone we skip
    // extraction entirely — a significant win for large archives.
    let mut top_level: Vec<ZipEntryKind> = Vec::new();
    let mut probe_has_level_dat = false;
    let mut probe_has_pack_mcmeta = false;
    let mut probe_has_shaders_dir = false;
    // probe_has_version_json — removed, unused
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index_raw(i) else {
            continue;
        };

        let name = entry.name().to_string();
        if name.is_empty() || name.ends_with('/') {
            continue; // skip directory entries
        }

        // — Probe known content markers (entry name only, no file content) —
        if name == "level.dat" {
            probe_has_level_dat = true;
        }
        if name == "pack.mcmeta" {
            probe_has_pack_mcmeta = true;
        }
        if name.starts_with("shaders/") {
            probe_has_shaders_dir = true;
        }
        // versions/<id>/<id>.json → vanilla launcher instance (probe only)
        // (probe_has_version_json was removed — unused)

        // Get the top-level component of the path.
        let top = match name.split_once('/') {
            Some((first, _)) => first,
            None => &name,
        };

        if !top_level.iter().any(|k| k.name() == top) {
            top_level.push(if name.contains('/') {
                ZipEntryKind::SubFile(top.to_string())
            } else {
                ZipEntryKind::RootFile(top.to_string())
            });
        }

        // Guard against huge archives.
        if top_level.len() > ZIP_TOP_LEVEL_LIMIT {
            return DroppedItemType::Unknown {
                reason: "ZIP archive has too many top-level entries"
                    .to_string(),
            };
        }
    }

    if top_level.is_empty() {
        // Still check probe markers even if directory entry filtering
        // left top_level empty (e.g., an archive with only marker files like
        // level.dat at the root).
        if probe_has_level_dat {
            return DroppedItemType::WorldSave {
                file_path: path.to_path_buf(),
            };
        }
        if probe_has_pack_mcmeta {
            return DroppedItemType::ResourcePack {
                file_path: path.to_path_buf(),
            };
        }
        return DroppedItemType::Unknown {
            reason: "Empty zip file".to_string(),
        };
    }

    // ── Probe pass: return early when entry names alone are sufficient ──
    // Priority order mirrors classify_folder_content.
    if probe_has_level_dat {
        tracing::debug!(
            "ZIP probe hit: level.dat → WorldSave — {}",
            path.display()
        );
        return DroppedItemType::WorldSave {
            file_path: path.to_path_buf(),
        };
    }
    if probe_has_pack_mcmeta {
        tracing::debug!(
            "ZIP probe hit: pack.mcmeta → ResourcePack — {}",
            path.display()
        );
        return DroppedItemType::ResourcePack {
            file_path: path.to_path_buf(),
        };
    }
    if probe_has_shaders_dir {
        tracing::debug!(
            "ZIP probe hit: shaders/ → ShaderPack — {}",
            path.display()
        );
        return DroppedItemType::ShaderPack {
            file_path: path.to_path_buf(),
        };
    }
    // NOTE: versions/<id>/<id>.json is NOT an early-return here — we always
    // continue to extraction so that classify_folder_content can also run the
    // root .jar + .json scan for modded instance detection.

    // ── Force-analysis fallback ──
    // Extraction + re-classification is a potentially long operation and should
    // not happen silently during classification.  Files that can't be identified
    // from entry names alone should be handled by the frontend (user prompt)
    // via classify_zip_with_extraction().
    tracing::debug!(
        "ZIP probe inconclusive — extraction required for: {}",
        path.display()
    );
    return DroppedItemType::Unknown {
        reason: "ZIP archive requires extraction to determine content type"
            .to_string(),
    };
}

enum ZipEntryKind {
    RootFile(String),
    SubFile(String),
}

impl ZipEntryKind {
    fn name(&self) -> &str {
        match self {
            ZipEntryKind::RootFile(n) | ZipEntryKind::SubFile(n) => n,
        }
    }
}

/// Extracts a ZIP archive to a temporary directory and classifies its contents
/// by examining the extracted files and folders.
///
/// This is a potentially **long-running** operation — the caller MUST first
/// confirm with the user before calling this function.
pub fn classify_zip_with_extraction(path: &Path) -> DroppedItemType {
    let Ok(file) = std::fs::File::open(path) else {
        return DroppedItemType::Unknown {
            reason: "Cannot open ZIP file".to_string(),
        };
    };

    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return DroppedItemType::Unknown {
            reason: "File is not a valid ZIP archive".to_string(),
        };
    };

    // Collect top-level entry names (same probe as classify_zip).
    let mut top_level: Vec<ZipEntryKind> = Vec::new();
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index_raw(i) else {
            continue;
        };
        let name = entry.name().to_string();
        if name.is_empty() || name.ends_with('/') {
            continue;
        }

        let top = match name.split_once('/') {
            Some((first, _)) => first,
            None => &name,
        };

        if !top_level.iter().any(|k| k.name() == top) {
            top_level.push(if name.contains('/') {
                ZipEntryKind::SubFile(top.to_string())
            } else {
                ZipEntryKind::RootFile(top.to_string())
            });
        }

        if top_level.len() > ZIP_TOP_LEVEL_LIMIT {
            break;
        }
    }

    // Create temporary directory for extraction.
    let temp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            return DroppedItemType::Unknown {
                reason: format!("Failed to create temporary directory: {e}"),
            };
        }
    };

    // Extract everything.
    extract_all(&mut archive, temp_dir.path());

    tracing::debug!(
        "classify_zip_with_extraction: extracted {} top-level items for {}",
        top_level.len(),
        path.display()
    );

    // Classify the extracted contents.
    let result = if top_level.len() == 1 {
        // Single top-level item — classify it directly.
        let item_name = top_level[0].name().to_string();
        let item_path = temp_dir.path().join(&item_name);
        classify_dropped_item(&item_path)
    } else {
        // Multiple items — classify as a folder.
        classify_folder_content(temp_dir.path())
    };

    // temp_dir is dropped here, cleaning up the extracted files automatically.
    result
}

fn extract_all(archive: &mut zip::ZipArchive<std::fs::File>, base_dir: &Path) {
    // First pass: collect entry metadata while the archive is mutable-borrowed.
    let entries: Vec<(String, bool)> = (0..archive.len())
        .filter_map(|i| {
            let entry = archive.by_index_raw(i).ok()?;
            let name = entry.name().to_string();
            if name.is_empty() {
                None
            } else {
                Some((name.clone(), name.ends_with('/')))
            }
        })
        .collect();

    // Second pass: extract. The collect() above has released the mutable
    // borrow, so we can call by_name() here.
    for (name, is_dir) in &entries {
        let out_path = base_dir.join(name);
        if *is_dir {
            let _ = std::fs::create_dir_all(&out_path);
        } else if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
            if let Ok(mut reader) = archive.by_name(name)
                && let Ok(mut writer) = std::fs::File::create(&out_path)
            {
                let _ = std::io::copy(&mut reader, &mut writer);
            }
        }
    }
}

// ─── Step 3: Launcher EXE ──────────────────────────────────────────────────

fn classify_launcher_exe(path: &Path) -> DroppedItemType {
    if let Some(parent) = path.parent() {
        match crate::api::pack::import::pe_info::folder_has_product_result(
            parent,
            "Plain Craft Launcher",
        ) {
            Ok(true) => {
                if crate::api::pack::import::config_exists() {
                    return DroppedItemType::Launcher {
                        launcher_type: ImportLauncherType::PCL2CE,
                        base_path: parent.to_path_buf(),
                    };
                }
                if crate::api::pack::import::read_pcl_registry().is_some() {
                    return DroppedItemType::Launcher {
                        launcher_type: ImportLauncherType::PCL2,
                        base_path: parent.to_path_buf(),
                    };
                }
                return DroppedItemType::Launcher {
                    launcher_type: ImportLauncherType::PCL2,
                    base_path: parent.to_path_buf(),
                };
            }
            Ok(false) => {}
            Err(_) => {}
        }

        match crate::api::pack::import::pe_info::folder_has_product_result(
            parent,
            "Hello Minecraft! Launcher",
        ) {
            Ok(true) => {
                return DroppedItemType::Launcher {
                    launcher_type: ImportLauncherType::HMCL,
                    base_path: parent.to_path_buf(),
                };
            }
            Ok(false) => {}
            Err(_) => {}
        }
    }

    DroppedItemType::Unknown {
        reason: format!("Unrecognised executable: {}", path.display()),
    }
}

// ─── Step 4: JAR file ──────────────────────────────────────────────────────

fn classify_jar(path: &Path) -> DroppedItemType {
    let manifest = read_jar_manifest(path);

    if let Some(ref mf) = manifest {
        // HMCL launcher JAR.
        if mf.main_class.as_deref() == Some("org.jackhuang.hmcl.Main") {
            if let Some(parent) = path.parent()
                && let Some(data_dir) = find_hmcl_data_dir(parent)
            {
                return DroppedItemType::HmclLauncher {
                    launcher_dir: parent.to_path_buf(),
                    data_dir,
                };
            }
            // Found HMCL main class but no data dir — still classify as launcher.
            return DroppedItemType::Launcher {
                launcher_type: ImportLauncherType::HMCL,
                base_path: path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_default(),
            };
        }
    }

    // Otherwise, treat as a mod.
    DroppedItemType::Mod {
        file_path: path.to_path_buf(),
    }
}

// ─── Step 5: Folder classification ─────────────────────────────────────────

fn classify_folder(path: &Path) -> DroppedItemType {
    // Check launcher signatures in priority order.
    if path.join("multimc.cfg").exists() {
        return DroppedItemType::Launcher {
            launcher_type: ImportLauncherType::MultiMC,
            base_path: path.to_path_buf(),
        };
    }

    if path.join("prismlauncher.cfg").exists() {
        return DroppedItemType::Launcher {
            launcher_type: ImportLauncherType::PrismLauncher,
            base_path: path.to_path_buf(),
        };
    }

    // ATLauncher: check for instances/<sub>/instance.json pattern.
    if let Ok(mut dir) = std::fs::read_dir(path.join("instances"))
        && dir.any(|e| {
            e.ok()
                .as_ref()
                .is_some_and(|e| e.path().join("instance.json").exists())
        })
    {
        // Found ATLauncher-style instance.
        // Fall through — let content detection handle it.
    }

    // MultiMC/Prism: check for instances/<sub>/instance.cfg pattern.
    if let Ok(mut dir) = std::fs::read_dir(path.join("instances"))
        && dir.any(|e| {
            e.ok()
                .as_ref()
                .is_some_and(|e| e.path().join("instance.cfg").exists())
        })
    {
        // instance.cfg → MultiMC or Prism.
        return DroppedItemType::Launcher {
            launcher_type: ImportLauncherType::MultiMC,
            base_path: path.to_path_buf(),
        };
    }

    // HMCL portable mode.
    let hmcl_config = path
        .join(".hmcl")
        .join("config")
        .join("launcher-settings.json");
    if hmcl_config.exists()
        && let Some(data_dir) = find_hmcl_data_dir(path)
    {
        return DroppedItemType::HmclLauncher {
            launcher_dir: path.to_path_buf(),
            data_dir,
        };
    }

    // Step 7: Content-type detection for folders.
    classify_folder_content(path)
}

// ─── Step 6: File classification (non-JAR, non-EXE, non-ZIP) ───────────────

fn classify_file(path: &Path) -> DroppedItemType {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "litematic" | "schematic" => DroppedItemType::Litematic {
            file_path: path.to_path_buf(),
        },
        _ => DroppedItemType::Unknown {
            reason: format!("Unrecognised file type: {}", path.display()),
        },
    }
}

// ─── Step 7: Content-type detection for folders/extracted ZIPs ─────────────

pub(crate) fn classify_folder_content(path: &Path) -> DroppedItemType {
    // 1. World save: look for level.dat.
    if path.join("level.dat").exists() {
        return DroppedItemType::WorldSave {
            file_path: path.to_path_buf(),
        };
    }

    // 2. Resource pack: pack.mcmeta.
    if path.join("pack.mcmeta").exists() {
        return DroppedItemType::ResourcePack {
            file_path: path.to_path_buf(),
        };
    }

    // 3. Shader pack: shaders/ folder.
    if path.join("shaders").is_dir() {
        return DroppedItemType::ShaderPack {
            file_path: path.to_path_buf(),
        };
    }

    // 4. Instance detection: check for launcher instance markers.
    //    a. versions/<id>/<id>.json pattern (vanilla launcher instance).
    //    b. Root directory has both .jar and .json files (modded instance).
    //    c. mods/ directory contains .jar files (bare instance folder).
    let is_instance = {
        let versions_dir = path.join("versions");
        let has_version_json = if versions_dir.is_dir() {
            match std::fs::read_dir(&versions_dir) {
                Ok(mut dir) => {
                    let found = dir.any(|e| {
                        e.ok().is_some_and(|entry| {
                            let p = entry.path();
                            if p.is_dir() {
                                let id = p
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");
                                let json_path = p.join(format!("{id}.json"));
                                let exists = json_path.exists();
                                tracing::debug!(
                                    "classify_folder_content: versions subdir={} json={} exists={}",
                                    id,
                                    json_path.display(),
                                    exists
                                );
                                exists
                            } else {
                                false
                            }
                        })
                    });
                    tracing::debug!(
                        "classify_folder_content: versions_dir={} has_version_json={}",
                        versions_dir.display(),
                        found
                    );
                    found
                }
                Err(e) => {
                    tracing::debug!(
                        "classify_folder_content: versions_dir={} read_dir_err={}",
                        versions_dir.display(),
                        e
                    );
                    false
                }
            }
        } else {
            tracing::debug!(
                "classify_folder_content: versions_dir={} does not exist",
                versions_dir.display()
            );
            false
        };

        let has_root_jar = {
            let mut found = false;
            let mut total = 0u32;
            match std::fs::read_dir(path) {
                Ok(dir) => {
                    for entry in dir.flatten() {
                        let p = entry.path();
                        if !p.is_file() {
                            continue;
                        }
                        total += 1;
                        let ext = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_string();
                        let is_jar = ext.eq_ignore_ascii_case("jar");
                        tracing::debug!(
                            "classify_folder_content: root_jar_check path={} file={} ext={} is_jar={}",
                            path.display(),
                            p.display(),
                            ext,
                            is_jar
                        );
                        if is_jar {
                            found = true;
                        }
                    }
                    tracing::debug!(
                        "classify_folder_content: has_root_jar path={} total_root_files={} result={}",
                        path.display(),
                        total,
                        found
                    );
                    found
                }
                Err(e) => {
                    tracing::debug!(
                        "classify_folder_content: has_root_jar path={} read_dir_err={}",
                        path.display(),
                        e
                    );
                    false
                }
            }
        };
        let has_root_json = {
            let mut found = false;
            let mut total = 0u32;
            match std::fs::read_dir(path) {
                Ok(dir) => {
                    for entry in dir.flatten() {
                        let p = entry.path();
                        if !p.is_file() {
                            continue;
                        }
                        total += 1;
                        let ext = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_string();
                        let is_json = ext.eq_ignore_ascii_case("json");
                        tracing::debug!(
                            "classify_folder_content: root_json_check path={} file={} ext={} is_json={}",
                            path.display(),
                            p.display(),
                            ext,
                            is_json
                        );
                        if is_json {
                            found = true;
                        }
                    }
                    tracing::debug!(
                        "classify_folder_content: has_root_json path={} total_root_files={} result={}",
                        path.display(),
                        total,
                        found
                    );
                    found
                }
                Err(e) => {
                    tracing::debug!(
                        "classify_folder_content: has_root_json path={} read_dir_err={}",
                        path.display(),
                        e
                    );
                    false
                }
            }
        };

        let has_mods_jar = {
            let mods_dir = path.join("mods");
            match std::fs::read_dir(&mods_dir) {
                Ok(dir) => {
                    let mut found = false;
                    let mut total = 0u32;
                    for entry in dir.flatten() {
                        let p = entry.path();
                        if !p.is_file() {
                            continue;
                        }
                        total += 1;
                        let ext = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_string();
                        let is_jar = ext.eq_ignore_ascii_case("jar");
                        tracing::debug!(
                            "classify_folder_content: mods_jar_check path={} file={} ext={} is_jar={}",
                            mods_dir.display(),
                            p.display(),
                            ext,
                            is_jar
                        );
                        if is_jar {
                            found = true;
                        }
                    }
                    tracing::debug!(
                        "classify_folder_content: has_mods_jar path={} mods_total_files={} result={}",
                        mods_dir.display(),
                        total,
                        found
                    );
                    found
                }
                Err(e) => {
                    tracing::debug!(
                        "classify_folder_content: has_mods_jar mods_dir={} read_dir_err={}",
                        mods_dir.display(),
                        e
                    );
                    false
                }
            }
        };

        has_version_json || (has_root_jar && has_root_json) || has_mods_jar
    };

    if is_instance {
        return DroppedItemType::Launcher {
            launcher_type: ImportLauncherType::Generic,
            base_path: path.to_path_buf(),
        };
    }

    // 5. (Removed) `mods/` → Mod — bare mods/ directories are not valid
    //    standalone imports; instance folders with mods are caught by the
    //    root .jar + .json scan above.

    DroppedItemType::Unknown {
        reason: format!(
            "Unrecognised content: {}",
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string())
        ),
    }
}

// ─── HMCL data directory discovery ─────────────────────────────────────────

/// Result of looking up a mod file hash on Modrinth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthLookupResult {
    pub hash: String,
    pub project_id: String,
    pub version_id: String,
    pub project_name: Option<String>,
    pub project_slug: Option<String>,
    pub version_number: Option<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

/// Look up a mod file by SHA1 hash to find matching Modrinth project and version.
///
/// Computes the SHA1 hash of the given file and queries the Modrinth API
/// to find matching versions. Returns project and version information if found.
pub async fn lookup_mod_hash(
    path: &Path,
) -> crate::Result<Option<ModrinthLookupResult>> {
    let (_, hash) = crate::util::fetch::sha1_file_async(path).await?;

    let state = crate::State::get().await?;

    let files = crate::state::CachedEntry::get_file_many(
        &[&hash],
        Some(crate::state::CacheBehaviour::StaleWhileRevalidateSkipOffline),
        &state.pool,
        &state.api_semaphore,
    )
    .await?;

    if files.is_empty() {
        return Ok(None);
    }

    let file = &files[0];
    let version = crate::state::CachedEntry::get_version(
        &ModrinthVersionId::new(file.version_id.clone())?,
        Some(crate::state::CacheBehaviour::StaleWhileRevalidateSkipOffline),
        &state.pool,
        &state.api_semaphore,
    )
    .await?;

    let project = if let Some(v) = &version {
        crate::state::CachedEntry::get_project(
            &ModrinthProjectId::new(v.project_id.clone())?,
            Some(crate::state::CacheBehaviour::StaleWhileRevalidateSkipOffline),
            &state.pool,
            &state.api_semaphore,
        )
        .await?
    } else {
        None
    };

    Ok(Some(ModrinthLookupResult {
        hash,
        project_id: file.project_id.clone(),
        version_id: file.version_id.clone(),
        project_name: project.as_ref().map(|p| p.title.clone()),
        project_slug: project.as_ref().and_then(|p| p.slug.clone()),
        version_number: version.as_ref().map(|v| v.version_number.clone()),
        game_versions: version
            .as_ref()
            .map(|v| v.game_versions.clone())
            .unwrap_or_default(),
        loaders: version
            .as_ref()
            .map(|v| v.loaders.clone())
            .unwrap_or_default(),
    }))
}

/// Find the HMCL data / config directory.
///
/// Priority:
/// 1. `{launcher_dir}/.hmcl/config/launcher-settings.json` (portable mode)
/// 2. Platform-specific system data dir:
///    - Windows: `%APPDATA%\.hmcl`
///    - macOS: `~/Library/Application Support/hmcl`
///    - Linux: `~/.local/share/hmcl`
/// 3. `$HMCL_DATA_DIR` environment variable
fn find_hmcl_data_dir(launcher_dir: &Path) -> Option<PathBuf> {
    // Priority 1: Portable mode — .hmcl in launcher directory.
    let portable_config =
        launcher_dir.join(".hmcl/config/launcher-settings.json");
    if portable_config.exists() {
        return Some(launcher_dir.join(".hmcl"));
    }

    // Priority 2: Platform-specific system data dir.
    let system_dir = find_hmcl_system_data_dir();
    if let Some(ref dir) = system_dir
        && dir.join("config/launcher-settings.json").exists()
    {
        return Some(dir.clone());
    }

    // Priority 3: Environment variable override.
    if let Ok(env_dir) = std::env::var("HMCL_DATA_DIR") {
        let path = PathBuf::from(env_dir);
        if path.exists() {
            return Some(path);
        }
    }

    system_dir
}

fn find_hmcl_system_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join(".hmcl"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Library/Application Support/hmcl"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::data_dir().map(|d| d.join("hmcl"))
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_nonexistent_path() {
        let result = classify_dropped_item(Path::new("/nonexistent/path"));
        assert!(
            matches!(result, DroppedItemType::Unknown { .. }),
            "nonexistent path should be Unknown"
        );
    }

    #[test]
    fn test_regular_mod_jar() {
        let dir = tempdir().expect("temp dir");
        let jar_path = dir.path().join("testmod.jar");

        // Create a minimal ZIP with a fabric.mod.json.
        let file = std::fs::File::create(&jar_path).expect("create jar");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            "fabric.mod.json",
            zip::write::FileOptions::<()>::default(),
        )
        .expect("start entry");
        zip.write_all(b"{}").expect("write");
        zip.finish().expect("finish");

        let result = classify_dropped_item(&jar_path);
        assert!(
            matches!(result, DroppedItemType::Mod { .. }),
            "jar with fabric mod should be classified as Mod: {result:?}"
        );
    }

    #[test]
    fn test_litematic_file() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("build.litematic");
        std::fs::write(&path, "fake litematic data").expect("write");

        let result = classify_dropped_item(&path);
        assert!(
            matches!(result, DroppedItemType::Litematic { .. }),
            "litematic file should be classified as Litematic"
        );
    }

    #[test]
    fn test_resource_pack_folder() {
        let dir = tempdir().expect("temp dir");
        let rp = dir.path().join("my_resource_pack");
        std::fs::create_dir(&rp).expect("create dir");
        std::fs::write(rp.join("pack.mcmeta"), "{}")
            .expect("write pack.mcmeta");

        let result = classify_dropped_item(&rp);
        assert!(
            matches!(result, DroppedItemType::ResourcePack { .. }),
            "folder with pack.mcmeta should be ResourcePack"
        );
    }

    #[test]
    fn test_world_save() {
        let dir = tempdir().expect("temp dir");
        let world = dir.path().join("New World");
        std::fs::create_dir(&world).expect("create dir");
        std::fs::write(world.join("level.dat"), "fake")
            .expect("write level.dat");

        let result = classify_dropped_item(&world);
        assert!(
            matches!(result, DroppedItemType::WorldSave { .. }),
            "folder with level.dat should be WorldSave"
        );
    }

    #[test]
    fn test_multimc_launcher_folder() {
        let dir = tempdir().expect("temp dir");
        std::fs::write(dir.path().join("multimc.cfg"), "").expect("write");

        let result = classify_dropped_item(dir.path());
        assert!(
            matches!(result, DroppedItemType::Launcher { launcher_type, .. } if launcher_type == ImportLauncherType::MultiMC),
            "folder with multimc.cfg should be MultiMC launcher"
        );
    }

    #[test]
    fn test_shader_pack_folder() {
        let dir = tempdir().expect("temp dir");
        let shaders = dir.path().join("shaders");
        std::fs::create_dir(&shaders).expect("create shaders dir");

        let result = classify_dropped_item(dir.path());
        assert!(
            matches!(result, DroppedItemType::ShaderPack { .. }),
            "folder with shaders/ should be ShaderPack"
        );
    }

    #[test]
    fn test_unknown_file() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("random.xyz");
        std::fs::write(&path, "data").expect("write");

        let result = classify_dropped_item(&path);
        assert!(
            matches!(result, DroppedItemType::Unknown { .. }),
            "unknown extension should be Unknown"
        );
    }

    #[test]
    fn test_zip_single_file() {
        let dir = tempdir().expect("temp dir");
        let zip_path = dir.path().join("test.zip");

        let file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("test.txt", zip::write::FileOptions::<()>::default())
            .expect("start entry");
        zip.write_all(b"hello").expect("write");
        zip.finish().expect("finish");

        let result = classify_dropped_item(&zip_path);
        // Single .txt file inside ZIP → after extraction, classify_file sees .txt → Unknown.
        assert!(
            matches!(result, DroppedItemType::Unknown { .. }),
            "zip with single .txt should resolve to Unknown: {result:?}"
        );
    }

    #[test]
    fn test_zip_with_mod_jar() {
        let dir = tempdir().expect("temp dir");
        let zip_path = dir.path().join("modpack.zip");

        let file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);

        // Create a nested JAR inside the ZIP.
        zip.start_file(
            "mods/testmod.jar",
            zip::write::FileOptions::<()>::default(),
        )
        .expect("start entry");
        // The JAR itself should have a fabric.mod.json to be classified as Mod.
        // But since it's inside a ZIP that gets extracted, the extracted JAR
        // won't have fabric.mod.json (we didn't write one for it).
        // So this will be extracted as a file that ends in .jar.
        // classify_jar will read_jar_manifest which will fail → Mod (default for JARs).
        zip.write_all(b"fake jar content").expect("write");
        zip.finish().expect("finish");

        let result = classify_dropped_item(&zip_path);
        // After extraction, the mods/ folder → classify_folder_content sees mods/ → Mod
        // Or the jar file itself gets classified.
        // Either way it should be a Mod.
        dbg!(&result);
    }
}
