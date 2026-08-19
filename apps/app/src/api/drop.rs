use serde::{Deserialize, Serialize};
use tauri::Emitter;
use theseus::drop_classifier::{
    DroppedCandidate, DroppedItemType, ModrinthLookupResult,
    classify_dropped_item_with_candidates, classify_zip_with_extraction,
    lookup_mod_hash,
};
use theseus::pack::import::{ImportLauncherType, get_importable_instances};
use theseus::{LockingProcess, get_locking_processes};
use tracing::{debug, info, warn};

/// A scanned importable instance: name plus the resolved filesystem path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedInstance {
    pub name: String,
    pub path: String,
}

/// One candidate inside a multi-candidate classification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateResult {
    pub item_type: String,
    pub file_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_base: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
}

/// Serializable classification result mapped from `DroppedItemType`.
///
/// All `PathBuf` fields are converted to `String` via `to_string_lossy()`.
/// The JSON representation uses an `item_type` tag (via `#[serde(tag = "item_type")]`)
/// so the frontend can discriminate variants with a string switch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "item_type")]
pub enum ClassificationResult {
    #[serde(rename = "launcher")]
    Launcher {
        launcher_type: String,
        base_path: String,
        #[serde(
            rename = "innerBase",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        inner_base: Option<String>,
    },
    #[serde(rename = "hmcl_launcher")]
    HmclLauncher {
        launcher_dir: String,
        data_dir: String,
    },
    #[serde(rename = "mod")]
    Mod { file_path: String },
    #[serde(rename = "litematic")]
    Litematic { file_path: String },
    #[serde(rename = "resource_pack")]
    ResourcePack {
        file_path: String,
        candidates: Vec<String>,
        #[serde(
            rename = "innerBase",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        inner_base: Option<String>,
    },
    #[serde(rename = "shader_pack")]
    ShaderPack {
        file_path: String,
        #[serde(
            rename = "innerBase",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        inner_base: Option<String>,
    },
    #[serde(rename = "world_save")]
    WorldSave {
        file_path: String,
        #[serde(
            rename = "innerBase",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        inner_base: Option<String>,
    },
    #[serde(rename = "modpack")]
    Modpack { file_path: String },
    #[serde(rename = "multiple")]
    Multiple {
        file_path: String,
        candidates: Vec<CandidateResult>,
    },
    #[serde(rename = "shortcut_resolved")]
    ShortcutResolved {
        original: String,
        resolved_to: Box<ClassificationResult>,
    },
    #[serde(rename = "unknown")]
    Unknown { reason: String },
}

impl From<DroppedCandidate> for CandidateResult {
    fn from(candidate: DroppedCandidate) -> Self {
        CandidateResult {
            item_type: candidate.item_type,
            file_path: candidate.file_path.to_string_lossy().to_string(),
            inner_base: candidate.inner_base,
            candidates: candidate.candidates,
        }
    }
}

impl From<DroppedItemType> for ClassificationResult {
    fn from(item: DroppedItemType) -> Self {
        match item {
            DroppedItemType::Launcher {
                launcher_type,
                base_path,
                inner_base,
            } => ClassificationResult::Launcher {
                launcher_type: launcher_type.to_string(),
                base_path: base_path.to_string_lossy().to_string(),
                inner_base,
            },
            DroppedItemType::HmclLauncher {
                launcher_dir,
                data_dir,
            } => ClassificationResult::HmclLauncher {
                launcher_dir: launcher_dir.to_string_lossy().to_string(),
                data_dir: data_dir.to_string_lossy().to_string(),
            },
            DroppedItemType::Mod { file_path } => ClassificationResult::Mod {
                file_path: file_path.to_string_lossy().to_string(),
            },
            DroppedItemType::Litematic { file_path } => {
                ClassificationResult::Litematic {
                    file_path: file_path.to_string_lossy().to_string(),
                }
            }
            DroppedItemType::ResourcePack {
                file_path,
                candidates,
                inner_base,
            } => ClassificationResult::ResourcePack {
                file_path: file_path.to_string_lossy().to_string(),
                candidates,
                inner_base,
            },
            DroppedItemType::ShaderPack {
                file_path,
                inner_base,
            } => ClassificationResult::ShaderPack {
                file_path: file_path.to_string_lossy().to_string(),
                inner_base,
            },
            DroppedItemType::WorldSave {
                file_path,
                inner_base,
            } => ClassificationResult::WorldSave {
                file_path: file_path.to_string_lossy().to_string(),
                inner_base,
            },
            DroppedItemType::ShortcutResolved {
                original,
                resolved_to,
            } => ClassificationResult::ShortcutResolved {
                original: original.to_string_lossy().to_string(),
                resolved_to: Box::new(ClassificationResult::from(*resolved_to)),
            },
            DroppedItemType::Modpack { file_path } => {
                ClassificationResult::Modpack {
                    file_path: file_path.to_string_lossy().to_string(),
                }
            }
            DroppedItemType::Multiple {
                file_path,
                candidates,
            } => ClassificationResult::Multiple {
                file_path: file_path.to_string_lossy().to_string(),
                candidates: candidates.into_iter().map(Into::into).collect(),
            },
            DroppedItemType::Unknown { reason } => {
                ClassificationResult::Unknown { reason }
            }
        }
    }
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("drop")
        .invoke_handler(tauri::generate_handler![
            drop_classify,
            drop_classify_extract,
            drop_extract_zip_to_temp,
            drop_scan_launcher_instances,
            drop_remove_temp_dir,
            drop_detect_file_lock,
            drop_extract_mod_metadata,
            drop_lookup_mod_hash,
        ])
        .build()
}

/// Classify a dropped file or folder path.
///
/// Returns a `ClassificationResult` with an `item_type` tag that the frontend
/// can use to decide what UI to show (confirm dialog, error, etc.).
#[tauri::command]
pub async fn drop_classify<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
    allow_nested_extraction: Option<bool>,
) -> Result<ClassificationResult, String> {
    debug!("Drop event received: {}", path);
    let path = std::path::PathBuf::from(&path);
    let path_label = path.to_string_lossy().to_string();
    let _ = app.emit(
        "drop_classify_progress",
        serde_json::json!({
            "phase": "classify",
            "currentItem": path_label,
            "processed": 0,
            "total": null,
        }),
    );
    // The first pass never unpacks nested archives; when one would be needed
    // the classification reports the total nested size so the frontend can
    // confirm the potentially slow unpack with the user before retrying.
    // Batch drops classify several files concurrently, so the classifier must
    // run on a blocking thread instead of occupying the async runtime.
    let result = tokio::task::spawn_blocking(move || {
        if allow_nested_extraction.unwrap_or(false) {
            classify_dropped_item_with_candidates(&path, true)
        } else {
            classify_dropped_item_with_candidates(&path, false)
        }
    })
    .await
    .map_err(|e| format!("Classification task panicked: {e}"))?;
    let _ = app.emit(
        "drop_classify_progress",
        serde_json::json!({
            "phase": "done",
            "currentItem": path_label,
            "processed": 1,
            "total": 1,
        }),
    );
    let classification = ClassificationResult::from(result);
    info!("Classification result: {:?}", classification);
    Ok(classification)
}

/// Classify ZIP

#[tauri::command]
pub async fn drop_classify_extract<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<ClassificationResult, String> {
    debug!("Drop classify with extraction: {}", path);
    let path = std::path::PathBuf::from(&path);
    let path_label = path.to_string_lossy().to_string();
    let _ = app.emit(
        "drop_classify_progress",
        serde_json::json!({
            "phase": "extract",
            "currentItem": &path_label,
            "processed": 0,
            "total": null,
        }),
    );
    let result = tokio::task::spawn_blocking(move || {
        classify_zip_with_extraction(&path)
    })
    .await
    .map_err(|e| format!("Extraction task panicked: {e}"))?;
    let _ = app.emit(
        "drop_classify_progress",
        serde_json::json!({
            "phase": "done",
            "currentItem": &path_label,
            "processed": 1,
            "total": 1,
        }),
    );
    let classification = ClassificationResult::from(result);
    info!(
        "Classification result (with extraction): {:?}",
        classification
    );
    Ok(classification)
}

/// Root directory under the system temp where compressed launcher folders
/// are extracted for scanning and importing. Entries are removed with
/// `drop_remove_temp_dir` once the frontend flow ends.
fn launcher_import_temp_base() -> std::path::PathBuf {
    std::env::temp_dir().join("axolotl-launcher-import")
}

/// Remove `drop-*` directories under `base` whose contents are older than
/// one day. The frontend cleans up after every flow, but a crashed process
/// would otherwise leave stale extractions behind forever.
fn sweep_stale_launcher_import_dirs(base: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(24 * 60 * 60));
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with("drop-") {
            continue;
        }
        let stale = match path.metadata().and_then(|m| m.modified()) {
            Ok(modified) => cutoff.is_none_or(|cutoff| modified < cutoff),
            Err(_) => false,
        };
        if stale {
            tracing::debug!(
                "Removing stale launcher import temp dir: {}",
                path.display()
            );
            let _ = std::fs::remove_dir_all(&path);
        }
    }
}

/// Extract a ZIP archive into a fresh temporary directory and return its
/// path. The frontend scans and imports instances from the extraction, then
/// calls [`drop_remove_temp_dir`] to clean it up — the archive is unpacked
/// exactly once.
#[tauri::command]
pub async fn drop_extract_zip_to_temp<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    zip_path: String,
) -> Result<String, String> {
    let zip_path = std::path::PathBuf::from(&zip_path);
    info!("Extracting launcher ZIP to temp: {}", zip_path.display());
    let _ = app.emit(
        "drop_classify_progress",
        serde_json::json!({
            "phase": "extract",
            "currentItem": zip_path.to_string_lossy(),
            "processed": 0,
            "total": null,
        }),
    );

    let base = launcher_import_temp_base();
    let zip_path_label = zip_path.to_string_lossy().to_string();
    let extracted =
        tokio::task::spawn_blocking(move || -> Result<String, String> {
            std::fs::create_dir_all(&base).map_err(|e| {
                format!("Failed to create temp base '{}': {e}", base.display())
            })?;
            sweep_stale_launcher_import_dirs(&base);
            let dir = base.join(format!(
                "drop-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ));
            std::fs::create_dir(&dir)
                .map_err(|e| format!("Failed to create temp directory: {e}"))?;
            theseus::drop_classifier::extract_zip_to_dir(&zip_path, &dir)
                .map_err(|e| {
                    let _ = std::fs::remove_dir_all(&dir);
                    tracing::warn!(
                        "Launcher ZIP extraction failed for '{}': {e}",
                        zip_path.display()
                    );
                    e
                })?;
            Ok(dir.to_string_lossy().to_string())
        })
        .await
        .map_err(|e| {
            tracing::warn!("Launcher ZIP extraction task panicked: {e}");
            format!("Extraction task panicked: {e}")
        })??;

    info!("Extracted launcher ZIP to: {extracted}");
    let _ = app.emit(
        "drop_classify_progress",
        serde_json::json!({
            "phase": "done",
            "currentItem": zip_path_label,
            "processed": 1,
            "total": 1,
        }),
    );
    Ok(extracted)
}

/// Remove a temporary directory created by [`drop_extract_zip_to_temp`].
/// Only paths inside the launcher import temp root are accepted.
#[tauri::command]
pub async fn drop_remove_temp_dir(path: String) -> Result<(), String> {
    let base = launcher_import_temp_base();
    let base = std::fs::canonicalize(&base)
        .map_err(|e| format!("Launcher import temp base missing: {e}"))?;
    let target = std::fs::canonicalize(&path)
        .map_err(|e| format!("Temp path missing: {e}"))?;
    if !target.starts_with(&base) {
        return Err(format!(
            "Refusing to remove '{}': not inside the launcher import temp root",
            target.display()
        ));
    }
    if !target.is_dir() {
        return Err(format!(
            "Refusing to remove '{}': not a directory",
            target.display()
        ));
    }
    tokio::task::spawn_blocking(move || {
        std::fs::remove_dir_all(&target).map_err(|e| {
            format!("Failed to remove temp dir '{}': {e}", target.display())
        })
    })
    .await
    .map_err(|e| format!("Cleanup task panicked: {e}"))?
}

/// Scan for importable instances in a launcher's data directory.
///
/// `launcher_type` must be one of the `ImportLauncherType` variant names
/// (e.g. `"MultiMC"`, `"PrismLauncher"`, `"HMCL"`).
#[tauri::command]
pub async fn drop_scan_launcher_instances<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    launcher_type: String,
    base_path: String,
) -> Result<Vec<ScannedInstance>, String> {
    info!(
        "Scanning launcher instances — type: {launcher_type}, path: {base_path}"
    );
    let base_path_label = base_path.clone();
    let _ = app.emit(
        "drop_classify_progress",
        serde_json::json!({
            "phase": "scan",
            "currentItem": &base_path_label,
            "processed": 0,
            "total": null,
        }),
    );
    let lt: ImportLauncherType =
        serde_json::from_str(&format!("\"{launcher_type}\"")).map_err(|e| {
            format!("Invalid launcher type '{launcher_type}': {e}")
        })?;
    let base = std::path::PathBuf::from(&base_path);
    let instances = get_importable_instances(lt, base)
        .await
        .map_err(|e| e.to_string())?;
    info!("Scan complete — found {} instance(s)", instances.len());
    let _ = app.emit(
        "drop_classify_progress",
        serde_json::json!({
            "phase": "done",
            "currentItem": &base_path_label,
            "processed": 1,
            "total": 1,
        }),
    );
    Ok(instances
        .into_iter()
        .map(|i| ScannedInstance {
            name: i.name,
            path: i.path,
        })
        .collect())
}

/// Detect processes holding a file lock on the given path.
///
/// Returns an empty list when detection is unavailable on the current platform
/// or the required tools are not installed.
#[tauri::command]
pub async fn drop_detect_file_lock(
    path: String,
) -> Result<Vec<LockingProcess>, String> {
    let path = std::path::PathBuf::from(&path);
    info!("Detecting file lock for: {}", path.display());
    let processes = get_locking_processes(&path);
    if !processes.is_empty() {
        warn!("File locked by {} process(es)", processes.len());
    }
    Ok(processes)
}

/// Extract mod metadata from a JAR file without installing it.
///
/// Reads the JAR bytes, extracts embedded mod metadata (fabric.mod.json,
/// quilt.mod.json, META-INF/mods.toml, etc.), and returns the parsed
/// `LocalModMetadata` as a JSON string.
#[tauri::command]
pub async fn drop_extract_mod_metadata(path: String) -> Result<String, String> {
    let path = std::path::PathBuf::from(&path);

    let meta = tokio::task::spawn_blocking(move || {
        let file_bytes = std::fs::read(&path)
            .map_err(|e| format!("Failed to read file: {e}"))?;
        let bytes = bytes::Bytes::from(file_bytes);
        theseus::mod_metadata::extract_mod_metadata(&bytes)
            .ok_or_else(|| "No mod metadata found in file".to_string())
    })
    .await
    .map_err(|e| format!("Metadata extraction task panicked: {e}"))??;
    serde_json::to_string(&meta)
        .map_err(|e| format!("Failed to serialize metadata: {e}"))
}

/// Look up a mod file by SHA1 hash to find matching Modrinth project and version.
///
/// Computes the SHA1 hash of the given file and queries the Modrinth API
/// to find matching versions. Returns project and version information if found.
#[tauri::command]
pub async fn drop_lookup_mod_hash(
    path: String,
) -> Result<Option<ModrinthLookupResult>, String> {
    let path = std::path::PathBuf::from(&path);
    info!("Looking up mod hash for: {}", path.display());

    lookup_mod_hash(&path)
        .await
        .map_err(|e| format!("Failed to lookup mod hash: {e}"))
}
