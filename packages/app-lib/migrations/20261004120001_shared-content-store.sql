CREATE TABLE store_blobs (
	digest TEXT PRIMARY KEY,
	size INTEGER NOT NULL CHECK (size >= 0),
	state TEXT NOT NULL CHECK (state IN ('ready', 'quarantined')),
	created_at INTEGER NOT NULL,
	last_used_at INTEGER NOT NULL
);

CREATE TABLE store_instance_files (
	instance_id TEXT NOT NULL,
	relative_path TEXT NOT NULL,
	digest TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	modified_at INTEGER NOT NULL,
	PRIMARY KEY (instance_id, relative_path),
	FOREIGN KEY (digest) REFERENCES store_blobs(digest) ON DELETE RESTRICT
);

CREATE TABLE store_retained_refs (
	digest TEXT NOT NULL,
	reason TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	PRIMARY KEY (digest, reason),
	FOREIGN KEY (digest) REFERENCES store_blobs(digest) ON DELETE CASCADE
);

CREATE TABLE store_operations (
	id TEXT PRIMARY KEY,
	digest TEXT NOT NULL,
	instance_id TEXT,
	relative_path TEXT,
	state TEXT NOT NULL CHECK (state IN ('staged', 'published', 'materialized', 'failed')),
	created_at INTEGER NOT NULL,
	updated_at INTEGER NOT NULL
);

CREATE INDEX store_blobs_retention ON store_blobs(state, last_used_at);
CREATE INDEX store_instance_files_digest ON store_instance_files(digest);
CREATE INDEX store_operations_state ON store_operations(state, updated_at);
