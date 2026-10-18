-- Skill audit (docs/SKILL_AUDIT.md): data fixes found by scripts/skill_audit.py.
-- Every statement is idempotent and live-safe: UPDATEs are guarded by the defective value, INSERTs by NOT EXISTS.

-- ---------------------------------------------------------------------------------------------
-- 1. Master-class "Instantly Magic" (Necromancer 110820 / Enchanter 210820) were seeded as rows with
--    every numeric column NULL, so the skill could never be cast.  The values below are the client's own
--    Skill_Magic_Main row (the magic_type4 row, buff_type 23 / 180 s, already exists on the server).
-- ---------------------------------------------------------------------------------------------
UPDATE magic SET
    t_1 = 18, before_action = 0, target_action = 0, self_effect = 0, flying_effect = 0, target_effect = 701,
    moral = 1, skill_level = 15, skill = 1108, msp = 100, hp = 0, s_sp = 0, item_group = 9, use_item = 0,
    cast_time = 0, recast_time = 1800, success_rate = 100, type1 = 4, type2 = 0, "range" = 0, etc = 0,
    use_standing = 0, skill_check = 0, icelightrate = 0
WHERE magic_num = 110820 AND type1 IS NULL;

UPDATE magic SET
    t_1 = 18, before_action = 0, target_action = 0, self_effect = 0, flying_effect = 0, target_effect = 701,
    moral = 1, skill_level = 15, skill = 2108, msp = 100, hp = 0, s_sp = 0, item_group = 9, use_item = 0,
    cast_time = 0, recast_time = 1800, success_rate = 100, type1 = 4, type2 = 0, "range" = 0, etc = 0,
    use_standing = 0, skill_check = 0, icelightrate = 0
WHERE magic_num = 210820 AND type1 IS NULL;

-- ---------------------------------------------------------------------------------------------
-- 2. Kurian / Porutu passives that carry another class's sSkill code (class*10 + skill tree).
--    sSkill drives the class gate (check_skill_class) and the tree the client shows the skill in.
--    The client tables get the same correction (client_patch/skill_fixes).
-- ---------------------------------------------------------------------------------------------
UPDATE magic SET skill = 1158 WHERE magic_num IN (115805, 115815) AND skill = 1068;
UPDATE magic SET skill = 1157 WHERE magic_num IN (115745, 115770) AND skill = 1147;
UPDATE magic SET skill = 2147 WHERE magic_num = 214745 AND skill = 1147;
UPDATE magic SET skill = 2157 WHERE magic_num IN (215745, 215770) AND skill = 1147;

-- ---------------------------------------------------------------------------------------------
-- 3. Single-row outliers against their identical siblings (see docs/SKILL_AUDIT.md section 4).
--    Rise (Karus Berserker) cost 0 MP / 100 HP; Guardian, Blade and Protector Rise cost 50 MP.
--    Mangling (Karus Guardian) had range 15; every other nation/class copy and the client use 0.
-- ---------------------------------------------------------------------------------------------
UPDATE magic SET msp = 50 WHERE magic_num = 105710 AND msp = 0;
UPDATE magic SET "range" = 0 WHERE magic_num = 106550 AND "range" = 15;

-- ---------------------------------------------------------------------------------------------
-- 4. Items that exist in the item table but whose magic row was never seeded (the client has the row).
--    Prayer of Priest (6 items, +50/55 % basic attack for 30 min) and the Continuance (duration) items.
-- ---------------------------------------------------------------------------------------------
INSERT INTO magic (magic_num, en_name, kr_name, description, t_1, before_action, target_action, self_effect,
                   flying_effect, target_effect, moral, skill_level, skill, msp, hp, s_sp, item_group, use_item,
                   cast_time, recast_time, success_rate, type1, type2, "range", etc, use_standing, skill_check,
                   icelightrate)
SELECT v.* FROM (VALUES
    (501160, 'Prayer Of Priest - Noah', 'Prayer Of Priest - Noah', 'Basic Attack increases by 55% for 30 min', -1, 0, 0, 0, 0, 601, 1, 0, 0, 0, 0, 0, 9, 978019000, 5, 55, 100, 4, 0, 25, 0, 0, 0, 0),
    (501161, 'Prayer Of Priest - Noah', 'Prayer Of Priest - Noah', 'Basic Attack increases by 55% for 30 min', -1, 0, 0, 0, 0, 601, 1, 0, 0, 0, 0, 0, 9, 978020000, 5, 55, 100, 4, 0, 25, 0, 0, 0, 0),
    (501162, 'Prayer Of Priest', 'Prayer Of Priest', 'Basic Attack increases by 50% for 30 min', -1, 0, 0, 0, 0, 601, 1, 0, 0, 0, 0, 0, 9, 811047000, 5, 55, 100, 4, 0, 25, 0, 0, 0, 0),
    (501163, 'Prayer Of Priest', 'Prayer Of Priest', 'Basic Attack increases by 50% for 30 min', -1, 0, 0, 0, 0, 601, 1, 0, 0, 0, 0, 0, 9, 811048000, 5, 55, 100, 4, 0, 25, 0, 0, 0, 0),
    (501164, 'Prayer Of Priest - Noah', 'Prayer Of Priest - Noah', 'Basic Attack increases by 55% for 30 min', -1, 0, 0, 0, 0, 601, 1, 0, 0, 0, 0, 0, 9, 811049000, 5, 55, 100, 4, 0, 25, 0, 0, 0, 0),
    (501165, 'Prayer Of Priest - Noah', 'Prayer Of Priest - Noah', 'Basic Attack increases by 55% for 30 min', -1, 0, 0, 0, 0, 601, 1, 0, 0, 0, 0, 0, 9, 811050000, 5, 55, 100, 4, 0, 25, 0, 0, 0, 0),
    (512010, 'Continuance Item', 'Continuance Item', 'Doubles the duration of help skills.', -1, 0, 0, 0, 0, 702, 240, 0, 0, 0, 0, 0, 9, 978125000, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0),
    (512011, 'Continuance Item', 'Continuance Item', 'Doubles the duration of help skills.', -1, 0, 0, 0, 0, 702, 240, 0, 0, 0, 0, 0, 9, 978126000, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0)
) AS v(magic_num, en_name, kr_name, description, t_1, before_action, target_action, self_effect, flying_effect,
       target_effect, moral, skill_level, skill, msp, hp, s_sp, item_group, use_item, cast_time, recast_time,
       success_rate, type1, type2, "range", etc, use_standing, skill_check, icelightrate)
WHERE NOT EXISTS (SELECT 1 FROM magic m WHERE m.magic_num = v.magic_num);

INSERT INTO magic_type4 (i_num, buff_type, radius, duration, attack_speed, speed, ac, ac_pct, attack, magic_attack,
                         max_hp, max_hp_pct, max_mp, max_mp_pct, str, sta, dex, intel, cha, fire_r, cold_r,
                         lightning_r, magic_r, disease_r, poison_r, exp_pct, special_amount, hit_rate, avoid_rate)
SELECT v.* FROM (VALUES
    (501160, 4, 0, 1800, 100, 100, 0, 100, 155, 100, 0, 100, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 100, 100),
    (501161, 4, 0, 1800, 100, 100, 0, 100, 155, 100, 0, 100, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 100, 100),
    (501162, 4, 0, 1800, 100, 100, 0, 100, 150, 100, 0, 100, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 100, 100),
    (501163, 4, 0, 1800, 100, 100, 0, 100, 150, 100, 0, 100, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 100, 100),
    (501164, 4, 0, 1800, 100, 100, 0, 100, 155, 100, 0, 100, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 100, 100),
    (501165, 4, 0, 1800, 100, 100, 0, 100, 155, 100, 0, 100, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 100, 100)
) AS v(i_num, buff_type, radius, duration, attack_speed, speed, ac, ac_pct, attack, magic_attack, max_hp,
       max_hp_pct, max_mp, max_mp_pct, str, sta, dex, intel, cha, fire_r, cold_r, lightning_r, magic_r,
       disease_r, poison_r, exp_pct, special_amount, hit_rate, avoid_rate)
WHERE NOT EXISTS (SELECT 1 FROM magic_type4 t WHERE t.i_num = v.i_num)
  AND EXISTS (SELECT 1 FROM magic m WHERE m.magic_num = v.i_num);
