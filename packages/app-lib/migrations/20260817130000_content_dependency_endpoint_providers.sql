-- A dependency edge can be proven by one provider while each endpoint belongs
-- to a different provider. Rebuild because SQLite cannot tighten the existing
-- provider CHECK constraint in place.
CREATE TABLE instance_content_dependencies_endpoint_providers (
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
	UNIQUE (
		content_set_id,
		parent_entry_id,
		child_entry_id,
		dependency_kind
	),
	FOREIGN KEY (content_set_id)
		REFERENCES instance_content_sets(id)
		ON DELETE CASCADE,
	FOREIGN KEY (parent_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE CASCADE,
	FOREIGN KEY (child_entry_id)
		REFERENCES instance_content_entries(id)
		ON DELETE CASCADE,
	CHECK (evidence_provider IN ('modrinth', 'curseforge', 'local')),
	CHECK (parent_provider IN ('modrinth', 'curseforge', 'local')),
	CHECK (child_provider IN ('modrinth', 'curseforge', 'local')),
	CHECK (dependency_kind IN ('required', 'include'))
);

INSERT INTO instance_content_dependencies_endpoint_providers
	SELECT id, content_set_id, parent_entry_id, child_entry_id,
		provider, provider, provider, dependency_kind,
		parent_project_id, parent_release_id,
		child_project_id, child_release_id, created_at, modified_at
	FROM instance_content_dependencies;

DROP TABLE instance_content_dependencies;

ALTER TABLE instance_content_dependencies_endpoint_providers
	RENAME TO instance_content_dependencies;

CREATE INDEX instance_content_dependencies_child
	ON instance_content_dependencies(content_set_id, child_entry_id);
CREATE INDEX instance_content_dependencies_parent
	ON instance_content_dependencies(content_set_id, parent_entry_id);
CREATE INDEX instance_content_dependencies_child_project
	ON instance_content_dependencies(child_provider, child_project_id);
