-- Complete standard high-class grade transitions only when both concrete
-- item definitions exist and agree on family, class, kind and equipment slot.
-- Keep existing rates/costs and all special/rebirth recipes unchanged.
WITH candidates AS (
    SELECT i.num AS origin, n.num AS target, i.str_name AS origin_name,
           n.str_name AS target_name, i.num % 10 AS grade
    FROM item i JOIN item n ON n.num = i.num + 1
    WHERE i.item_class = 3 AND n.item_class = 3
      AND i.item_type IN (4, 5) AND n.item_type = i.item_type
      AND i.kind = n.kind AND i.slot = n.slot
      AND i.num % 10 BETWEEN 1 AND 8
      AND btrim(i.str_name) LIKE '%(+' || (i.num % 10)::text || ')'
      AND btrim(n.str_name) LIKE '%(+' || (n.num % 10)::text || ')'
      AND regexp_replace(btrim(i.str_name), '\(\+[0-9]+\)$', '')
          = regexp_replace(btrim(n.str_name), '\(\+[0-9]+\)$', '')
      AND EXISTS (SELECT 1 FROM item_upgrade_settings s
                  WHERE s.item_type = i.item_type AND s.item_grade = i.num % 10
                    AND s.req_item_id1 = 379021000 AND s.req_item_id2 = 0
                    AND s.success_rate > 0)
      AND NOT EXISTS (SELECT 1 FROM new_upgrade u
                      WHERE u.origin_number = i.num AND u.req_item = 379021000)
), numbered AS (
    SELECT *, row_number() OVER (ORDER BY origin) AS rn FROM candidates
)
INSERT INTO new_upgrade (n_index, str_note, origin_number, n_str_note, new_number, req_item, grade)
SELECT (SELECT COALESCE(max(n_index), 0) FROM new_upgrade) + rn,
       origin_name, origin, target_name, target, 379021000, grade
FROM numbered;
