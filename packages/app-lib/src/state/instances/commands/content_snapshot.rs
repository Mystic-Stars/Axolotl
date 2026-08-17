use std::collections::{HashMap, HashSet};

use sqlx::Row;

use super::list_content::list_all_content;
use super::sync_content_files::{
    project_type_for_file, sync_instance_content_files,
};
use crate::State;
use crate::state::instances::adapters::{
    filesystem,
    sqlite::{content_rows, instance_rows},
};
use crate::state::instances::{
    ContentDependencyInfo, ContentDependencyRef, ContentItemCapabilities,
    ContentOwnershipKind, InstanceContentPack, InstanceContentSnapshot,
    InstanceContentSnapshotItem, InstanceContentWarning, InstanceLink,
    ManualDownloadOperationKind, ManualDownloadState,
    PackMemberMaterializationState, PackMemberOverrideKind,
    PendingManualDownload,
};
use crate::state::{
    CacheBehaviour, CachedEntry, ContentItem, ContentItemProject,
    ContentItemVersion, ContentProvider, ContentProviderRef,
    ContentRequirement, ContentSourceKind,
};

pub(crate) async fn get_content_snapshot(
    instance_id: &str,
    refresh_remote: bool,
    state: &State,
) -> crate::Result<InstanceContentSnapshot> {
    let instance = instance_rows::get_instance_by_id(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;
    let content_set =
        content_rows::get_applied_content_set(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(
                    "Instance has no applied content set".to_string(),
                )
            })?;
    let link =
        instance_rows::get_instance_link(instance_id, &state.pool).await?;
    // Opening the content page must be a database read. A full file-system
    // reconciliation hashes and upserts every file, which is prohibitively
    // expensive for large modpacks and unnecessary after launcher-owned
    // mutations. Explicit refreshes and external file-change events use the
    // remote refresh path below.
    let files = if refresh_remote {
        sync_instance_content_files(&instance, state).await?
    } else {
        content_rows::get_instance_files(&instance.id, &state.pool).await?
    };
    let mut warnings = Vec::new();
    if refresh_remote
        && let InstanceLink::CurseForgeModpack {
            project_id,
            version_id,
        } = &link
    {
        let needs_reconciliation = sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(
				SELECT 1 FROM instance_pack_members
				WHERE content_set_id = ? AND reconciled = 0
			)",
        )
        .bind(&content_set.id)
        .fetch_one(&state.pool)
        .await?
            != 0;
        let needs_manual_recovery = sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(
				SELECT 1 FROM instance_pack_members member
				WHERE member.content_set_id = ?
					AND member.provider = 'curseforge'
					AND member.required = 1
					AND member.materialization_state = 'missing'
			)",
        )
        .bind(&content_set.id)
        .fetch_one(&state.pool)
        .await?
            != 0;
        let needs_local_classification = sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(
                SELECT 1
                FROM instance_files file
                LEFT JOIN instance_content_entries entry
                    ON entry.content_set_id = ? AND entry.file_id = file.id
                WHERE file.instance_id = ? AND file.missing = 0
                    AND entry.id IS NULL
            )",
        )
        .bind(&content_set.id)
        .bind(instance_id)
        .fetch_one(&state.pool)
        .await?
            != 0;
        let parsed_ids = project_id
            .parse::<u32>()
            .ok()
            .zip(version_id.parse::<u32>().ok());
        let result = match parsed_ids {
            Some((project_id, version_id)) => {
                match crate::api::curseforge::get_modpack_expected_members(
                    project_id, version_id,
                )
                .await
                {
                    Ok(expected) => {
                        let should_reconcile = needs_reconciliation
                            || needs_local_classification
                            || !expected.overrides.is_empty();
                        if should_reconcile
                            && let Err(error) = reconcile_curseforge_members(
                                instance_id,
                                &content_set.id,
                                &expected,
                                state,
                            )
                            .await
                        {
                            Err(error)
                        } else if should_reconcile || needs_manual_recovery {
                            reconcile_curseforge_manual_downloads(
                                instance_id,
                                &content_set.id,
                                &expected.members,
                                state,
                            )
                            .await
                        } else {
                            Ok(())
                        }
                    }
                    Err(error) => Err(error),
                }
            }
            None => Err(crate::ErrorKind::InputError(
                "Linked CurseForge pack IDs are invalid".to_string(),
            )
            .into()),
        };
        if let Err(error) = result {
            warnings.push(InstanceContentWarning {
                code: "pack_membership_reconciliation_failed".to_string(),
                message: error.to_string(),
                provider: Some(ContentProvider::CurseForge),
            });
        }
    }

    let entries =
        content_rows::get_content_entries(&content_set.id, &state.pool).await?;
    let members =
        content_rows::get_pack_members(&content_set.id, &state.pool).await?;
    let pending_manual_downloads =
        content_rows::get_pending_manual_downloads(instance_id, &state.pool)
            .await?;

    let mut refs_by_entry = HashMap::new();
    let mut origin_by_entry = HashMap::new();
    let provider_rows = sqlx::query(
        "SELECT ref.content_entry_id, ref.provider,
                ref.provider_project_id, ref.provider_release_id,
                ref.is_origin
         FROM instance_content_provider_refs ref
         INNER JOIN instance_content_entries entry
            ON entry.id = ref.content_entry_id
         WHERE entry.content_set_id = ?
         ORDER BY ref.content_entry_id, ref.provider",
    )
    .bind(&content_set.id)
    .fetch_all(&state.pool)
    .await?;
    for row in provider_rows {
        let entry_id = row.try_get::<String, _>("content_entry_id")?;
        let provider = row.try_get::<String, _>("provider")?;
        let provider_project_id =
            row.try_get::<String, _>("provider_project_id")?;
        let provider_release_id =
            row.try_get::<Option<String>, _>("provider_release_id")?;
        let reference = ContentProviderRef::from_database(
            &provider,
            &provider_project_id,
            provider_release_id.as_deref(),
        )?;
        refs_by_entry
            .entry(entry_id.clone())
            .or_insert_with(Vec::new)
            .push(reference);
        if row.try_get::<i64, _>("is_origin")? != 0 {
            origin_by_entry
                .insert(entry_id, ContentProvider::from_str(&provider)?);
        }
    }

    let entries_by_file_id = entries
        .iter()
        .filter_map(|entry| {
            entry.file_id.as_ref().map(|file_id| (file_id, entry))
        })
        .collect::<HashMap<_, _>>();
    let members_by_entry_id = members
        .iter()
        .filter_map(|member| {
            member
                .content_entry_id
                .as_ref()
                .map(|entry_id| (entry_id, member))
        })
        .collect::<HashMap<_, _>>();
    let mut local_metadata_by_entry = HashMap::new();
    for file in &files {
        let Some(entry) = entries_by_file_id.get(&file.id) else {
            continue;
        };
        let Some(json) = file.local_mod_data.as_ref() else {
            continue;
        };
        if let Ok(metadata) =
            serde_json::from_str::<crate::mod_metadata::LocalModMetadata>(json)
        {
            local_metadata_by_entry.insert(entry.id.clone(), metadata);
        }
    }
    let mut represented_members = HashSet::new();
    let mut items = Vec::new();

    for file in files.into_iter().filter(|file| !file.missing) {
        let Some(project_type) = project_type_for_file(&file) else {
            continue;
        };
        let entry = entries_by_file_id.get(&file.id).copied();
        let member =
            entry.and_then(|entry| members_by_entry_id.get(&entry.id).copied());
        if let Some(member) = member {
            represented_members.insert(member.id.as_str());
        }
        let ownership_kind = entry
            .map_or(ContentOwnershipKind::LocalDiscovered, |entry| {
                entry.ownership_kind
            });
        let provider_refs = entry
            .and_then(|entry| refs_by_entry.get(&entry.id))
            .cloned()
            .unwrap_or_default();
        let origin_provider = entry
            .and_then(|entry| origin_by_entry.get(&entry.id))
            .copied();
        let local_mod = file.local_mod_data.as_ref().and_then(|json| {
            serde_json::from_str::<crate::mod_metadata::LocalModMetadata>(json)
                .ok()
        });
        let title = local_mod
            .as_ref()
            .and_then(|metadata| metadata.name.clone())
            .or_else(|| {
                std::path::Path::new(&file.file_name)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| file.file_name.clone());
        let external = provider_refs.is_empty();
        let content = ContentItem {
            file_name: file.file_name.clone(),
            file_path: file.relative_path.clone(),
            id: file.sha1.clone(),
            size: file.size,
            enabled: entry
                .map_or(file.enabled, |entry| entry.enabled && file.enabled),
            project_type,
            project: Some(ContentItemProject {
                id: local_mod
                    .as_ref()
                    .map(|metadata| format!("local:{}", metadata.mod_id))
                    .unwrap_or_else(|| format!("local:file:{}", file.sha1)),
                slug: local_mod
                    .as_ref()
                    .map(|metadata| metadata.mod_id.clone()),
                title,
                icon_url: file.icon_path.clone(),
                license: None,
            }),
            version: local_mod.as_ref().and_then(|metadata| {
                metadata.version.clone().map(|version| ContentItemVersion {
                    id: format!("local:{}", metadata.mod_id),
                    version_number: version,
                    file_name: file.file_name.clone(),
                    date_published: None,
                })
            }),
            owner: None,
            update: None,
            date_added: Some(file.modified_at.to_rfc3339()),
            provider_refs,
            origin_provider,
            rollback: None,
            environment: None,
            source_kind: entry.map(|entry| entry.source_kind),
            external,
            loader: local_mod.as_ref().and_then(|meta| meta.loader.clone()),
        };
        items.push(snapshot_item(
            Some(file.id.clone()),
            entry.map(|entry| entry.id.clone()),
            member,
            ownership_kind,
            project_type,
            file.relative_path,
            Some(content),
        ));
    }

    for member in &members {
        if represented_members.contains(member.id.as_str()) {
            continue;
        }
        if !filesystem::is_scannable_project_path(
            member.project_type,
            &member.expected_relative_path,
        ) {
            continue;
        }
        items.push(snapshot_item(
            None,
            member.content_entry_id.clone(),
            Some(member),
            ContentOwnershipKind::PackManaged,
            member.project_type,
            member.expected_relative_path.clone(),
            None,
        ));
    }

    // Dependency discovery can fetch metadata for every member of a pack.
    // Even an explicit refresh returns the file snapshot first; the slower
    // metadata pass publishes one lightweight follow-up snapshot when done.
    if refresh_remote {
        let instance_id = instance_id.to_string();
        let entries = entries.clone();
        let refs_by_entry = refs_by_entry.clone();
        let content_set = content_set.clone();
        let local_metadata_by_entry = local_metadata_by_entry.clone();
        tokio::spawn(async move {
            let state = match State::get().await {
                Ok(state) => state,
                Err(error) => {
                    tracing::warn!(
                        "Content dependency backfill skipped: {error}"
                    );
                    return;
                }
            };
            let backfill = match backfill_dependency_edges_from_metadata(
                &entries,
                &refs_by_entry,
                &content_set,
                &local_metadata_by_entry,
                &state,
            )
            .await
            {
                Ok(backfill) => backfill,
                Err(error) => {
                    tracing::warn!(
                        "Content dependency backfill failed: {error}"
                    );
                    return;
                }
            };
            if backfill.is_empty() {
                return;
            }

            let _instance_lock =
                state.lock_instance_content(&instance_id).await;
            let mut tx = match state.pool.begin_with("BEGIN IMMEDIATE").await {
                Ok(tx) => tx,
                Err(error) => {
                    tracing::warn!(
                        "Content dependency revision update failed: {error}"
                    );
                    return;
                }
            };
            for edge in &backfill.edges {
                if let Err(error) =
                    content_rows::upsert_content_dependency_edge_in_transaction(
                        edge, &mut tx,
                    )
                    .await
                {
                    tracing::warn!(
                        "Content dependency backfill write failed: {error}"
                    );
                    return;
                }
            }
            for entry_id in &backfill.backfilled_entry_ids {
                if let Err(error) = content_rows::set_content_entry_dependency_backfilled_in_transaction(
                    entry_id,
                    &mut tx,
                )
                .await
                {
                    tracing::warn!("Content dependency backfill write failed: {error}");
                    return;
                }
            }
            let revision =
                match content_rows::bump_content_set_revision_in_transaction(
                    &content_set.id,
                    &mut tx,
                )
                .await
                {
                    Ok(revision) => revision,
                    Err(error) => {
                        tracing::warn!(
                            "Content dependency revision update failed: {error}"
                        );
                        return;
                    }
                };
            if let Err(error) = tx.commit().await {
                tracing::warn!(
                    "Content dependency revision commit failed: {error}"
                );
                return;
            }
            if let Err(error) = crate::event::emit::emit_instance(
                &instance_id,
                crate::event::InstancePayloadType::ContentChanged { revision },
            )
            .await
            {
                tracing::warn!(
                    "Content dependency change event failed: {error}"
                );
            }
        });
    }

    attach_dependency_info(&mut items, &entries, &content_set.id, state)
        .await?;

    let cache_behaviour = if refresh_remote {
        CacheBehaviour::MustRevalidate
    } else {
        CacheBehaviour::CacheOnly
    };
    match list_all_content(
        instance_id,
        Some(cache_behaviour),
        refresh_remote,
        state,
    )
    .await
    {
        Ok(enriched) => {
            let enriched = enriched
                .into_iter()
                .map(|item| (item.file_path.clone(), item))
                .collect::<HashMap<_, _>>();
            for item in &mut items {
                if let Some(content) =
                    enriched.get(&item.expected_relative_path)
                {
                    item.content = Some(content.clone());
                    item.capabilities.can_update = content.update.is_some()
                        && item.materialization_state
                            == PackMemberMaterializationState::Present;
                }
            }
        }
        Err(error) => warnings.push(InstanceContentWarning {
            code: "provider_metadata_unavailable".to_string(),
            message: error.to_string(),
            provider: None,
        }),
    }

    let remote_pack = match super::list_content::get_linked_modpack_info(
        instance_id,
        None,
        Some(cache_behaviour),
        state,
    )
    .await
    {
        Ok(pack) => pack,
        Err(error) => {
            warnings.push(InstanceContentWarning {
                code: "pack_metadata_unavailable".to_string(),
                message: error.to_string(),
                provider: pack_provider(&link),
            });
            None
        }
    };
    let pack = pack_snapshot(
        &instance.name,
        instance.icon_path.clone(),
        &link,
        &members,
        remote_pack,
    );

    items.sort_by(|left, right| {
        left.expected_relative_path
            .to_lowercase()
            .cmp(&right.expected_relative_path.to_lowercase())
    });

    Ok(InstanceContentSnapshot {
        instance_id: instance_id.to_string(),
        revision: content_rows::get_applied_content_set(
            instance_id,
            &state.pool,
        )
        .await?
        .map_or(content_set.revision, |set| set.revision),
        pack,
        items,
        pending_manual_downloads,
        warnings,
    })
}

#[derive(Clone)]
struct CurseForgeReconciliationCandidate {
    entry_id: String,
    relative_path: String,
    enabled: bool,
    missing: bool,
    provider_project_id: Option<String>,
    provider_release_id: Option<String>,
}

pub(crate) async fn reconcile_curseforge_members(
    instance_id: &str,
    content_set_id: &str,
    expected: &crate::api::curseforge::CurseForgePackExpectedContent,
    state: &State,
) -> crate::Result<()> {
    use chrono::Utc;

    let _instance_lock = state.lock_instance_content(instance_id).await;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let rows = sqlx::query(
        "SELECT entry.id AS entry_id, file.relative_path, entry.enabled,
			file.missing, ref.provider_project_id, ref.provider_release_id
		 FROM instance_content_entries entry
		 INNER JOIN instance_files file ON file.id = entry.file_id
		 LEFT JOIN instance_content_provider_refs ref
			ON ref.content_entry_id = entry.id AND ref.provider = 'curseforge'
		 WHERE entry.content_set_id = ?
		 ORDER BY entry.modified_at DESC",
    )
    .bind(content_set_id)
    .fetch_all(&mut *tx)
    .await?;
    let candidates = rows
        .into_iter()
        .map(|row| {
            Ok(CurseForgeReconciliationCandidate {
                entry_id: row.try_get("entry_id")?,
                relative_path: row.try_get("relative_path")?,
                enabled: row.try_get::<i64, _>("enabled")? != 0,
                missing: row.try_get::<i64, _>("missing")? != 0,
                provider_project_id: row.try_get("provider_project_id")?,
                provider_release_id: row.try_get("provider_release_id")?,
            })
        })
        .collect::<crate::Result<Vec<_>>>()?;
    let member_rows = sqlx::query(
        "SELECT id, member_key, override_kind, materialization_state
		 FROM instance_pack_members WHERE content_set_id = ?",
    )
    .bind(content_set_id)
    .fetch_all(&mut *tx)
    .await?;
    let mut existing_members = member_rows
        .into_iter()
        .map(|row| {
            let key = row.try_get::<String, _>("member_key")?;
            Ok((
                key,
                (
                    row.try_get::<String, _>("id")?,
                    PackMemberOverrideKind::from_str(
                        &row.try_get::<String, _>("override_kind")?,
                    )?,
                    PackMemberMaterializationState::from_str(
                        &row.try_get::<String, _>("materialization_state")?,
                    )?,
                ),
            ))
        })
        .collect::<crate::Result<HashMap<_, _>>>()?;
    let pending_rows = sqlx::query(
        "SELECT provider_project_id, provider_release_id
		 FROM instance_pending_manual_downloads
		 WHERE instance_id = ? AND provider = 'curseforge'
			AND state IN ('waiting', 'matched', 'error')",
    )
    .bind(instance_id)
    .fetch_all(&mut *tx)
    .await?;
    let pending = pending_rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("provider_project_id")?,
                row.try_get::<String, _>("provider_release_id")?,
            ))
        })
        .collect::<crate::Result<HashSet<_>>>()?;

    sqlx::query(
        "UPDATE instance_content_entries
		 SET ownership_kind = 'user_added', modified_at = ?
		 WHERE content_set_id = ? AND ownership_kind = 'pack_managed'",
    )
    .bind(Utc::now().timestamp())
    .bind(content_set_id)
    .execute(&mut *tx)
    .await?;

    let mut claimed_entries = HashSet::new();
    for expected_member in &expected.members {
        let project_id = expected_member.project_id.to_string();
        let file_id = expected_member.file_id.to_string();
        let expected_path =
            normalized_content_path(&expected_member.expected_relative_path);
        let candidate = candidates
            .iter()
            .find(|candidate| {
                !claimed_entries.contains(&candidate.entry_id)
                    && candidate.provider_project_id.as_deref()
                        == Some(project_id.as_str())
                    && candidate.provider_release_id.as_deref()
                        == Some(file_id.as_str())
            })
            .or_else(|| {
                candidates.iter().find(|candidate| {
                    !claimed_entries.contains(&candidate.entry_id)
                        && candidate.provider_project_id.as_deref()
                            == Some(project_id.as_str())
                })
            })
            .or_else(|| {
                candidates.iter().find(|candidate| {
                    !claimed_entries.contains(&candidate.entry_id)
                        && normalized_content_path(&candidate.relative_path)
                            == expected_path
                })
            });
        if let Some(candidate) = candidate {
            claimed_entries.insert(candidate.entry_id.clone());
            sqlx::query(
                "UPDATE instance_content_entries
				 SET ownership_kind = 'pack_managed', modified_at = ?
				 WHERE id = ?",
            )
            .bind(Utc::now().timestamp())
            .bind(&candidate.entry_id)
            .execute(&mut *tx)
            .await?;
        }

        let member_key = format!(
            "curseforge:{}:{}",
            expected_member.project_id,
            expected_member.project_type.get_name()
        );
        let existing = existing_members.remove(&member_key);
        let inherited_override = existing
            .as_ref()
            .map_or(PackMemberOverrideKind::None, |(_, kind, _)| *kind);
        let override_kind =
            if inherited_override != PackMemberOverrideKind::None {
                inherited_override
            } else if candidate.is_some_and(|candidate| {
                candidate.provider_project_id.as_deref()
                    == Some(project_id.as_str())
                    && candidate.provider_release_id.as_deref()
                        != Some(file_id.as_str())
            }) {
                PackMemberOverrideKind::Version
            } else if candidate.is_some_and(|candidate| !candidate.enabled) {
                PackMemberOverrideKind::Disabled
            } else {
                PackMemberOverrideKind::None
            };
        let materialization_state = if override_kind
            == PackMemberOverrideKind::Removed
            && candidate.is_none()
        {
            PackMemberMaterializationState::Removed
        } else if pending.contains(&(project_id.clone(), file_id.clone())) {
            PackMemberMaterializationState::PendingManual
        } else if candidate.is_some_and(|candidate| !candidate.missing) {
            PackMemberMaterializationState::Present
        } else {
            PackMemberMaterializationState::Missing
        };
        let now = Utc::now();
        let member_id = existing
            .map(|(id, _, _)| id)
            .unwrap_or_else(|| format!("pack-member:{}", uuid::Uuid::new_v4()));
        content_rows::upsert_pack_member_in_transaction(
            &crate::state::instances::PackMember {
                id: member_id.clone(),
                content_set_id: content_set_id.to_string(),
                content_entry_id: candidate
                    .map(|candidate| candidate.entry_id.clone()),
                member_key,
                project_type: expected_member.project_type,
                expected_relative_path: expected_member
                    .expected_relative_path
                    .clone(),
                provider: Some(ContentProvider::CurseForge),
                provider_project_id: Some(project_id),
                provider_release_id: Some(file_id),
                required: expected_member.required,
                expected_sha1: expected_member.expected_sha1.clone(),
                expected_size: expected_member.expected_size,
                expected_fingerprint: expected_member.expected_fingerprint,
                materialization_state,
                override_kind,
                reconciled: true,
                created_at: now,
                modified_at: now,
            },
            &mut tx,
        )
        .await?;
        sqlx::query(
            "UPDATE instance_pack_members
			 SET required = ?, reconciled = 1 WHERE id = ?",
        )
        .bind(i64::from(expected_member.required))
        .bind(member_id)
        .execute(&mut *tx)
        .await?;
    }

    for (_, (member_id, _, _)) in existing_members {
        sqlx::query("DELETE FROM instance_pack_members WHERE id = ?")
            .bind(member_id)
            .execute(&mut *tx)
            .await?;
    }
    reconcile_curseforge_override_entries_in_transaction(
        instance_id,
        content_set_id,
        &expected.overrides,
        &mut tx,
    )
    .await?;
    content_rows::bump_content_set_revision_in_transaction(
        content_set_id,
        &mut tx,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn reconcile_curseforge_override_entries_in_transaction(
    instance_id: &str,
    content_set_id: &str,
    expected: &[crate::api::curseforge::CurseForgePackExpectedOverride],
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> crate::Result<()> {
    let rows = sqlx::query(
        "SELECT file.id AS file_id, file.relative_path, file.enabled,
            (
                SELECT entry.id
                FROM instance_content_entries entry
                WHERE entry.content_set_id = ? AND entry.file_id = file.id
                ORDER BY entry.modified_at DESC
                LIMIT 1
            ) AS entry_id
         FROM instance_files file
         WHERE file.instance_id = ? AND file.missing = 0",
    )
    .bind(content_set_id)
    .bind(instance_id)
    .fetch_all(&mut **tx)
    .await?;
    let files = rows
        .into_iter()
        .map(|row| {
            let relative_path = row.try_get::<String, _>("relative_path")?;
            Ok((
                normalized_content_path(&relative_path),
                (
                    row.try_get::<String, _>("file_id")?,
                    row.try_get::<i64, _>("enabled")? != 0,
                    row.try_get::<Option<String>, _>("entry_id")?,
                ),
            ))
        })
        .collect::<crate::Result<HashMap<_, _>>>()?;
    let expected = expected
        .iter()
        .map(|item| {
            (normalized_content_path(&item.expected_relative_path), item)
        })
        .collect::<HashMap<_, _>>();

    for (relative_path, (file_id, enabled, entry_id)) in files {
        let override_content = expected.get(&relative_path).copied();
        if entry_id.is_some() && override_content.is_none() {
            continue;
        }
        let Some(project_type) =
            override_content.map(|item| item.project_type).or_else(|| {
                filesystem::project_type_from_relative_path(&relative_path)
            })
        else {
            continue;
        };
        let is_pack_override = override_content.is_some();
        content_rows::upsert_content_entry_from_parts_in_transaction(
            content_rows::UpsertContentEntry {
                instance_id,
                content_set_id,
                file_id: Some(&file_id),
                project_type,
                source_kind: if is_pack_override {
                    ContentSourceKind::CurseForge
                } else {
                    ContentSourceKind::Local
                },
                ownership_kind: if is_pack_override {
                    ContentOwnershipKind::PackManaged
                } else {
                    ContentOwnershipKind::UserAdded
                },
                auto_dependency: false,
                server_requirement: ContentRequirement::Required,
                client_requirement: ContentRequirement::Required,
                enabled,
            },
            tx,
        )
        .await?;
    }
    Ok(())
}

async fn reconcile_curseforge_manual_downloads(
    instance_id: &str,
    content_set_id: &str,
    expected: &[crate::api::curseforge::CurseForgePackExpectedMember],
    state: &State,
) -> crate::Result<()> {
    let restricted = expected
        .iter()
        .filter(|member| member.required && member.manual_download.is_some())
        .collect::<Vec<_>>();
    if restricted.is_empty() {
        return Ok(());
    }

    let _instance_lock = state.lock_instance_content(instance_id).await;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let member_rows = sqlx::query(
        "SELECT id, content_entry_id, provider_project_id,
                provider_release_id, materialization_state, override_kind
         FROM instance_pack_members
         WHERE content_set_id = ? AND provider = 'curseforge'",
    )
    .bind(content_set_id)
    .fetch_all(&mut *tx)
    .await?;
    let members = member_rows
        .into_iter()
        .map(|row| {
            Ok((
                (
                    row.try_get::<Option<String>, _>("provider_project_id")?,
                    row.try_get::<Option<String>, _>("provider_release_id")?,
                ),
                (
                    row.try_get::<String, _>("id")?,
                    row.try_get::<Option<String>, _>("content_entry_id")?,
                    PackMemberMaterializationState::from_str(
                        &row.try_get::<String, _>("materialization_state")?,
                    )?,
                    PackMemberOverrideKind::from_str(
                        &row.try_get::<String, _>("override_kind")?,
                    )?,
                ),
            ))
        })
        .collect::<crate::Result<HashMap<_, _>>>()?;
    let pending_rows = sqlx::query(
        "SELECT provider_project_id, provider_release_id
         FROM instance_pending_manual_downloads
         WHERE instance_id = ? AND provider = 'curseforge'
            AND state IN ('waiting', 'matched', 'error')",
    )
    .bind(instance_id)
    .fetch_all(&mut *tx)
    .await?;
    let mut pending = pending_rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("provider_project_id")?,
                row.try_get::<String, _>("provider_release_id")?,
            ))
        })
        .collect::<crate::Result<HashSet<_>>>()?;
    let mut changed = false;

    for expected_member in restricted {
        let key = (
            Some(expected_member.project_id.to_string()),
            Some(expected_member.file_id.to_string()),
        );
        let Some((
            member_id,
            content_entry_id,
            materialization_state,
            override_kind,
        )) = members.get(&key)
        else {
            continue;
        };
        if matches!(
            *materialization_state,
            PackMemberMaterializationState::Present
                | PackMemberMaterializationState::Removed
        ) {
            continue;
        }

        let project_id = expected_member.project_id.to_string();
        let file_id = expected_member.file_id.to_string();
        if !pending.contains(&(project_id.clone(), file_id.clone())) {
            let Some(mut manual_download) =
                expected_member.manual_download.clone()
            else {
                continue;
            };
            manual_download.operation_kind =
                ManualDownloadOperationKind::PackInstall;
            let now = chrono::Utc::now();
            content_rows::upsert_pending_manual_download_in_transaction(
                &PendingManualDownload {
                    id: format!("manual-download:{}", uuid::Uuid::new_v4()),
                    instance_id: instance_id.to_string(),
                    pack_member_id: Some(member_id.clone()),
                    content_entry_id: content_entry_id.clone(),
                    operation_kind: ManualDownloadOperationKind::PackInstall,
                    operation_target_id: None,
                    project_type: expected_member.project_type,
                    provider: ContentProvider::CurseForge,
                    provider_project_id: project_id.clone(),
                    provider_release_id: file_id.clone(),
                    file_name: manual_download.file_name.clone(),
                    website_url: manual_download.website_url.clone(),
                    target_relative_path: expected_member
                        .expected_relative_path
                        .clone(),
                    expected_sha1: expected_member.expected_sha1.clone(),
                    expected_size: expected_member.expected_size,
                    expected_fingerprint: expected_member.expected_fingerprint,
                    state: ManualDownloadState::Waiting,
                    context: serde_json::to_value(&manual_download)?,
                    created_at: now,
                    modified_at: now,
                },
                &mut tx,
            )
            .await?;
            pending.insert((project_id, file_id));
            changed = true;
        }

        if *materialization_state
            != PackMemberMaterializationState::PendingManual
        {
            content_rows::set_pack_member_override_in_transaction(
                member_id,
                PackMemberMaterializationState::PendingManual,
                *override_kind,
                &mut tx,
            )
            .await?;
            changed = true;
        }
    }

    if changed {
        content_rows::bump_content_set_revision_in_transaction(
            content_set_id,
            &mut tx,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn normalized_content_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches(".disabled")
        .to_ascii_lowercase()
}

#[derive(Default)]
struct DependencyBackfill {
    edges: Vec<crate::state::instances::ContentDependencyEdge>,
    backfilled_entry_ids: HashSet<String>,
}

impl DependencyBackfill {
    fn is_empty(&self) -> bool {
        self.edges.is_empty() && self.backfilled_entry_ids.is_empty()
    }
}

async fn backfill_dependency_edges_from_metadata(
    entries: &[crate::state::instances::ContentEntry],
    refs_by_entry: &HashMap<String, Vec<ContentProviderRef>>,
    content_set: &crate::state::instances::ContentSet,
    local_metadata_by_entry: &HashMap<
        String,
        crate::mod_metadata::LocalModMetadata,
    >,
    state: &State,
) -> crate::Result<DependencyBackfill> {
    // Only entries that have never had their edges persisted are backfilled.
    // Checking the table as a whole would leave entries installed before the
    // edges feature without edges as soon as any other entry had edges.
    let existing = content_rows::get_content_dependency_edges(
        &content_set.id,
        &state.pool,
    )
    .await?;
    let entries_with_edges = existing
        .iter()
        .map(|edge| edge.parent_entry_id.as_str())
        .collect::<HashSet<_>>();
    let modrinth_origins = entries
        .iter()
        .filter(|entry| !entries_with_edges.contains(entry.id.as_str()))
        .filter_map(|entry| {
            refs_by_entry.get(&entry.id).and_then(|references| {
                references.iter().find_map(|reference| match reference {
                    ContentProviderRef::Modrinth {
                        project_id,
                        version_id: Some(version_id),
                    } => Some((
                        entry.id.clone(),
                        (project_id.clone(), version_id.clone()),
                    )),
                    _ => None,
                })
            })
        })
        .collect::<HashMap<_, _>>();
    let version_ids = modrinth_origins
        .values()
        .map(|(_, version_id)| version_id.clone())
        .collect::<Vec<_>>();
    let versions_by_id = if version_ids.is_empty() {
        HashMap::new()
    } else {
        CachedEntry::get_version_many(
            &version_ids,
            None,
            &state.pool,
            &state.api_semaphore,
        )
        .await?
        .into_iter()
        .map(|version| (version.id.clone(), version))
        .collect::<HashMap<_, _>>()
    };
    let mut backfill = DependencyBackfill::default();

    for entry in entries {
        if entries_with_edges.contains(entry.id.as_str()) {
            continue;
        }
        let Some((project_id, version_id)) = modrinth_origins.get(&entry.id)
        else {
            continue;
        };
        let Some(version) = versions_by_id.get(version_id.as_str()) else {
            continue;
        };
        for dependency in &version.dependencies {
            if !matches!(
                dependency.dependency_type,
                crate::state::DependencyType::Required
            ) {
                continue;
            }
            let Some(dependency_project_id) = dependency.project_id.clone()
            else {
                continue;
            };
            let dependency_project_id = if content_set.loader
                == crate::state::ModLoader::Quilt
                && dependency_project_id == "P7dR8mSH"
            {
                "qvIfYCYJ".to_string()
            } else {
                dependency_project_id
            };
            let child_entry =
                if let Some(version_id) = dependency.version_id.as_deref() {
                    match content_rows::get_content_entry_by_provider_ref(
                        &content_set.id,
                        ContentProvider::Modrinth,
                        &dependency_project_id,
                        version_id,
                        &state.pool,
                    )
                    .await?
                    {
                        Some(entry) => Some(entry),
                        None => {
                            // The installed version may differ from the
                            // declared one; match by project as a fallback.
                            content_rows::get_content_entry_by_provider_project(
                                &content_set.id,
                                ContentProvider::Modrinth,
                                &dependency_project_id,
                                &state.pool,
                            )
                            .await?
                        }
                    }
                } else {
                    content_rows::get_content_entry_by_provider_project(
                        &content_set.id,
                        ContentProvider::Modrinth,
                        &dependency_project_id,
                        &state.pool,
                    )
                    .await?
                };
            let Some(child_entry) = child_entry else {
                continue;
            };
            let child_release_id = refs_by_entry
                .get(&child_entry.id)
                .and_then(|references| {
                    references.iter().find_map(|reference| match reference {
                        ContentProviderRef::Modrinth {
                            version_id: Some(version_id),
                            ..
                        } => Some(version_id.to_string()),
                        _ => None,
                    })
                })
                .unwrap_or_else(|| {
                    dependency.version_id.clone().unwrap_or_default()
                });
            if child_release_id.is_empty() {
                continue;
            }
            let now = chrono::Utc::now();
            backfill.edges.push(crate::state::instances::ContentDependencyEdge {
                id: format!("content-dependency:{}", uuid::Uuid::new_v4()),
                content_set_id: content_set.id.clone(),
                parent_entry_id: entry.id.clone(),
                child_entry_id: child_entry.id,
                evidence_provider: ContentProvider::Modrinth,
                parent_provider: ContentProvider::Modrinth,
                child_provider: ContentProvider::Modrinth,
                dependency_kind: crate::state::instances::ContentDependencyKind::Required,
                parent_project_id: project_id.to_string(),
                parent_release_id: version_id.to_string(),
                child_project_id: dependency_project_id,
                child_release_id,
                created_at: now,
                modified_at: now,
            });
        }
    }

    backfill_curseforge_dependency_edges(
        entries,
        refs_by_entry,
        &entries_with_edges,
        content_set,
        state,
        &mut backfill,
    )
    .await?;

    backfill_local_dependency_edges(
        entries,
        refs_by_entry,
        &entries_with_edges,
        content_set,
        local_metadata_by_entry,
        &mut backfill,
    )
    .await?;

    Ok(backfill)
}

/// Link CurseForge-identified entries (installed files and pack members alike)
/// through the dependency relations of their installed file. Pack members are
/// not special-cased: every CurseForge entry without persisted edges is
/// resolved the same way, in one batched API request.
async fn backfill_curseforge_dependency_edges(
    entries: &[crate::state::instances::ContentEntry],
    refs_by_entry: &HashMap<String, Vec<ContentProviderRef>>,
    entries_with_edges: &HashSet<&str>,
    content_set: &crate::state::instances::ContentSet,
    state: &State,
    backfill: &mut DependencyBackfill,
) -> crate::Result<()> {
    let backfilled_entry_ids =
        content_rows::get_dependency_backfilled_entry_ids(
            &content_set.id,
            &state.pool,
        )
        .await?;
    let mut pending = Vec::new();
    for entry in entries {
        if entries_with_edges.contains(entry.id.as_str())
            || backfilled_entry_ids.contains(&entry.id)
        {
            continue;
        }
        let Some((project_id, file_id)) =
            refs_by_entry.get(&entry.id).and_then(|references| {
                references.iter().find_map(|reference| match reference {
                    ContentProviderRef::CurseForge {
                        project_id,
                        file_id: Some(file_id),
                    } => Some((project_id.get(), file_id.get())),
                    _ => None,
                })
            })
        else {
            continue;
        };
        pending.push((entry, project_id, file_id));
    }
    if pending.is_empty() {
        return Ok(());
    }

    // One batched request covers every pending entry. A failure here must not
    // break the content page: skip this snapshot and retry on the next one.
    let file_ids = pending
        .iter()
        .map(|(_, _, file_id)| *file_id)
        .collect::<Vec<_>>();
    let files = match crate::api::curseforge::get_files_many(file_ids).await {
        Ok(files) => files,
        Err(error) => {
            tracing::warn!("CurseForge dependency backfill skipped: {error}");
            return Ok(());
        }
    };
    let dependencies_by_file_id = files
        .into_iter()
        .map(|file| (file.id, file.dependencies))
        .collect::<HashMap<_, _>>();

    let is_quilt = content_set.loader == crate::state::ModLoader::Quilt;
    for (entry, project_id, file_id) in pending {
        let Some(dependencies) = dependencies_by_file_id.get(&file_id) else {
            continue;
        };
        let mut resolved_all = true;
        for dependency in dependencies {
            if !matches!(
                dependency.relation_type,
                crate::api::curseforge::DEPENDENCY_RELATION_REQUIRED
                    | crate::api::curseforge::DEPENDENCY_RELATION_INCLUDE
            ) {
                continue;
            }
            let dependency_project_id = if is_quilt
                && dependency.mod_id
                    == crate::api::curseforge::FABRIC_API_CURSEFORGE_PROJECT_ID
            {
                crate::api::curseforge::QUILTED_FABRIC_API_CURSEFORGE_PROJECT_ID
            } else {
                dependency.mod_id
            };
            let Some(child_entry) =
                content_rows::get_content_entry_by_provider_project(
                    &content_set.id,
                    ContentProvider::CurseForge,
                    &dependency_project_id.to_string(),
                    &state.pool,
                )
                .await?
            else {
                // Leave the entry unmarked so a later install of the missing
                // dependency is picked up by a future backfill pass.
                resolved_all = false;
                continue;
            };
            let child_release_id = refs_by_entry
                .get(&child_entry.id)
                .and_then(|references| {
                    references.iter().find_map(|reference| match reference {
                        ContentProviderRef::CurseForge {
                            file_id: Some(file_id),
                            ..
                        } => Some(file_id.get().to_string()),
                        _ => None,
                    })
                })
                .unwrap_or_default();
            let now = chrono::Utc::now();
            backfill.edges.push(
                crate::state::instances::ContentDependencyEdge {
                    id: format!("content-dependency:{}", uuid::Uuid::new_v4()),
                    content_set_id: content_set.id.clone(),
                    parent_entry_id: entry.id.clone(),
                    child_entry_id: child_entry.id,
                    evidence_provider: ContentProvider::CurseForge,
                    parent_provider: ContentProvider::CurseForge,
                    child_provider: ContentProvider::CurseForge,
                    dependency_kind: if dependency.relation_type
                        == crate::api::curseforge::DEPENDENCY_RELATION_INCLUDE
                    {
                        crate::state::instances::ContentDependencyKind::Include
                    } else {
                        crate::state::instances::ContentDependencyKind::Required
                    },
                    parent_project_id: project_id.to_string(),
                    parent_release_id: file_id.to_string(),
                    child_project_id: dependency_project_id.to_string(),
                    child_release_id,
                    created_at: now,
                    modified_at: now,
                },
            );
        }
        if resolved_all {
            backfill.backfilled_entry_ids.insert(entry.id.clone());
        }
    }
    Ok(())
}

/// Link locally identified files (no online provider refs) through the
/// dependency declarations embedded in their own metadata. Mod ids from
/// `fabric.mod.json`, `quilt.mod.json`, and Forge `mods.toml` are matched
/// against other locally identified files only, mirroring Prism Launcher's
/// local mod id matching.
async fn backfill_local_dependency_edges(
    entries: &[crate::state::instances::ContentEntry],
    refs_by_entry: &HashMap<String, Vec<ContentProviderRef>>,
    entries_with_edges: &HashSet<&str>,
    content_set: &crate::state::instances::ContentSet,
    local_metadata_by_entry: &HashMap<
        String,
        crate::mod_metadata::LocalModMetadata,
    >,
    backfill: &mut DependencyBackfill,
) -> crate::Result<()> {
    let local_entry_ids = entries
        .iter()
        .filter(|entry| !refs_by_entry.contains_key(&entry.id))
        .map(|entry| entry.id.as_str())
        .collect::<HashSet<_>>();
    if local_entry_ids.is_empty() {
        return Ok(());
    }

    let mut entry_id_by_mod_id = HashMap::new();
    for entry in entries {
        if !local_entry_ids.contains(entry.id.as_str()) {
            continue;
        }
        let Some(metadata) = local_metadata_by_entry.get(&entry.id) else {
            continue;
        };
        entry_id_by_mod_id
            .entry(metadata.mod_id.as_str())
            .or_insert(entry.id.as_str());
    }

    for entry in entries {
        if !local_entry_ids.contains(entry.id.as_str())
            || entries_with_edges.contains(entry.id.as_str())
        {
            continue;
        }
        let Some(metadata) = local_metadata_by_entry.get(&entry.id) else {
            continue;
        };
        let Some(dependencies) = metadata.dependencies.as_ref() else {
            continue;
        };
        for dependency in dependencies {
            let Some(child_entry_id) =
                entry_id_by_mod_id.get(dependency.mod_id.as_str())
            else {
                continue;
            };
            if *child_entry_id == entry.id {
                continue;
            }
            let Some(child_metadata) =
                local_metadata_by_entry.get(*child_entry_id)
            else {
                continue;
            };
            let now = chrono::Utc::now();
            backfill.edges.push(crate::state::instances::ContentDependencyEdge {
                id: format!("content-dependency:{}", uuid::Uuid::new_v4()),
                content_set_id: content_set.id.clone(),
                parent_entry_id: entry.id.clone(),
                child_entry_id: (*child_entry_id).to_string(),
                evidence_provider: ContentProvider::Local,
                parent_provider: ContentProvider::Local,
                child_provider: ContentProvider::Local,
                dependency_kind: crate::state::instances::ContentDependencyKind::Required,
                parent_project_id: format!("local:{}", metadata.mod_id),
                parent_release_id: metadata.version.clone().unwrap_or_default(),
                child_project_id: format!("local:{}", child_metadata.mod_id),
                child_release_id: child_metadata.version.clone().unwrap_or_default(),
                created_at: now,
                modified_at: now,
            });
        }
    }
    Ok(())
}

async fn attach_dependency_info(
    items: &mut [InstanceContentSnapshotItem],
    entries: &[crate::state::instances::ContentEntry],
    content_set_id: &str,
    state: &State,
) -> crate::Result<()> {
    let edges =
        content_rows::get_content_dependency_edges(content_set_id, &state.pool)
            .await?;
    let mut required_by: HashMap<String, Vec<ContentDependencyRef>> =
        HashMap::new();
    let mut requires: HashMap<String, Vec<ContentDependencyRef>> =
        HashMap::new();
    for edge in &edges {
        required_by
            .entry(edge.child_entry_id.clone())
            .or_default()
            .push(ContentDependencyRef {
                provider: edge.parent_provider,
                project_id: edge.parent_project_id.clone(),
                release_id: edge.parent_release_id.clone(),
            });
        requires
            .entry(edge.parent_entry_id.clone())
            .or_default()
            .push(ContentDependencyRef {
                provider: edge.child_provider,
                project_id: edge.child_project_id.clone(),
                release_id: edge.child_release_id.clone(),
            });
    }

    let entry_by_id = entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<HashMap<_, _>>();
    for item in items {
        let Some(entry_id) = item.entry_id.as_deref() else {
            continue;
        };
        let Some(entry) = entry_by_id.get(entry_id) else {
            continue;
        };
        let required_by = required_by.remove(entry_id).unwrap_or_default();
        let requires = requires.remove(entry_id).unwrap_or_default();
        if !entry.auto_dependency
            && required_by.is_empty()
            && requires.is_empty()
        {
            continue;
        }
        item.dependency = Some(ContentDependencyInfo {
            auto_dependency: entry.auto_dependency,
            orphaned: entry.auto_dependency && required_by.is_empty(),
            required_by,
            requires,
        });
    }
    Ok(())
}

fn snapshot_item(
    file_id: Option<String>,
    entry_id: Option<String>,
    member: Option<&crate::state::instances::PackMember>,
    ownership_kind: ContentOwnershipKind,
    project_type: crate::state::ProjectType,
    expected_relative_path: String,
    content: Option<ContentItem>,
) -> InstanceContentSnapshotItem {
    let provider = member.and_then(|member| member.provider).or_else(|| {
        content.as_ref().and_then(|content| content.origin_provider)
    });
    let provider_project_id = member
        .and_then(|member| member.provider_project_id.clone())
        .or_else(|| {
            content.as_ref().and_then(|content| {
                content
                    .provider_refs
                    .first()
                    .map(ContentProviderRef::database_project_id)
            })
        });
    let provider_release_id = member
        .and_then(|member| member.provider_release_id.clone())
        .or_else(|| {
            content.as_ref().and_then(|content| {
                content
                    .provider_refs
                    .first()
                    .and_then(ContentProviderRef::database_release_id)
            })
        });
    let stored_materialization_state = member
        .map_or(PackMemberMaterializationState::Present, |member| {
            member.materialization_state
        });
    let materialization_state = snapshot_materialization_state(
        stored_materialization_state,
        content.is_some(),
    );
    let override_kind = member
        .map_or(PackMemberOverrideKind::None, |member| member.override_kind);
    let present = content.is_some()
        && materialization_state == PackMemberMaterializationState::Present;
    let has_provider = content
        .as_ref()
        .is_some_and(|content| !content.provider_refs.is_empty());

    InstanceContentSnapshotItem {
        file_id,
        entry_id,
        member_id: member.map(|member| member.id.clone()),
        ownership_kind,
        materialization_state,
        override_kind,
        expected_relative_path,
        required: member.is_none_or(|member| member.required),
        project_type,
        provider,
        provider_project_id,
        provider_release_id,
        capabilities: ContentItemCapabilities {
            can_toggle: present,
            can_delete: present,
            can_update: present && has_provider,
            can_change_version: present && has_provider,
            can_restore_pack_default: member.is_some()
                && (override_kind != PackMemberOverrideKind::None
                    || materialization_state
                        == PackMemberMaterializationState::Missing),
        },
        content,
        dependency: None,
    }
}

fn snapshot_materialization_state(
    stored: PackMemberMaterializationState,
    content_is_present: bool,
) -> PackMemberMaterializationState {
    if content_is_present && stored == PackMemberMaterializationState::Missing {
        PackMemberMaterializationState::Present
    } else {
        stored
    }
}

fn pack_snapshot(
    instance_name: &str,
    icon_path: Option<String>,
    link: &InstanceLink,
    members: &[crate::state::instances::PackMember],
    metadata: Option<crate::state::LinkedModpackInfo>,
) -> Option<InstanceContentPack> {
    let (provider, project_id, version_id, fallback_name) = match link {
        InstanceLink::ModrinthModpack {
            project_id,
            version_id,
        }
        | InstanceLink::ServerProjectModpack {
            content_project_id: project_id,
            content_version_id: version_id,
            ..
        } => (
            Some(ContentProvider::Modrinth),
            Some(project_id.clone()),
            Some(version_id.clone()),
            instance_name.to_string(),
        ),
        InstanceLink::CurseForgeModpack {
            project_id,
            version_id,
        } => (
            Some(ContentProvider::CurseForge),
            Some(project_id.clone()),
            Some(version_id.clone()),
            instance_name.to_string(),
        ),
        InstanceLink::ImportedModpack { name, .. } => (
            None,
            None,
            None,
            name.clone().unwrap_or_else(|| instance_name.to_string()),
        ),
        _ if members.is_empty() => return None,
        _ => (None, None, None, instance_name.to_string()),
    };
    let reconciled = members.iter().all(|member| member.reconciled);
    let name = metadata
        .as_ref()
        .map(|metadata| metadata.project.title.clone())
        .unwrap_or(fallback_name);

    Some(InstanceContentPack {
        name,
        icon_path,
        provider,
        project_id,
        version_id,
        reconciled,
        can_update: provider.is_some() && reconciled,
        metadata,
    })
}

fn pack_provider(link: &InstanceLink) -> Option<ContentProvider> {
    match link {
        InstanceLink::CurseForgeModpack { .. } => {
            Some(ContentProvider::CurseForge)
        }
        InstanceLink::ModrinthModpack { .. }
        | InstanceLink::ServerProjectModpack { .. } => {
            Some(ContentProvider::Modrinth)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_content_does_not_keep_a_stale_missing_state() {
        assert_eq!(
            snapshot_materialization_state(
                PackMemberMaterializationState::Missing,
                true,
            ),
            PackMemberMaterializationState::Present,
        );
        assert_eq!(
            snapshot_materialization_state(
                PackMemberMaterializationState::Missing,
                false,
            ),
            PackMemberMaterializationState::Missing,
        );
    }
}
