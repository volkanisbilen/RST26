-- Field bots: remove the artificial population limits (was 40 per nation and zone, 150 total).
-- Only a technical safety ceiling remains: 1000 per nation and zone (5000 bot ids overall).
-- Adds live_note: the game server writes the reason a zone has fewer bots than configured
-- (shown in the admin panel). Idempotent. Table owner must stay koserver.

ALTER TABLE field_bot_zone ADD COLUMN IF NOT EXISTS live_note TEXT NOT NULL DEFAULT '';

DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'field_bot_zone_count_chk'
               AND conrelid = 'field_bot_zone'::regclass AND pg_get_constraintdef(oid) LIKE '%40%') THEN
        ALTER TABLE field_bot_zone DROP CONSTRAINT field_bot_zone_count_chk;
        ALTER TABLE field_bot_zone ADD CONSTRAINT field_bot_zone_count_chk
            CHECK (karus_count BETWEEN 0 AND 1000 AND elmorad_count BETWEEN 0 AND 1000);
    END IF;
END $$;
