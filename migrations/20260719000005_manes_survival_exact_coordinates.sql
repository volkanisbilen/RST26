-- Exact Manes Survival castle and high-grade ring coordinates.
-- User-verified map coordinates, 2026-07-19.

BEGIN;

UPDATE manes_survival_spawn AS s
SET spawn_x = v.spawn_x,
    spawn_z = v.spawn_z,
    spawn_range = v.spawn_range
FROM (VALUES
    -- High-grade normal monsters: ellipse bounded by X 447..588, Z 348..580.
    (10720,588,464,10),(10721,572,539,10),(10722,530,578,10),
    (10723,482,564,10),(10724,451,504,10),(10725,451,424,10),
    (10726,482,364,10),(10727,530,350,10),(10728,572,389,10),
    -- Four high-grade bosses inside the castle.
    (10729,547,480,4),(10730,461,480,4),
    (10731,461,400,4),(10732,547,400,4),
    -- Dark Dragon exact center position.
    (10733,505,440,0)
) AS v(npc_id,spawn_x,spawn_z,spawn_range)
WHERE s.npc_id = v.npc_id;

DO $$
BEGIN
    IF (SELECT COUNT(*) FROM manes_survival_spawn
        WHERE npc_id BETWEEN 10720 AND 10733) <> 14 THEN
        RAISE EXCEPTION 'Manes Survival high-grade coordinate update failed';
    END IF;
END $$;

COMMIT;
