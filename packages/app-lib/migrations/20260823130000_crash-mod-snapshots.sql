CREATE TABLE crash_analysis_mod_snapshots (
	instance_id TEXT NOT NULL,
	filename TEXT NOT NULL,
	size INTEGER NOT NULL,
	sha256 TEXT NOT NULL,
	PRIMARY KEY (instance_id, filename)
);
