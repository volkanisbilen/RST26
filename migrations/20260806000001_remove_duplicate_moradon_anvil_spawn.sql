-- Moradon already loads its interactive anvil from object_event_pos (s_index 5001).
-- The nearby npc_spawn copy creates a second runtime NPC on the same client object.
DELETE FROM npc_spawn
WHERE zone_id = 21
  AND npc_id = 5001
  AND is_monster = FALSE
  AND left_x = 816
  AND top_z = 607;
