-- The previous migration (20261017000003) hardcoded n_index 19-54 for the new
-- ArdReam/Ronark Land Base boss summon rows, assuming the table only had
-- Ronark Land's 18 rows. The live table already had other zones' rows using
-- those exact indexes, so every insert there silently no-opped (ON CONFLICT
-- DO NOTHING). Re-insert using indexes computed from the table's real current
-- max, which can't collide.
DO $$
DECLARE
    next_idx INTEGER;
BEGIN
    SELECT COALESCE(MAX(n_index), 0) + 1 INTO next_idx FROM chaos_stone_summon_list;

    IF NOT EXISTS (SELECT 1 FROM chaos_stone_summon_list WHERE zone_id = 72) THEN
        INSERT INTO chaos_stone_summon_list (n_index, zone_id, sid, monster_spawn_family)
        SELECT next_idx + ROW_NUMBER() OVER () - 1, 72, sid, monster_spawn_family
        FROM chaos_stone_summon_list WHERE zone_id = 71 ORDER BY n_index;

        SELECT COALESCE(MAX(n_index), 0) + 1 INTO next_idx FROM chaos_stone_summon_list;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM chaos_stone_summon_list WHERE zone_id = 73) THEN
        INSERT INTO chaos_stone_summon_list (n_index, zone_id, sid, monster_spawn_family)
        SELECT next_idx + ROW_NUMBER() OVER () - 1, 73, sid, monster_spawn_family
        FROM chaos_stone_summon_list WHERE zone_id = 71 ORDER BY n_index;
    END IF;
END $$;
