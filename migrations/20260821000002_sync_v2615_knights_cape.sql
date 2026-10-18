-- Synchronise PostgreSQL's KNIGHTS_CAPE catalogue with the authoritative
-- KO_DATABASE_SERVER_001.dbo.KNIGHTS_CAPE2369 / v2615 Cloak.tbl data.
--
-- Do not edit the historical 20260303000013 migration: it may already be
-- recorded by sqlx.  This corrective migration is deliberately idempotent.

WITH cape AS (
    SELECT
        s_cape_index,
        (s_cape_index % 100) AS suffix,
        (s_cape_index / 100) AS family
    FROM knights_cape
)
UPDATE knights_cape AS k
SET
    n_buy_price = CASE
        WHEN c.s_cape_index = 0 THEN 0
        WHEN c.suffix BETWEEN 1 AND 9 AND c.family BETWEEN 0 AND 5 THEN
            CASE (c.suffix % 3)
                WHEN 1 THEN (20 + c.family * 10) * 1000000
                WHEN 2 THEN (50 + c.family * 10) * 1000000
                ELSE (100 + c.family * 10) * 1000000
            END
        WHEN c.suffix BETWEEN 65 AND 80 THEN 300000000
        ELSE 0
    END,
    by_grade = CASE
        WHEN c.s_cape_index = 0 THEN 3
        WHEN c.suffix BETWEEN 1 AND 9 AND c.family BETWEEN 0 AND 5 THEN
            CASE (c.suffix % 3) WHEN 1 THEN 3 WHEN 2 THEN 2 ELSE 1 END
        ELSE 0
    END,
    n_buy_loyalty = CASE
        WHEN c.suffix BETWEEN 10 AND 18 THEN 36000
        WHEN c.suffix = 29 THEN 180000
        WHEN c.suffix = 30 THEN 288000
        WHEN c.suffix = 31 THEN 432000
        WHEN c.suffix = 32 THEN 648000
        WHEN c.suffix = 33 THEN 864000
        WHEN c.suffix BETWEEN 40 AND 48 THEN 360000
        WHEN c.suffix = 60 THEN 1080000
        WHEN c.suffix = 61 THEN 1368000
        WHEN c.suffix = 62 THEN 1728000
        WHEN c.suffix = 63 THEN 2160000
        WHEN c.suffix = 64 THEN 2880000
        WHEN c.suffix BETWEEN 65 AND 80 THEN 150000
        ELSE 0
    END,
    by_ranking = CASE
        WHEN c.s_cape_index = 0 THEN 2
        WHEN c.suffix BETWEEN 1 AND 9 AND c.family BETWEEN 0 AND 5 THEN 2
        WHEN c.suffix BETWEEN 10 AND 18 OR c.suffix = 29 THEN 3
        WHEN c.suffix BETWEEN 30 AND 33 THEN c.suffix - 26
        WHEN c.suffix BETWEEN 40 AND 48 OR c.suffix = 60
             OR c.suffix BETWEEN 65 AND 80 THEN 8
        WHEN c.suffix BETWEEN 61 AND 64 THEN c.suffix - 52
        WHEN c.s_cape_index BETWEEN 97 AND 99 THEN 12
        ELSE 0
    END,
    b_type = CASE
        WHEN c.s_cape_index = 0 THEN 1
        WHEN c.suffix BETWEEN 1 AND 9 AND c.family BETWEEN 0 AND 5 THEN 1
        WHEN c.suffix BETWEEN 10 AND 18 OR c.suffix = 29
             OR c.suffix BETWEEN 30 AND 33 OR c.suffix BETWEEN 40 AND 48
             OR c.suffix BETWEEN 60 AND 64 THEN 2
        WHEN c.suffix BETWEEN 65 AND 80 OR c.s_cape_index BETWEEN 97 AND 99 THEN 3
        ELSE 0
    END,
    b_ticket = CASE WHEN c.suffix BETWEEN 65 AND 80 THEN 1 ELSE 0 END,
    bonus_type = CASE
        WHEN c.suffix BETWEEN 60 AND 64 THEN c.suffix - 53
        WHEN c.suffix BETWEEN 65 AND 80 THEN 7
        ELSE 0
    END
FROM cape AS c
WHERE c.s_cape_index = k.s_cape_index;

