-- Resurrection scroll audit (type-5 RESURRECTION_SELF rows, live DB 2026-10-19).
--
-- Every scroll item in the live item table points at an existing magic row whose
-- magic_type5 exp_recover matches the item name/description (50/60/80/90/100 %):
--   910022000->480002 (50), 800036000->480003 (60), 800039000->480004 (100),
--   810036000->480006 (60), 900136000->480010 (80), 800103000->480011 (100),
--   800049000->480012 (100), 810277000->480015 (80), 900699000->480017 (60),
--   811061000->480018 (100, repointed from the missing 494040 earlier),
--   910948000->488031 (80), 811010000->488032 (90).
-- Orphan magic rows 480005/480007/480008/480009 reference items that do not exist
-- (800041000 is the HP Rice Cake, 800095000/800280000/800097000 are absent); harmless.
--
-- Only data defects found: the 480018 magic row has no English name / description
-- (the client's own Skill_Magic_Main row 480018 is unnamed too) and 480015's Korean name
-- says (50) for an 80 % scroll. Idempotent cosmetic fixes:
UPDATE magic SET en_name = 'Resurrection Scroll(100)'
 WHERE magic_num = 480018 AND (en_name IS NULL OR en_name = '');
UPDATE magic SET description = 'Recover 100% of the experience point'
 WHERE magic_num = 480018 AND (description IS NULL OR description = '');
UPDATE magic SET kr_name = 'Resurrection Scroll(80)'
 WHERE magic_num = 480015 AND kr_name = 'Resurrection Scroll(50)';
