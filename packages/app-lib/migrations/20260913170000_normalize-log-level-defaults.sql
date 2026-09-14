-- The original log-level migration defaulted existing rows to TRACE. Move
-- those rows to the new Release default; Beta normalizes this to DEBUG during
-- application initialization.
UPDATE settings SET log_level = 'info' WHERE log_level = 'trace';
