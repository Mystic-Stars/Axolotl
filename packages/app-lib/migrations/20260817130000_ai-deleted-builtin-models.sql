-- Track deleted builtin AI models so they don't reappear on load
CREATE TABLE ai_deleted_builtin_models (
	provider_id TEXT NOT NULL,
	model_id TEXT NOT NULL,
	deleted_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
	PRIMARY KEY (provider_id, model_id)
);
