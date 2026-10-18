-- Synchronize the server packet with the exact 28 reward rows in the
-- v2615 Data/Attendance.tbl supplied with the client.
--
-- This intentionally uses a new migration version. The earlier Attendance
-- synchronization migration may already be recorded by SQLx, so editing it
-- would not repair an existing database.

INSERT INTO daily_reward (day_index, item_id) VALUES
    (0,  900145000),
    (1,  900146000),
    (2,  910252000),
    (3,  910250000),
    (4,  910249000),
    (5,  910251000),
    (6,  910938000),
    (7,  910250000),
    (8,  900015000),
    (9,  900145000),
    (10, 900146000),
    (11, 910252000),
    (12, 811095000),
    (13, 910938000),
    (14, 910249000),
    (15, 900015000),
    (16, 900145000),
    (17, 900146000),
    (18, 811101000),
    (19, 811090000),
    (20, 931773000),
    (21, 910251000),
    (22, 900015000),
    (23, 910925000),
    (24, 900175000)
ON CONFLICT (day_index) DO UPDATE SET
    item_id = EXCLUDED.item_id;

INSERT INTO daily_reward_cumulative (id, item1, item2, item3)
VALUES (1, 810504000, 810686000, 900120000)
ON CONFLICT (id) DO UPDATE SET
    item1 = EXCLUDED.item1,
    item2 = EXCLUDED.item2,
    item3 = EXCLUDED.item3;
