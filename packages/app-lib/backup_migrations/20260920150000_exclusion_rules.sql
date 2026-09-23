ALTER TABLE backup_snapshots
	ADD COLUMN scope_mode TEXT NOT NULL DEFAULT 'included_roots'
	CHECK (scope_mode IN ('included_roots', 'excluded_paths'));

DROP TABLE backup_selections;

CREATE TABLE backup_exclusions (
	instance_id TEXT NOT NULL REFERENCES instance_backup_configs(instance_id) ON DELETE CASCADE,
	path TEXT NOT NULL,
	kind TEXT NOT NULL CHECK (kind IN ('file', 'directory')),
	PRIMARY KEY (instance_id, path)
);

CREATE TABLE snapshot_exclusions (
	snapshot_id TEXT NOT NULL REFERENCES backup_snapshots(id) ON DELETE CASCADE,
	path TEXT NOT NULL,
	kind TEXT NOT NULL CHECK (kind IN ('file', 'directory')),
	PRIMARY KEY (snapshot_id, path)
);
