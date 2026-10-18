-- ArdReam (72) / Ronark Land Base (73) level brackets and chaos stone boss setup.
-- ArdReam: 35-59, Ronark Land Base: 45-69 (Ronark Land/71 already 70-83, unchanged).
UPDATE zone_info SET max_level = 59 WHERE zone_no = 72;
UPDATE zone_info SET min_level = 45, max_level = 69 WHERE zone_no = 73;

-- Ronark Land Base chaos stone was defined but never opened/positioned.
-- Open it at the requested spawn point for all four ranks.
UPDATE chaos_stone_spawn
SET is_open = true, spawn_x = 526, spawn_z = 532
WHERE zone_id = 73;

-- Give ArdReam's and Ronark Land Base's chaos stones the same boss summon
-- pool used by Ronark Land (71), so killing the stone spawns a boss there too.
-- n_index has no sequence (manually assigned PK in the seed data), so pick
-- the next free block explicitly.
INSERT INTO chaos_stone_summon_list (n_index, zone_id, sid, monster_spawn_family) VALUES
(19, 72, 8254, 1),
(20, 72, 8915, 1),
(21, 72, 8907, 1),
(22, 72, 8259, 1),
(23, 72, 8251, 1),
(24, 72, 8258, 1),
(25, 72, 8253, 2),
(26, 72, 8260, 2),
(27, 72, 8814, 2),
(28, 72, 8256, 2),
(29, 72, 8820, 2),
(30, 72, 8250, 2),
(31, 72, 8255, 3),
(32, 72, 8257, 3),
(33, 72, 8908, 3),
(34, 72, 8252, 3),
(35, 72, 8916, 3),
(36, 72, 8917, 3),
(37, 73, 8254, 1),
(38, 73, 8915, 1),
(39, 73, 8907, 1),
(40, 73, 8259, 1),
(41, 73, 8251, 1),
(42, 73, 8258, 1),
(43, 73, 8253, 2),
(44, 73, 8260, 2),
(45, 73, 8814, 2),
(46, 73, 8256, 2),
(47, 73, 8820, 2),
(48, 73, 8250, 2),
(49, 73, 8255, 3),
(50, 73, 8257, 3),
(51, 73, 8908, 3),
(52, 73, 8252, 3),
(53, 73, 8916, 3),
(54, 73, 8917, 3)
ON CONFLICT DO NOTHING;
