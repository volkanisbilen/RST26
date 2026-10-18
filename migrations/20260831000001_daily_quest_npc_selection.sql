-- Daily quests are opt-in.  Old server builds inserted every definition into
-- every character's progress map; retain those rows for audit/history but do
-- not load them unless the player explicitly selects the quest at an NPC.
ALTER TABLE user_daily_quest
    ADD COLUMN IF NOT EXISTS is_selected BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_user_daily_quest_selected
    ON user_daily_quest (character_id, is_selected);

-- The imported MSSQL list stopped at level 72 while this server creates and
-- actively uses level-83 characters.  These are the same high-level tasks;
-- only their acceptance range is extended.
UPDATE daily_quests
SET max_level = 83
WHERE max_level = 72;

-- 31880 is a dedicated daily-quest NPC. Its visual/model fields are copied
-- from a client-known manager model, but no existing NPC template or Lua
-- route is reused. The Rust interaction handler owns its menu.
UPDATE npc_template
SET str_name = '[Manager]Billbor'
WHERE s_sid = 18005 AND is_monster = FALSE;

INSERT INTO npc_template
SELECT
    31880::SMALLINT, FALSE, '[Daily Quest Manager] Aelion',
    s_pid, s_size, i_weapon_1, i_weapon_2, by_group, 0, 46,
    by_family, by_rank, by_title, 0, s_level, i_exp, i_loyalty,
    i_hp_point, s_mp_point, s_atk, s_ac, s_hit_rate, s_evade_rate,
    s_damage, s_attack_delay, by_speed_1, by_speed_2, s_standtime,
    s_item, i_magic_1, i_magic_2, i_magic_3, s_fire_r, s_cold_r,
    s_lightning_r, s_magic_r, s_disease_r, s_poison_r, s_bulk,
    by_attack_range, by_search_range, by_tracing_range, i_money,
    by_direct_attack, by_magic_attack, area_range
FROM npc_template
WHERE s_sid = 14436 AND is_monster = FALSE
ON CONFLICT (s_sid, is_monster) DO UPDATE SET
    str_name = EXCLUDED.str_name,
    s_pid = EXCLUDED.s_pid,
    by_act_type = EXCLUDED.by_act_type,
    by_type = EXCLUDED.by_type,
    i_selling_group = EXCLUDED.i_selling_group;

-- Remove only the five temporary 18005 placements produced by the first
-- development run of this migration; retain Billbor's original Moradon spawn
-- at (816,671) and any normal quest usage.
DELETE FROM npc_spawn
WHERE npc_id = 18005
  AND is_monster = FALSE
  AND (
      (zone_id = 21 AND left_x = 457 AND top_z = 666)
   OR (zone_id = 1  AND left_x = 595 AND top_z = 1563)
   OR (zone_id = 2  AND left_x = 1301 AND top_z = 972)
   OR (zone_id = 11 AND left_x = 506 AND top_z = 539)
   OR (zone_id = 71 AND left_x = 1021 AND top_z = 1013)
  );

-- Put one manager near the existing gate/safe-route in every zone for which
-- the imported daily_quests table actually defines tasks.
DO $$
DECLARE
    manager_spawns SMALLINT[][] := ARRAY[
        ARRAY[21, 457, 666, 90],  -- Moradon safety gate
        ARRAY[1,  595, 1563, 90], -- Karus/Luferson safety gate
        ARRAY[2,  1301, 972, 270],-- El Morad safety gate
        ARRAY[11, 506, 539, 180], -- Eslant entrance route
        ARRAY[71, 1021, 1013, 180]-- Ronark / Bifrost gate
    ];
    row_data SMALLINT[];
BEGIN
    FOREACH row_data SLICE 1 IN ARRAY manager_spawns LOOP
        IF NOT EXISTS (
            SELECT 1 FROM npc_spawn
            WHERE zone_id = row_data[1]
              AND npc_id = 31880
              AND is_monster = FALSE
              AND left_x = row_data[2]
              AND top_z = row_data[3]
        ) THEN
            INSERT INTO npc_spawn
                (zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
                 special_type, trap_number, left_x, top_z, num_npc, spawn_range,
                 regen_time, direction, dot_cnt, path, room)
            VALUES
                (row_data[1], 31880, FALSE, 0, 0, 0,
                 0, 0, row_data[2], row_data[3], 1, 0,
                 0, row_data[4], 0, NULL, 0);
        END IF;
    END LOOP;
END $$;
