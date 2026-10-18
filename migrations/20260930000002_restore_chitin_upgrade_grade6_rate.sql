-- Restore the canonical grade-6 Blessed Upgrade Scroll settings only if the
-- earlier defensive migration inserted its temporary fallback values.
UPDATE item_upgrade_settings
SET item_req_coins = 50000,
    success_rate = 3500
WHERE item_type IN (4, 5)
  AND item_grade = 6
  AND req_item_id1 = 379021000
  AND req_item_id2 = 0
  AND item_req_coins = 10000
  AND success_rate = 3000;

-- A normal full import already has these records. If either is missing, use
-- the canonical source values (50,000 Noah / 35%).
INSERT INTO item_upgrade_settings
    (req_item_id1, req_item_name1, req_item_id2, req_item_name2,
     upgrade_note, item_type, item_rate, item_grade, item_req_coins, success_rate)
SELECT 379021000, 'Blessed Upgrade Scroll', 0, '', 'Normal Item Upgrade',
       expected.item_type, 33, 6, 50000, 3500
FROM (VALUES (4::smallint), (5::smallint)) AS expected(item_type)
WHERE NOT EXISTS (
    SELECT 1
    FROM item_upgrade_settings s
    WHERE s.item_type = expected.item_type
      AND s.item_grade = 6
      AND s.req_item_id1 = 379021000
      AND s.req_item_id2 = 0
);
