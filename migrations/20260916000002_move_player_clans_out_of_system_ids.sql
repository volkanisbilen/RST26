-- Move accidentally-created player clans out of reserved auto-clan IDs.
--
-- Reference server reserves:
--   Karus auto clan   = 1
--   El Morad auto clan = 15001
--
-- If a real player clan is stored with one of these IDs, v2615 treats it like
-- an auto/system clan in several clan/cape paths. This breaks equipped cloak
-- visibility and the cape merchant catalogue. The Rust allocator is fixed to
-- skip these IDs; this migration repairs existing live data safely.

DO $$
DECLARE
    old_id SMALLINT;
    new_id SMALLINT;
    original_name TEXT;
    temp_name TEXT;
BEGIN
    FOR old_id IN
        SELECT id_num
        FROM knights
        WHERE id_num IN (1, 15001)
          AND COALESCE(chief, '') <> ''
          AND EXISTS (SELECT 1 FROM userdata u WHERE u.knights = knights.id_num)
        ORDER BY id_num
    LOOP
        IF old_id = 1 THEN
            SELECT candidate::SMALLINT
            INTO new_id
            FROM generate_series(2, 14999) AS candidate
            WHERE NOT EXISTS (SELECT 1 FROM knights k WHERE k.id_num = candidate)
            ORDER BY candidate
            LIMIT 1;
        ELSE
            SELECT candidate::SMALLINT
            INTO new_id
            FROM generate_series(15002, 32000) AS candidate
            WHERE NOT EXISTS (SELECT 1 FROM knights k WHERE k.id_num = candidate)
            ORDER BY candidate
            LIMIT 1;
        END IF;

        IF new_id IS NULL THEN
            RAISE EXCEPTION 'No free player clan ID available for reserved clan %', old_id;
        END IF;

        SELECT id_name
        INTO original_name
        FROM knights
        WHERE id_num = old_id;

        temp_name := left(original_name, 12) || '_mig' || old_id::TEXT;

        INSERT INTO knights (
            id_num, flag, nation, ranking, id_name, members, chief,
            vice_chief_1, vice_chief_2, vice_chief_3, gold, domination,
            points, mark, s_mark_version, s_mark_len, s_cape, b_cape_r,
            b_cape_g, b_cape_b, s_cast_cape, b_cast_cape_r, b_cast_cape_g,
            b_cast_cape_b, b_cast_time, s_alliance_knights, clan_point_fund,
            str_clan_notice, by_siege_flag, n_lose, n_victory,
            clan_point_method, n_money, dw_time, warehouse_data, str_serial,
            s_premium_time, s_premium_in_use, dt_create_time
        )
        SELECT
            new_id, flag, nation, ranking, temp_name, members, chief,
            vice_chief_1, vice_chief_2, vice_chief_3, gold, domination,
            points, mark, s_mark_version, s_mark_len, s_cape, b_cape_r,
            b_cape_g, b_cape_b, s_cast_cape, b_cast_cape_r, b_cast_cape_g,
            b_cast_cape_b, b_cast_time, s_alliance_knights, clan_point_fund,
            str_clan_notice, by_siege_flag, n_lose, n_victory,
            clan_point_method, n_money, dw_time, warehouse_data, str_serial,
            s_premium_time, s_premium_in_use, dt_create_time
        FROM knights
        WHERE id_num = old_id;

        UPDATE userdata
        SET knights = new_id
        WHERE knights = old_id;

        UPDATE user_knightdata
        SET s_clan_id = new_id
        WHERE s_clan_id = old_id
          AND NOT EXISTS (
              SELECT 1
              FROM user_knightdata existing
              WHERE existing.s_clan_id = new_id
                AND existing.str_user_id = user_knightdata.str_user_id
          );

        DELETE FROM user_knightdata
        WHERE s_clan_id = old_id;

        UPDATE knights
        SET s_alliance_knights = new_id
        WHERE s_alliance_knights = old_id;

        UPDATE knights_alliance
        SET s_main_alliance_knights = new_id
        WHERE s_main_alliance_knights = old_id;

        UPDATE knights_alliance
        SET s_sub_alliance_knights = new_id
        WHERE s_sub_alliance_knights = old_id;

        UPDATE knights_alliance
        SET s_mercenary_clan_1 = new_id
        WHERE s_mercenary_clan_1 = old_id;

        UPDATE knights_alliance
        SET s_mercenary_clan_2 = new_id
        WHERE s_mercenary_clan_2 = old_id;

        UPDATE knights_castellan
        SET id_num = new_id
        WHERE id_num = old_id
          AND NOT EXISTS (SELECT 1 FROM knights_castellan kc WHERE kc.id_num = new_id);

        DELETE FROM knights_castellan
        WHERE id_num = old_id;

        DELETE FROM knights
        WHERE id_num = old_id;

        UPDATE knights
        SET id_name = original_name
        WHERE id_num = new_id;

        RAISE NOTICE 'Moved reserved player clan ID % to %', old_id, new_id;
    END LOOP;
END $$;
