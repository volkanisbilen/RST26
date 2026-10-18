-- Linux uses case-sensitive Lua paths. Normalise these two active quest helper
-- bindings to the filenames installed in Quests/.
UPDATE quest_helper
SET str_lua_filename = '14301_Hapa.lua'
WHERE s_npc_id = 14301
  AND lower(str_lua_filename) = '14301_hapa.lua';

UPDATE quest_helper
SET str_lua_filename = '13016_Keite.lua'
WHERE s_npc_id = 13016
  AND lower(str_lua_filename) = '13016_keite.lua';
