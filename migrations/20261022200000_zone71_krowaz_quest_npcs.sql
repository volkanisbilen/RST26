-- Zone 71 (Ronark Land): the Krowaz entrance riders (both nations) and the quest NPCs that had a Lua script and
-- quest_helper rows but no spawn / no template. Positions are estimates inside each nation's NPC cluster
-- (Karus x1345-1398 z1057-1113, El Morad x602-634 z883-938); fine-tune them in the admin map editor.

-- Hill (Karus, group 1) and Rily (El Morad, group 2) had no template: clone the nation's Magic Association NPC.
DO $$
DECLARE r record;
BEGIN
  FOR r IN SELECT * FROM (VALUES
      (15279, 'Hill', 29072), (15280, 'Hill', 29072), (31820, 'Hill', 29072), (31822, 'Hill', 29072),
      (31821, 'Rily', 29073), (31823, 'Rily', 29073)) AS v(sid, nm, src)
  LOOP
    IF NOT EXISTS (SELECT 1 FROM npc_template WHERE s_sid = r.sid AND NOT is_monster) THEN
      CREATE TEMP TABLE _t ON COMMIT DROP AS SELECT * FROM npc_template WHERE s_sid = r.src AND NOT is_monster;
      UPDATE _t SET s_sid = r.sid, str_name = r.nm;
      INSERT INTO npc_template SELECT * FROM _t;
      DROP TABLE _t;
    END IF;
  END LOOP;
END $$;

INSERT INTO npc_spawn (zone_id, npc_id, is_monster, act_type, left_x, top_z, num_npc, spawn_range, direction)
SELECT 71, v.npc, false, 0, v.x, v.z, 1, 0, v.dir
FROM (VALUES
  (32285, 1388, 1078, 0),   -- Krowaz rider (Karus)
  (25020, 1380, 1078, 0),   -- Pluto
  (15279, 1358, 1080, 0), (15280, 1358, 1085, 0), (31820, 1358, 1090, 0), (31822, 1358, 1095, 0),  -- Hill
  (32289,  620,  905, 0),   -- Krowaz rider (El Morad)
  (25006,  614,  912, 0),   -- Uranos
  (31821,  622,  915, 0), (31823,  622,  920, 0)                                                   -- Rily
) AS v(npc, x, z, dir)
WHERE NOT EXISTS (SELECT 1 FROM npc_spawn s WHERE s.zone_id = 71 AND s.npc_id = v.npc AND NOT s.is_monster);
