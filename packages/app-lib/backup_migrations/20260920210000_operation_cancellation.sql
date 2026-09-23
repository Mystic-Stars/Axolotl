ALTER TABLE backup_operations
	ADD COLUMN cancel_requested INTEGER NOT NULL DEFAULT 0;
