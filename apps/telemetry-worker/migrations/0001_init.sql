CREATE TABLE installations (
    installation_hash TEXT PRIMARY KEY,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    first_seen_day TEXT NOT NULL,
    app_version TEXT NOT NULL,
    platform TEXT NOT NULL,
    arch TEXT NOT NULL
);

CREATE TABLE daily_active (
    day TEXT NOT NULL,
    installation_hash TEXT NOT NULL,
    app_version TEXT NOT NULL,
    platform TEXT NOT NULL,
    arch TEXT NOT NULL,
    PRIMARY KEY (day, installation_hash),
    FOREIGN KEY (installation_hash) REFERENCES installations (installation_hash)
);

CREATE TABLE error_reports (
    event_id TEXT PRIMARY KEY,
    installation_hash TEXT NOT NULL,
    day TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    platform TEXT NOT NULL,
    arch TEXT NOT NULL,
    error_type TEXT NOT NULL,
    message TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL,
    object_key TEXT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (installation_hash) REFERENCES installations (installation_hash)
);

CREATE INDEX error_reports_day ON error_reports (day);
CREATE INDEX error_reports_fingerprint ON error_reports (fingerprint, day);

CREATE TABLE error_groups (
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    first_seen_day TEXT NOT NULL,
    last_seen_day TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL,
    installation_count INTEGER NOT NULL,
    latest_error_type TEXT NOT NULL,
    latest_message TEXT NOT NULL,
    sample_object_key TEXT NULL,
    PRIMARY KEY (fingerprint, app_version)
);

CREATE TABLE error_daily (
    day TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL,
    installation_count INTEGER NOT NULL,
    PRIMARY KEY (day, fingerprint, app_version)
);

CREATE TABLE accepted_batches (
    batch_id TEXT PRIMARY KEY,
    installation_hash TEXT NOT NULL,
    accepted_at INTEGER NOT NULL
);

CREATE TABLE daily_totals (
    day TEXT PRIMARY KEY,
    new_installations INTEGER NOT NULL DEFAULT 0,
    active_installations INTEGER NOT NULL DEFAULT 0,
    error_occurrences INTEGER NOT NULL DEFAULT 0,
    distinct_error_groups INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE error_context_budget (
    day TEXT PRIMARY KEY,
    object_count INTEGER NOT NULL CHECK (object_count <= 2000)
);

CREATE TABLE error_context_samples (
    day TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    sample_count INTEGER NOT NULL CHECK (sample_count <= 3),
    PRIMARY KEY (day, fingerprint, app_version)
);

CREATE TABLE error_context_reservations (
    event_id TEXT PRIMARY KEY,
    day TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    app_version TEXT NOT NULL,
    object_key TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);

CREATE TRIGGER installations_daily_total
AFTER INSERT ON installations
BEGIN
    INSERT INTO daily_totals (day, new_installations)
    VALUES (NEW.first_seen_day, 1)
    ON CONFLICT (day) DO UPDATE
        SET new_installations = new_installations + 1;
END;

CREATE TRIGGER daily_active_total
AFTER INSERT ON daily_active
BEGIN
    INSERT INTO daily_totals (day, active_installations)
    VALUES (NEW.day, 1)
    ON CONFLICT (day) DO UPDATE
        SET active_installations = active_installations + 1;
END;

CREATE TRIGGER error_report_aggregates
AFTER INSERT ON error_reports
BEGIN
    INSERT INTO error_groups
        (
            fingerprint,
            app_version,
            first_seen_day,
            last_seen_day,
            occurrence_count,
            installation_count,
            latest_error_type,
            latest_message,
            sample_object_key
        )
    VALUES
        (
            NEW.fingerprint,
            NEW.app_version,
            NEW.day,
            NEW.day,
            NEW.occurrence_count,
            1,
            NEW.error_type,
            NEW.message,
            NEW.object_key
        )
    ON CONFLICT (fingerprint, app_version) DO UPDATE
        SET
            last_seen_day = excluded.last_seen_day,
            occurrence_count = occurrence_count + excluded.occurrence_count,
            installation_count = installation_count + CASE
                WHEN NOT EXISTS (
                    SELECT 1
                    FROM error_reports AS previous
                    WHERE
                        previous.fingerprint = NEW.fingerprint
                        AND previous.app_version = NEW.app_version
                        AND previous.installation_hash = NEW.installation_hash
                        AND previous.event_id != NEW.event_id
                ) THEN 1
                ELSE 0
            END,
            latest_error_type = excluded.latest_error_type,
            latest_message = excluded.latest_message,
            sample_object_key = COALESCE(
                error_groups.sample_object_key,
                excluded.sample_object_key
            );

    INSERT INTO error_daily
        (day, fingerprint, app_version, occurrence_count, installation_count)
    VALUES (NEW.day, NEW.fingerprint, NEW.app_version, NEW.occurrence_count, 1)
    ON CONFLICT (day, fingerprint, app_version) DO UPDATE
        SET
            occurrence_count = occurrence_count + excluded.occurrence_count,
            installation_count = installation_count + CASE
                WHEN NOT EXISTS (
                    SELECT 1
                    FROM error_reports AS previous
                    WHERE
                        previous.day = NEW.day
                        AND previous.fingerprint = NEW.fingerprint
                        AND previous.app_version = NEW.app_version
                        AND previous.installation_hash = NEW.installation_hash
                        AND previous.event_id != NEW.event_id
                ) THEN 1
                ELSE 0
            END;

    INSERT INTO daily_totals (day, error_occurrences, distinct_error_groups)
    VALUES
        (
            NEW.day,
            NEW.occurrence_count,
            CASE
                WHEN NOT EXISTS (
                    SELECT 1
                    FROM error_reports AS previous
                    WHERE
                        previous.day = NEW.day
                        AND previous.fingerprint = NEW.fingerprint
                        AND previous.app_version = NEW.app_version
                        AND previous.event_id != NEW.event_id
                ) THEN 1
                ELSE 0
            END
        )
    ON CONFLICT (day) DO UPDATE
        SET
            error_occurrences = error_occurrences + excluded.error_occurrences,
            distinct_error_groups = distinct_error_groups + excluded.distinct_error_groups;
END;

CREATE TRIGGER error_context_reservation_budget
AFTER INSERT ON error_context_reservations
BEGIN
    INSERT INTO error_context_budget (day, object_count)
    VALUES (NEW.day, 1)
    ON CONFLICT (day) DO UPDATE SET object_count = object_count + 1;

    INSERT INTO error_context_samples (day, fingerprint, app_version, sample_count)
    VALUES (NEW.day, NEW.fingerprint, NEW.app_version, 1)
    ON CONFLICT (day, fingerprint, app_version) DO UPDATE
        SET sample_count = sample_count + 1;
END;
