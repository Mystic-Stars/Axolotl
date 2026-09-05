-- Direct association ("直接关联") instances point at an externally managed
-- traditional .minecraft layout without copying, symlinking, or reinstalling
-- anything. All columns are NULL for ordinary and pre-migration instances.
ALTER TABLE instances ADD COLUMN linked_launcher TEXT NULL;
ALTER TABLE instances ADD COLUMN linked_launcher_root TEXT NULL;
ALTER TABLE instances ADD COLUMN linked_dot_minecraft TEXT NULL;
ALTER TABLE instances ADD COLUMN linked_version_id TEXT NULL;
ALTER TABLE instances ADD COLUMN linked_version_json_path TEXT NULL;
