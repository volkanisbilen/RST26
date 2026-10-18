-- Rotate regular Moradon NPC facing 90 degrees from the current DB value.
--
-- MORANKER statue facing is intentionally left untouched. Only the requested
-- A2/A6 pedestal coordinates are fixed below.

UPDATE npc_spawn
   SET direction = MOD(MOD(direction, 256) + 64, 256)
 WHERE zone_id = 21
   AND is_monster = FALSE
   AND npc_id NOT BETWEEN 31882 AND 31887;

UPDATE moraranker_statue_slot
   SET x = 850.0,
       z = 561.0
 WHERE slot_index = 4
   AND npc_type = 'V';

UPDATE moraranker_statue_slot
   SET x = 773.0,
       z = 561.0
 WHERE slot_index = 2
   AND npc_type = 'T';
