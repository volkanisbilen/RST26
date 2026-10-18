-- Move the Moradon Chaotic Generator and add the requested Sundries and Inn
-- Hostess using existing client-supported NPC templates.

-- There should be a single Moradon Chaotic Generator at the requested point.
DELETE FROM npc_spawn
WHERE zone_id = 21 AND npc_id = 603;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
) VALUES
    (21, 603, FALSE, 1, 0, 0, 0, 0, 815, 640, 1, 0, 30, 90, 0, NULL, 0);

-- 8162 is the existing Moradon-compatible Sundries template/shop.
DELETE FROM npc_spawn
WHERE zone_id = 21 AND npc_id = 8162 AND is_monster = FALSE
  AND left_x = 810 AND top_z = 631;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
) VALUES
    (21, 8162, FALSE, 1, 0, 0, 0, 0, 810, 631, 1, 0, 30, 90, 0, NULL, 0);

-- 16096 is the existing Moradon Inn Hostess template; keep the original
-- spawns elsewhere and add this additional copy at the requested location.
DELETE FROM npc_spawn
WHERE zone_id = 21 AND npc_id = 16096 AND is_monster = FALSE
  AND left_x = 822 AND top_z = 631;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
) VALUES
    (21, 16096, FALSE, 1, 0, 0, 0, 0, 822, 631, 1, 0, 30, 90, 0, NULL, 0);
