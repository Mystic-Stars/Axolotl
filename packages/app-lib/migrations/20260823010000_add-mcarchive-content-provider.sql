DROP INDEX IF EXISTS instance_content_provider_refs_project;
DROP INDEX IF EXISTS instance_content_provider_refs_release;
DROP INDEX IF EXISTS instance_content_provider_refs_identity;
DROP INDEX IF EXISTS instance_content_provider_refs_origin;
DROP INDEX IF EXISTS instance_content_update_checks_release;
DROP INDEX IF EXISTS instance_pack_members_content_set;
DROP INDEX IF EXISTS instance_pack_members_content_entry;
DROP INDEX IF EXISTS instance_pack_members_provider;
DROP INDEX IF EXISTS instance_pack_members_state;
DROP INDEX IF EXISTS instance_pending_manual_download_identity;
DROP INDEX IF EXISTS instance_pending_manual_downloads_instance_state;
DROP INDEX IF EXISTS instance_content_dependencies_child;
DROP INDEX IF EXISTS instance_content_dependencies_parent;
DROP INDEX IF EXISTS instance_content_dependencies_child_project;
DROP INDEX IF EXISTS content_favorites_saved_at_idx;

ALTER TABLE instance_content_provider_refs RENAME TO instance_content_provider_refs_old;
ALTER TABLE instance_content_update_checks RENAME TO instance_content_update_checks_old;
ALTER TABLE instance_pack_members RENAME TO instance_pack_members_old;
ALTER TABLE instance_pending_manual_downloads RENAME TO instance_pending_manual_downloads_old;
ALTER TABLE instance_content_dependencies RENAME TO instance_content_dependencies_old;
ALTER TABLE content_favorites RENAME TO content_favorites_old;

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
	CHECK (provider IN ('modrinth', 'curseforge', 'mcarchive')),
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

INSERT INTO instance_content_provider_refs (
	id, content_entry_id, provider, provider_project_id,
	provider_release_id, is_origin
)
SELECT id, content_entry_id, provider, provider_project_id,
	provider_release_id, is_origin
FROM instance_content_provider_refs_old;

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
	CHECK (provider IS NULL OR provider IN ('modrinth', 'curseforge', 'mcarchive'))
);

CREATE INDEX instance_content_update_checks_release
	ON instance_content_update_checks(provider, provider_release_id);

INSERT INTO instance_content_update_checks (
	content_entry_id, update_channel, provider, provider_project_id,
	provider_release_id, checked_at
)
SELECT content_entry_id, update_channel, provider, provider_project_id,
	provider_release_id, checked_at
FROM instance_content_update_checks_old;

CREATE TABLE instance_pack_members (
	id TEXT NOT NULL,
	content_set_id TEXT NOT NULL,
	content_entry_id TEXT NULL,
	member_key TEXT NOT NULL,
	project_type TEXT NOT NULL,
	expected_relative_path TEXT NOT NULL,
	provider TEXT NULL,
	provider_project_id TEXT NULL,
	provider_release_id TEXT NULL,
	required INTEGER NOT NULL DEFAULT 1,
	expected_sha1 TEXT NULL,
	expected_size INTEGER NULL,
	expected_fingerprint INTEGER NULL,
	materialization_state TEXT NOT NULL DEFAULT 'present',
	override_kind TEXT NOT NULL DEFAULT 'none',
	reconciled INTEGER NOT NULL DEFAULT 1,
	created_at INTEGER NOT NULL,
	modified_at INTEGER NOT NULL,

	PRIMARY KEY (id),
	UNIQUE (content_set_id, member_key),
	FOREIGN KEY (content_set_id)
		REFERENCES instance_content_sets(id)
		ON DELETE CASCADE,
	FOREIGN KEY (content_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE SET NULL,
	CHECK (provider IS NULL OR provider IN ('modrinth', 'curseforge', 'mcarchive')),
	CHECK (required IN (0, 1)),
	CHECK (materialization_state IN ('present', 'pending_manual', 'missing', 'removed')),
	CHECK (override_kind IN ('none', 'disabled', 'removed', 'version')),
	CHECK (reconciled IN (0, 1))
);

CREATE INDEX instance_pack_members_content_set
	ON instance_pack_members(content_set_id);
CREATE INDEX instance_pack_members_content_entry
	ON instance_pack_members(content_entry_id);
CREATE INDEX instance_pack_members_provider
	ON instance_pack_members(provider, provider_project_id, provider_release_id);
CREATE INDEX instance_pack_members_state
	ON instance_pack_members(content_set_id, materialization_state);

INSERT INTO instance_pack_members (
	id, content_set_id, content_entry_id, member_key, project_type,
	expected_relative_path, provider, provider_project_id, provider_release_id,
	required, expected_sha1, expected_size, expected_fingerprint,
	materialization_state, override_kind, reconciled, created_at, modified_at
)
SELECT id, content_set_id, content_entry_id, member_key, project_type,
	expected_relative_path, provider, provider_project_id, provider_release_id,
	required, expected_sha1, expected_size, expected_fingerprint,
	materialization_state, override_kind, reconciled, created_at, modified_at
FROM instance_pack_members_old;

CREATE TABLE instance_pending_manual_downloads (
	id TEXT NOT NULL,
	instance_id TEXT NOT NULL,
	pack_member_id TEXT NULL,
	content_entry_id TEXT NULL,
	operation_kind TEXT NOT NULL,
	operation_target_id TEXT NULL,
	project_type TEXT NOT NULL,
	provider TEXT NOT NULL,
	provider_project_id TEXT NOT NULL,
	provider_release_id TEXT NOT NULL,
	file_name TEXT NOT NULL,
	website_url TEXT NULL,
	target_relative_path TEXT NOT NULL,
	expected_sha1 TEXT NULL,
	expected_size INTEGER NULL,
	expected_fingerprint INTEGER NULL,
	state TEXT NOT NULL DEFAULT 'waiting',
	context JSONB NOT NULL DEFAULT '{}',
	created_at INTEGER NOT NULL,
	modified_at INTEGER NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE,
	FOREIGN KEY (pack_member_id)
		REFERENCES instance_pack_members(id)
		ON DELETE CASCADE,
	FOREIGN KEY (content_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE SET NULL,
	CHECK (provider IN ('modrinth', 'curseforge', 'mcarchive')),
	CHECK (operation_kind IN ('pack_install', 'pack_update', 'content_install', 'content_update')),
	CHECK (state IN ('waiting', 'matched', 'imported', 'error', 'cancelled'))
);

CREATE UNIQUE INDEX instance_pending_manual_download_identity
	ON instance_pending_manual_downloads(
		instance_id, operation_kind, provider, provider_project_id,
		provider_release_id
	)
	WHERE state IN ('waiting', 'matched');
CREATE INDEX instance_pending_manual_downloads_instance_state
	ON instance_pending_manual_downloads(instance_id, state);

INSERT INTO instance_pending_manual_downloads (
	id, instance_id, pack_member_id, content_entry_id, operation_kind,
	operation_target_id, project_type, provider, provider_project_id,
	provider_release_id, file_name, website_url, target_relative_path,
	expected_sha1, expected_size, expected_fingerprint, state, context,
	created_at, modified_at
)
SELECT id, instance_id, pack_member_id, content_entry_id, operation_kind,
	operation_target_id, project_type, provider, provider_project_id,
	provider_release_id, file_name, website_url, target_relative_path,
	expected_sha1, expected_size, expected_fingerprint, state, context,
	created_at, modified_at
FROM instance_pending_manual_downloads_old;

CREATE TABLE instance_content_dependencies (
	id TEXT NOT NULL,
	content_set_id TEXT NOT NULL,
	parent_entry_id TEXT NOT NULL,
	child_entry_id TEXT NOT NULL,
	evidence_provider TEXT NOT NULL,
	parent_provider TEXT NOT NULL,
	child_provider TEXT NOT NULL,
	dependency_kind TEXT NOT NULL,
	parent_project_id TEXT NOT NULL,
	parent_release_id TEXT NOT NULL,
	child_project_id TEXT NOT NULL,
	child_release_id TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	modified_at INTEGER NOT NULL,

	PRIMARY KEY (id),
	UNIQUE (content_set_id, parent_entry_id, child_entry_id, dependency_kind),
	FOREIGN KEY (content_set_id)
		REFERENCES instance_content_sets(id)
		ON DELETE CASCADE,
	FOREIGN KEY (parent_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE CASCADE,
	FOREIGN KEY (child_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE CASCADE,
	CHECK (evidence_provider IN ('modrinth', 'curseforge', 'mcarchive', 'local')),
	CHECK (parent_provider IN ('modrinth', 'curseforge', 'mcarchive', 'local')),
	CHECK (child_provider IN ('modrinth', 'curseforge', 'mcarchive', 'local')),
	CHECK (dependency_kind IN ('required', 'include'))
);

CREATE INDEX instance_content_dependencies_child
	ON instance_content_dependencies(content_set_id, child_entry_id);
CREATE INDEX instance_content_dependencies_parent
	ON instance_content_dependencies(content_set_id, parent_entry_id);
CREATE INDEX instance_content_dependencies_child_project
	ON instance_content_dependencies(child_provider, child_project_id);

INSERT INTO instance_content_dependencies (
	id, content_set_id, parent_entry_id, child_entry_id, evidence_provider,
	parent_provider, child_provider, dependency_kind, parent_project_id,
	parent_release_id, child_project_id, child_release_id, created_at, modified_at
)
SELECT id, content_set_id, parent_entry_id, child_entry_id, evidence_provider,
	parent_provider, child_provider, dependency_kind, parent_project_id,
	parent_release_id, child_project_id, child_release_id, created_at, modified_at
FROM instance_content_dependencies_old;

CREATE TABLE content_favorites (
	provider TEXT NOT NULL CHECK (provider IN ('modrinth', 'curseforge', 'mcarchive')),
	project_id TEXT NOT NULL CHECK (length(project_id) > 0),
	content_type TEXT NOT NULL CHECK (content_type IN ('mod', 'resourcepack', 'datapack', 'shader')),
	saved_at INTEGER NOT NULL,

	PRIMARY KEY (provider, project_id)
);

CREATE INDEX content_favorites_saved_at_idx ON content_favorites (saved_at DESC);

INSERT INTO content_favorites (provider, project_id, content_type, saved_at)
SELECT provider, project_id, content_type, saved_at
FROM content_favorites_old;

DROP TABLE instance_pending_manual_downloads_old;
DROP TABLE instance_pack_members_old;
DROP TABLE instance_content_update_checks_old;
DROP TABLE instance_content_provider_refs_old;
DROP TABLE instance_content_dependencies_old;
DROP TABLE content_favorites_old;
