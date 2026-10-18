-- 29514 is present in the active v2615 NPC_us.tbl as [Scroll Merchant]
-- (PID 19005). Use that exact client identity for the Moradon shop instead of
-- a visually unrelated premium-merchant fallback.
UPDATE npc_template
SET str_name = '[Scroll Merchant]',
    s_pid = 19005,
    by_group = 0,
    by_act_type = 0,
    by_type = 21,
    i_selling_group = 280000,
    by_search_range = 0,
    by_tracing_range = 0
WHERE s_sid = 29514 AND is_monster = FALSE;

DELETE FROM npc_spawn
WHERE zone_id = 21
  AND npc_id IN (29514, 29515)
  AND is_monster = FALSE;

INSERT INTO npc_spawn
    (zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
     special_type, trap_number, left_x, top_z, num_npc, spawn_range,
     regen_time, direction, dot_cnt, path, room)
VALUES
    (21, 29514, FALSE, 0, 0, 0, 0, 0, 816, 553, 1, 0, 0, 90, 0, NULL, 0);
