CREATE TABLE log_share_settings (
	id INTEGER NOT NULL CHECK (id = 0),
	share_provider TEXT NOT NULL DEFAULT 'logshare',
	ai_source TEXT NOT NULL DEFAULT 'logshare',
	auto_upload INTEGER NOT NULL DEFAULT TRUE,
	multi_file INTEGER NOT NULL DEFAULT TRUE,
	metadata_enabled INTEGER NOT NULL DEFAULT TRUE,
	no_storage INTEGER NOT NULL DEFAULT FALSE,
	show_progress INTEGER NOT NULL DEFAULT TRUE,
	PRIMARY KEY (id)
);

INSERT INTO log_share_settings (id) VALUES (0);

CREATE TABLE shared_logs (
	id TEXT NOT NULL PRIMARY KEY,
	url TEXT NOT NULL,
	raw TEXT NOT NULL,
	token TEXT NOT NULL,
	provider TEXT NOT NULL DEFAULT 'logshare',
	instance_id TEXT NULL,
	instance_name TEXT NULL,
	truncated INTEGER NOT NULL DEFAULT FALSE,
	created_at INTEGER NOT NULL
);

CREATE INDEX shared_logs_created_at_idx ON shared_logs (created_at DESC);

ALTER TABLE crash_analysis_ai_settings ADD COLUMN ai_source TEXT NOT NULL DEFAULT 'logshare';