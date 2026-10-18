-- Fix Draki Tower combat/rest spawn flags for existing databases.
-- Combat stages must spawn monsters with AI; rest stages must spawn NPCs.

UPDATE draki_monster_list AS dml
SET is_monster = (dts.draki_tower_npc_state = 0)
FROM draki_tower_stages AS dts
WHERE dts.id = dml.stage_id
  AND dml.is_monster IS DISTINCT FROM (dts.draki_tower_npc_state = 0);
