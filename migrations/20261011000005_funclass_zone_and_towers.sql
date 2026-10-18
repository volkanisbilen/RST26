-- Fun Class (Cinderella War) runs on the mini maps like the C++ server (GetCindirellaForcedZone):
-- 107 Mini Colony Zone by default. Snow War (110) is not a Fun Class map.
UPDATE cindwar_setting SET zone_id = 107;

-- Guard towers around both bases of the Fun Class maps (107, 115).
-- 5400 = Karus guard tower (group 1), 5300 = El Morad guard tower (group 2); same templates
-- the Ronark Land spawns use. Positions are offsets from the nation start position.
DELETE FROM npc_spawn WHERE zone_id IN (107, 115) AND npc_id IN (5300, 5400) AND is_monster = FALSE;

INSERT INTO npc_spawn
    (zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
     special_type, trap_number, left_x, top_z, num_npc, spawn_range,
     regen_time, direction, dot_cnt, path, room)
SELECT sp.zone_id, 5400, FALSE, 104, 0, 0, 0, 0,
       sp.karus_x + o.dx, sp.karus_z + o.dz, 1, 0, 30, 0, 0, NULL, 0
FROM start_position sp
CROSS JOIN (VALUES (14, 14), (-14, 14), (14, -14), (-14, -14), (0, 24), (24, 0)) AS o(dx, dz)
WHERE sp.zone_id IN (107, 115)
UNION ALL
SELECT sp.zone_id, 5300, FALSE, 104, 0, 0, 0, 0,
       sp.elmorad_x + o.dx, sp.elmorad_z + o.dz, 1, 0, 30, 0, 0, NULL, 0
FROM start_position sp
CROSS JOIN (VALUES (14, 14), (-14, 14), (14, -14), (-14, -14), (0, 24), (24, 0)) AS o(dx, dz)
WHERE sp.zone_id IN (107, 115);
