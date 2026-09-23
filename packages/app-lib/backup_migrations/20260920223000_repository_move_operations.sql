PRAGMA foreign_keys = OFF;

CREATE TABLE backup_operations_new (
	id TEXT PRIMARY KEY,
	operation_type TEXT NOT NULL
		CHECK (operation_type IN ('create', 'restore', 'repository_move')),
	instance_id TEXT NOT NULL,
	snapshot_id TEXT,
	state TEXT NOT NULL
		CHECK (state IN (
			'queued',
			'scanning',
			'hashing',
			'saving',
			'validating',
			'materializing',
			'applying',
			'completed',
			'cancelled',
			'failed',
			'interrupted'
		)),
	processed_bytes INTEGER NOT NULL DEFAULT 0,
	total_bytes INTEGER NOT NULL DEFAULT 0,
	cancellable INTEGER NOT NULL DEFAULT 1,
	error TEXT,
	created_at INTEGER NOT NULL,
	updated_at INTEGER NOT NULL,
	finished_at INTEGER,
	cancel_requested INTEGER NOT NULL DEFAULT 0
);

INSERT INTO backup_operations_new (
	id, operation_type, instance_id, snapshot_id, state,
	processed_bytes, total_bytes, cancellable, error,
	created_at, updated_at, finished_at, cancel_requested
)
SELECT
	id, operation_type, instance_id, snapshot_id, state,
	processed_bytes, total_bytes, cancellable, error,
	created_at, updated_at, finished_at, cancel_requested
FROM backup_operations;

DROP TABLE backup_operations;
ALTER TABLE backup_operations_new RENAME TO backup_operations;

CREATE INDEX backup_operations_instance_created
	ON backup_operations(instance_id, created_at DESC);

CREATE INDEX backup_operations_active
	ON backup_operations(state, updated_at DESC)
	WHERE state NOT IN ('completed', 'cancelled', 'failed', 'interrupted');

PRAGMA foreign_keys = ON;
