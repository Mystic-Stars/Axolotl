CREATE TABLE crash_analysis_ai_settings (
	id INTEGER NOT NULL CHECK (id = 0),
	enabled INTEGER NOT NULL DEFAULT FALSE,
	provider_id TEXT NOT NULL DEFAULT '',
	model_id TEXT NOT NULL DEFAULT '',
	PRIMARY KEY (id)
);

INSERT INTO crash_analysis_ai_settings (id) VALUES (0);
