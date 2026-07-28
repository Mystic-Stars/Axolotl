ALTER TABLE instance_content_provider_refs
	RENAME TO instance_content_provider_refs_legacy;

ALTER TABLE instance_content_update_checks
	RENAME TO instance_content_update_checks_legacy;

ALTER TABLE instance_content_entries
	RENAME TO instance_content_entries_legacy;

-- SQLite keeps index names unchanged when a table is renamed. Drop the
-- legacy indexes before creating their provider-qualified replacements;
-- otherwise the new CREATE INDEX statements fail with "already exists".
DROP INDEX IF EXISTS instance_content_entries_instance_id;
DROP INDEX IF EXISTS instance_content_entries_content_set_id;
DROP INDEX IF EXISTS instance_content_entries_file_id;
DROP INDEX IF EXISTS instance_content_entries_project_id;
DROP INDEX IF EXISTS instance_content_entries_version_id;
DROP INDEX IF EXISTS instance_content_entries_source_kind;
DROP INDEX IF EXISTS instance_content_update_checks_update_version_id;
DROP INDEX IF EXISTS instance_content_provider_refs_project;
DROP INDEX IF EXISTS instance_content_provider_refs_version;
DROP INDEX IF EXISTS instance_content_provider_refs_primary;

CREATE TABLE instance_content_entries (
	id TEXT NOT NULL,
	instance_id TEXT NOT NULL,
	content_set_id TEXT NOT NULL,
	file_id TEXT NULL,

	project_type TEXT NOT NULL,
	source_kind TEXT NOT NULL,
	server_requirement TEXT NOT NULL,
	client_requirement TEXT NOT NULL,
	enabled INTEGER NOT NULL DEFAULT 1,

	added_at INTEGER NOT NULL,
	modified_at INTEGER NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE,
	FOREIGN KEY (content_set_id)
		REFERENCES instance_content_sets(id)
		ON DELETE CASCADE,
	FOREIGN KEY (file_id) REFERENCES instance_files(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS instance_content_entries_instance_id
	ON instance_content_entries(instance_id);
CREATE INDEX IF NOT EXISTS instance_content_entries_content_set_id
	ON instance_content_entries(content_set_id);
CREATE INDEX IF NOT EXISTS instance_content_entries_file_id
	ON instance_content_entries(file_id);
CREATE INDEX IF NOT EXISTS instance_content_entries_source_kind
	ON instance_content_entries(source_kind);

INSERT INTO instance_content_entries (
	id,
	instance_id,
	content_set_id,
	file_id,
	project_type,
	source_kind,
	server_requirement,
	client_requirement,
	enabled,
	added_at,
	modified_at
)
SELECT
	id,
	instance_id,
	content_set_id,
	file_id,
	project_type,
	source_kind,
	server_requirement,
	client_requirement,
	enabled,
	added_at,
	modified_at
FROM instance_content_entries_legacy;

CREATE TABLE instance_content_provider_refs (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	content_entry_id TEXT NOT NULL,
	provider TEXT NOT NULL,
	provider_project_id TEXT NOT NULL,
	provider_release_id TEXT NULL,
	is_origin INTEGER NOT NULL DEFAULT 0,

	FOREIGN KEY (content_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE CASCADE,
	CHECK (provider IN ('modrinth', 'curseforge')),
	CHECK (is_origin IN (0, 1))
);

CREATE INDEX IF NOT EXISTS instance_content_provider_refs_project
	ON instance_content_provider_refs(provider, provider_project_id);

CREATE INDEX IF NOT EXISTS instance_content_provider_refs_release
	ON instance_content_provider_refs(provider, provider_release_id);

CREATE UNIQUE INDEX IF NOT EXISTS instance_content_provider_refs_identity
	ON instance_content_provider_refs(
		content_entry_id,
		provider,
		provider_project_id,
		COALESCE(provider_release_id, '')
	);

CREATE UNIQUE INDEX IF NOT EXISTS instance_content_provider_refs_origin
	ON instance_content_provider_refs(content_entry_id)
	WHERE is_origin = 1;

INSERT OR IGNORE INTO instance_content_provider_refs (
	content_entry_id,
	provider,
	provider_project_id,
	provider_release_id,
	is_origin
)
SELECT
	content_entry_id,
	provider,
	project_id,
	version_id,
	primary_ref
FROM instance_content_provider_refs_legacy
WHERE length(trim(project_id)) > 0
	AND (
		provider = 'modrinth'
		OR (
			provider = 'curseforge'
			AND project_id NOT GLOB '*[^0-9]*'
			AND CAST(project_id AS INTEGER) > 0
			AND (
				version_id IS NULL
				OR (
					version_id NOT GLOB '*[^0-9]*'
					AND CAST(version_id AS INTEGER) > 0
				)
			)
		)
	);

INSERT OR IGNORE INTO instance_content_provider_refs (
	content_entry_id,
	provider,
	provider_project_id,
	provider_release_id,
	is_origin
)
SELECT
	legacy.id,
	'curseforge',
	legacy.project_id,
	legacy.version_id,
	1
FROM instance_content_entries_legacy legacy
WHERE legacy.source_kind = 'curseforge'
	AND legacy.project_id IS NOT NULL
	AND legacy.project_id NOT GLOB '*[^0-9]*'
	AND CAST(legacy.project_id AS INTEGER) > 0
	AND NOT EXISTS (
		SELECT 1
		FROM instance_content_provider_refs ref
		WHERE ref.content_entry_id = legacy.id
			AND ref.provider = 'curseforge'
	);

INSERT OR IGNORE INTO instance_content_provider_refs (
	content_entry_id,
	provider,
	provider_project_id,
	provider_release_id,
	is_origin
)
SELECT
	legacy.id,
	'modrinth',
	json_extract(cache.data, '$.project_id'),
	json_extract(cache.data, '$.version_id'),
	CASE
		WHEN legacy.source_kind IN ('modrinth_modpack', 'modrinth_hosting')
		THEN 1
		ELSE 0
	END
FROM instance_content_entries_legacy legacy
INNER JOIN instance_files file ON file.id = legacy.file_id
INNER JOIN cache
	ON cache.data_type = 'file_hash'
	AND json_extract(cache.data, '$.path') =
		legacy.instance_id || '/' || file.relative_path
WHERE json_extract(cache.data, '$.project_id') IS NOT NULL
	AND json_extract(cache.data, '$.version_id') IS NOT NULL;

CREATE TABLE instance_content_update_checks (
	content_entry_id TEXT NOT NULL,
	update_channel TEXT NOT NULL,
	provider TEXT NULL,
	provider_project_id TEXT NULL,
	provider_release_id TEXT NULL,
	checked_at INTEGER NOT NULL,

	PRIMARY KEY (content_entry_id),
	FOREIGN KEY (content_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE CASCADE,
	CHECK (provider IS NULL OR provider IN ('modrinth', 'curseforge'))
);

CREATE INDEX IF NOT EXISTS instance_content_update_checks_release
	ON instance_content_update_checks(provider, provider_release_id);

INSERT INTO instance_content_update_checks (
	content_entry_id,
	update_channel,
	provider,
	provider_project_id,
	provider_release_id,
	checked_at
)
SELECT
	legacy.content_entry_id,
	legacy.update_channel,
	ref.provider,
	ref.provider_project_id,
	CASE
		WHEN legacy.update_version_id IS NULL THEN NULL
		ELSE legacy.update_version_id
	END,
	legacy.checked_at
FROM instance_content_update_checks_legacy legacy
LEFT JOIN instance_content_provider_refs ref
	ON ref.content_entry_id = legacy.content_entry_id
	AND ref.is_origin = 1
WHERE ref.provider = 'modrinth';

DROP TABLE instance_content_provider_refs_legacy;
DROP TABLE instance_content_update_checks_legacy;
DROP TABLE instance_content_entries_legacy;
