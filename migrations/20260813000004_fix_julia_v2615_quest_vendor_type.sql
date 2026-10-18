-- Julia is a Lua-driven quest/exchange vendor, not an NPC_REFUGEE (type 46).
-- The v2615 Moradon NPCs with the same interaction path use the neutral
-- quest-vendor tuple group=3, actType=7, type=47 (Kaira/Iruta/Hemes/Patrick).
-- With type 46/group 0 the client only sends WIZ_TARGET_HP and never sends
-- WIZ_NPC_EVENT, leaving 31741_Julia.lua unreachable.
UPDATE npc_template
   SET by_group = 3,
       by_act_type = 7,
       by_type = 47,
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

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM npc_template
         WHERE s_sid = 31741
           AND is_monster = FALSE
           AND by_group = 3
           AND by_act_type = 7
           AND by_type = 47
    ) THEN
        RAISE EXCEPTION 'Julia v2615 quest-vendor classification was not applied';
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM quest_helper
         WHERE s_npc_id = 31741
           AND b_zone = 21
           AND n_event_trigger_index = 100
           AND str_lua_filename = '31741_Julia.lua'
    ) THEN
        RAISE EXCEPTION 'Julia quest helper route is missing';
    END IF;
END $$;
