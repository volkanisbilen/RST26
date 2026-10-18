-- 31880 is already reserved by the v2615 client for an El Morad ladder-rank
-- NPC. Do not overload it: the client uses the NPC_us.tbl identity as well as
-- the wire PID. Replace only the temporary daily-manager records with a new,
-- dedicated proto (31999) that is added to the client patch's NPC_us.tbl.
DELETE FROM npc_spawn
WHERE npc_id = 31880
  AND is_monster = FALSE
  AND (
      (zone_id = 21 AND left_x = 457 AND top_z = 666)
   OR (zone_id = 1  AND left_x = 595 AND top_z = 1563)
   OR (zone_id = 2  AND left_x = 1301 AND top_z = 972)
   OR (zone_id = 11 AND left_x = 506 AND top_z = 539)
   OR (zone_id = 71 AND left_x = 1021 AND top_z = 1013)
  );

DELETE FROM npc_template
WHERE s_sid = 31880
  AND is_monster = FALSE
  AND str_name = '[Daily Quest Manager] Aelion';

INSERT INTO npc_template
SELECT
    31999::SMALLINT, FALSE, '[Daily Quest Manager] Aelion',
    s_pid, s_size, i_weapon_1, i_weapon_2, 0, 0, 46,
    0, 0, 0, 0, s_level, 0, 0,
    GREATEST(i_hp_point, 1000), s_mp_point, 1, s_ac, s_hit_rate, s_evade_rate,
    0, s_attack_delay, by_speed_1, by_speed_2, s_standtime,
    0, 0, 0, 0, s_fire_r, s_cold_r,
    s_lightning_r, s_magic_r, s_disease_r, s_poison_r, s_bulk,
    0, 0, 0, 0,
    0, 0, area_range
FROM npc_template
WHERE s_sid = 14436 AND is_monster = FALSE
ON CONFLICT (s_sid, is_monster) DO UPDATE SET
    str_name = EXCLUDED.str_name,
    s_pid = EXCLUDED.s_pid,
    by_group = EXCLUDED.by_group,
    by_act_type = EXCLUDED.by_act_type,
    by_type = EXCLUDED.by_type,
    i_selling_group = EXCLUDED.i_selling_group,
    by_search_range = EXCLUDED.by_search_range,
    by_tracing_range = EXCLUDED.by_tracing_range;

DO $$
DECLARE
    manager_spawns SMALLINT[][] := ARRAY[
        ARRAY[21, 457, 666, 90],
        ARRAY[1,  595, 1563, 90],
        ARRAY[2,  1301, 972, 270],
        ARRAY[11, 506, 539, 180],
        ARRAY[71, 1021, 1013, 180]
    ];
    row_data SMALLINT[];
BEGIN
    FOREACH row_data SLICE 1 IN ARRAY manager_spawns LOOP
        IF NOT EXISTS (
            SELECT 1 FROM npc_spawn
            WHERE zone_id = row_data[1] AND npc_id = 31999
              AND is_monster = FALSE AND left_x = row_data[2] AND top_z = row_data[3]
        ) THEN
            INSERT INTO npc_spawn
                (zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
                 special_type, trap_number, left_x, top_z, num_npc, spawn_range,
                 regen_time, direction, dot_cnt, path, room)
            VALUES
                (row_data[1], 31999, FALSE, 0, 0, 0, 0, 0,
                 row_data[2], row_data[3], 1, 0, 0, row_data[4], 0, NULL, 0);
        END IF;
    END LOOP;
END $$;
