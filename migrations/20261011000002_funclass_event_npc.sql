-- Moradon "Fun Class Event" NPC (proto 31998), placed between the two Moradon warp gates
-- (Karus gate 797.7/526.9, El Morad gate 837.4/526.8). Right-click is handled in Rust
-- (client_event.rs): it enters the running Fun Class event. The client needs proto 31998 in
-- its NPC_us.tbl (client_patch/funclass_npc).
INSERT INTO npc_template
SELECT
    31998::SMALLINT, FALSE, '[Fun Class Event] Cinderella',
    s_pid, s_size, i_weapon_1, i_weapon_2, 0, 0, 46,
    0, 0, 0, 0, s_level, 0, 0,
    GREATEST(i_hp_point, 1000), s_mp_point, 1, s_ac, s_hit_rate, s_evade_rate,
    0, s_attack_delay, by_speed_1, by_speed_2, s_standtime,
    0, 0, 0, 0, s_fire_r, s_cold_r,
    s_lightning_r, s_magic_r, s_disease_r, s_poison_r, s_bulk,
    0, 0, 0, 0,
    0, 0, area_range
FROM npc_template
WHERE s_sid = 31999 AND is_monster = FALSE
ON CONFLICT (s_sid, is_monster) DO UPDATE SET
    str_name = EXCLUDED.str_name,
    s_pid = EXCLUDED.s_pid,
    by_group = EXCLUDED.by_group,
    by_act_type = EXCLUDED.by_act_type,
    by_type = EXCLUDED.by_type;

INSERT INTO npc_spawn
    (zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
     special_type, trap_number, left_x, top_z, num_npc, spawn_range,
     regen_time, direction, dot_cnt, path, room)
SELECT 21, 31998, FALSE, 0, 0, 0, 0, 0, 817, 531, 1, 0, 0, 180, 0, NULL, 0
WHERE NOT EXISTS (
    SELECT 1 FROM npc_spawn WHERE zone_id = 21 AND npc_id = 31998 AND is_monster = FALSE
);
