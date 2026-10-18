-- Runtime event fixes. This is intentionally a new migration: already-applied
-- SQLx migrations must never be edited because their checksum is immutable.

-- A bucket changes at 18:00 Europe/Istanbul. Per-user lazy reset means a server
-- that was offline at 18:00 cannot leave a player permanently at zero entries.
ALTER TABLE user_draki_tower_data
    ADD COLUMN IF NOT EXISTS draki_limit_reset_bucket DATE;

UPDATE user_draki_tower_data
SET b_draki_enterance_limit = 3,
    draki_limit_reset_bucket =
        (CURRENT_TIMESTAMP AT TIME ZONE 'Europe/Istanbul' - INTERVAL '18 hours')::date;

-- 2615 packet captures identify 31772 as a non-monster Event Manager. Give it
-- the same neutral/static NPC semantics used by regular Moradon service NPCs.
UPDATE npc_template
SET str_name = '[Event Manager] Aset',
    by_group = 3,
    by_act_type = 1,
    i_hp_point = 30000,
    s_mp_point = 0,
    s_atk = 0,
    s_damage = 0,
    by_speed_1 = 0,
    by_speed_2 = 0,
    by_search_range = 0,
    by_tracing_range = 0
WHERE s_sid = 31772 AND is_monster = FALSE;
