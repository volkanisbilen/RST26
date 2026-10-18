-- The client-native Premium Merchant proto 29515 is placed at 816,555 in
-- KO_DATABASE_SERVER_001.dbo.K_NPCPOS.  Keeping the same verified point avoids
-- the model being hidden by Moradon's nearby static geometry at 816,553.
UPDATE npc_spawn
SET left_x = 816,
    top_z = 555,
    direction = 90,
    spawn_range = 0,
    num_npc = 1,
    act_type = 1
WHERE zone_id = 21
  AND npc_id = 29515
  AND is_monster = FALSE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM npc_spawn s
        JOIN npc_template t
          ON t.s_sid = s.npc_id
         AND t.is_monster = s.is_monster
        WHERE s.zone_id = 21
          AND s.npc_id = 29515
          AND s.is_monster = FALSE
          AND s.left_x = 816
          AND s.top_z = 555
          AND s.num_npc = 1
          AND t.s_pid = 30700
          AND t.i_selling_group = 280000
    ) THEN
        RAISE EXCEPTION 'Native Moradon scroll merchant position was not applied';
    END IF;
END $$;
