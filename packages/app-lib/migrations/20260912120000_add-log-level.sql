ALTER TABLE settings ADD COLUMN log_level TEXT NOT NULL DEFAULT 'trace' CHECK (
	log_level IN ('error', 'warn', 'info', 'debug', 'trace')
);
