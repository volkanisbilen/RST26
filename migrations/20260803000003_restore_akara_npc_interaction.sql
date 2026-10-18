-- Restore the sniffer-verified v2615 Akara identity and make it a friendly
-- Moradon NPC. The earlier reconstructed 31774 row left by_group at 0; the
-- client consequently classified the statue as an attack target and skipped
-- the WIZ_NPC_EVENT right-click dispatch.

UPDATE npc_template
SET
    str_name = '<Goddess Akara Statue>',
    s_pid = 30001,
    s_size = 100,
    i_weapon_1 = 0,
    i_weapon_2 = 0,
    by_group = 3,
    by_act_type = 1,
    by_type = 174,
    i_selling_group = 0,
    s_level = 50
WHERE s_sid = 31774
  AND is_monster = false;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM npc_template
        WHERE s_sid = 31774
          AND is_monster = false
          AND s_pid = 30001
          AND by_group = 3
          AND by_act_type = 1
          AND by_type = 174
    ) THEN
        RAISE EXCEPTION 'Akara migration failed: 31774 NPC template is missing';
    END IF;
END
$$;

DELETE FROM npc_spawn
WHERE zone_id = 21
  AND npc_id IN (30001, 31774)
  AND is_monster = false
  AND left_x = 830
  AND top_z = 668;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, num_npc, left_x, top_z,
    act_type, regen_type, dungeon_family, special_type, trap_number,
    spawn_range, regen_time, direction, dot_cnt, path, room
)
VALUES (
    21, 31774, false, 1, 830, 668,
    0, 0, 0, 0, 0,
    0, 0, 180, 0, '', 0
);
