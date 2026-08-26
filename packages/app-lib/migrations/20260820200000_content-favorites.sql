CREATE TABLE content_favorites (
	provider TEXT NOT NULL CHECK (provider IN ('modrinth', 'curseforge')),
	project_id TEXT NOT NULL CHECK (length(project_id) > 0),
	content_type TEXT NOT NULL CHECK (content_type IN ('mod', 'resourcepack', 'datapack', 'shader')),
	saved_at INTEGER NOT NULL,

	PRIMARY KEY (provider, project_id)
);

CREATE INDEX content_favorites_saved_at_idx ON content_favorites (saved_at DESC);
