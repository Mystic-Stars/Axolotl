DROP TRIGGER custom_minecraft_skins_user_uuid_insert_check;
DROP TRIGGER custom_minecraft_skins_user_uuid_update_check;
DROP TRIGGER custom_minecraft_skins_user_uuid_update_cascade;
DROP TRIGGER offline_minecraft_skins_user_uuid_insert_check;
DROP TRIGGER offline_minecraft_skins_user_uuid_update_check;
DROP TRIGGER offline_minecraft_skins_user_uuid_update_cascade;

CREATE TABLE minecraft_users_yggdrasil (
	uuid TEXT NOT NULL,
	active INTEGER NOT NULL DEFAULT FALSE,
	username TEXT NOT NULL,
	access_token TEXT NOT NULL,
	refresh_token TEXT NOT NULL,
	expires INTEGER NOT NULL,
	account_type TEXT NOT NULL DEFAULT 'microsoft'
		CHECK (account_type IN ('microsoft', 'offline', 'yggdrasil')),
	yggdrasil_api_root TEXT NOT NULL DEFAULT '',
	yggdrasil_server_name TEXT NOT NULL DEFAULT '',
	yggdrasil_login TEXT NOT NULL DEFAULT '',
	yggdrasil_client_token TEXT NOT NULL DEFAULT '',

	PRIMARY KEY (uuid)
);

INSERT INTO minecraft_users_yggdrasil (
	uuid, active, username, access_token, refresh_token, expires, account_type
)
SELECT
	uuid, active, username, access_token, refresh_token, expires, account_type
FROM minecraft_users;

DROP TABLE minecraft_users;
ALTER TABLE minecraft_users_yggdrasil RENAME TO minecraft_users;

CREATE TRIGGER custom_minecraft_skins_user_uuid_insert_check
	BEFORE INSERT ON custom_minecraft_skins FOR EACH ROW
	BEGIN
		SELECT CASE WHEN NOT EXISTS (
			SELECT 1 FROM minecraft_users WHERE uuid = NEW.minecraft_user_uuid
		) THEN RAISE(ABORT, 'Cannot add a custom skin for an unknown Minecraft user UUID') END;
	END;

CREATE TRIGGER custom_minecraft_skins_user_uuid_update_check
	BEFORE UPDATE ON custom_minecraft_skins FOR EACH ROW
	BEGIN
		SELECT CASE WHEN NOT EXISTS (
			SELECT 1 FROM minecraft_users WHERE uuid = NEW.minecraft_user_uuid
		) THEN RAISE(ABORT, 'Cannot change a custom skin to refer to an unknown Minecraft user UUID') END;
	END;

CREATE TRIGGER custom_minecraft_skins_user_uuid_update_cascade
	AFTER UPDATE OF uuid ON minecraft_users FOR EACH ROW
	BEGIN
		UPDATE custom_minecraft_skins
		SET minecraft_user_uuid = NEW.uuid
		WHERE minecraft_user_uuid = OLD.uuid;
	END;

CREATE TRIGGER offline_minecraft_skins_user_uuid_insert_check
	BEFORE INSERT ON offline_minecraft_skins FOR EACH ROW
	BEGIN
		SELECT CASE WHEN NOT EXISTS (
			SELECT 1 FROM minecraft_users WHERE uuid = NEW.minecraft_user_uuid
		) THEN RAISE(ABORT, 'Cannot add an offline skin for an unknown Minecraft user UUID') END;
	END;

CREATE TRIGGER offline_minecraft_skins_user_uuid_update_check
	BEFORE UPDATE ON offline_minecraft_skins FOR EACH ROW
	BEGIN
		SELECT CASE WHEN NOT EXISTS (
			SELECT 1 FROM minecraft_users WHERE uuid = NEW.minecraft_user_uuid
		) THEN RAISE(ABORT, 'Cannot change an offline skin to refer to an unknown Minecraft user UUID') END;
	END;

CREATE TRIGGER offline_minecraft_skins_user_uuid_update_cascade
	AFTER UPDATE OF uuid ON minecraft_users FOR EACH ROW
	BEGIN
		UPDATE offline_minecraft_skins
		SET minecraft_user_uuid = NEW.uuid
		WHERE minecraft_user_uuid = OLD.uuid;
	END;
