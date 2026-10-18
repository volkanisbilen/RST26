-- Full Moon Rift balance: players deal ordinary hits, so the waves are far weaker than first set.
UPDATE npc_template SET i_hp_point=1500, s_ac=150 WHERE s_sid BETWEEN 9997 AND 9999 AND is_monster=TRUE;
UPDATE npc_template SET i_hp_point=2500, s_ac=150 WHERE s_sid BETWEEN 10001 AND 10004 AND is_monster=TRUE;
UPDATE npc_template SET i_hp_point=4000, s_ac=150 WHERE s_sid BETWEEN 10006 AND 10010 AND is_monster=TRUE;
UPDATE npc_template SET i_hp_point=6000, s_ac=150 WHERE s_sid BETWEEN 10012 AND 10018 AND is_monster=TRUE;
UPDATE npc_template SET i_hp_point=20000,  s_ac=200 WHERE s_sid=10000 AND is_monster=TRUE;
UPDATE npc_template SET i_hp_point=35000,  s_ac=200 WHERE s_sid=10005 AND is_monster=TRUE;
UPDATE npc_template SET i_hp_point=60000,  s_ac=200 WHERE s_sid=10011 AND is_monster=TRUE;
UPDATE npc_template SET i_hp_point=100000, s_ac=200 WHERE s_sid=10019 AND is_monster=TRUE;
