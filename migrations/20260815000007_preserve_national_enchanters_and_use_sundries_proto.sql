-- Preserve all three native National Enchanters. 31508 is a shared proto and
-- must not be repurposed globally just to create one merchant.
UPDATE npc_template
SET str_name = '[National Enchanter]',
    s_pid = 19005,
    by_group = 3,
    by_act_type = 1,
    by_type = 213,
    i_selling_group = 0,
    by_search_range = 0,
    by_tracing_range = 35
WHERE s_sid = 31508 AND is_monster = FALSE;

DELETE FROM npc_spawn
WHERE zone_id = 21 AND npc_id = 31508 AND is_monster = FALSE;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
) VALUES
    (21, 31508, FALSE, 104, 0, 0, 0, 0, 825, 550, 1, 0, 75, 90, 0, NULL, 0),
    (21, 31508, FALSE, 104, 0, 0, 0, 0, 404, 528, 1, 0, 75, 90, 0, NULL, 0),
    (21, 31508, FALSE, 104, 0, 0, 0, 0, 681, 429, 1, 0, 75, 40, 0, NULL, 0);

-- 29513 is a client-defined Sundries merchant proto with a normal merchant
-- model and no native server spawn. Use it for the requested custom shop.
UPDATE npc_template
SET str_name = '[Scroll Merchant]',
    s_pid = 12100,
    by_group = 0,
    by_act_type = 1,
    by_type = 21,
    i_selling_group = 280000,
    by_search_range = 0,
    by_tracing_range = 0
WHERE s_sid = 29513 AND is_monster = FALSE;

DELETE FROM npc_spawn
WHERE zone_id = 21 AND npc_id IN (29513, 29514) AND is_monster = FALSE;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
) VALUES
    (21, 29513, FALSE, 1, 0, 0, 0, 0, 816, 553, 1, 0, 30, 90, 0, NULL, 0);
