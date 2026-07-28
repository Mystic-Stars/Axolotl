ALTER TABLE instance_content_provider_refs
	RENAME TO instance_content_provider_refs_single_legacy;

DROP INDEX IF EXISTS instance_content_provider_refs_project;
DROP INDEX IF EXISTS instance_content_provider_refs_release;
DROP INDEX IF EXISTS instance_content_provider_refs_origin;
DROP INDEX IF EXISTS instance_content_provider_refs_identity;

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

CREATE INDEX instance_content_provider_refs_project
	ON instance_content_provider_refs(provider, provider_project_id);

CREATE INDEX instance_content_provider_refs_release
	ON instance_content_provider_refs(provider, provider_release_id);

CREATE UNIQUE INDEX instance_content_provider_refs_identity
	ON instance_content_provider_refs(
		content_entry_id,
		provider,
		provider_project_id,
		COALESCE(provider_release_id, '')
	);

CREATE UNIQUE INDEX instance_content_provider_refs_origin
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
	provider_project_id,
	provider_release_id,
	is_origin
FROM instance_content_provider_refs_single_legacy
WHERE length(trim(provider_project_id)) > 0
	AND (
		(
			provider = 'modrinth'
			AND (
				provider_release_id IS NULL
				OR length(trim(provider_release_id)) > 0
			)
		)
		OR (
			provider = 'curseforge'
			AND provider_project_id NOT GLOB '*[^0-9]*'
			AND CAST(provider_project_id AS INTEGER) > 0
			AND (
				provider_release_id IS NULL
				OR (
					provider_release_id NOT GLOB '*[^0-9]*'
					AND CAST(provider_release_id AS INTEGER) > 0
				)
			)
		)
	);

UPDATE instance_content_provider_refs
SET is_origin = 0
WHERE content_entry_id IN (
	SELECT id
	FROM instance_content_entries
	WHERE source_kind = 'curseforge'
);

INSERT OR IGNORE INTO instance_content_provider_refs (
	content_entry_id,
	provider,
	provider_project_id,
	provider_release_id,
	is_origin
)
SELECT
	ref.content_entry_id,
	'curseforge',
	ref.provider_project_id,
	ref.provider_release_id,
	0
FROM instance_content_provider_refs ref
INNER JOIN instance_content_entries entry
	ON entry.id = ref.content_entry_id
WHERE entry.source_kind = 'curseforge'
	AND ref.provider = 'modrinth'
	AND ref.provider_project_id NOT GLOB '*[^0-9]*'
	AND CAST(ref.provider_project_id AS INTEGER) > 0
	AND (
		ref.provider_release_id IS NULL
		OR (
			ref.provider_release_id NOT GLOB '*[^0-9]*'
			AND CAST(ref.provider_release_id AS INTEGER) > 0
		)
	)
	AND NOT EXISTS (
		SELECT 1
		FROM instance_content_provider_refs existing
		WHERE existing.content_entry_id = ref.content_entry_id
			AND existing.provider = 'curseforge'
	)
	AND ref.id = (
		SELECT MIN(candidate.id)
		FROM instance_content_provider_refs candidate
		WHERE candidate.content_entry_id = ref.content_entry_id
			AND candidate.provider = 'modrinth'
			AND candidate.provider_project_id NOT GLOB '*[^0-9]*'
			AND CAST(candidate.provider_project_id AS INTEGER) > 0
			AND (
				candidate.provider_release_id IS NULL
				OR (
					candidate.provider_release_id NOT GLOB '*[^0-9]*'
					AND CAST(candidate.provider_release_id AS INTEGER) > 0
				)
			)
	);

UPDATE instance_content_provider_refs
SET is_origin = 1
WHERE id IN (
	SELECT MIN(ref.id)
	FROM instance_content_provider_refs ref
	INNER JOIN instance_content_entries entry
		ON entry.id = ref.content_entry_id
	WHERE entry.source_kind = 'curseforge'
		AND ref.provider = 'curseforge'
	GROUP BY ref.content_entry_id
);

DELETE FROM instance_content_provider_refs
WHERE provider = 'modrinth'
	AND content_entry_id IN (
		SELECT id
		FROM instance_content_entries
		WHERE source_kind = 'curseforge'
	)
	AND NOT EXISTS (
		SELECT 1
		FROM instance_content_entries entry
		INNER JOIN instance_files file ON file.id = entry.file_id
		INNER JOIN cache
			ON cache.data_type = 'file_hash'
			AND json_extract(
				CASE WHEN json_valid(cache.data) THEN cache.data ELSE '{}' END,
				'$.path'
			) =
				entry.instance_id || '/' || file.relative_path
		WHERE entry.id = instance_content_provider_refs.content_entry_id
			AND json_extract(
				CASE WHEN json_valid(cache.data) THEN cache.data ELSE '{}' END,
				'$.project_id'
			) =
				instance_content_provider_refs.provider_project_id
			AND json_extract(
				CASE WHEN json_valid(cache.data) THEN cache.data ELSE '{}' END,
				'$.version_id'
			) IS
				instance_content_provider_refs.provider_release_id
	);

DELETE FROM instance_content_update_checks
WHERE provider = 'modrinth'
	AND content_entry_id IN (
		SELECT id
		FROM instance_content_entries
		WHERE source_kind = 'curseforge'
	);

DROP TABLE instance_content_provider_refs_single_legacy;
