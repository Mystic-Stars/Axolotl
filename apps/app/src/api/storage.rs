use serde::Serialize;
use std::path::PathBuf;
use tauri::{Emitter, Runtime};
use tauri_plugin_opener::OpenerExt;
use theseus::storage::{
    StorageNode, StoragePath, StoragePathKind, StorageSize, StorageTree,
    assemble_storage_tree, load_storage_cache, save_storage_cache,
    scan_cache_category, scan_database_category, scan_instances_category,
    scan_meta_category, scan_root_other,
};

use crate::api::Result;

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("storage")
        .invoke_handler(tauri::generate_handler![
            storage_scan_start,
            storage_open_paths,
        ])
        .build()
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
enum StorageScanEvent {
    Started,
    Category { category: StorageNode },
    Complete { tree: StorageTree },
    Error { message: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StorageOpenResult {
    pub opened: Vec<String>,
    pub failed: Vec<StorageOpenFailure>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StorageOpenFailure {
    pub path: String,
    pub reason: String,
}

#[tauri::command]
pub async fn storage_scan_start<R: Runtime>(
    app: tauri::AppHandle<R>,
    force: bool,
) -> Result<()> {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_scan(&app, force).await {
            let _ = app.emit(
                "storage-scan",
                StorageScanEvent::Error {
                    message: error.to_string(),
                },
            );
        }
    });
    Ok(())
}

async fn run_scan<R: Runtime>(
    app: &tauri::AppHandle<R>,
    force: bool,
) -> crate::api::Result<()> {
    if !force {
        if let Some(tree) = load_storage_cache().await {
            let _ =
                app.emit("storage-scan", StorageScanEvent::Complete { tree });
            return Ok(());
        }
    }

    let _ = app.emit("storage-scan", StorageScanEvent::Started);

    let mut categories = Vec::new();
    let mut known = StorageSize::default();

    if let Some(node) = scan_instances_category().await? {
        known += node.size;
        categories.push(node.clone());
        let _ = app.emit(
            "storage-scan",
            StorageScanEvent::Category { category: node },
        );
    }
    if let Some(node) = scan_cache_category().await? {
        known += node.size;
        categories.push(node.clone());
        let _ = app.emit(
            "storage-scan",
            StorageScanEvent::Category { category: node },
        );
    }
    if let Some(node) = scan_meta_category().await? {
        known += node.size;
        categories.push(node.clone());
        let _ = app.emit(
            "storage-scan",
            StorageScanEvent::Category { category: node },
        );
    }
    if let Some(node) = scan_database_category().await? {
        known += node.size;
        categories.push(node.clone());
        let _ = app.emit(
            "storage-scan",
            StorageScanEvent::Category { category: node },
        );
    }

    let root_other = scan_root_other(known).await?;
    if let Some(node) = &root_other {
        let _ = app.emit(
            "storage-scan",
            StorageScanEvent::Category {
                category: node.clone(),
            },
        );
    }

    let tree = assemble_storage_tree(categories, root_other);
    if let Err(error) = save_storage_cache(&tree).await {
        tracing::warn!(
            error = %error,
            "Failed to persist launcher storage cache; continuing without cache"
        );
    }

    let _ = app.emit("storage-scan", StorageScanEvent::Complete { tree });
    Ok(())
}

#[tauri::command]
pub async fn storage_open_paths<R: Runtime>(
    app: tauri::AppHandle<R>,
    paths: Vec<StoragePath>,
) -> StorageOpenResult {
    let mut opened = Vec::new();
    let mut failed = Vec::new();

    for storage_path in paths {
        let path = PathBuf::from(&storage_path.path);
        let result = match storage_path.kind {
            StoragePathKind::File => {
                app.opener().reveal_item_in_dir(path.clone()).map(|_| ())
            }
            StoragePathKind::Directory => app
                .opener()
                .open_path(path.to_string_lossy(), None::<&str>)
                .map(|_| ()),
        };

        match result {
            Ok(()) => opened.push(storage_path.path),
            Err(error) => {
                tracing::warn!(
                    path = %storage_path.path,
                    error = %error,
                    "Failed to open storage path"
                );
                failed.push(StorageOpenFailure {
                    path: storage_path.path,
                    reason: error.to_string(),
                });
            }
        }
    }

    StorageOpenResult { opened, failed }
}
