CREATE TABLE instance_post_upgrade_notices (
    instance_id TEXT PRIMARY KEY NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    upgrade_job_id TEXT NOT NULL,
    target_game_version TEXT NOT NULL,
    consecutive_clean_launches INTEGER NOT NULL DEFAULT 0,
    warnings_json TEXT NOT NULL,
    created TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
