use std::{collections::HashMap, path::PathBuf};

use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::{
    State,
    api::pack::{
        import::finish_import,
        install_from::{self, CreatePackDescription, PackDependency},
    },
    install::{InstallPhaseDetails, InstallProgressReporter},
};

async fn open_source_db(
    base_path: &PathBuf,
) -> crate::Result<sqlx::SqlitePool> {
    let db_path = base_path.join("app.db");
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        .create_if_missing(false);

    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}

async fn source_config_dir(
    base_path: &PathBuf,
    pool: &sqlx::SqlitePool,
) -> crate::Result<PathBuf> {
    let custom_dir: Option<String> =
        sqlx::query_scalar("SELECT custom_dir FROM settings WHERE id = 0")
            .fetch_optional(pool)
            .await?
            .flatten();
    Ok(custom_dir.map_or_else(|| base_path.clone(), PathBuf::from))
}

async fn has_table(
    pool: &sqlx::SqlitePool,
    table: &str,
) -> crate::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind(table)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

pub async fn get_importable_instances_with_paths(
    base_path: PathBuf,
) -> crate::Result<Vec<(String, PathBuf)>> {
    let pool = open_source_db(&base_path).await?;
    let config_dir = source_config_dir(&base_path, &pool).await?;
    let profiles_dir = config_dir.join("profiles");
    let rows = if has_table(&pool, "instances").await? {
        sqlx::query("SELECT path FROM instances ORDER BY name COLLATE NOCASE")
            .fetch_all(&pool)
            .await?
    } else {
        sqlx::query("SELECT path FROM profiles ORDER BY name COLLATE NOCASE")
            .fetch_all(&pool)
            .await?
    };

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let path = row.try_get::<String, _>("path").ok()?;
            let full = profiles_dir.join(&path);
            full.is_dir().then_some((path, full))
        })
        .collect())
}

pub async fn get_importable_instances(
    base_path: PathBuf,
) -> crate::Result<Vec<String>> {
    get_importable_instances_with_paths(base_path)
        .await
        .map(|v| v.into_iter().map(|(n, _)| n).collect())
}

fn dependencies(
    game_version: String,
    loader: String,
    loader_version: Option<String>,
) -> HashMap<PackDependency, String> {
    let mut dependencies =
        HashMap::from([(PackDependency::Minecraft, game_version)]);
    if let Some(loader_version) = loader_version {
        let dependency = match loader.as_str() {
            "fabric" => Some(PackDependency::FabricLoader),
            "forge" => Some(PackDependency::Forge),
            "neoforge" | "neo_forge" => Some(PackDependency::NeoForge),
            "quilt" => Some(PackDependency::QuiltLoader),
            _ => None,
        };
        if let Some(dependency) = dependency {
            dependencies.insert(dependency, loader_version);
        }
    }
    dependencies
}

pub async fn import_instance(
    base_path: PathBuf,
    instance_path: String,
    instance_id: &str,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let pool = open_source_db(&base_path).await?;
    let config_dir = source_config_dir(&base_path, &pool).await?;
    let source = config_dir.join("profiles").join(&instance_path);

    let (name, game_version, loader, loader_version, icon_path) = if has_table(
        &pool,
        "instances",
    )
    .await?
    {
        let row = sqlx::query(
                "SELECT i.name, s.game_version, s.loader, s.loader_version, i.icon_path \
                 FROM instances i JOIN instance_content_sets s \
                 ON s.id = i.applied_content_set_id WHERE i.path = ?",
            )
            .bind(&instance_path)
            .fetch_one(&pool)
            .await?;
        (
            row.try_get("name")?,
            row.try_get("game_version")?,
            row.try_get("loader")?,
            row.try_get("loader_version")?,
            row.try_get::<Option<String>, _>("icon_path")?,
        )
    } else {
        let row = sqlx::query(
            "SELECT name, game_version, mod_loader AS loader, \
                 mod_loader_version AS loader_version, icon_path \
                 FROM profiles WHERE path = ?",
        )
        .bind(&instance_path)
        .fetch_one(&pool)
        .await?;
        (
            row.try_get("name")?,
            row.try_get("game_version")?,
            row.try_get("loader")?,
            row.try_get("loader_version")?,
            row.try_get::<Option<String>, _>("icon_path")?,
        )
    };

    let icon = match icon_path {
        Some(path) => super::recache_icon(config_dir.join(path)).await?,
        None => None,
    };
    let description = CreatePackDescription {
        icon,
        override_title: Some(name),
        project_id: None,
        version_id: None,
        instance_id: instance_id.to_string(),
        source_filename: None,
    };
    install_from::set_instance_information(
        instance_id.to_string(),
        &description,
        "Imported from Modrinth source installation",
        None,
        &dependencies(game_version, loader, loader_version),
        false,
    )
    .await?;

    let state = State::get().await?;
    finish_import(instance_id, source, &state.io_semaphore, reporter, details)
        .await
}
