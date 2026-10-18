-- Fire & Ice golem war: the client's own Flame Golem (9929) / Ice Golem (9930) identities.
-- 80000 HP, no aggression (they just stand around the Bowl), no experience / loot.
UPDATE npc_template
SET i_hp_point = 80000, s_level = 83, i_exp = 0, i_loyalty = 0, s_item = 0,
    by_search_range = 0, by_tracing_range = 0, by_direct_attack = 0, by_magic_attack = 0
WHERE s_sid IN (9929, 9930) AND is_monster = TRUE;
