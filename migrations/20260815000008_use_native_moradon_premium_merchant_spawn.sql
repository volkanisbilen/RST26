-- The previous custom spawn used 29513, which exists in NPC.tbl but is not a
-- native Moradon spawn in the reference database.  The v2369 reference
-- K_NPCPOS places client-defined proto 29515 ([Premium Merchant], pid 30700)
-- at 816,555.  Reuse that proven Moradon model/proto for the requested shop
-- and place it at the requested 816,553 coordinate.
--
-- Only the server-side function/name/selling group are changed.  The client
-- already owns NPC.tbl row 29515, so no client patch is required for it to
-- render.
UPDATE npc_template
SET str_name = '[Scroll Merchant]',
    s_pid = 30700,
    s_size = 100,
    by_group = 3,
    by_act_type = 1,
    by_type = 21,
    i_selling_group = 280000,
    i_hp_point = GREATEST(i_hp_point, 1000),
    by_search_range = 0,
    by_tracing_range = 0
WHERE s_sid = 29515 AND is_monster = FALSE;

-- Remove the non-native attempts and create exactly one deterministic spawn.
DELETE FROM npc_spawn
WHERE zone_id = 21
  AND npc_id IN (29513, 29514, 29515)
  AND is_monster = FALSE;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
) VALUES (
    21, 29515, FALSE, 1, 0, 0,
    0, 0, 816, 553, 1, 0,
    30, 90, 0, NULL, 0
);

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
          AND s.top_z = 553
          AND s.num_npc = 1
          AND t.s_pid = 30700
          AND t.by_type = 21
          AND t.i_selling_group = 280000
    ) THEN
        RAISE EXCEPTION 'Moradon scroll merchant 29515 was not configured';
    END IF;
END $$;
