ALTER TABLE instances
ADD COLUMN pinned_at INTEGER NULL;

CREATE TABLE instance_daily_playtime (
	played_on TEXT NOT NULL,
	instance_id TEXT NOT NULL,
	instance_name TEXT NOT NULL,
	played_seconds INTEGER NOT NULL DEFAULT 0,
	session_count INTEGER NOT NULL DEFAULT 0,

	PRIMARY KEY (played_on, instance_id)
);

CREATE INDEX instance_daily_playtime_played_on
	ON instance_daily_playtime(played_on);
