-- Custom font families for the launcher interface and monospace surfaces.
-- NULL means "follow the launcher default".
ALTER TABLE settings ADD COLUMN ui_font TEXT;
ALTER TABLE settings ADD COLUMN mono_font TEXT;
