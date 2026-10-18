-- Ultima's Dungeon registers through the client's native registration window (no NPC).
DELETE FROM npc_spawn WHERE npc_id = 31997 AND is_monster = FALSE;
DELETE FROM npc_template WHERE s_sid = 31997 AND is_monster = FALSE;
