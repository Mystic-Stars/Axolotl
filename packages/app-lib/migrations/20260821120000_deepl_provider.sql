ALTER TABLE translation_settings ADD COLUMN deepl_api_endpoint TEXT NOT NULL DEFAULT 'https://api-free.deepl.com/v2/translate';
ALTER TABLE translation_settings ADD COLUMN deepl_api_key TEXT NULL;