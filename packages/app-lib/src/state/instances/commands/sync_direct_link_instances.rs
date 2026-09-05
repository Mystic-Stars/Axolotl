use super::create_direct_link_instance::create_direct_link_instance;
use crate::api::pack::import::{
    ImportLauncherType,
    direct_link::{direct_link_group, resolve_direct_link},
};
use crate::event::{InstancePayloadType, emit::emit_instance};
use crate::state::instances::{
    CreateDirectLinkInstance, EditInstance, adapters::sqlite::instance_rows,
};
use crate::state::{AppliedContentSetPatch, State};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectLinkSyncReport {
    pub imported: u32,
    pub updated: u32,
    pub removed: u32,
    pub missing: u32,
    pub errors: Vec<String>,
}

/// Reconciles configured external `.minecraft` roots with Axolotl's instance
/// records. The external filesystem is authoritative: new version folders are
/// associated, changed JSON metadata is refreshed, and records whose version
/// JSON disappeared are removed without touching any remaining files.
pub(crate) async fn sync_direct_link_instances(
    roots: Vec<PathBuf>,
    state: &State,
) -> crate::Result<DirectLinkSyncReport> {
    let mut report = DirectLinkSyncReport::default();
    let mut canonical_roots = Vec::new();
    for root in roots {
        match crate::util::io::canonicalize(&root) {
            Ok(root) if root.is_dir() => canonical_roots.push(root),
            Ok(_) => report.missing += 1,
            Err(error) => {
                report.errors.push(format!("{}: {error}", root.display()))
            }
        }
    }
    canonical_roots.sort();
    canonical_roots.dedup();

    let existing = crate::state::list_instances(&state.pool)
        .await?
        .into_iter()
        .collect::<Vec<_>>();
    let mut seen_json = Vec::<PathBuf>::new();

    for root in &canonical_roots {
        let versions = root.join("versions");
        let entries = match std::fs::read_dir(&versions) {
            Ok(entries) => entries,
            Err(error) => {
                report
                    .errors
                    .push(format!("{}: {error}", versions.display()));
                continue;
            }
        };
        for entry in entries.flatten() {
            let folder = entry.path();
            if !folder.is_dir() {
                continue;
            }
            let folder_name = entry.file_name().to_string_lossy().to_string();
            let instance_folder = Path::new("versions").join(&folder_name);
            let resolved = match resolve_direct_link(
                ImportLauncherType::Generic,
                root.clone(),
                instance_folder.to_string_lossy().to_string(),
                None,
            )
            .await
            {
                Ok(resolved) => resolved,
                Err(_) => continue,
            };
            seen_json.push(resolved.version_json.clone());

            let existing_instance = existing.iter().find(|metadata| {
                metadata
                    .instance
                    .linked_version_json_path
                    .as_deref()
                    .is_some_and(|path| {
                        Path::new(path) == resolved.version_json
                    })
                    || metadata
                        .instance
                        .game_dir_override
                        .as_deref()
                        .is_some_and(|path| Path::new(path) == folder)
            });

            if let Some(metadata) = existing_instance {
                let instance = &metadata.instance;
                let fields_changed = instance.linked_version_id.as_deref()
                    != Some(resolved.version_id.as_str())
                    || instance.linked_dot_minecraft.as_deref()
                        != Some(
                            resolved.dot_minecraft.to_string_lossy().as_ref(),
                        );
                let content_changed = metadata.applied_content_set.game_version
                    != resolved.game_version
                    || metadata.applied_content_set.loader != resolved.loader;
                let groups = direct_link_group(&resolved.dot_minecraft)
                    .into_iter()
                    .collect::<Vec<_>>();
                let groups_changed = metadata.groups != groups;
                if fields_changed || content_changed || groups_changed {
                    if content_changed {
                        crate::state::edit_instance(
                            &instance.id,
                            EditInstance {
                                content_set_patch: Some(
                                    AppliedContentSetPatch {
                                        game_version: Some(
                                            resolved.game_version.clone(),
                                        ),
                                        loader: Some(resolved.loader),
                                        ..AppliedContentSetPatch::default()
                                    },
                                ),
                                ..EditInstance::default()
                            },
                            &state.pool,
                        )
                        .await?;
                    }
                    let mut tx = state.pool.begin().await?;
                    instance_rows::set_direct_link_fields(
                        &instance.id,
                        &instance_rows::DirectLinkFields {
                            launcher: Some(resolved.launcher_key().to_string()),
                            launcher_root: Some(
                                resolved
                                    .launcher_root
                                    .to_string_lossy()
                                    .to_string(),
                            ),
                            dot_minecraft: Some(
                                resolved
                                    .dot_minecraft
                                    .to_string_lossy()
                                    .to_string(),
                            ),
                            version_id: Some(resolved.version_id.clone()),
                            version_json_path: Some(
                                resolved
                                    .version_json
                                    .to_string_lossy()
                                    .to_string(),
                            ),
                        },
                        &mut tx,
                    )
                    .await?;
                    instance_rows::replace_instance_groups(
                        &instance.id,
                        &groups,
                        &mut tx,
                    )
                    .await?;
                    tx.commit().await?;
                    let _ = emit_instance(
                        &instance.id,
                        InstancePayloadType::Edited,
                    )
                    .await;
                    report.updated += 1;
                }
            } else {
                let instance = create_direct_link_instance(
                    CreateDirectLinkInstance {
                        name: Some(folder_name),
                        launcher_type: ImportLauncherType::Generic,
                        base_path: root.clone(),
                        instance_folder: instance_folder
                            .to_string_lossy()
                            .to_string(),
                        instance_path: None,
                    },
                    state,
                )
                .await?;
                let _ =
                    emit_instance(&instance.id, InstancePayloadType::Created)
                        .await;
                report.imported += 1;
            }
        }
    }

    // Ordinary instances created with a version-isolated game-dir override
    // are also associated with a configured root. If that root is removed
    // from Settings before the next scan promotes the record to a direct
    // link, drop only the Axolotl record here as well.
    for metadata in &existing {
        if metadata.instance.linked_dot_minecraft.is_some() {
            continue;
        }
        let Some(game_dir_override) =
            metadata.instance.game_dir_override.as_deref()
        else {
            continue;
        };
        let Some(root) = version_isolated_root(game_dir_override) else {
            continue;
        };
        let Some(root) = crate::util::io::canonicalize(root).ok() else {
            continue;
        };
        if canonical_roots.iter().any(|candidate| candidate == &root) {
            continue;
        }
        instance_rows::delete_instance_by_id(
            &metadata.instance.id,
            &state.pool,
        )
        .await?;
        let _ =
            emit_instance(&metadata.instance.id, InstancePayloadType::Removed)
                .await;
        report.removed += 1;
    }

    for metadata in existing {
        let Some(json_path) =
            metadata.instance.linked_version_json_path.as_deref()
        else {
            continue;
        };
        let Some(root) = metadata.instance.linked_dot_minecraft.as_deref()
        else {
            continue;
        };
        let canonical_root = crate::util::io::canonicalize(root).ok();
        if canonical_root.as_ref().is_none_or(|root| {
            !canonical_roots.iter().any(|candidate| candidate == root)
        }) {
            // Configured roots are authoritative. Removing a root from Settings
            // only drops Axolotl's association; the external files remain intact.
            instance_rows::delete_instance_by_id(
                &metadata.instance.id,
                &state.pool,
            )
            .await?;
            let _ = emit_instance(
                &metadata.instance.id,
                InstancePayloadType::Removed,
            )
            .await;
            report.removed += 1;
            continue;
        }
        let json_path = PathBuf::from(json_path);
        if !json_path.exists()
            && !seen_json.iter().any(|path| path == &json_path)
        {
            // External deletion is authoritative, but there is nothing left
            // to delete on disk. Only remove the stale Axolotl record.
            instance_rows::delete_instance_by_id(
                &metadata.instance.id,
                &state.pool,
            )
            .await?;
            let _ = emit_instance(
                &metadata.instance.id,
                InstancePayloadType::Removed,
            )
            .await;
            report.removed += 1;
        }
    }

    Ok(report)
}

fn version_isolated_root(path: &str) -> Option<PathBuf> {
    let version_dir = Path::new(path);
    if version_dir
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        != Some("versions")
    {
        return None;
    }
    version_dir.parent()?.parent().map(Path::to_path_buf)
}
