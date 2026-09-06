-- 0005: stop retaining legacy error telemetry.
--
-- Error events are no longer accepted by the worker. Keep the historical
-- tables in place so older database snapshots and dashboard migrations remain
-- readable, but remove all previously collected error payloads and reset the
-- error counters exposed through daily_totals.

DELETE FROM error_reports;
DELETE FROM error_daily_installations;
DELETE FROM error_group_installations;
DELETE FROM error_daily;
DELETE FROM error_range_stats;
DELETE FROM error_groups;
DELETE FROM error_context_reservations;
DELETE FROM error_context_samples;
DELETE FROM error_context_budget;
-- Platforms are repopulated from heartbeat batches and may previously have
-- been introduced only by an error report.
DELETE FROM platforms;

UPDATE daily_totals
SET
	error_occurrences = 0,
	distinct_error_groups = 0;
