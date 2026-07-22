ALTER TABLE settings ADD COLUMN auto_concurrent_downloads INTEGER NOT NULL DEFAULT TRUE;
ALTER TABLE settings
ADD COLUMN minecraft_metadata_source TEXT NOT NULL DEFAULT 'auto' CHECK (
	minecraft_metadata_source IN ('auto', 'official_only', 'mirror_preferred')
);
ALTER TABLE settings
ADD COLUMN minecraft_file_source TEXT NOT NULL DEFAULT 'auto' CHECK (
	minecraft_file_source IN ('auto', 'official_only', 'mirror_preferred')
);
ALTER TABLE settings
ADD COLUMN modrinth_source TEXT NOT NULL DEFAULT 'auto' CHECK (
	modrinth_source IN ('auto', 'official_only', 'mirror_preferred')
);
ALTER TABLE settings
ADD COLUMN curseforge_source TEXT NOT NULL DEFAULT 'auto' CHECK (
	curseforge_source IN ('auto', 'official_only', 'mirror_preferred')
);

UPDATE settings
SET
	auto_concurrent_downloads = TRUE,
	max_concurrent_downloads = MIN(MAX(max_concurrent_downloads, 1), 64),
	minecraft_metadata_source = CASE
		WHEN use_minecraft_mirror = FALSE
		AND use_modrinth_mirror = TRUE
		AND use_curseforge_mirror = TRUE THEN 'auto'
		WHEN use_minecraft_mirror = TRUE THEN 'mirror_preferred'
		ELSE 'official_only'
	END,
	minecraft_file_source = CASE
		WHEN use_minecraft_mirror = FALSE
		AND use_modrinth_mirror = TRUE
		AND use_curseforge_mirror = TRUE THEN 'auto'
		WHEN use_minecraft_mirror = TRUE THEN 'mirror_preferred'
		ELSE 'official_only'
	END,
	modrinth_source = CASE
		WHEN use_minecraft_mirror = FALSE
		AND use_modrinth_mirror = TRUE
		AND use_curseforge_mirror = TRUE THEN 'auto'
		WHEN use_modrinth_mirror = TRUE THEN 'mirror_preferred'
		ELSE 'official_only'
	END,
	curseforge_source = CASE
		WHEN use_minecraft_mirror = FALSE
		AND use_modrinth_mirror = TRUE
		AND use_curseforge_mirror = TRUE THEN 'auto'
		WHEN use_curseforge_mirror = TRUE THEN 'mirror_preferred'
		ELSE 'official_only'
	END;
