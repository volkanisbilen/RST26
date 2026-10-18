-- The live row pointed npc_id 602 at the monster namespace, resolving it as
-- "Wild smilodon" instead of the v2615 NPC_GATE2 "Bifrost Gate" template.
-- Keep the native spawn row/id and correct only its namespace and requested
-- Ronark Land position.
UPDATE npc_spawn
SET is_monster = FALSE,
    act_type = 100,
    left_x = 1021,
    top_z = 1013,
    num_npc = 1,
    spawn_range = 0,
    direction = 0
WHERE zone_id = 71
  AND npc_id = 602;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM npc_spawn s
        JOIN npc_template t
          ON t.s_sid = s.npc_id
         AND t.is_monster = s.is_monster
        WHERE s.zone_id = 71
          AND s.npc_id = 602
          AND s.is_monster = FALSE
          AND s.left_x = 1021
          AND s.top_z = 1013
          AND t.by_type = 150
    ) THEN
        RAISE EXCEPTION 'Ronark Land Bifrost Gate (NPC 602 / type 150) could not be configured';
    END IF;
END
$$;
