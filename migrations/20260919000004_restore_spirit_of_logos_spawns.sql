-- Restore the canonical Spirit of Logos placements removed by later zone rebuilds.
-- Source: legacy K_NPCPOS2369 / the project's complete NPC spawn import.
WITH logos(zone_id, npc_id, left_x, top_z, spawn_range) AS (
    VALUES
      (1,25186,385,1177,0),(1,25187,1771,402,0),(1,25188,729,1583,0),(1,25189,1102,395,0),
      (1,25190,1323,1791,0),(1,25191,591,462,0),(1,25192,1689,1144,0),(1,25193,335,1021,0),
      (2,25194,346,1127,0),(2,25195,330,1638,0),(2,25196,1056,1635,0),(2,25197,1607,995,0),
      (2,25198,1490,1579,0),(2,25199,1551,460,0),(2,25200,656,670,0),(2,25201,974,379,0),
      (11,25202,722,381,0),(11,25203,698,212,0),(11,25204,157,123,0),(11,25205,224,519,0),
      (11,25206,200,687,0),(11,25207,237,892,0),(11,25208,602,858,0),(11,25209,892,754,0),
      (12,25210,722,381,0),(12,25211,698,212,0),(12,25212,157,123,0),(12,25213,224,519,0),
      (12,25214,200,687,0),(12,25215,237,892,0),(12,25216,602,858,0),(12,25217,892,754,0),
      (19,25202,722,381,0),(19,25203,698,212,0),(19,25204,157,123,0),(19,25205,224,519,0),
      (19,25206,200,687,0),(19,25207,237,892,0),(19,25208,602,858,0),(19,25209,892,754,0)
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
