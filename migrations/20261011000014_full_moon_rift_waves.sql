-- Full Moon Rift: 20-wave plan (Easy 5 / Normal 15 / Hard 20), army monsters 9997-10019.
DELETE FROM df_stage_list;
DELETE FROM df_monster_list;
INSERT INTO df_stage_list (id, difficulty, difficulty_name, stage_id) VALUES
(101, 1, 'Easy', 1),
(102, 1, 'Easy', 2),
(103, 1, 'Easy', 3),
(104, 1, 'Easy', 4),
(105, 1, 'Easy', 5),
(201, 2, 'Normal', 1),
(202, 2, 'Normal', 2),
(203, 2, 'Normal', 3),
(204, 2, 'Normal', 4),
(205, 2, 'Normal', 5),
(206, 2, 'Normal', 6),
(207, 2, 'Normal', 7),
(208, 2, 'Normal', 8),
(209, 2, 'Normal', 9),
(210, 2, 'Normal', 10),
(211, 2, 'Normal', 11),
(212, 2, 'Normal', 12),
(213, 2, 'Normal', 13),
(214, 2, 'Normal', 14),
(215, 2, 'Normal', 15),
(301, 3, 'Hard', 1),
(302, 3, 'Hard', 2),
(303, 3, 'Hard', 3),
(304, 3, 'Hard', 4),
(305, 3, 'Hard', 5),
(306, 3, 'Hard', 6),
(307, 3, 'Hard', 7),
(308, 3, 'Hard', 8),
(309, 3, 'Hard', 9),
(310, 3, 'Hard', 10),
(311, 3, 'Hard', 11),
(312, 3, 'Hard', 12),
(313, 3, 'Hard', 13),
(314, 3, 'Hard', 14),
(315, 3, 'Hard', 15),
(316, 3, 'Hard', 16),
(317, 3, 'Hard', 17),
(318, 3, 'Hard', 18),
(319, 3, 'Hard', 19),
(320, 3, 'Hard', 20);
INSERT INTO df_monster_list (id, difficulty, monster_id, is_monster, pos_x, pos_z, s_count, s_direction, s_radius_range) VALUES
(101, 1, 9997, FALSE, 60, 42, 5, 0, 10),
(102, 1, 9998, FALSE, 60, 42, 5, 0, 10),
(103, 1, 9999, FALSE, 60, 42, 5, 0, 10),
(201, 1, 9997, FALSE, 60, 42, 4, 0, 10),
(202, 1, 9998, FALSE, 60, 42, 5, 0, 10),
(203, 1, 9999, FALSE, 60, 42, 4, 0, 10),
(301, 1, 9997, FALSE, 60, 42, 4, 0, 10),
(302, 1, 9998, FALSE, 60, 42, 4, 0, 10),
(303, 1, 9999, FALSE, 60, 42, 4, 0, 10),
(401, 1, 9997, FALSE, 60, 42, 5, 0, 10),
(402, 1, 9998, FALSE, 60, 42, 5, 0, 10),
(403, 1, 9999, FALSE, 60, 42, 6, 0, 10),
(501, 1, 9997, FALSE, 60, 42, 4, 0, 10),
(502, 1, 9998, FALSE, 60, 42, 5, 0, 10),
(503, 1, 9999, FALSE, 60, 42, 4, 0, 10),
(504, 1, 10000, FALSE, 60, 42, 1, 0, 0),
(601, 1, 10001, FALSE, 60, 42, 3, 0, 10),
(602, 1, 10002, FALSE, 60, 42, 5, 0, 10),
(603, 1, 10003, FALSE, 60, 42, 6, 0, 10),
(604, 1, 10004, FALSE, 60, 42, 5, 0, 10),
(701, 1, 10001, FALSE, 60, 42, 3, 0, 10),
(702, 1, 10002, FALSE, 60, 42, 6, 0, 10),
(703, 1, 10003, FALSE, 60, 42, 4, 0, 10),
(704, 1, 10004, FALSE, 60, 42, 4, 0, 10),
(801, 1, 10001, FALSE, 60, 42, 3, 0, 10),
(802, 1, 10002, FALSE, 60, 42, 5, 0, 10),
(803, 1, 10003, FALSE, 60, 42, 3, 0, 10),
(804, 1, 10004, FALSE, 60, 42, 3, 0, 10),
(901, 1, 10001, FALSE, 60, 42, 3, 0, 10),
(902, 1, 10002, FALSE, 60, 42, 5, 0, 10),
(903, 1, 10003, FALSE, 60, 42, 4, 0, 10),
(904, 1, 10004, FALSE, 60, 42, 4, 0, 10),
(1001, 1, 10001, FALSE, 60, 42, 3, 0, 10),
(1002, 1, 10002, FALSE, 60, 42, 5, 0, 10),
(1003, 1, 10003, FALSE, 60, 42, 4, 0, 10),
(1004, 1, 10004, FALSE, 60, 42, 4, 0, 10),
(1005, 1, 10005, FALSE, 60, 42, 1, 0, 0),
(1101, 1, 10006, FALSE, 60, 42, 4, 0, 10),
(1102, 1, 10007, FALSE, 60, 42, 4, 0, 10),
(1103, 1, 10008, FALSE, 60, 42, 4, 0, 10),
(1104, 1, 10009, FALSE, 60, 42, 3, 0, 10),
(1105, 1, 10010, FALSE, 60, 42, 4, 0, 10),
(1201, 1, 10006, FALSE, 60, 42, 4, 0, 10),
(1202, 1, 10007, FALSE, 60, 42, 4, 0, 10),
(1203, 1, 10008, FALSE, 60, 42, 3, 0, 10),
(1204, 1, 10009, FALSE, 60, 42, 4, 0, 10),
(1205, 1, 10010, FALSE, 60, 42, 4, 0, 10),
(1301, 1, 10006, FALSE, 60, 42, 4, 0, 10),
(1302, 1, 10007, FALSE, 60, 42, 4, 0, 10),
(1303, 1, 10008, FALSE, 60, 42, 5, 0, 10),
(1304, 1, 10009, FALSE, 60, 42, 4, 0, 10),
(1305, 1, 10010, FALSE, 60, 42, 4, 0, 10),
(1401, 1, 10006, FALSE, 60, 42, 4, 0, 10),
(1402, 1, 10007, FALSE, 60, 42, 4, 0, 10),
(1403, 1, 10008, FALSE, 60, 42, 4, 0, 10),
(1404, 1, 10009, FALSE, 60, 42, 4, 0, 10),
(1405, 1, 10010, FALSE, 60, 42, 4, 0, 10),
(1501, 1, 10006, FALSE, 60, 42, 4, 0, 10),
(1502, 1, 10007, FALSE, 60, 42, 4, 0, 10),
(1503, 1, 10008, FALSE, 60, 42, 4, 0, 10),
(1504, 1, 10009, FALSE, 60, 42, 4, 0, 10),
(1505, 1, 10010, FALSE, 60, 42, 4, 0, 10),
(1506, 1, 10011, FALSE, 60, 42, 1, 0, 0),
(1601, 1, 10012, FALSE, 60, 42, 3, 0, 10),
(1602, 1, 10013, FALSE, 60, 42, 3, 0, 10),
(1603, 1, 10014, FALSE, 60, 42, 3, 0, 10),
(1604, 1, 10015, FALSE, 60, 42, 3, 0, 10),
(1605, 1, 10016, FALSE, 60, 42, 3, 0, 10),
(1606, 1, 10017, FALSE, 60, 42, 4, 0, 10),
(1607, 1, 10018, FALSE, 60, 42, 4, 0, 10),
(1701, 1, 10012, FALSE, 60, 42, 3, 0, 10),
(1702, 1, 10013, FALSE, 60, 42, 3, 0, 10),
(1703, 1, 10014, FALSE, 60, 42, 3, 0, 10),
(1704, 1, 10015, FALSE, 60, 42, 3, 0, 10),
(1705, 1, 10016, FALSE, 60, 42, 4, 0, 10),
(1706, 1, 10017, FALSE, 60, 42, 4, 0, 10),
(1707, 1, 10018, FALSE, 60, 42, 4, 0, 10),
(1801, 1, 10012, FALSE, 60, 42, 3, 0, 10),
(1802, 1, 10013, FALSE, 60, 42, 3, 0, 10),
(1803, 1, 10014, FALSE, 60, 42, 3, 0, 10),
(1804, 1, 10015, FALSE, 60, 42, 4, 0, 10),
(1805, 1, 10016, FALSE, 60, 42, 4, 0, 10),
(1806, 1, 10017, FALSE, 60, 42, 4, 0, 10),
(1807, 1, 10018, FALSE, 60, 42, 4, 0, 10),
(1901, 1, 10012, FALSE, 60, 42, 3, 0, 10),
(1902, 1, 10013, FALSE, 60, 42, 3, 0, 10),
(1903, 1, 10014, FALSE, 60, 42, 4, 0, 10),
(1904, 1, 10015, FALSE, 60, 42, 4, 0, 10),
(1905, 1, 10016, FALSE, 60, 42, 4, 0, 10),
(1906, 1, 10017, FALSE, 60, 42, 4, 0, 10),
(1907, 1, 10018, FALSE, 60, 42, 3, 0, 10),
(2001, 1, 10012, FALSE, 60, 42, 3, 0, 10),
(2002, 1, 10013, FALSE, 60, 42, 4, 0, 10),
(2003, 1, 10014, FALSE, 60, 42, 3, 0, 10),
(2004, 1, 10015, FALSE, 60, 42, 3, 0, 10),
(2005, 1, 10016, FALSE, 60, 42, 3, 0, 10),
(2006, 1, 10017, FALSE, 60, 42, 3, 0, 10),
(2007, 1, 10018, FALSE, 60, 42, 3, 0, 10),
(2008, 1, 10019, FALSE, 60, 42, 1, 0, 0);
UPDATE npc_template SET s_level=80, i_hp_point=8000, s_damage=300, s_atk=300, i_exp=20000, s_standtime=3000 WHERE s_sid=9997 AND is_monster=TRUE;
UPDATE npc_template SET s_level=80, i_hp_point=8000, s_damage=300, s_atk=300, i_exp=20000, s_standtime=3000 WHERE s_sid=9998 AND is_monster=TRUE;
UPDATE npc_template SET s_level=80, i_hp_point=8000, s_damage=300, s_atk=300, i_exp=20000, s_standtime=3000 WHERE s_sid=9999 AND is_monster=TRUE;
UPDATE npc_template SET s_level=82, i_hp_point=12000, s_damage=400, s_atk=400, i_exp=40000, s_standtime=3000 WHERE s_sid=10001 AND is_monster=TRUE;
UPDATE npc_template SET s_level=82, i_hp_point=12000, s_damage=400, s_atk=400, i_exp=40000, s_standtime=3000 WHERE s_sid=10002 AND is_monster=TRUE;
UPDATE npc_template SET s_level=82, i_hp_point=12000, s_damage=400, s_atk=400, i_exp=40000, s_standtime=3000 WHERE s_sid=10003 AND is_monster=TRUE;
UPDATE npc_template SET s_level=82, i_hp_point=12000, s_damage=400, s_atk=400, i_exp=40000, s_standtime=3000 WHERE s_sid=10004 AND is_monster=TRUE;
UPDATE npc_template SET s_level=84, i_hp_point=18000, s_damage=500, s_atk=500, i_exp=80000, s_standtime=3000 WHERE s_sid=10006 AND is_monster=TRUE;
UPDATE npc_template SET s_level=84, i_hp_point=18000, s_damage=500, s_atk=500, i_exp=80000, s_standtime=3000 WHERE s_sid=10007 AND is_monster=TRUE;
UPDATE npc_template SET s_level=84, i_hp_point=18000, s_damage=500, s_atk=500, i_exp=80000, s_standtime=3000 WHERE s_sid=10008 AND is_monster=TRUE;
UPDATE npc_template SET s_level=84, i_hp_point=18000, s_damage=500, s_atk=500, i_exp=80000, s_standtime=3000 WHERE s_sid=10009 AND is_monster=TRUE;
UPDATE npc_template SET s_level=84, i_hp_point=18000, s_damage=500, s_atk=500, i_exp=80000, s_standtime=3000 WHERE s_sid=10010 AND is_monster=TRUE;
UPDATE npc_template SET s_level=86, i_hp_point=25000, s_damage=600, s_atk=600, i_exp=120000, s_standtime=3000 WHERE s_sid=10012 AND is_monster=TRUE;
UPDATE npc_template SET s_level=86, i_hp_point=25000, s_damage=600, s_atk=600, i_exp=120000, s_standtime=3000 WHERE s_sid=10013 AND is_monster=TRUE;
UPDATE npc_template SET s_level=86, i_hp_point=25000, s_damage=600, s_atk=600, i_exp=120000, s_standtime=3000 WHERE s_sid=10014 AND is_monster=TRUE;
UPDATE npc_template SET s_level=86, i_hp_point=25000, s_damage=600, s_atk=600, i_exp=120000, s_standtime=3000 WHERE s_sid=10015 AND is_monster=TRUE;
UPDATE npc_template SET s_level=86, i_hp_point=25000, s_damage=600, s_atk=600, i_exp=120000, s_standtime=3000 WHERE s_sid=10016 AND is_monster=TRUE;
UPDATE npc_template SET s_level=86, i_hp_point=25000, s_damage=600, s_atk=600, i_exp=120000, s_standtime=3000 WHERE s_sid=10017 AND is_monster=TRUE;
UPDATE npc_template SET s_level=86, i_hp_point=25000, s_damage=600, s_atk=600, i_exp=120000, s_standtime=3000 WHERE s_sid=10018 AND is_monster=TRUE;
UPDATE npc_template SET s_level=80, i_hp_point=100000, s_damage=450, s_atk=450, i_exp=300000 WHERE s_sid=10000 AND is_monster=TRUE;
UPDATE npc_template SET s_level=82, i_hp_point=160000, s_damage=600, s_atk=600, i_exp=500000 WHERE s_sid=10005 AND is_monster=TRUE;
UPDATE npc_template SET s_level=84, i_hp_point=250000, s_damage=750, s_atk=750, i_exp=800000 WHERE s_sid=10011 AND is_monster=TRUE;
UPDATE npc_template SET s_level=86, i_hp_point=400000, s_damage=900, s_atk=900, i_exp=1500000 WHERE s_sid=10019 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Kapikulu Royal Guard' WHERE s_sid=9997 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Timarli' WHERE s_sid=9998 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Sipahi Heavy Calvary' WHERE s_sid=9999 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Mammoth III' WHERE s_sid=10000 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Astaroth', s_pid=20506 WHERE s_sid=10001 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Ahtal' WHERE s_sid=10002 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Amy', s_pid=20512 WHERE s_sid=10003 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Muspell' WHERE s_sid=10004 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Freed Gigantic Hammer' WHERE s_sid=10005 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Enhanced Portu' WHERE s_sid=10006 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Centaur', s_pid=9926 WHERE s_sid=10007 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Enhanced Rhino' WHERE s_sid=10008 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Disposed Subject No.30' WHERE s_sid=10009 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Draki''s Elite' WHERE s_sid=10010 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Black Draki' WHERE s_sid=10011 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Pride', s_pid=9881 WHERE s_sid=10012 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Gluttony', s_pid=9882 WHERE s_sid=10013 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Wrath', s_pid=9883 WHERE s_sid=10014 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Sloth', s_pid=9884 WHERE s_sid=10015 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Lust', s_pid=9885 WHERE s_sid=10016 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Envy', s_pid=9886 WHERE s_sid=10017 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Greed', s_pid=9887 WHERE s_sid=10018 AND is_monster=TRUE;
UPDATE npc_template SET str_name='Ultima', s_pid=9888 WHERE s_sid=10019 AND is_monster=TRUE;
