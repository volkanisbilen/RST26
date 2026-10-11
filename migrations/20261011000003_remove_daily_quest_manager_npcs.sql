-- The daily quest now runs through the proxy panel (EXT 0xD3); the Aelion manager NPCs are no longer used.
DELETE FROM npc_spawn WHERE npc_id = 31999 AND is_monster = FALSE;
