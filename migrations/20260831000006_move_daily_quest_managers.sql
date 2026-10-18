-- Move the dedicated Daily Quest Manager (proto 31999) to the requested
-- gate-side coordinates. Keep this separate from already-applied migrations
-- so sqlx migration checksums remain stable.
WITH requested_positions(zone_id, left_x, top_z) AS (
    VALUES
        (1::SMALLINT,  454::SMALLINT, 1619::SMALLINT),
        (2::SMALLINT, 1589::SMALLINT,  424::SMALLINT),
        (11::SMALLINT, 511::SMALLINT,  534::SMALLINT),
        (71::SMALLINT,1006::SMALLINT, 1013::SMALLINT)
)
UPDATE npc_spawn AS spawn
SET left_x = requested.left_x,
    top_z = requested.top_z,
    act_type = 7,
    spawn_range = 0,
    regen_time = 30
FROM requested_positions AS requested
WHERE spawn.zone_id = requested.zone_id
  AND spawn.npc_id = 31999
  AND spawn.is_monster = FALSE;

-- Fail instead of silently leaving a zone without its manager if an earlier
-- installation is missing one of the expected spawn rows.
DO $$
DECLARE
    missing_count INTEGER;
BEGIN
    SELECT COUNT(*)
    INTO missing_count
    FROM (VALUES (1::SMALLINT), (2::SMALLINT), (11::SMALLINT), (71::SMALLINT)) AS zones(zone_id)
    WHERE NOT EXISTS (
        SELECT 1
        FROM npc_spawn
        WHERE npc_spawn.zone_id = zones.zone_id
          AND npc_spawn.npc_id = 31999
          AND npc_spawn.is_monster = FALSE
    );

    IF missing_count <> 0 THEN
        RAISE EXCEPTION 'Daily Quest Manager spawn missing in % requested zone(s)', missing_count;
    END IF;
END $$;
