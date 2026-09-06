-- Error telemetry is no longer collected or uploaded. Remove legacy local
-- samples and queued error payloads while retaining the tables for upgrades
-- from older installations.
DELETE FROM telemetry_outbox WHERE event_type <> 'heartbeat';
DELETE FROM telemetry_error_daily;
