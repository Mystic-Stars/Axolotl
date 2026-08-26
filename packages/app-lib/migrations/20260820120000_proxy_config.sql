ALTER TABLE settings ADD COLUMN proxy_mode TEXT NOT NULL DEFAULT 'system'
    CHECK (proxy_mode IN ('none', 'system', 'custom'));
ALTER TABLE settings ADD COLUMN proxy_url TEXT NOT NULL DEFAULT '';
ALTER TABLE settings ADD COLUMN proxy_username TEXT NOT NULL DEFAULT '';
ALTER TABLE settings ADD COLUMN proxy_password TEXT NOT NULL DEFAULT '';
