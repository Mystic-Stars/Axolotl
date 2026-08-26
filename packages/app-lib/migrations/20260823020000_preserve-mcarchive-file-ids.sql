DROP INDEX IF EXISTS instance_content_provider_refs_project;
DROP INDEX IF EXISTS instance_content_provider_refs_release;
DROP INDEX IF EXISTS instance_content_provider_refs_identity;
DROP INDEX IF EXISTS instance_content_provider_refs_origin;

ALTER TABLE instance_content_provider_refs RENAME TO instance_content_provider_refs_old;

CREATE TABLE instance_content_provider_refs (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	content_entry_id TEXT NOT NULL,
	provider TEXT NOT NULL,
	provider_project_id TEXT NOT NULL,
	provider_release_id TEXT NULL,
	provider_file_id TEXT NULL,
	is_origin INTEGER NOT NULL DEFAULT 0,

	FOREIGN KEY (content_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE CASCADE,
	CHECK (provider IN ('modrinth', 'curseforge', 'mcarchive')),
	CHECK (is_origin IN (0, 1))
);

CREATE INDEX instance_content_provider_refs_project
	ON instance_content_provider_refs(provider, provider_project_id);
CREATE INDEX instance_content_provider_refs_release
	ON instance_content_provider_refs(provider, provider_release_id);
CREATE INDEX instance_content_provider_refs_file
	ON instance_content_provider_refs(provider, provider_file_id);
CREATE UNIQUE INDEX instance_content_provider_refs_identity
	ON instance_content_provider_refs(
		content_entry_id,
		provider,
		provider_project_id,
		COALESCE(provider_release_id, ''),
		COALESCE(provider_file_id, '')
	);
CREATE UNIQUE INDEX instance_content_provider_refs_origin
	ON instance_content_provider_refs(content_entry_id)
	WHERE is_origin = 1;

INSERT INTO instance_content_provider_refs (
	id, content_entry_id, provider, provider_project_id,
	provider_release_id, provider_file_id, is_origin
)
SELECT
	id,
	content_entry_id,
	provider,
	provider_project_id,
	provider_release_id,
	CASE WHEN provider = 'curseforge' THEN provider_release_id ELSE NULL END,
	is_origin
FROM instance_content_provider_refs_old;

DROP TABLE instance_content_provider_refs_old;
