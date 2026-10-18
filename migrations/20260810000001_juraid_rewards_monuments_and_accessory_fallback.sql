-- Juraid Mountain follow-up fixes:
-- - Add Deva room Monument templates used by the runtime spawn logic.
-- - Keep Enhanced Silver Gem exchange menu text present for Rugur/Belldan flows.

INSERT INTO npc_template (
    s_sid, is_monster, str_name, s_pid, s_size, i_weapon_1, i_weapon_2,
    by_group, by_act_type, by_type, by_family, by_rank, by_title, i_selling_group,
    s_level, i_exp, i_loyalty, i_hp_point, s_mp_point, s_atk, s_ac, s_hit_rate,
    s_evade_rate, s_damage, s_attack_delay, by_speed_1, by_speed_2, s_standtime,
    s_item, i_magic_1, i_magic_2, i_magic_3, s_fire_r, s_cold_r, s_lightning_r,
    s_magic_r, s_disease_r, s_poison_r, s_bulk, by_attack_range, by_search_range,
    by_tracing_range, i_money, by_direct_attack, by_magic_attack, area_range
) VALUES
    (8113, TRUE, 'Karus Juraid Monument', 6700, 120, 0, 0, 3, 1, 0, 1, 0, 0, 0,
     100, 0, 0, 100000, 0, 1, 500, 2500, 1700, 300, 1500, 0, 0, 1000,
     8113, 0, 0, 0, 250, 250, 250, 250, 250, 250, 100, 5, 10, 20, 0, 0, 0, 0.0),
    (8114, TRUE, 'El Morad Juraid Monument', 6700, 120, 0, 0, 3, 1, 0, 1, 0, 0, 0,
     100, 0, 0, 100000, 0, 1, 500, 2500, 1700, 300, 1500, 0, 0, 1000,
     8114, 0, 0, 0, 250, 250, 250, 250, 250, 250, 100, 5, 10, 20, 0, 0, 0, 0.0)
ON CONFLICT (s_sid, is_monster) DO UPDATE SET
    str_name = EXCLUDED.str_name,
    s_pid = EXCLUDED.s_pid,
    s_size = EXCLUDED.s_size,
    by_group = EXCLUDED.by_group,
    by_act_type = EXCLUDED.by_act_type,
    by_type = EXCLUDED.by_type,
    s_level = EXCLUDED.s_level,
    i_hp_point = EXCLUDED.i_hp_point,
    s_item = EXCLUDED.s_item;

INSERT INTO quest_talk (i_num, str_talk) VALUES
    (45608, 'What would you like to exchange the [Fortified Sterling Silver Gemstone] for?'),
    (45609, 'You came to exchange the [Fortified Sterling Silver Gemstone] for a [Silver Gem]! Would you like to proceed with the exchange?!'),
    (45610, 'The [Fortified Sterling Silver Gemstone] is not in your inventory. Please check your inventory and try talking to me again.')
ON CONFLICT (i_num) DO NOTHING;

INSERT INTO quest_menu (i_num, str_menu) VALUES
    (45608, 'Enhanced Silver Gem Exchange List'),
    (45609, 'Exchange Fortified Sterling Silver Gemstone for Silvery Gem')
ON CONFLICT (i_num) DO UPDATE SET str_menu = EXCLUDED.str_menu;
