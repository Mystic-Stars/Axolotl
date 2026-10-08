ALTER TABLE settings ADD COLUMN doh_enabled INTEGER NOT NULL DEFAULT TRUE
    CHECK (doh_enabled IN (0, 1));
ALTER TABLE settings ADD COLUMN doh_server TEXT NOT NULL DEFAULT 'https://doh.pub/dns-query';
