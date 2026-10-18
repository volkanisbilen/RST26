-- The Daily Quest Manager is a static quest NPC, using the same non-hostile
-- interaction tuple as standard managers.  Keep Moradon's manager immediately
-- beside the requested scroll-merchant area.
UPDATE npc_template
SET by_group = 3,
    by_act_type = 7,
    by_type = 46,
    by_family = 1,
    by_rank = 3,
    by_title = 3,
    by_search_range = 0,
    by_tracing_range = 0,
    by_direct_attack = 0,
    by_magic_attack = 0
WHERE s_sid = 31999 AND is_monster = FALSE;

UPDATE npc_spawn
SET left_x = 823,
    top_z = 552,
    direction = 90,
    act_type = 7,
    spawn_range = 0,
    regen_time = 30
WHERE zone_id = 21 AND npc_id = 31999 AND is_monster = FALSE;
