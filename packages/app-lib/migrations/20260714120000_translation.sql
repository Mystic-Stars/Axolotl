CREATE TABLE translation_settings (
	id INTEGER NOT NULL CHECK (id = 0),
	provider TEXT NOT NULL DEFAULT 'microsoft',
	target_language TEXT NOT NULL DEFAULT '',
	mode TEXT NOT NULL DEFAULT 'bilingual',
	auto_translate INTEGER NOT NULL DEFAULT FALSE,
	style TEXT NOT NULL DEFAULT 'weakened',
	openai_base_url TEXT NOT NULL DEFAULT 'https://api.openai.com/v1',
	openai_model TEXT NOT NULL DEFAULT 'gpt-4o-mini',
	openai_api_key TEXT NULL,
	deeplx_base_url TEXT NOT NULL DEFAULT 'https://api.deeplx.org/{{apiKey}}/translate',
	deeplx_api_key TEXT NULL,
	PRIMARY KEY (id)
);

INSERT INTO translation_settings (id) VALUES (0);

CREATE TABLE translation_cache (
	key TEXT NOT NULL PRIMARY KEY,
	translation TEXT NOT NULL,
	created_at INTEGER NOT NULL
);

CREATE INDEX translation_cache_created_at ON translation_cache(created_at);
