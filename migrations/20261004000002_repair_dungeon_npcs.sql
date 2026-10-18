-- Restore interactive dungeon gates and correct Delos' NPC/monster spawn flags.
-- The imported 2625 spawn list reused IDs that have both an NPC and monster
-- template.  In these zones the rows are interactive objects, so they must
-- resolve through the non-monster template.

-- Delos castle town NPC positions were accidentally loaded as monster rows;
-- this is why guards/merchants appeared as Lycan, Loup-garou, etc.
UPDATE npc_spawn
SET is_monster = FALSE
WHERE zone_id = 30
  AND npc_id IN (501, 502, 503, 504, 505)
  AND is_monster = TRUE;

-- 7001 is the dungeon teleport-gate template in both abyss maps.  The source
-- spawn data contains one gate per floor, but the rows were marked as monster
-- (Orc bandit), leaving no usable floor transition NPCs.
UPDATE npc_spawn
SET is_monster = FALSE
WHERE zone_id IN (32, 33)
  AND npc_id = 7001
  AND is_monster = TRUE;

-- Moradon gate dialogs must not advertise the Abyss destinations.  The
-- dungeon remains reachable through the Delos gatekeeper/keys.
DELETE FROM quest_menu WHERE i_num IN (4214, 4215);
DELETE FROM quest_talk WHERE i_num IN (4214, 4215);
