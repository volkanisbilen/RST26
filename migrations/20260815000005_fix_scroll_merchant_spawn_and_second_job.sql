-- The custom 29514 spawn is present server-side but is not rendered reliably by
-- the v2615 client.  Reuse the client-defined, otherwise unused 31508 National
-- Enchanter proto which already has a Moradon placement and the same visual PID.
-- No quest helper or Lua script is bound to 31508.
DELETE FROM npc_spawn
WHERE zone_id = 21 AND npc_id = 29514 AND is_monster = FALSE;

UPDATE npc_template
SET str_name = '[Scroll Merchant]',
    s_pid = 19005,
    by_group = 0,
    by_act_type = 1,
    by_type = 21,
    i_selling_group = 280000,
    by_search_range = 0,
    by_tracing_range = 0
WHERE s_sid = 31508 AND is_monster = FALSE;

UPDATE npc_spawn
SET left_x = 816,
    top_z = 553,
    num_npc = 1,
    spawn_range = 0,
    direction = 90,
    room = 0,
    act_type = 1
WHERE zone_id = 21 AND npc_id = 31508 AND is_monster = FALSE;

-- Keep this migration self-healing if a reduced database omitted the native row.
INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
)
SELECT 21, 31508, FALSE, 1, 0, 0, 0, 0, 816, 553, 1, 0, 30, 90, 0, NULL, 0
WHERE NOT EXISTS (
    SELECT 1 FROM npc_spawn
    WHERE zone_id = 21 AND npc_id = 31508 AND is_monster = FALSE
);
