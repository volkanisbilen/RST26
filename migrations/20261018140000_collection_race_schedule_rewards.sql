-- Collection Race: twelve runs a day (every 2 hours around the clock) + varied box rewards.
--
-- SCHEDULE. The game server starts a Collection Race by itself from collection_race_settings
-- (auto_start / auto_hour / auto_minute, server-local time, every day -- this table has no weekday
-- column) since the scheduler that reads them was added together with this migration
-- (handler/collection_race.rs::auto_start_tick). The twelve existing events are given one slot each:
-- event N starts at HH:15 with HH = 2 * (N - 1)   ->   00:15, 02:15, ... 22:15.
-- event_time is 90 minutes for all of them, so each run ends 30 minutes before the next one starts.
-- (HH:15 keeps clear of the other scheduled events: Manes 00:30/12:05/18:00, Ultima 07/17/23:00,
-- Rift 08:00/12:50/20:00, Bifrost :30/:45, FT :30, Fire & Ice :40.)
--
-- REWARDS. Each event keeps its existing rewards. The third "Bowl Event" reward of every event is a
-- random-item roll (item 900004000) with a pool id that the game never honoured (it always rolls pool
-- 0, which is empty), so it never paid anything. Those twelve rows now pay a real box instead, a
-- different box tier per event; event 2 (which had only two rows) gets a third row.
--   rate = chance in % that the reward is paid when the player finishes the race.
--
-- Safe to run more than once (rows are matched on their old content) and on a live server.
-- Apply BEFORE deploying the new game binary (the binary starts events from these rows):
--   sudo -u postgres psql -d ko_server -v ON_ERROR_STOP=1 \
--        -f migrations/20261018140000_collection_race_schedule_rewards.sql
-- The last statement queues a hot reload of collection_race_settings for the running server
-- (rewards are read from the table every time an event starts).
BEGIN;

-- 0) the boxes must exist in the item table
DO $$
DECLARE missing TEXT;
BEGIN
    SELECT string_agg(v.id::text, ', ') INTO missing
    FROM (VALUES (999915000),(811053000),(508194000),(810341000),(810678000),(379156000),
                 (509194000),(810596000),(379155000)) AS v(id)
    WHERE NOT EXISTS (SELECT 1 FROM item i WHERE i.num = v.id);
    IF missing IS NOT NULL THEN
        RAISE EXCEPTION 'collection race: reward boxes missing from item table: %', missing;
    END IF;
END $$;

-- 1) schedule: (event_index, hour, minute)
UPDATE collection_race_settings s
SET auto_start = TRUE, auto_hour = v.h, auto_minute = v.m
FROM (VALUES
-- SCHEDULE-BEGIN
    (1, 0, 15),
    (2, 2, 15),
    (3, 4, 15),
    (4, 6, 15),
    (5, 8, 15),
    (6, 10, 15),
    (7, 12, 15),
    (8, 14, 15),
    (9, 16, 15),
    (10, 18, 15),
    (11, 20, 15),
    (12, 22, 15)
-- SCHEDULE-END
) AS v(idx, h, m)
WHERE s.event_index = v.idx;

-- the schedule must be complete: twelve events, even hours, HH:15
DO $$
DECLARE n INT;
BEGIN
    SELECT count(*) INTO n FROM collection_race_settings
    WHERE event_index BETWEEN 1 AND 12 AND auto_start AND auto_minute = 15
      AND auto_hour = 2 * (event_index - 1) AND event_time BETWEEN 1 AND 119;
    IF n <> 12 THEN
        RAISE EXCEPTION 'collection race: expected settings 1..12 (event_time < 120 min), found % scheduled', n;
    END IF;
END $$;

-- 2) rewards: (idx of the old random row, event, description, box, count, rate %)
UPDATE collection_race_reward r
SET description = v.descr, item_id = v.item, item_count = v.cnt, rate = v.rate,
    item_time = 0, item_flag = 0, item_session = 0
FROM (VALUES
    (24, 1,  'Black Lion Weapon Box',      999915000, 2, 60),
    (2,  2,  'Akara Accessory Box',        811053000, 1, 50),
    (5,  3,  'Accessory Box',              508194000, 1, 40),
    (8,  4,  'Dragon Box',                 810341000, 1, 50),
    (11, 5,  'Old Draki Supply Box',       810678000, 1, 50),
    (14, 6,  'Akara Accessory Box',        811053000, 1, 50),
    (17, 7,  'Blue Treasure Chest',        379156000, 1, 50),
    (20, 8,  'Exp Item - Chitin Shell Box',509194000, 1, 35),
    (26, 9,  'Superior Draki Supply Box',  810596000, 1, 40),
    (29, 10, 'Dragon Box',                 810341000, 2, 50),
    (32, 11, 'Green Treasure Chest',       379155000, 1, 60),
    (35, 12, 'Black Lion Weapon Box',      999915000, 1, 60)
) AS v(idx, event_id, descr, item, cnt, rate)
WHERE r.idx = v.idx AND r.event_id = v.event_id AND r.item_id = 900004000;

-- event 2 only had two reward rows: add a third (second box)
INSERT INTO collection_race_reward
    (idx, event_id, description, item_id, item_count, rate, item_time, item_flag, item_session)
SELECT (SELECT COALESCE(MAX(idx), 0) + 1 FROM collection_race_reward),
       2, 'Dragon Box', 810341000, 1, 50, 0, 0, 0
WHERE EXISTS (SELECT 1 FROM collection_race_settings WHERE event_index = 2)
  AND NOT EXISTS (SELECT 1 FROM collection_race_reward WHERE event_id = 2 AND item_id = 810341000);

-- 3) hot reload of the event definitions on the running game server
INSERT INTO admin_command (command, args)
SELECT 'reload', '{"what":"collection_race"}'::jsonb
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'admin_command');

COMMIT;
