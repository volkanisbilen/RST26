-- Final Juraid object/reward-drop corrections. Kept in a new migration so
-- previously applied SQLx migration checksums remain immutable.

-- The bridge model lives in the historical monster table, but on the v2615
-- wire it must be announced as a static NPC (isMonster=2). Preserve the exact
-- PID 6700 visual contract while adding the non-monster lookup used at spawn.
INSERT INTO npc_template (
    s_sid, is_monster, str_name, s_pid, s_size, i_weapon_1, i_weapon_2,
    by_group, by_act_type, by_type, by_family, by_rank, by_title, i_selling_group,
    s_level, i_exp, i_loyalty, i_hp_point, s_mp_point, s_atk, s_ac, s_hit_rate,
    s_evade_rate, s_damage, s_attack_delay, by_speed_1, by_speed_2, s_standtime,
    s_item, i_magic_1, i_magic_2, i_magic_3, s_fire_r, s_cold_r, s_lightning_r,
    s_magic_r, s_disease_r, s_poison_r, s_bulk, by_attack_range, by_search_range,
    by_tracing_range, i_money, by_direct_attack, by_magic_attack, area_range
)
SELECT
    s_sid, FALSE, str_name, s_pid, s_size, i_weapon_1, i_weapon_2,
    3, 1, by_type, by_family, by_rank, by_title, i_selling_group,
    s_level, i_exp, i_loyalty, i_hp_point, s_mp_point, 0, s_ac, s_hit_rate,
    s_evade_rate, 0, s_attack_delay, 0, 0, s_standtime,
    s_item, i_magic_1, i_magic_2, i_magic_3, s_fire_r, s_cold_r, s_lightning_r,
    s_magic_r, s_disease_r, s_poison_r, s_bulk, by_attack_range, 0,
    0, i_money, by_direct_attack, by_magic_attack, area_range
FROM npc_template
WHERE s_sid = 8110 AND is_monster = TRUE
ON CONFLICT (s_sid, is_monster) DO UPDATE SET
    str_name = EXCLUDED.str_name,
    s_pid = EXCLUDED.s_pid,
    s_size = EXCLUDED.s_size,
    by_group = 3,
    by_act_type = 1,
    by_speed_1 = 0,
    by_speed_2 = 0,
    by_search_range = 0,
    by_tracing_range = 0,
    s_atk = 0,
    s_damage = 0;

-- Monuments remain attackable event objectives, but must never acquire an AI
-- target or move away from their fixed Deva-room coordinates.
UPDATE npc_template
SET by_act_type = 1,
    by_speed_1 = 0,
    by_speed_2 = 0,
    by_search_range = 0,
    by_tracing_range = 0
WHERE s_sid IN (9702, 9703) AND is_monster = TRUE;

-- Deva Bird chest: modestly raise Gold Bar (15% -> 20%) and Silver Bar
-- (50% -> 60%), retain the existing Silvery Gemstone roll, and add a 10%
-- Fortified Sterling Silver Gemstone roll in the first unused slot.
UPDATE monster_item
SET item01 = 389196000, percent01 = 8000,
    item02 = 379068000, percent02 = 2000,
    item03 = 379067000, percent03 = 6000,
    item04 = 811137000, percent04 = 1000
WHERE s_index = 8106;
