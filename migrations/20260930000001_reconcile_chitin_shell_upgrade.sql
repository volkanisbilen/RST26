-- Reconcile missing Chitin Shell +6 -> +7 Blessed Upgrade Scroll recipes.
-- The full upgrade import already contains these in a complete database; this
-- idempotent repair covers installations whose imported recipe rows were partial.
WITH candidates AS (
    SELECT i.num AS origin_number,
           n.num AS new_number,
           i.str_name AS str_note,
           n.str_name AS n_str_note,
           (i.num % 10)::smallint AS grade
    FROM item i
    JOIN item n ON n.num = i.num + 1
    WHERE i.item_class = 3
      AND n.item_class = 3
      AND i.item_type IN (4, 5)
      AND n.item_type = i.item_type
      AND i.kind = n.kind
      AND i.slot = n.slot
      AND i.num % 10 = 6
      AND btrim(i.str_name) ILIKE '%Chitin Shell%(+6)'
      AND btrim(n.str_name) ILIKE '%Chitin Shell%(+7)'
      AND regexp_replace(btrim(i.str_name), '\(\+[0-9]+\)$', '')
          = regexp_replace(btrim(n.str_name), '\(\+[0-9]+\)$', '')
      AND NOT EXISTS (
          SELECT 1
          FROM new_upgrade u
          WHERE u.origin_number = i.num
            AND u.new_number = n.num
            AND u.req_item = 379021000
      )
), numbered AS (
    SELECT *, row_number() OVER (ORDER BY origin_number) AS rn
    FROM candidates
)
INSERT INTO new_upgrade
    (n_index, str_note, origin_number, n_str_note, new_number, req_item, grade)
SELECT (SELECT COALESCE(max(n_index), 0) FROM new_upgrade) + rn,
       str_note, origin_number, n_str_note, new_number, 379021000, grade
FROM numbered;

-- Preserve the imported success rates; only add the canonical grade-6 setting
-- if a damaged/partial DB is missing it. 3,000 = 30% as in the base data.
INSERT INTO item_upgrade_settings
    (req_item_id1, req_item_name1, req_item_id2, req_item_name2,
     upgrade_note, item_type, item_rate, item_grade, item_req_coins, success_rate)
SELECT 379021000, 'Blessed Upgrade Scroll', 0, '', 'Normal Item Upgrade',
       required_type, 33, 6, 10000, 3000
FROM (VALUES (4::smallint), (5::smallint)) AS expected(required_type)
WHERE NOT EXISTS (
    SELECT 1
    FROM item_upgrade_settings s
    WHERE s.item_type = expected.required_type
      AND s.item_grade = 6
      AND s.req_item_id1 = 379021000
      AND s.req_item_id2 = 0
);
