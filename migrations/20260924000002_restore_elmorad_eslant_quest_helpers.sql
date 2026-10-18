-- Quest_Helper.tbl identifies these existing quest IDs with zone 12.
-- Imported server rows still point to old shared Eslant (19) NPC IDs.
-- Retain quest IDs, progress, rewards, requirements and event numbers.
UPDATE quest_helper AS q
SET b_zone = 12, s_npc_id = m.new_npc, str_lua_filename = m.lua_file
FROM (VALUES
    (15424,14424,'14424_dela.lua'),
    (15438,14438,'14438_Beldan.lua'),
    (15439,14439,'14439_Agata.lua'),
    (26005,25005,'25005_Earth.lua'),
    (26276,25276,'25276_Rily.lua')
) AS m(old_npc,new_npc,lua_file)
WHERE q.b_zone = 19 AND q.s_npc_id = m.old_npc;
