-- Juraid Mountain rooms are runtime-owned. Leaving these 2615 Juraid SIDs as
-- static room spawns lets them mix with the event wave/gate state machine.
DELETE FROM npc_spawn
WHERE zone_id = 87
  AND room > 0
  AND npc_id IN (8100, 8101, 8102, 8103, 8104, 8105, 8106, 8110, 8113, 8114);
