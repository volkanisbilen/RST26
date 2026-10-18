-- Dual Recovery Potion: align with the 2625 client tables.
--
-- The client item table points item 811182000 / 811183000 at magic 494051 / 494052
-- (Skill_Magic_Main "Dual Potion"), so those are the skill ids the client sends when the
-- potion is used. The server only knew the earlier custom ids 811182 / 811183 and dropped the
-- cast. Add the client ids with the same HP 720 / MP 1920 Type 3 profile and point the items
-- at them. The old ids stay valid.
INSERT INTO magic (
    magic_num, en_name, kr_name, description, t_1, before_action, target_action,
    self_effect, flying_effect, target_effect, moral, skill_level, skill, msp, hp,
    s_sp, item_group, use_item, cast_time, recast_time, success_rate, type1,
    type2, "range", etc, use_standing, skill_check, icelightrate
) VALUES
    (494051, 'Dual Potion', 'Dual Potion', 'HP 720 / MP 1920 Dual Recovery Potion', -1, 0, 0,
     2602, 0, 0, 1, 0, 0, 0, 0, 0, 9, 811182000, 0, 0, 100, 3, 0, 0, 0, 0, 0, 0),
    (494052, 'Dual Potion (Untradable)', 'Dual Potion (Untradable)', 'HP 720 / MP 1920 Dual Recovery Potion', -1, 0, 0,
     2602, 0, 0, 1, 0, 0, 0, 0, 0, 9, 811183000, 0, 0, 100, 3, 0, 0, 0, 0, 0, 0)
ON CONFLICT (magic_num) DO UPDATE SET
    en_name = EXCLUDED.en_name, kr_name = EXCLUDED.kr_name,
    description = EXCLUDED.description, moral = EXCLUDED.moral,
    item_group = EXCLUDED.item_group, use_item = EXCLUDED.use_item,
    success_rate = EXCLUDED.success_rate, type1 = EXCLUDED.type1, type2 = EXCLUDED.type2;

INSERT INTO magic_type3 (
    i_num, direct_type, first_damage, time_damage, duration, attribute, radius,
    add_dmg_perc_to_user, add_dmg_perc_to_npc
) VALUES
    (494051, 1, 720, 1920, 0, 0, 0, 100, 100),
    (494052, 1, 720, 1920, 0, 0, 0, 100, 100)
ON CONFLICT (i_num) DO UPDATE SET
    direct_type = EXCLUDED.direct_type, first_damage = EXCLUDED.first_damage,
    time_damage = EXCLUDED.time_damage, duration = EXCLUDED.duration,
    attribute = EXCLUDED.attribute, radius = EXCLUDED.radius,
    add_dmg_perc_to_user = EXCLUDED.add_dmg_perc_to_user,
    add_dmg_perc_to_npc = EXCLUDED.add_dmg_perc_to_npc;

UPDATE item SET effect1 = 494051 WHERE num = 811182000;
UPDATE item SET effect1 = 494052 WHERE num = 811183000;
