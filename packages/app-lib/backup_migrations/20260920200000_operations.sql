CREATE TABLE backup_operations (
	id TEXT PRIMARY KEY,
	operation_type TEXT NOT NULL
		CHECK (operation_type IN ('create', 'restore')),
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
	finished_at INTEGER
);

CREATE INDEX backup_operations_instance_created
	ON backup_operations(instance_id, created_at DESC);

CREATE INDEX backup_operations_active
	ON backup_operations(state, updated_at DESC)
	WHERE state NOT IN ('completed', 'cancelled', 'failed', 'interrupted');
