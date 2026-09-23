CREATE TABLE repository_meta (
	id INTEGER PRIMARY KEY CHECK (id = 0),
	schema_name TEXT NOT NULL,
	created_at INTEGER NOT NULL
);

INSERT INTO repository_meta (id, schema_name, created_at)
VALUES (0, 'axolotl-instance-backups', unixepoch('subsec') * 1000);

CREATE TABLE instance_backup_configs (
	instance_id TEXT PRIMARY KEY,
	instance_name TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	modified_at INTEGER NOT NULL
);

CREATE TABLE backup_selections (
	instance_id TEXT NOT NULL REFERENCES instance_backup_configs(instance_id) ON DELETE CASCADE,
	path TEXT NOT NULL,
	PRIMARY KEY (instance_id, path)
);

CREATE TABLE backup_snapshots (
	id TEXT PRIMARY KEY,
	instance_id TEXT NOT NULL REFERENCES instance_backup_configs(instance_id) ON DELETE CASCADE,
	instance_name TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	file_count INTEGER NOT NULL,
	symlink_count INTEGER NOT NULL,
	logical_size INTEGER NOT NULL,
	added_size INTEGER NOT NULL
);

CREATE INDEX backup_snapshots_instance_created
	ON backup_snapshots(instance_id, created_at DESC);

CREATE TABLE snapshot_roots (
	snapshot_id TEXT NOT NULL REFERENCES backup_snapshots(id) ON DELETE CASCADE,
	path TEXT NOT NULL,
	existed INTEGER NOT NULL,
	PRIMARY KEY (snapshot_id, path)
);

CREATE TABLE backup_objects (
	hash TEXT PRIMARY KEY,
	size INTEGER NOT NULL,
	ref_count INTEGER NOT NULL CHECK (ref_count >= 0)
);

CREATE TABLE snapshot_entries (
	snapshot_id TEXT NOT NULL REFERENCES backup_snapshots(id) ON DELETE CASCADE,
	path TEXT NOT NULL,
	kind TEXT NOT NULL CHECK (kind IN ('file', 'directory', 'symlink')),
	object_hash TEXT REFERENCES backup_objects(hash),
	size INTEGER NOT NULL DEFAULT 0,
	modified_at INTEGER,
	unix_mode INTEGER,
	link_target TEXT,
	link_is_directory INTEGER,
	PRIMARY KEY (snapshot_id, path),
	CHECK ((kind = 'file' AND object_hash IS NOT NULL) OR (kind != 'file' AND object_hash IS NULL)),
	CHECK ((kind = 'symlink' AND link_target IS NOT NULL) OR (kind != 'symlink' AND link_target IS NULL))
);

CREATE INDEX snapshot_entries_object_hash
	ON snapshot_entries(object_hash) WHERE object_hash IS NOT NULL;

CREATE TABLE object_gc_queue (
	hash TEXT PRIMARY KEY,
	queued_at INTEGER NOT NULL
);

CREATE TABLE pending_instance_deletions (
	instance_id TEXT PRIMARY KEY,
	marked_at INTEGER NOT NULL
);
