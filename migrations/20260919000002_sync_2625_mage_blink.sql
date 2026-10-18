-- v2625 client truth: Skill_Magic_8.tbl identifies Blink as warp type 26.
-- The original PostgreSQL seed came from an older client and used type 20.
UPDATE magic_type8
SET warp_type = 26,
    radius = 0,
    kick_distance = 0
WHERE i_num IN (110774, 210774);

