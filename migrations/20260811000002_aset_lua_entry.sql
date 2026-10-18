-- [Event Manager] Aset (31772) has a complete v2615 Lua menu but the imported
-- QUEST_HELPER data has no root row for this NPC.  ClientEvent only dispatches
-- Lua through a root helper, so the click previously ended at "no quest
-- helpers".  Keep this as a new migration; applied SQLx files are immutable.
INSERT INTO quest_helper (
    n_index, b_message_type, b_level, n_exp, b_class, b_nation,
    b_quest_type, b_zone, s_npc_id, s_event_data_index, b_event_status,
    n_event_trigger_index, n_event_complete_index, n_exchange_index,
    n_event_talk_index, str_lua_filename, s_quest_menu, s_npc_main,
    s_quest_solo
) VALUES (
    317720100, 2, 0, 0, 5, 3,
    0, 21, 31772, 0, 0,
    100, 0, 0,
    45054, '31772_Aset.lua', 0, 0,
    0
)
ON CONFLICT (n_index) DO UPDATE SET
    b_level = EXCLUDED.b_level,
    b_class = EXCLUDED.b_class,
    b_nation = EXCLUDED.b_nation,
    b_zone = EXCLUDED.b_zone,
    s_npc_id = EXCLUDED.s_npc_id,
    s_event_data_index = EXCLUDED.s_event_data_index,
    b_event_status = EXCLUDED.b_event_status,
    n_event_trigger_index = EXCLUDED.n_event_trigger_index,
    str_lua_filename = EXCLUDED.str_lua_filename;
