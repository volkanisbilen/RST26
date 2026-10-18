-- Some legacy rows are restored after initial quest imports. Apply the exact
-- case-sensitive Kate filename to every helper row for this NPC.
UPDATE quest_helper
SET str_lua_filename = '13016_Keite.lua'
WHERE s_npc_id = 13016;
