-- The client's zones 107 / 115 are freezone_b_2012event / freezone_a_event2012 (Zones.tbl 1070 / 1150),
-- small maps of the freezone_a size. The server had them on 2003_karus_oldtime / freezone_a with
-- start positions from a much bigger map, so entering sent coordinates outside the client's map
-- (client crash). Use the matching SMDs and the Ardream (zone 72, same map size) start positions.
UPDATE zone_info SET smd_name = 'freezone_b_2012event.smd' WHERE zone_no = 107;
UPDATE zone_info SET smd_name = 'freezone_a_event2012.smd' WHERE zone_no = 115;

UPDATE start_position t
SET karus_x = s.karus_x, karus_z = s.karus_z, elmorad_x = s.elmorad_x, elmorad_z = s.elmorad_z,
    karus_gate_x = s.karus_gate_x, karus_gate_z = s.karus_gate_z,
    elmo_gate_x = s.elmo_gate_x, elmo_gate_z = s.elmo_gate_z,
    range_x = s.range_x, range_z = s.range_z
FROM start_position s
WHERE s.zone_id = 72 AND t.zone_id IN (107, 115);

-- guard towers again, around the corrected bases
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
