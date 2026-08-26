CREATE TABLE ingestion_daily (
	day TEXT NOT NULL,
	installation_hash TEXT NOT NULL,
	accepted_batches INTEGER NOT NULL DEFAULT 0,
	PRIMARY KEY (day, installation_hash)
)
WITHOUT ROWID;

CREATE TABLE ingestion_global_daily (
	day TEXT PRIMARY KEY,
	accepted_batches INTEGER NOT NULL DEFAULT 0
)
WITHOUT ROWID;

INSERT INTO ingestion_daily (day, installation_hash, accepted_batches)
SELECT date(accepted_at, 'unixepoch'), installation_hash, COUNT(*)
FROM accepted_batches
GROUP BY date(accepted_at, 'unixepoch'), installation_hash;

INSERT INTO ingestion_global_daily (day, accepted_batches)
SELECT date(accepted_at, 'unixepoch'), COUNT(*)
FROM accepted_batches
GROUP BY date(accepted_at, 'unixepoch');
