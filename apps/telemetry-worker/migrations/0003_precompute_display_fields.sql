-- 0003: precompute display fields on error_daily so dashboard list queries stay pure aggregations.
--
-- The errors list live branch previously resolved per-group correlated subqueries
-- (distinct-install count, latest type/message, sample existence) on every page
-- load, costing ~145k rows read per execution. These are now maintained by the
-- worker on every upsert and only read back, cutting a list execution to ~15k rows.

ALTER TABLE error_daily ADD COLUMN latest_error_type TEXT NOT NULL DEFAULT 'Unknown';
ALTER TABLE error_daily ADD COLUMN latest_message TEXT NOT NULL DEFAULT '';
ALTER TABLE error_daily ADD COLUMN has_sample INTEGER NOT NULL DEFAULT 0;
ALTER TABLE error_range_stats ADD COLUMN has_sample INTEGER NOT NULL DEFAULT 0;

-- Backfill exact per-day distinct installs for reports ingested before the
-- error_daily_installations table existed.
INSERT OR IGNORE INTO error_daily_installations (day, fingerprint, app_version, installation_hash)
SELECT day, fingerprint, app_version, installation_hash
FROM error_reports
WHERE day >= date('now', '-1 days');

-- Backfill display fields for existing rows from the best available sources.
UPDATE error_daily
SET
    latest_error_type = (
        SELECT er.error_type
        FROM error_reports AS er
        WHERE er.fingerprint = error_daily.fingerprint AND er.app_version = error_daily.app_version
        ORDER BY er.occurred_at DESC, er.event_id DESC
        LIMIT 1
    ),
    latest_message = (
        SELECT er.message
        FROM error_reports AS er
        WHERE er.fingerprint = error_daily.fingerprint AND er.app_version = error_daily.app_version
        ORDER BY er.occurred_at DESC, er.event_id DESC
        LIMIT 1
    ),
    has_sample = CASE
        WHEN EXISTS (
            SELECT 1
            FROM error_context_reservations AS r
            WHERE
                r.fingerprint = error_daily.fingerprint
                AND r.app_version = error_daily.app_version
        ) THEN 1
        ELSE 0
    END;
