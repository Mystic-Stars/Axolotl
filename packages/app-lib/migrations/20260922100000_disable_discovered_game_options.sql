-- Discovered options are catalogued for review, but must not be synced until
-- the user explicitly enables them.
UPDATE synced_game_option_preferences
SET enabled = 0
WHERE source = 'discovery_default' AND enabled = 1;
