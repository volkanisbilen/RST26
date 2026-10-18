-- Restore the v2615 Moradon classifications and facing values.
--
-- Julia was classified as NPC type 46 by the v2615 zone-21 sync.  Type 0
-- makes this client treat her as an attack/HP target and it never sends
-- WIZ_NPC_EVENT, so the Lua menu cannot be reached.
UPDATE npc_template
   SET by_type = 46,
       by_act_type = 1,
       i_selling_group = 0,
       s_pid = 31200,
       i_weapon_1 = 0,
       i_weapon_2 = 0
 WHERE s_sid = 31741
   AND is_monster = FALSE;

UPDATE npc_spawn
   SET is_monster = FALSE,
       act_type = 1
 WHERE zone_id = 21
   AND npc_id = 31741;

-- 20260804000003 and 20260805000001..3 cumulatively left regular Moradon
-- NPCs at source_direction + 128.  Adding 128 once more restores the exact
-- byte-angle values imported by the v2615 zone-21 sync (modulo 256).
-- MORANKER statues use moraranker_statue_slot and are intentionally excluded.
UPDATE npc_spawn
   SET direction = MOD(MOD(direction, 256) + 128, 256)
 WHERE zone_id = 21
   AND is_monster = FALSE
   AND npc_id NOT BETWEEN 31882 AND 31887;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM npc_template
         WHERE s_sid = 31741
           AND is_monster = FALSE
           AND by_type = 46
           AND by_act_type = 1
    ) THEN
        RAISE EXCEPTION 'Julia v2615 NPC classification was not applied';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM npc_spawn
         WHERE zone_id = 21
           AND npc_id = 31741
           AND is_monster = FALSE
           AND direction = 76
    ) THEN
        RAISE EXCEPTION 'Julia v2615 spawn/facing was not restored';
    END IF;
END $$;
