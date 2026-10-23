-- Hemes (16085, Moradon): top-level menu entries for Job Change (scrolls 700112000 / 700113000) and
-- Nation Transfer (scroll 810096000). The root list of this NPC is built from quest_helper rows
-- (s_quest_menu text id -> trigger event in 16085_Hamus.lua), so the Lua events 2860 / 2899 need rows here.
-- (superseded: rows deleted again below)
INSERT INTO quest_helper (n_index, b_message_type, b_level, n_exp, b_class, b_nation, b_quest_type, b_zone, s_npc_id,
    s_event_data_index, b_event_status, n_event_trigger_index, n_event_complete_index, n_exchange_index, n_event_talk_index,
    str_lua_filename, s_quest_menu, s_npc_main, s_quest_solo)
SELECT v.idx, b_message_type, b_level, n_exp, b_class, b_nation, b_quest_type, b_zone, s_npc_id,
       v.q, 4, v.trig, 0, 0, 0, str_lua_filename, v.menu, s_npc_main, s_quest_solo
FROM quest_helper, (VALUES (317720101, 10070, 2860, 45486), (317720102, 10071, 2899, 7772)) AS v(idx, q, trig, menu)
WHERE quest_helper.n_index = 4258
  AND NOT EXISTS (SELECT 1 FROM quest_helper x WHERE x.n_index = v.idx);
-- the client builds the Hemes menu from Lua, these rows were not needed (and could change which event a click starts)
DELETE FROM quest_helper WHERE n_index IN (317720101, 317720102);
