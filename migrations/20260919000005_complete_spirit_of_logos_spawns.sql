-- Complete Spirit of Logos placements from the authoritative K_NPCPOS2369.
-- Migration 00004 restored the reduced project import; this adds every nation,
-- Eslant and Moradon counterpart without modifying an already-applied migration.

-- These rows belonged to the reduced import but are not present in the active
-- reference database. Remove only the exact rows introduced by 00004.
DELETE FROM npc_spawn
WHERE (zone_id = 12 AND npc_id BETWEEN 25210 AND 25217)
   OR (zone_id = 19 AND npc_id BETWEEN 25202 AND 25209);

WITH logos(zone_id, npc_id, left_x, top_z, spawn_range) AS (
    SELECT z.zone_id, n.npc_id, n.left_x, n.top_z, 0
    FROM (VALUES (1),(5),(6)) z(zone_id)
    CROSS JOIN (VALUES
        (25186,385,1177),(25187,1771,402),(25188,729,1583),(25189,1102,395),
        (25190,1323,1791),(25191,591,462),(25192,1689,1144),(25193,335,1021)
    ) n(npc_id,left_x,top_z)
    UNION ALL
    SELECT z.zone_id, n.npc_id, n.left_x, n.top_z, 0
    FROM (VALUES (2),(7),(8)) z(zone_id)
    CROSS JOIN (VALUES
        (25194,346,1127),(25195,330,1638),(25196,1056,1635),(25197,1607,995),
        (25198,1490,1579),(25199,1551,460),(25200,656,670),(25201,974,379)
    ) n(npc_id,left_x,top_z)
    UNION ALL
    SELECT z.zone_id, n.npc_id, n.left_x, n.top_z, 10
    FROM (VALUES (11),(12),(13),(14),(15),(16)) z(zone_id)
    CROSS JOIN (VALUES
        (25202,722,381),(25203,698,212),(25204,157,123),(25205,224,519),
        (25206,200,687),(25207,237,892),(25208,602,858),(25209,892,754)
    ) n(npc_id,left_x,top_z)
    UNION ALL
    SELECT z.zone_id, n.npc_id, n.left_x, n.top_z, 0
    FROM (VALUES (21),(22),(23),(24),(25)) z(zone_id)
    CROSS JOIN (VALUES
        (25178,603,410),(25179,395,718),(25180,518,312),(25181,297,777),
        (25182,255,388),(25183,350,902),(25184,493,563),(25185,517,841)
    ) n(npc_id,left_x,top_z)
)
INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
)
SELECT zone_id, npc_id, FALSE, 100, 0, 0, 0, 0, left_x, top_z,
       1, spawn_range, 32400, 0, 0, NULL, 0
FROM logos l
WHERE NOT EXISTS (
    SELECT 1 FROM npc_spawn s
    WHERE s.zone_id = l.zone_id AND s.npc_id = l.npc_id
      AND s.left_x = l.left_x AND s.top_z = l.top_z
);
