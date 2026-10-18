-- Grade-ring layout derived from the user-approved outer map bounds.
-- Outer low anchors use X 260..728 / Z 207..644; higher grades move inward.

BEGIN;

UPDATE manes_survival_spawn AS s
SET spawn_x = v.spawn_x,
    spawn_z = v.spawn_z,
    spawn_range = v.spawn_range
FROM (VALUES
    -- Low normal monsters: outer walkable perimeter.
    (10701,345,207,24),(10702,260,440,24),(10703,345,581,24),
    (10704,722,644,24),(10705,728,219,24),
    -- Low bosses: official inner-field ring.
    (10706,695,440,8),(10707,505,615,8),
    (10708,315,440,8),(10709,505,265,8),
    -- Mid normal monsters: wider ring outside the verified high ring.
    (10710,670,440,20),(10711,587,605,20),(10712,422,605,20),
    (10713,340,440,20),(10714,422,275,20),(10715,587,275,20),
    -- Four mid bosses remain inside the central castle.
    (10716,464,464,5),(10717,560,464,5),
    (10718,464,560,5),(10719,560,560,5)
) AS v(npc_id,spawn_x,spawn_z,spawn_range)
WHERE s.npc_id = v.npc_id;

DO $$
BEGIN
    IF (SELECT COUNT(*) FROM manes_survival_spawn
        WHERE npc_id BETWEEN 10701 AND 10719) <> 19 THEN
        RAISE EXCEPTION 'Manes Survival low/mid ring update failed';
    END IF;
END $$;

COMMIT;
