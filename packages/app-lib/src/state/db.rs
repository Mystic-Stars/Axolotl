use crate::state::DirectoryInfo;
use sha2::{Digest, Sha384};
use sqlx::migrate::{Migration, Migrator};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions,
};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::time::Duration;

static MIGRATOR: Migrator = sqlx::migrate!();

const INITIAL_MIGRATION_VERSION: i64 = 20240711194701;
const COLLIDING_JAVA_DISCOVERY_MIGRATION_VERSION: i64 = 20260722120000;
const JAVA_DISCOVERY_MIGRATION_VERSION: i64 = 20260722120001;
const TEMPORARY_JAVA_DISCOVERY_MIGRATION_VERSION: i64 = 20260723121000;

// This migration was changed by the launcher rebrand after it had already
// shipped. Keep the checksums of the original LF and CRLF variants so existing
// installations can move to the current canonical migration without losing
// their database.
const LEGACY_INITIAL_MIGRATION_CHECKSUMS: &[&str] = &[
    "49364b3e1b0d0169579ed93eb1f8e215216b84300a816891d0d922d3e03c69101e17e2bbe91ac1f54234c77cbd6b8bc3",
    "d95bfef1c3b2b530d2efd810202c85f93a9342ab40497b15653eea9b129806333cf610eebcecfa91accaa53a14bfc5df",
];
const COLLIDING_JAVA_DISCOVERY_MIGRATION_CHECKSUMS: &[&str] = &[
    "986c9afb410ad7086617c3707611c3b9a46be69bc33e2a0bd1b32611266301f536e28137a47b11337622a953c29ad595",
    "bfb8686214294786f8e81ea05f06bb08deeb4183da3d1230ebf379bc2ba9f5c5521f3590306bea668c292e03c3aacd85",
];
const TEMPORARY_JAVA_DISCOVERY_MIGRATION_CHECKSUMS: &[&str] = &[
    "35cbd4e0a4528bee302f06000e0971aad2f575488ebb5c04ec6849e15efc6f3f996395c11f9cc431c62ba4d9e3a41cc3",
    "7b99e048d7eb88cbfdd913cc3d799c6acabd58142204cde336da593fa3ca6d4f44336fd5d42a5822ab1e0b485352eb9b",
];

pub(crate) async fn connect(
    app_identifier: &str,
) -> crate::Result<Pool<Sqlite>> {
    let settings_dir = DirectoryInfo::initial_settings_dir_path(app_identifier)
        .ok_or(crate::ErrorKind::FSError(
            "Could not find valid config dir".to_string(),
        ))?;

    crate::util::io::create_dir_all(&settings_dir).await?;

    let db_path = settings_dir.join("app.db");

    connect_app_db(&db_path).await
}

async fn connect_app_db(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
    super::db_backup::maybe_backup_existing_app_db(db_path).await?;
    open_migrated_app_db(db_path).await
}

async fn open_migrated_app_db(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
    let pool = open_app_db_pool(db_path).await?;

    if let Err(err) = stale_data_cleanup(&pool).await {
        tracing::warn!(
            "Failed to clean up stale data from state database before migrations: {err}"
        );
    }

    reconcile_compatible_migration_checksums(&pool).await?;
    reconcile_existing_java_discovery_migration(&pool).await?;
    MIGRATOR.run(&pool).await?;
    record_current_app_version(&pool).await?;

    if let Err(err) = stale_data_cleanup(&pool).await {
        tracing::warn!(
            "Failed to clean up stale data from state database: {err}"
        );
    }

    Ok(pool)
}

/// Reconciles historical migration checksums that differ only because of line
/// endings, plus the known pre-rebrand form of the initial migration.
///
/// SQLx hashes the raw migration bytes, so an otherwise identical migration
/// built with LF, CRLF, or mixed line endings receives a different checksum.
/// Unknown checksums are deliberately left untouched for SQLx to reject.
async fn reconcile_compatible_migration_checksums(
    pool: &Pool<Sqlite>,
) -> crate::Result<()> {
    let has_migrations_table: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')",
    )
    .fetch_one(pool)
    .await?;

    if !has_migrations_table {
        return Ok(());
    }

    let applied_migrations: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations")
            .fetch_all(pool)
            .await?;

    for (version, applied_checksum) in applied_migrations {
        let Some(migration) = MIGRATOR
            .iter()
            .find(|migration| migration.version == version)
        else {
            continue;
        };
        let current_checksum: &[u8] = migration.checksum.as_ref();

        if applied_checksum.as_slice() == current_checksum {
            continue;
        }

        if version == COLLIDING_JAVA_DISCOVERY_MIGRATION_VERSION
            && COLLIDING_JAVA_DISCOVERY_MIGRATION_CHECKSUMS
                .contains(&checksum_as_hex(&applied_checksum).as_str())
        {
            reconcile_colliding_java_discovery_migration(pool).await?;
            update_migration_checksum(pool, version, current_checksum).await?;
            tracing::warn!(
                version,
                "Reconciled colliding Java discovery migration version"
            );
            continue;
        }

        if !is_compatible_migration_checksum(
            version,
            &applied_checksum,
            migration,
        ) {
            continue;
        }

        update_migration_checksum(pool, version, current_checksum).await?;

        tracing::warn!(
            version,
            "Reconciled a compatible historical migration checksum"
        );
    }

    Ok(())
}

async fn update_migration_checksum(
    pool: &Pool<Sqlite>,
    version: i64,
    checksum: &[u8],
) -> crate::Result<()> {
    sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
        .bind(checksum)
        .bind(version)
        .execute(pool)
        .await?;
    Ok(())
}

async fn reconcile_colliding_java_discovery_migration(
    pool: &Pool<Sqlite>,
) -> crate::Result<()> {
    let onboarding_version_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('settings') WHERE name = 'onboarding_version')",
    )
    .fetch_one(pool)
    .await?;
    if !onboarding_version_exists {
        sqlx::query(
            "ALTER TABLE settings ADD COLUMN onboarding_version INTEGER NOT NULL DEFAULT 0",
        )
        .execute(pool)
        .await?;
    }

    let instance_tour_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('settings') WHERE name = 'onboarding_instance_tour_completed')",
    )
    .fetch_one(pool)
    .await?;
    if !instance_tour_exists {
        sqlx::query(
            "ALTER TABLE settings ADD COLUMN onboarding_instance_tour_completed INTEGER NOT NULL DEFAULT TRUE",
        )
        .execute(pool)
        .await?;
        sqlx::query(
            "UPDATE settings SET onboarding_instance_tour_completed = CASE WHEN onboarded = 1 THEN TRUE ELSE FALSE END",
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn reconcile_existing_java_discovery_migration(
    pool: &Pool<Sqlite>,
) -> crate::Result<()> {
    let has_migrations_table: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')",
    )
    .fetch_one(pool)
    .await?;
    if !has_migrations_table {
        return Ok(());
    }

    let columns: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT name, type, \"notnull\", pk FROM pragma_table_info('discovered_javas') ORDER BY cid",
    )
    .fetch_all(pool)
    .await?;
    let expected_columns = [
        ("path", "TEXT", 1, 1),
        ("major_version", "INTEGER", 1, 0),
        ("full_version", "TEXT", 1, 0),
        ("architecture", "TEXT", 1, 0),
        ("file_size", "INTEGER", 1, 0),
        ("file_mtime_ms", "INTEGER", 1, 0),
    ];
    let schema_matches = columns.len() == expected_columns.len()
        && columns.iter().zip(expected_columns).all(
            |((name, data_type, not_null, primary_key), expected)| {
                name == expected.0
                    && data_type.eq_ignore_ascii_case(expected.1)
                    && *not_null == expected.2
                    && *primary_key == expected.3
            },
        );
    if !schema_matches {
        return Ok(());
    }

    let migration = MIGRATOR
        .iter()
        .find(|migration| migration.version == JAVA_DISCOVERY_MIGRATION_VERSION)
        .expect("Java discovery migration should be embedded");
    let canonical_applied: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = ?)",
    )
    .bind(JAVA_DISCOVERY_MIGRATION_VERSION)
    .fetch_one(pool)
    .await?;
    let temporary_checksum: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT checksum FROM _sqlx_migrations WHERE version = ?",
    )
    .bind(TEMPORARY_JAVA_DISCOVERY_MIGRATION_VERSION)
    .fetch_optional(pool)
    .await?;
    let temporary_is_known =
        temporary_checksum.as_ref().is_some_and(|checksum| {
            TEMPORARY_JAVA_DISCOVERY_MIGRATION_CHECKSUMS
                .contains(&checksum_as_hex(checksum).as_str())
        });

    let mut transaction = pool.begin().await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS discovered_javas_major_version ON discovered_javas (major_version)",
    )
    .execute(&mut *transaction)
    .await?;
    if !canonical_applied {
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (?, ?, TRUE, ?, 0)",
        )
        .bind(migration.version)
        .bind(migration.description.as_ref())
        .bind(migration.checksum.as_ref())
        .execute(&mut *transaction)
        .await?;
    }
    if temporary_is_known {
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = ?")
            .bind(TEMPORARY_JAVA_DISCOVERY_MIGRATION_VERSION)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;

    tracing::warn!(
        version = JAVA_DISCOVERY_MIGRATION_VERSION,
        removed_temporary_version = temporary_is_known,
        "Reconciled existing Java discovery table with canonical migration"
    );
    Ok(())
}

fn is_compatible_migration_checksum(
    version: i64,
    applied_checksum: &[u8],
    migration: &Migration,
) -> bool {
    let normalized_lf = migration.sql.replace("\r\n", "\n").replace('\r', "\n");
    let normalized_crlf = normalized_lf.replace('\n', "\r\n");

    if checksum_matches(applied_checksum, normalized_lf.as_bytes())
        || checksum_matches(applied_checksum, normalized_crlf.as_bytes())
    {
        return true;
    }

    version == INITIAL_MIGRATION_VERSION
        && LEGACY_INITIAL_MIGRATION_CHECKSUMS
            .contains(&checksum_as_hex(applied_checksum).as_str())
}

fn checksum_matches(checksum: &[u8], contents: &[u8]) -> bool {
    let calculated: [u8; 48] = Sha384::digest(contents).into();
    checksum == calculated
}

fn checksum_as_hex(checksum: &[u8]) -> String {
    use std::fmt::Write;

    checksum.iter().fold(
        String::with_capacity(checksum.len() * 2),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    )
}

async fn open_app_db_pool(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
    let conn_options = SqliteConnectOptions::new()
        .filename(db_path)
        .busy_timeout(Duration::from_secs(30))
        .journal_mode(SqliteJournalMode::Wal)
        .optimize_on_close(true, None)
        .create_if_missing(true);

    Ok(SqlitePoolOptions::new()
        .max_connections(100)
        .connect_with(conn_options)
        .await?)
}

async fn record_current_app_version(pool: &Pool<Sqlite>) -> crate::Result<()> {
    sqlx::query!(
        "
		INSERT INTO app_metadata (key, value, updated_at)
		VALUES ('app_version', ?, unixepoch())
		ON CONFLICT(key) DO UPDATE SET
			value = excluded.value,
			updated_at = excluded.updated_at
		",
        env!("CARGO_PKG_VERSION"),
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cleans up data from the database that is no longer referenced, but must be
/// kept around for a little while to allow users to recover from accidental
/// deletions.
async fn stale_data_cleanup(pool: &Pool<Sqlite>) -> crate::Result<()> {
    let mut tx = pool.begin().await?;

    let has_skin_tables = sqlx::query!(
		"SELECT COUNT(*) AS \"count!: i64\" FROM sqlite_master WHERE type = 'table' AND name IN ('custom_minecraft_skins', 'minecraft_users')",
	)
	.fetch_one(&mut *tx)
	.await?
	.count == 2;

    if has_skin_tables {
        sqlx::query!(
			"DELETE FROM custom_minecraft_skins WHERE minecraft_user_uuid NOT IN (SELECT uuid FROM minecraft_users)"
		)
		.execute(&mut *tx)
		.await?;
    }

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn initial_migration() -> &'static Migration {
        MIGRATOR
            .iter()
            .find(|migration| migration.version == INITIAL_MIGRATION_VERSION)
            .expect("initial migration should be embedded")
    }

    fn checksum(contents: &[u8]) -> Vec<u8> {
        Sha384::digest(contents).to_vec()
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("ASCII hex");
                u8::from_str_radix(pair, 16).expect("valid hex")
            })
            .collect()
    }

    #[test]
    fn accepts_lf_and_crlf_variants_of_the_same_migration() {
        let migration = initial_migration();
        let lf = migration.sql.replace("\r\n", "\n").replace('\r', "\n");
        let crlf = lf.replace('\n', "\r\n");

        assert!(is_compatible_migration_checksum(
            migration.version,
            &checksum(lf.as_bytes()),
            migration,
        ));
        assert!(is_compatible_migration_checksum(
            migration.version,
            &checksum(crlf.as_bytes()),
            migration,
        ));
    }

    #[test]
    fn accepts_only_the_known_legacy_initial_migration() {
        let migration = initial_migration();
        let legacy_checksum = decode_hex(LEGACY_INITIAL_MIGRATION_CHECKSUMS[0]);

        assert!(is_compatible_migration_checksum(
            INITIAL_MIGRATION_VERSION,
            &legacy_checksum,
            migration,
        ));
        assert!(!is_compatible_migration_checksum(
            INITIAL_MIGRATION_VERSION + 1,
            &legacy_checksum,
            migration,
        ));
    }

    #[test]
    fn rejects_an_unknown_content_change() {
        let migration = initial_migration();
        let changed_checksum = checksum(
            format!("{}\n-- unknown change", migration.sql).as_bytes(),
        );

        assert!(!is_compatible_migration_checksum(
            migration.version,
            &changed_checksum,
            migration,
        ));
    }

    #[test]
    fn embedded_migration_versions_are_unique() {
        let mut versions = HashSet::new();
        for migration in MIGRATOR.iter() {
            assert!(
                versions.insert(migration.version),
                "duplicate migration version {}",
                migration.version
            );
        }
    }

    #[tokio::test]
    async fn repairs_schema_from_colliding_java_discovery_migration() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE settings (onboarded INTEGER NOT NULL DEFAULT 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO settings (onboarded) VALUES (0), (1)")
            .execute(&pool)
            .await
            .unwrap();

        reconcile_colliding_java_discovery_migration(&pool)
            .await
            .unwrap();

        let values: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT onboarding_version, onboarding_instance_tour_completed FROM settings ORDER BY onboarded",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(values, vec![(0, 0), (0, 1)]);
    }

    #[tokio::test]
    async fn claims_existing_java_table_for_canonical_migration() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "
            CREATE TABLE _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            )
            ",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "
            CREATE TABLE discovered_javas (
                path TEXT NOT NULL PRIMARY KEY,
                major_version INTEGER NOT NULL,
                full_version TEXT NOT NULL,
                architecture TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                file_mtime_ms INTEGER NOT NULL
            )
            ",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (?, 'temporary', TRUE, ?, 0)",
        )
        .bind(TEMPORARY_JAVA_DISCOVERY_MIGRATION_VERSION)
        .bind(decode_hex(
            TEMPORARY_JAVA_DISCOVERY_MIGRATION_CHECKSUMS[0],
        ))
        .execute(&pool)
        .await
        .unwrap();

        reconcile_existing_java_discovery_migration(&pool)
            .await
            .unwrap();

        let migration = MIGRATOR
            .iter()
            .find(|migration| {
                migration.version == JAVA_DISCOVERY_MIGRATION_VERSION
            })
            .unwrap();
        let canonical_checksum: Vec<u8> = sqlx::query_scalar(
            "SELECT checksum FROM _sqlx_migrations WHERE version = ?",
        )
        .bind(JAVA_DISCOVERY_MIGRATION_VERSION)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(canonical_checksum, migration.checksum.as_ref());
        let temporary_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = ?)",
        )
        .bind(TEMPORARY_JAVA_DISCOVERY_MIGRATION_VERSION)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!temporary_exists);
        let index_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'discovered_javas_major_version')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(index_exists);
    }

    #[tokio::test]
    async fn does_not_claim_incompatible_java_table() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "
            CREATE TABLE _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            )
            ",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE TABLE discovered_javas (path TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();

        reconcile_existing_java_discovery_migration(&pool)
            .await
            .unwrap();

        let canonical_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = ?)",
        )
        .bind(JAVA_DISCOVERY_MIGRATION_VERSION)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!canonical_exists);
    }
}
