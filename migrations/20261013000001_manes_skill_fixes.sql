-- Manes Survival skills that had no data and therefore always failed:
--   Wild Advent, Blink, Rush, Pull+1..+4 (no MAGIC_TYPE8 row) and Instantly Magic (empty MAGIC row)
BEGIN;

INSERT INTO magic_type8 (i_num, "name", description, target, radius, warp_type, exp_recover, kick_distance)
SELECT d.dst, s."name", s.description, s.target, s.radius, s.warp_type, s.exp_recover, s.kick_distance
FROM (VALUES (491242, 108770), (491278, 110774), (491321, 114509),
             (491322, 115570), (491323, 115570), (491324, 115570), (491325, 115570)) AS d(dst, src)
JOIN magic_type8 s ON s.i_num = d.src
ON CONFLICT (i_num) DO UPDATE SET
    "name" = EXCLUDED."name", description = EXCLUDED.description, target = EXCLUDED.target,
    radius = EXCLUDED.radius, warp_type = EXCLUDED.warp_type,
    exp_recover = EXCLUDED.exp_recover, kick_distance = EXCLUDED.kick_distance;

-- Instantly Magic: a self buff like Concentration (its MAGIC_TYPE4 row, buff 23, already exists)
UPDATE magic SET
    t_1 = 18, before_action = 0, target_action = 0, self_effect = 0, flying_effect = 0, target_effect = 702,
    moral = 1, skill_level = 1, skill = 3200, msp = 200, hp = 0, s_sp = 0, item_group = 9, use_item = 0,
    cast_time = 15, recast_time = 400, success_rate = 100, type1 = 4, type2 = 0, "range" = 0, etc = 0,
    use_standing = 0, skill_check = 0, icelightrate = 0
WHERE magic_num = 491284;

COMMIT;
