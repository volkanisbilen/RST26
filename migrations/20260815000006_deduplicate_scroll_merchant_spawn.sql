-- 31508 had three legacy Moradon spawn rows. Keep exactly one merchant at the
-- requested coordinate so the client does not receive overlapping NPC actors.
DELETE FROM npc_spawn
WHERE zone_id = 21
  AND npc_id = 31508
  AND is_monster = FALSE
  AND id NOT IN (
      SELECT MIN(id)
      FROM npc_spawn
      WHERE zone_id = 21 AND npc_id = 31508 AND is_monster = FALSE
  );

UPDATE npc_spawn
SET left_x = 816,
    top_z = 553,
    num_npc = 1,
    spawn_range = 0,
    direction = 90,
    room = 0,
    act_type = 1
WHERE zone_id = 21 AND npc_id = 31508 AND is_monster = FALSE;
