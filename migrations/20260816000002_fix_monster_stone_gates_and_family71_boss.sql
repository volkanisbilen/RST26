-- Every Castle Gate row in the Monster Stone spawn table was imported with
-- direction 45. The v2615 room geometry requires direction 90.
UPDATE monster_stone_respawn_list
SET by_direction = 90
WHERE lower(str_name) = 'gate'
   OR s_sid IN (7032, 7033, 7034);

-- Family 71's final monster is Grief Reaper (runtime proto 7008). Mark it as
-- the boss so room completion and the boss-grace timer run after its death.
UPDATE monster_stone_respawn_list
SET is_boss = TRUE
WHERE zone_id = 82
  AND family = 71
  AND s_sid = 7008;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM monster_stone_respawn_list
        WHERE (lower(str_name) = 'gate' OR s_sid IN (7032, 7033, 7034))
          AND by_direction <> 90
    ) THEN
        RAISE EXCEPTION 'Monster Stone gate direction migration is incomplete';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM monster_stone_respawn_list
        WHERE zone_id = 82
          AND family = 71
          AND s_sid = 7008
          AND is_boss = TRUE
    ) THEN
        RAISE EXCEPTION 'Family 71 Grief Reaper boss flag was not applied';
    END IF;
END $$;
