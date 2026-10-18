-- 29514 existed in the v2615 client table but was left with an incomplete
-- server template (group/act/family zero, 1,000 HP).  Those values make the
-- client treat a shop model as a hostile creature.  Clone every behavioural
-- field from the working Potion Merchant, retaining Scroll Merchant's exact
-- client model, name and shop group.
UPDATE npc_template AS target
SET s_size = source.s_size,
    i_weapon_1 = source.i_weapon_1,
    i_weapon_2 = source.i_weapon_2,
    by_group = source.by_group,
    by_act_type = source.by_act_type,
    by_type = source.by_type,
    by_family = source.by_family,
    by_rank = source.by_rank,
    by_title = source.by_title,
    s_level = source.s_level,
    i_exp = source.i_exp,
    i_loyalty = source.i_loyalty,
    i_hp_point = source.i_hp_point,
    s_mp_point = source.s_mp_point,
    s_atk = source.s_atk,
    s_ac = source.s_ac,
    s_hit_rate = source.s_hit_rate,
    s_evade_rate = source.s_evade_rate,
    s_damage = source.s_damage,
    s_attack_delay = source.s_attack_delay,
    by_speed_1 = source.by_speed_1,
    by_speed_2 = source.by_speed_2,
    s_standtime = source.s_standtime,
    s_item = source.s_item,
    i_magic_1 = source.i_magic_1,
    i_magic_2 = source.i_magic_2,
    i_magic_3 = source.i_magic_3,
    s_fire_r = source.s_fire_r,
    s_cold_r = source.s_cold_r,
    s_lightning_r = source.s_lightning_r,
    s_magic_r = source.s_magic_r,
    s_disease_r = source.s_disease_r,
    s_poison_r = source.s_poison_r,
    s_bulk = source.s_bulk,
    by_attack_range = source.by_attack_range,
    by_search_range = source.by_search_range,
    by_tracing_range = source.by_tracing_range,
    i_money = source.i_money,
    by_direct_attack = source.by_direct_attack,
    by_magic_attack = source.by_magic_attack,
    area_range = source.area_range,
    s_pid = 19005,
    str_name = '[Scroll Merchant]',
    i_selling_group = 280000
FROM npc_template AS source
WHERE target.s_sid = 29514
  AND target.is_monster = FALSE
  AND source.s_sid = 29505
  AND source.is_monster = FALSE;

UPDATE npc_spawn
SET act_type = 1,
    regen_time = 30,
    spawn_range = 0,
    direction = 90
WHERE zone_id = 21
  AND npc_id = 29514
  AND is_monster = FALSE;
