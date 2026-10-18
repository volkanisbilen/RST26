-- Clan grade is driven by Ladder Points: the sum of each member's monthly NP.
-- `knights.points` is an INTEGER while legacy data can exceed that range, so
-- preserve correct ordering without allowing a startup-blocking overflow.
UPDATE knights k
SET points = LEAST(COALESCE((
    SELECT SUM(u.loyalty_monthly)
    FROM userdata u
    WHERE u.knights = k.id_num
), 0), 2147483647)::INTEGER;

CREATE OR REPLACE FUNCTION compute_knights_rating()
RETURNS void
LANGUAGE plpgsql
AS $function$
BEGIN
    UPDATE knights k
    SET points = LEAST(COALESCE((
        SELECT SUM(u.loyalty_monthly)
        FROM userdata u
        WHERE u.knights = k.id_num
    ), 0), 2147483647)::INTEGER;

    DELETE FROM knights_rating;

    INSERT INTO knights_rating (nation, rank_pos, clan_id, points)
    SELECT
        k.nation,
        ROW_NUMBER() OVER (PARTITION BY k.nation ORDER BY k.points DESC, k.id_num ASC)::INTEGER,
        k.id_num,
        k.points
    FROM knights k
    WHERE k.points > 0;

    UPDATE knights SET ranking = 0;

    UPDATE knights k
    SET ranking = kr.rank_pos::SMALLINT
    FROM knights_rating kr
    WHERE k.id_num = kr.clan_id;
END;
$function$;
