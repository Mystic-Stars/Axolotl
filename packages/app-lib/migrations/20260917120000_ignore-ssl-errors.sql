ALTER TABLE settings
ADD COLUMN ignore_ssl_errors INTEGER NOT NULL DEFAULT FALSE
CHECK (ignore_ssl_errors IN (0, 1));
