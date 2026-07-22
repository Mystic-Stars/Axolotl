UPDATE settings
SET max_concurrent_downloads = MIN(MAX(max_concurrent_downloads, 1), 256);
