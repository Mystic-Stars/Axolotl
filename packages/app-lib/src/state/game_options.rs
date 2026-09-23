use super::{CanonicalValue, GameOptionKind, StoredOption, StoredPreference};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

pub(crate) async fn shared_game_options_exist(
    pool: &SqlitePool,
) -> crate::Result<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM synced_game_option_state WHERE singleton = 1)").fetch_one(pool).await?)
}

pub(crate) async fn game_options_sync_is_enabled(
    pool: &SqlitePool,
) -> crate::Result<bool> {
    Ok(sqlx::query_scalar("SELECT globally_enabled FROM sync_feature_settings WHERE feature = 'game_options'").fetch_optional(pool).await?.unwrap_or(false))
}

pub(crate) async fn load_game_options_sync_state(
    pool: &SqlitePool,
    current_catalog_revision: u32,
) -> crate::Result<(u64, u32)> {
    let row = sqlx::query("SELECT revision, catalog_revision FROM synced_game_option_state WHERE singleton = 1").fetch_optional(pool).await?;
    Ok(row
        .map(|r| {
            (
                r.get::<i64, _>("revision").max(0) as u64,
                (r.get::<i64, _>("catalog_revision").max(1) as u32)
                    .max(current_catalog_revision),
            )
        })
        .unwrap_or((0, current_catalog_revision)))
}

pub(crate) async fn load_shared_game_options(
    pool: &SqlitePool,
) -> crate::Result<HashMap<String, StoredOption>> {
    let rows = sqlx::query("SELECT option_id, kind, raw_key, canonical_value_json, seeded, revision FROM synced_game_option_values").fetch_all(pool).await?;
    let mut out = HashMap::with_capacity(rows.len());
    for row in rows {
        let option_id: String = row.get("option_id");
        let kind: String = row.get("kind");
        let raw_key: Option<String> = row.try_get("raw_key")?;
        let json: Option<String> = row.try_get("canonical_value_json")?;
        let seeded: bool = row.get("seeded");
        let revision: i64 = row.get("revision");
        out.insert(
            option_id.clone(),
            StoredOption {
                option_id,
                kind: if kind == "external" {
                    GameOptionKind::External
                } else {
                    GameOptionKind::Vanilla
                },
                raw_key,
                value: json.as_deref().map(serde_json::from_str).transpose()?,
                seeded,
                revision: revision.max(0) as u64,
            },
        );
    }
    Ok(out)
}

pub(crate) async fn load_game_option_preferences(
    pool: &SqlitePool,
) -> crate::Result<HashMap<String, StoredPreference>> {
    Ok(sqlx::query("SELECT option_id, enabled, revision FROM synced_game_option_preferences").fetch_all(pool).await?.into_iter().map(|r| { let option_id: String = r.get("option_id"); let enabled: bool = r.get("enabled"); let revision: i64 = r.get("revision"); (option_id, StoredPreference { enabled, revision: revision.max(0) as u64 }) }).collect())
}
