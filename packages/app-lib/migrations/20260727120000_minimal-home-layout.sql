ALTER TABLE settings
ADD COLUMN home_layout TEXT NOT NULL DEFAULT 'standard'
CHECK (home_layout IN ('standard', 'minimal'));

ALTER TABLE settings
ADD COLUMN minimal_home_instance_id TEXT NULL
REFERENCES instances(id) ON DELETE SET NULL;
