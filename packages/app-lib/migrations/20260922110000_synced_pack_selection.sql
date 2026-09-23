ALTER TABLE synced_pack_catalog ADD COLUMN selected INTEGER NOT NULL DEFAULT 1 CHECK (selected IN (0, 1));
ALTER TABLE synced_pack_catalog ADD COLUMN selection_order INTEGER;
