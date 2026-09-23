-- Axolotl equivalent migration for instance settings synchronization and the
-- unified screenshot index. Kept as one atomic migration for this fork.

CREATE TABLE sync_feature_settings (
	feature TEXT PRIMARY KEY NOT NULL,
	globally_enabled INTEGER NOT NULL CHECK (globally_enabled IN (0, 1)),
	new_instance_default INTEGER NOT NULL CHECK (new_instance_default IN (0, 1))
);

INSERT INTO sync_feature_settings (feature, globally_enabled, new_instance_default)
VALUES
	('command_history', 0, 0),
	('multiplayer_servers', 0, 0),
	('creative_hotbars', 0, 0),
	('screenshots', 0, 0),
	('game_options', 0, 0),
	('resource_packs', 0, 0),
	('data_packs', 0, 0);

CREATE TABLE instance_sync_preferences (
	instance_id TEXT NOT NULL,
	feature TEXT NOT NULL,
	enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
	PRIMARY KEY (instance_id, feature),
	FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE,
	FOREIGN KEY (feature) REFERENCES sync_feature_settings (feature) ON DELETE CASCADE
);

CREATE INDEX instance_sync_preferences_feature_enabled
	ON instance_sync_preferences (feature, enabled);

INSERT INTO instance_sync_preferences (instance_id, feature, enabled)
SELECT instances.id, features.feature, features.enabled
FROM instances
CROSS JOIN (
	SELECT 'command_history' AS feature, 0 AS enabled
	UNION ALL SELECT 'multiplayer_servers', 0
	UNION ALL SELECT 'creative_hotbars', 0
	UNION ALL SELECT 'screenshots', 0
) AS features;

INSERT INTO instance_sync_preferences (instance_id, feature, enabled)
SELECT id, 'game_options', 0 FROM instances;

INSERT INTO instance_sync_preferences (instance_id, feature, enabled)
SELECT instances.id, features.feature, 0
FROM instances
CROSS JOIN (
	SELECT 'resource_packs' AS feature
	UNION ALL SELECT 'data_packs'
) AS features;

CREATE TABLE synced_game_option_state (
	singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
	revision INTEGER NOT NULL CHECK (revision >= 0),
	catalog_revision INTEGER NOT NULL CHECK (catalog_revision >= 1)
);

CREATE TABLE synced_game_option_values (
	option_id TEXT PRIMARY KEY NOT NULL,
	kind TEXT NOT NULL CHECK (kind IN ('vanilla', 'external')),
	raw_key TEXT,
	canonical_type TEXT NOT NULL,
	canonical_value_json TEXT,
	value_codec TEXT NOT NULL,
	seeded INTEGER NOT NULL CHECK (seeded IN (0, 1)),
	revision INTEGER NOT NULL CHECK (revision >= 0),
	origin TEXT NOT NULL CHECK (origin IN ('app_editor', 'instance', 'source_seed')),
	source_game_version TEXT,
	source_instance_id TEXT,
	updated_at INTEGER NOT NULL
);

CREATE TABLE synced_game_option_preferences (
	option_id TEXT PRIMARY KEY NOT NULL,
	enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
	source TEXT NOT NULL CHECK (source IN ('catalog_default', 'discovery_default', 'user')),
	revision INTEGER NOT NULL CHECK (revision >= 0)
);

CREATE TABLE instance_game_option_pack_bases (
	instance_id TEXT PRIMARY KEY NOT NULL,
	pack_version_id TEXT,
	source TEXT NOT NULL CHECK (source IN ('client_overrides', 'overrides', 'none')),
	sha1 TEXT,
	encoding TEXT,
	document BLOB,
	FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE
);

CREATE TABLE instance_game_option_update_state (
	instance_id TEXT PRIMARY KEY NOT NULL,
	had_file INTEGER NOT NULL CHECK (had_file IN (0, 1)),
	sha1 TEXT,
	document BLOB,
	FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE
);

CREATE TABLE game_option_locale_origins (
	scope TEXT NOT NULL,
	option_id TEXT NOT NULL,
	source_instance_id TEXT,
	source_game_version TEXT,
	backfilled INTEGER NOT NULL DEFAULT 0,
	observation_json TEXT,
	origin_json TEXT,
	PRIMARY KEY (scope, option_id)
);

INSERT INTO game_option_locale_origins
	(scope, option_id, source_instance_id, source_game_version, backfilled)
SELECT '', option_id, source_instance_id, source_game_version, 1
FROM synced_game_option_values;

CREATE TRIGGER record_game_option_locale_origin
AFTER INSERT ON synced_game_option_values
BEGIN
	INSERT INTO game_option_locale_origins
		(scope, option_id, source_instance_id, source_game_version)
	VALUES ('', NEW.option_id, NEW.source_instance_id, NEW.source_game_version)
	ON CONFLICT(scope, option_id) DO NOTHING;
END;

CREATE TABLE instance_sync_checkpoints (
	instance_id TEXT NOT NULL,
	feature TEXT NOT NULL,
	variant TEXT NOT NULL CHECK (variant IN ('default', 'legacy', 'components')),
	expected_sha1 TEXT NOT NULL,
	merge_base BLOB,
	source_revision INTEGER NOT NULL CHECK (source_revision >= 0),
	status TEXT NOT NULL CHECK (status IN ('pending', 'ready')),
	link_mode TEXT CHECK (link_mode IS NULL OR link_mode IN ('copy', 'hard', 'symbolic')),
	PRIMARY KEY (instance_id, feature, variant),
	FOREIGN KEY (instance_id, feature)
		REFERENCES instance_sync_preferences (instance_id, feature) ON DELETE CASCADE
);

CREATE TABLE synced_hotbar_state (
	singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
	schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
	revision INTEGER NOT NULL CHECK (revision >= 0),
	nbt BLOB NOT NULL
);

CREATE TABLE synced_server_state (
	singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
	revision INTEGER NOT NULL CHECK (revision >= 0)
);

CREATE TABLE synced_servers (
	id TEXT PRIMARY KEY NOT NULL,
	position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
	nbt BLOB NOT NULL
);

CREATE TABLE instance_servers (
	instance_id TEXT NOT NULL,
	id TEXT NOT NULL,
	source TEXT NOT NULL CHECK (source IN ('modpack', 'local_desynced')),
	excluded_synced_server_id TEXT,
	nbt BLOB NOT NULL,
	position INTEGER NOT NULL CHECK (position >= 0),
	PRIMARY KEY (instance_id, id),
	UNIQUE (instance_id, position),
	FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE,
	FOREIGN KEY (excluded_synced_server_id) REFERENCES synced_servers (id) ON DELETE SET NULL
);

CREATE TABLE instance_server_projection_entries (
	instance_id TEXT NOT NULL,
	owner TEXT NOT NULL CHECK (owner IN ('synced', 'instance')),
	server_id TEXT NOT NULL,
	nbt BLOB NOT NULL,
	position INTEGER NOT NULL CHECK (position >= 0),
	PRIMARY KEY (instance_id, owner, server_id),
	UNIQUE (instance_id, position),
	FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE
);

CREATE TABLE instance_server_pack_state (
	instance_id TEXT PRIMARY KEY NOT NULL,
	version_id TEXT,
	FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE
);

CREATE TABLE screenshots (
	id TEXT NOT NULL PRIMARY KEY,
	instance_id TEXT NOT NULL,
	file_name TEXT NOT NULL,
	content_hash TEXT NOT NULL,
	file_size INTEGER NOT NULL,
	modified_at INTEGER NOT NULL,
	created_at INTEGER NOT NULL,
	UNIQUE (instance_id, file_name),
	FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
);

CREATE INDEX screenshots_instance_id ON screenshots(instance_id);
CREATE INDEX screenshots_instance_hash ON screenshots(instance_id, content_hash, file_size);

CREATE TABLE screenshot_groups (
	id TEXT NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	display_order INTEGER NOT NULL DEFAULT 0,
	CHECK (length(trim(name)) > 0)
);

CREATE INDEX screenshot_groups_display_order ON screenshot_groups(display_order);

CREATE TABLE screenshot_group_memberships (
	screenshot_id TEXT NOT NULL PRIMARY KEY,
	group_id TEXT NOT NULL,
	FOREIGN KEY (screenshot_id) REFERENCES screenshots(id) ON DELETE CASCADE,
	FOREIGN KEY (group_id) REFERENCES screenshot_groups(id) ON DELETE CASCADE
);

CREATE INDEX screenshot_group_memberships_group_id ON screenshot_group_memberships(group_id);

ALTER TABLE screenshots ADD COLUMN editor_state TEXT;
ALTER TABLE settings ADD COLUMN sync_features_across_devices INTEGER NOT NULL DEFAULT FALSE;

ALTER TABLE settings ADD COLUMN show_files_tab_in_instances INTEGER NOT NULL DEFAULT TRUE CHECK (show_files_tab_in_instances IN (0, 1));
ALTER TABLE settings ADD COLUMN show_worlds_tab_in_instances INTEGER NOT NULL DEFAULT TRUE CHECK (show_worlds_tab_in_instances IN (0, 1));
ALTER TABLE settings ADD COLUMN show_screenshots_tab_in_instances INTEGER NOT NULL DEFAULT FALSE CHECK (show_screenshots_tab_in_instances IN (0, 1));
ALTER TABLE settings ADD COLUMN show_skin_selector_in_sidebar INTEGER NOT NULL DEFAULT TRUE CHECK (show_skin_selector_in_sidebar IN (0, 1));

UPDATE settings SET
	show_files_tab_in_instances = COALESCE(json_extract(feature_flags, '$.show_files_tab_in_instances'), TRUE),
	show_worlds_tab_in_instances = COALESCE(json_extract(feature_flags, '$.show_worlds_tab_in_instances'), TRUE),
	show_screenshots_tab_in_instances = COALESCE(json_extract(feature_flags, '$.show_screenshots_tab_in_instances'), FALSE),
	show_skin_selector_in_sidebar = COALESCE(json_extract(feature_flags, '$.show_skin_selector_in_sidebar'), TRUE),
	feature_flags = json_remove(feature_flags,
		'$.show_files_tab_in_instances', '$.show_worlds_tab_in_instances',
		'$.show_screenshots_tab_in_instances', '$.show_skin_selector_in_sidebar');

CREATE TABLE synced_pack_catalog (
	id TEXT PRIMARY KEY NOT NULL,
	project_type TEXT NOT NULL CHECK (project_type IN ('resourcepack', 'datapack')),
	file_name TEXT NOT NULL,
	sha1 TEXT NOT NULL UNIQUE,
	size INTEGER NOT NULL CHECK (size >= 0),
	game_versions_json TEXT NOT NULL DEFAULT '[]',
	enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
	created_at INTEGER NOT NULL,
	modified_at INTEGER NOT NULL
);

CREATE INDEX synced_pack_catalog_type ON synced_pack_catalog(project_type, modified_at);

CREATE TABLE synced_pack_instances (
	pack_id TEXT NOT NULL,
	instance_id TEXT NOT NULL,
	excluded INTEGER NOT NULL DEFAULT 0 CHECK (excluded IN (0, 1)),
	materialized_path TEXT,
	modified_at INTEGER NOT NULL,
	PRIMARY KEY (pack_id, instance_id),
	FOREIGN KEY (pack_id) REFERENCES synced_pack_catalog(id) ON DELETE CASCADE,
	FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
);

CREATE INDEX synced_pack_instances_instance ON synced_pack_instances(instance_id, excluded);
