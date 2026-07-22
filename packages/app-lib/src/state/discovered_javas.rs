use std::path::Path;
use std::time::UNIX_EPOCH;

use sqlx::Row;

use super::JavaVersion;

/// A Java installation found by a system scan, cached together with the
/// file signature of its executable so staleness can be detected cheaply.
#[derive(Debug, Clone)]
pub struct DiscoveredJava {
    pub java: JavaVersion,
    pub file_size: i64,
    pub file_mtime_ms: i64,
}

/// Returns (size, mtime in milliseconds) of the file at `path`, or None
/// if it does not exist or cannot be read.
pub fn java_file_signature(path: &Path) -> Option<(i64, i64)> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let mtime_ms = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis() as i64;
    Some((metadata.len() as i64, mtime_ms))
}

impl DiscoveredJava {
    /// Builds a cache entry for a verified Java installation, stamping the
    /// current signature of its executable. Returns None if the executable
    /// can no longer be read.
    pub fn from_java(java: JavaVersion) -> Option<Self> {
        let (file_size, file_mtime_ms) =
            java_file_signature(Path::new(&java.path))?;
        Some(Self {
            java,
            file_size,
            file_mtime_ms,
        })
    }

    pub async fn get_all(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Vec<Self>> {
        let rows = sqlx::query(
            "
            SELECT path, major_version, full_version, architecture,
                file_size, file_mtime_ms
            FROM discovered_javas
            ORDER BY major_version, path
            ",
        )
        .fetch_all(exec)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Self {
                java: JavaVersion {
                    parsed_version: row.get::<i64, _>("major_version") as u32,
                    version: row.get("full_version"),
                    architecture: row.get("architecture"),
                    path: row.get("path"),
                },
                file_size: row.get("file_size"),
                file_mtime_ms: row.get("file_mtime_ms"),
            })
            .collect())
    }

    pub async fn upsert(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query(
            "
            INSERT INTO discovered_javas (
                path, major_version, full_version, architecture,
                file_size, file_mtime_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (path) DO UPDATE SET
                major_version = $2,
                full_version = $3,
                architecture = $4,
                file_size = $5,
                file_mtime_ms = $6
            ",
        )
        .bind(&self.java.path)
        .bind(self.java.parsed_version as i64)
        .bind(&self.java.version)
        .bind(&self.java.architecture)
        .bind(self.file_size)
        .bind(self.file_mtime_ms)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn remove(
        path: &str,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query("DELETE FROM discovered_javas WHERE path = $1")
            .bind(path)
            .execute(exec)
            .await?;

        Ok(())
    }

    /// Replaces the entire cache with the results of a fresh scan.
    pub async fn replace_all(
        pool: &sqlx::SqlitePool,
        entries: &[Self],
    ) -> crate::Result<()> {
        let mut transaction = pool.begin().await?;

        sqlx::query("DELETE FROM discovered_javas")
            .execute(&mut *transaction)
            .await?;

        for entry in entries {
            sqlx::query(
                "
                INSERT INTO discovered_javas (
                    path, major_version, full_version, architecture,
                    file_size, file_mtime_ms
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (path) DO UPDATE SET
                    major_version = $2,
                    full_version = $3,
                    architecture = $4,
                    file_size = $5,
                    file_mtime_ms = $6
                ",
            )
            .bind(&entry.java.path)
            .bind(entry.java.parsed_version as i64)
            .bind(&entry.java.version)
            .bind(&entry.java.architecture)
            .bind(entry.file_size)
            .bind(entry.file_mtime_ms)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}
