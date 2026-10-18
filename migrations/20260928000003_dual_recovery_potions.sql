-- Dual Recovery Potion: restores HP and MP through the regular Type 3 potion path.
-- Magic IDs are unique server-side entries; item.effect1 points at these rows.
INSERT INTO magic (
    magic_num, en_name, kr_name, description, t_1, before_action, target_action,
    self_effect, flying_effect, target_effect, moral, skill_level, skill, msp, hp,
    s_sp, item_group, use_item, cast_time, recast_time, success_rate, type1,
    type2, "range", etc, use_standing, skill_check, icelightrate
) VALUES
    (811182, 'Dual Potion', 'Dual Potion', 'HP 720 / MP 1920 Dual Recovery Potion', 0, 0, 0,
     0, 0, 0, 1, 0, 0, 0, 0, 0, 9, 811182000, 0, 0, 100, 3, 0, 0, 0, 0, 0, 0),
    (811183, 'Dual Potion (Untradable)', 'Dual Potion (Untradable)', 'HP 720 / MP 1920 Dual Recovery Potion', 0, 0, 0,
     0, 0, 0, 1, 0, 0, 0, 0, 0, 9, 811183000, 0, 0, 100, 3, 0, 0, 0, 0, 0, 0)
ON CONFLICT (magic_num) DO UPDATE SET
    en_name = EXCLUDED.en_name, kr_name = EXCLUDED.kr_name,
    description = EXCLUDED.description, moral = EXCLUDED.moral,
    item_group = EXCLUDED.item_group, use_item = EXCLUDED.use_item,
    success_rate = EXCLUDED.success_rate, type1 = EXCLUDED.type1, type2 = EXCLUDED.type2;

INSERT INTO magic_type3 (
    i_num, direct_type, first_damage, time_damage, duration, attribute, radius,
    add_dmg_perc_to_user, add_dmg_perc_to_npc
) VALUES
    (811182, 1, 720, 1920, 0, 0, 0, 100, 100),
    (811183, 1, 720, 1920, 0, 0, 0, 100, 100)
ON CONFLICT (i_num) DO UPDATE SET
    direct_type = EXCLUDED.direct_type, first_damage = EXCLUDED.first_damage,
    time_damage = EXCLUDED.time_damage, duration = EXCLUDED.duration,
    attribute = EXCLUDED.attribute, radius = EXCLUDED.radius,
    add_dmg_perc_to_user = EXCLUDED.add_dmg_perc_to_user,
    add_dmg_perc_to_npc = EXCLUDED.add_dmg_perc_to_npc;

INSERT INTO item (
    num, extension, str_name, description, item_plus_id, item_alteration,
    item_icon_id1, item_icon_id2, kind, slot, race, class, weight, duration,
    buy_price, sell_price, ac, countable, effect1, effect2, req_level,
    req_level_max, bound
) VALUES
    (811182000, 22, 'Dual Potion', 'HP 720 / MP 1920 Dual Recovery Potion | Cannot be used with Genie.', NULL, 0,
     81118200, 81118200, 97, 17, 0, 0, 45, 1, 50, 0, 0, 1, 811182, 0, 0, 100, 0),
    (811183000, 22, 'Dual Potion (Untradable)', 'HP 720 / MP 1920 Dual Recovery Potion | Cannot be used with Genie.', NULL, 0,
     81118200, 81118200, 97, 17, 20, 0, 45, 1, 50, 0, 0, 1, 811183, 0, 0, 100, 0)
ON CONFLICT (num) DO UPDATE SET
    extension = EXCLUDED.extension, str_name = EXCLUDED.str_name,
    description = EXCLUDED.description, item_icon_id1 = EXCLUDED.item_icon_id1,
    item_icon_id2 = EXCLUDED.item_icon_id2, kind = EXCLUDED.kind,
    slot = EXCLUDED.slot, race = EXCLUDED.race, class = EXCLUDED.class,
    weight = EXCLUDED.weight, duration = EXCLUDED.duration,
    buy_price = EXCLUDED.buy_price, sell_price = EXCLUDED.sell_price,
    countable = EXCLUDED.countable, effect1 = EXCLUDED.effect1,
    effect2 = EXCLUDED.effect2, req_level_max = EXCLUDED.req_level_max,
    bound = EXCLUDED.bound;
