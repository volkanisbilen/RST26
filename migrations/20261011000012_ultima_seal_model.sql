-- The seal's model: 9949 (temple center artifact) exists in the client's NPC_Looks.tbl, 10100 does not
-- (it rendered as a kecoon). The client NPC_us.tbl row of proto 10101 was changed to 9949 as well.
UPDATE npc_template SET s_pid = 9949 WHERE s_sid = 10101 AND is_monster = FALSE;
