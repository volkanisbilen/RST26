-- Ultima's Dungeon NPC templates (proto ids must also exist in the client's NPC_us.tbl,
-- see client_patch/ultima_npcs) and the Ultima Keeper NPC in Moradon.
-- Boss tiers by room size (4-10 / 11-20 / 21-30 players): HP 1M / 2M / 3M, defense 2683 / 4025 / 8051,
-- attack 7338 / 11007 / 22015.
DELETE FROM npc_template WHERE s_sid IN (31990, 31991, 31992, 31993, 31994, 31997);

CREATE TEMP TABLE _ult AS SELECT * FROM npc_template WHERE s_sid = 9888 AND is_monster = TRUE;
UPDATE _ult SET s_sid = 31990, str_name = 'Ultima', s_pid = 9888, s_level = 83, i_exp = 0, i_loyalty = 0,
    i_hp_point = 1000000, s_ac = 2683, s_damage = 7338, s_item = 0;
INSERT INTO npc_template SELECT * FROM _ult;
UPDATE _ult SET s_sid = 31991, i_hp_point = 2000000, s_ac = 4025, s_damage = 11007;
INSERT INTO npc_template SELECT * FROM _ult;
UPDATE _ult SET s_sid = 31992, i_hp_point = 3000000, s_ac = 8051, s_damage = 22015;
INSERT INTO npc_template SELECT * FROM _ult;
DROP TABLE _ult;

-- the seal: a static, attackable structure (150000 HP, each basic attack removes 500)
CREATE TEMP TABLE _seal AS SELECT * FROM npc_template WHERE s_sid = 8618 AND is_monster = TRUE;
UPDATE _seal SET s_sid = 31993, str_name = 'Seal of Ultima', s_pid = 30030, s_size = 120,
    i_hp_point = 150000, s_ac = 0, s_damage = 0, s_atk = 0, i_exp = 0, i_loyalty = 0, s_item = 0,
    by_search_range = 0, by_tracing_range = 0, by_direct_attack = 0, by_magic_attack = 0;
INSERT INTO npc_template SELECT * FROM _seal;
DROP TABLE _seal;

-- escape structure: friendly (group 3), so it cannot be attacked or targeted for damage
CREATE TEMP TABLE _esc AS SELECT * FROM npc_template WHERE s_sid = 31998 AND is_monster = FALSE;
UPDATE _esc SET s_sid = 31994, str_name = 'Safe Circle', s_pid = 5301, s_size = 100, by_group = 3, by_act_type = 1;
INSERT INTO npc_template SELECT * FROM _esc;
UPDATE _esc SET s_sid = 31997, str_name = '[Ultima''s Dungeon] Keeper', s_pid = 30500;
INSERT INTO npc_template SELECT * FROM _esc;
DROP TABLE _esc;

DELETE FROM npc_spawn WHERE npc_id = 31997 AND is_monster = FALSE;
INSERT INTO npc_spawn
    (zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
     special_type, trap_number, left_x, top_z, num_npc, spawn_range,
     regen_time, direction, dot_cnt, path, room)
VALUES (21, 31997, FALSE, 0, 0, 0, 0, 0, 833, 520, 1, 0, 0, 180, 0, NULL, 0);
