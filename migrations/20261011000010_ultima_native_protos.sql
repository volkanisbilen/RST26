-- Ultima's Dungeon uses the monster identities the 2625 client already has in mob_us.tbl:
--   10100 / 10110 / 10111  "[Boss] Ultima"                  (bosses for 4-10 / 11-20 / 21-30 players)
--   10101                  "Demolition Seal Artifact"       (the seal)
--   10103                  "[Protection] Ultima Seal Artifact" (escape structures)
-- Boss: HP 1M / 2M / 3M, defense 2683 / 4025 / 8051, attack 7338 / 11007 / 22015.
DELETE FROM npc_template WHERE s_sid IN (31990, 31991, 31992, 31993, 31994, 31997);
DELETE FROM npc_template WHERE is_monster = TRUE AND s_sid IN (10100, 10101, 10103, 10110, 10111);

CREATE TEMP TABLE _ult AS SELECT * FROM npc_template WHERE s_sid = 9888 AND is_monster = TRUE;
UPDATE _ult SET s_sid = 10100, str_name = '[Boss] Ultima', s_pid = 9950, s_size = 150, s_level = 83,
    i_exp = 0, i_loyalty = 0, i_hp_point = 1000000, s_ac = 2683, s_damage = 7338, s_item = 0;
INSERT INTO npc_template SELECT * FROM _ult;
UPDATE _ult SET s_sid = 10110, i_hp_point = 2000000, s_ac = 4025, s_damage = 11007;
INSERT INTO npc_template SELECT * FROM _ult;
UPDATE _ult SET s_sid = 10111, i_hp_point = 3000000, s_ac = 8051, s_damage = 22015;
INSERT INTO npc_template SELECT * FROM _ult;
DROP TABLE _ult;

-- static, harmless structures (the server keeps them from being damaged / only loses the seal's fixed hit)
CREATE TEMP TABLE _st AS SELECT * FROM npc_template WHERE s_sid = 8618 AND is_monster = TRUE;
UPDATE _st SET s_sid = 10101, str_name = 'Demolition Seal Artifact', s_pid = 9949, s_size = 60,
    i_hp_point = 150000, s_ac = 0, s_damage = 0, s_atk = 0, i_exp = 0, i_loyalty = 0, s_item = 0,
    by_search_range = 0, by_tracing_range = 0, by_direct_attack = 0, by_magic_attack = 0;
INSERT INTO npc_template SELECT * FROM _st;
UPDATE _st SET s_sid = 10103, str_name = '[Protection] Ultima Seal Artifact', s_pid = 9947, i_hp_point = 1000000;
INSERT INTO npc_template SELECT * FROM _st;
DROP TABLE _st;
