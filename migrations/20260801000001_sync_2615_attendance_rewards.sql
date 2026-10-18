-- Force the server-side Attendance rewards to the v2615 client baseline.
-- The original seed used ON CONFLICT DO NOTHING, leaving existing databases
-- with stale reward rows even after the server source was updated.

INSERT INTO daily_reward (day_index, item_id) VALUES
    (0,  900145000),
    (1,  900146000),
    (2,  910252000),
    (3,  910250000),
    (4,  910249000),
    (5,  910251000),
    (6,  910938000),
    (7,  700085000),
    (8,  900015000),
    (9,  900145000),
    (10, 900146000),
    (11, 910252000),
    (12, 811095000),
    (13, 910938000),
    (14, 700085000),
    (15, 900015000),
    (16, 900145000),
    (17, 900146000),
    (18, 811101000),
    (19, 700089000),
    (20, 931773000),
    (21, 910251000),
    (22, 900015000),
    (23, 910925000),
    (24, 900175000)
ON CONFLICT (day_index) DO UPDATE SET
    item_id = EXCLUDED.item_id;

INSERT INTO daily_reward_cumulative (id, item1, item2, item3)
VALUES (1, 347000000, 347000000, 347000000)
ON CONFLICT (id) DO UPDATE SET
    item1 = EXCLUDED.item1,
    item2 = EXCLUDED.item2,
    item3 = EXCLUDED.item3;
