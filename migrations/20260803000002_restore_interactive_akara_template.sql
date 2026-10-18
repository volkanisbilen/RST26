-- Restore the v2615 interactive Goddess Akara Statue in Moradon.
--
-- The reconstructed 31774/type174 row has picture 30001, but type 174 does
-- not generate WIZ_CLIENT_EVENT/WIZ_NPC_EVENT when this client right-clicks
-- it. Keep the sniffer-verified position while binding the spawn to the full
-- original 30001/type0 template already shipped in npc_template.

DELETE FROM npc_spawn
WHERE zone_id = 21
  AND npc_id IN (31774, 30001)
  AND is_monster = false
  AND left_x = 830
  AND top_z = 668;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, num_npc, left_x, top_z,
    act_type, regen_type, dungeon_family, special_type, trap_number,
    spawn_range, regen_time, direction, dot_cnt, path, room
)
SELECT
    21, 30001, false, 1, 830, 668,
    0, 0, 0, 0, 0,
    0, 0, 180, 0, '', 0
WHERE EXISTS (
    SELECT 1
    FROM npc_template
    WHERE s_sid = 30001
      AND is_monster = false
      AND s_pid = 30001
      AND by_type = 0
)
AND NOT EXISTS (
    SELECT 1
    FROM npc_spawn
    WHERE zone_id = 21
      AND npc_id = 30001
      AND is_monster = false
      AND left_x = 830
      AND top_z = 668
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM npc_spawn
        WHERE zone_id = 21
          AND npc_id = 30001
          AND is_monster = false
          AND left_x = 830
          AND top_z = 668
    ) THEN
        RAISE EXCEPTION 'Akara migration failed: complete 30001/type0 template is missing';
    END IF;
END
$$;
