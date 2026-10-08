-- 0002: eliminate per-event aggregation write/read amplification and add missing indexes.
--
-- The previous design ran three aggregate UPSERTs (error_groups / error_daily /
-- daily_totals) inside an AFTER INSERT trigger for every error report, each with
-- a NOT EXISTS subquery that scanned the whole fingerprint history. That single
-- INSERT statement accounted for ~8.2M rows read and ~147k rows written per day,
-- far beyond the Workers Free daily limits (5M rows read / 100k rows written).
--
-- This migration:
--   1. Drops the per-event aggregation triggers; counts now flow through a
--      realtime error_daily upsert in the worker and nightly cron rollups.
--   2. Rebuilds error_daily and accepted_batches as WITHOUT ROWID so every
--      upsert/insert costs one written row instead of two.
--   3. Adds seen/dimension tables for O(1) distinct-install tracking.
--   4. Swaps the error_reports index so filter queries become point lookups.
--   5. Backfills the new tables from existing data.

DROP TRIGGER error_report_aggregates;
DROP TRIGGER installations_daily_total;
DROP TRIGGER daily_active_total;

ALTER TABLE error_daily RENAME TO error_daily_legacy;
CREATE TABLE error_daily (
    day TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL,
    installation_count INTEGER NOT NULL,
    PRIMARY KEY (day, fingerprint, app_version)
)
WITHOUT ROWID;
INSERT INTO error_daily (day, fingerprint, app_version, occurrence_count, installation_count)
SELECT day, fingerprint, app_version, occurrence_count, installation_count
FROM error_daily_legacy;
DROP TABLE error_daily_legacy;

ALTER TABLE accepted_batches RENAME TO accepted_batches_legacy;
CREATE TABLE accepted_batches (
    batch_id TEXT PRIMARY KEY,
    installation_hash TEXT NOT NULL,
    accepted_at INTEGER NOT NULL
)
WITHOUT ROWID;
INSERT INTO accepted_batches (batch_id, installation_hash, accepted_at)
SELECT batch_id, installation_hash, accepted_at FROM accepted_batches_legacy;
DROP TABLE accepted_batches_legacy;

CREATE TABLE error_daily_installations (
    day TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    installation_hash TEXT NOT NULL,
    PRIMARY KEY (day, fingerprint, app_version, installation_hash)
)
WITHOUT ROWID;

CREATE TABLE error_group_installations (
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    installation_hash TEXT NOT NULL,
    first_seen_day TEXT NOT NULL,
    PRIMARY KEY (fingerprint, app_version, installation_hash)
)
WITHOUT ROWID;

CREATE TABLE wau_seen (
    installation_hash TEXT PRIMARY KEY
)
WITHOUT ROWID;

CREATE TABLE mau_seen (
    installation_hash TEXT PRIMARY KEY
)
WITHOUT ROWID;

CREATE TABLE daily_active_dims (
    dimension TEXT NOT NULL,
    day TEXT NOT NULL,
    label TEXT NOT NULL,
    install_count INTEGER NOT NULL,
    PRIMARY KEY (dimension, day, label)
)
WITHOUT ROWID;

CREATE TABLE platforms (
    platform TEXT PRIMARY KEY
)
WITHOUT ROWID;

CREATE TABLE error_range_stats (
    range_days INTEGER NOT NULL,
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL,
    installation_count INTEGER NOT NULL,
    latest_error_type TEXT NOT NULL,
    latest_message TEXT NOT NULL,
    PRIMARY KEY (range_days, fingerprint, app_version)
)
WITHOUT ROWID;

DROP INDEX error_reports_fingerprint;
CREATE INDEX error_reports_group_day ON error_reports (fingerprint, app_version, day);
CREATE INDEX installations_first_seen_day ON installations (first_seen_day);
CREATE INDEX error_context_reservations_day ON error_context_reservations (day);
CREATE INDEX error_context_reservations_fingerprint_created ON error_context_reservations (
    fingerprint,
    created_at
);
CREATE INDEX error_context_reservations_created_at ON error_context_reservations (created_at);
CREATE INDEX error_groups_last_seen_day ON error_groups (last_seen_day);
CREATE INDEX accepted_batches_accepted_at ON accepted_batches (accepted_at);
CREATE INDEX daily_active_installation ON daily_active (installation_hash, day);

CREATE TRIGGER daily_active_rollups
AFTER INSERT ON daily_active
BEGIN
    INSERT OR IGNORE INTO wau_seen (installation_hash) VALUES (NEW.installation_hash);
    INSERT OR IGNORE INTO mau_seen (installation_hash) VALUES (NEW.installation_hash);
    INSERT INTO daily_active_dims (dimension, day, label, install_count)
    VALUES ('version', NEW.day, NEW.app_version, 1)
    ON CONFLICT (dimension, day, label) DO UPDATE SET install_count = install_count + 1;
    INSERT INTO daily_active_dims (dimension, day, label, install_count)
    VALUES ('platform', NEW.day, NEW.platform, 1)
    ON CONFLICT (dimension, day, label) DO UPDATE SET install_count = install_count + 1;
    INSERT INTO daily_active_dims (dimension, day, label, install_count)
    VALUES ('arch', NEW.day, NEW.arch, 1)
    ON CONFLICT (dimension, day, label) DO UPDATE SET install_count = install_count + 1;
END;

INSERT OR IGNORE INTO wau_seen (installation_hash)
SELECT DISTINCT installation_hash FROM daily_active WHERE day >= date('now', '-6 days');
INSERT OR IGNORE INTO mau_seen (installation_hash)
SELECT DISTINCT installation_hash FROM daily_active WHERE day >= date('now', '-29 days');
INSERT OR IGNORE INTO platforms (platform)
SELECT DISTINCT platform FROM error_reports WHERE day >= date('now', '-30 days');
INSERT OR IGNORE INTO error_group_installations
    (fingerprint, app_version, installation_hash, first_seen_day)
SELECT fingerprint, app_version, installation_hash, MIN(day)
FROM error_reports
GROUP BY fingerprint, app_version, installation_hash;
INSERT OR IGNORE INTO daily_active_dims (dimension, day, label, install_count)
SELECT 'version', day, app_version, COUNT(DISTINCT installation_hash)
FROM daily_active
GROUP BY day, app_version;
INSERT OR IGNORE INTO daily_active_dims (dimension, day, label, install_count)
SELECT 'platform', day, platform, COUNT(DISTINCT installation_hash)
FROM daily_active
GROUP BY day, platform;
INSERT OR IGNORE INTO daily_active_dims (dimension, day, label, install_count)
SELECT 'arch', day, arch, COUNT(DISTINCT installation_hash)
FROM daily_active
GROUP BY day, arch;
