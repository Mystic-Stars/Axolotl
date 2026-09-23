ALTER TABLE settings ADD COLUMN custom_window_title_enabled INTEGER NOT NULL DEFAULT FALSE;
ALTER TABLE settings ADD COLUMN default_window_title TEXT NOT NULL DEFAULT 'Minecraft';
