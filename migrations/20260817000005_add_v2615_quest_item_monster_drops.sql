-- Restore physical quest-item drops by cross-referencing the active v2615
-- Quest_Helper.tbl, Quest_Monster_Exchange.tbl and Item_Exchange.tbl.
--
-- This migration runs after the exact MSSQL drop snapshot. It only occupies
-- slots that are zero in that snapshot and never replaces an ordinary drop.
-- Items that are outputs of an earlier quest exchange are deliberately not
-- added as monster drops.

-- Quest 295 / exchange 128: Troll Berserker classified document.
-- TBL requires one document while its monster objective is 100 kills, hence
-- the client-defined expected rate is 1% (100 / 10000).
UPDATE monster_item
SET item07 = 900068000, percent07 = 100
WHERE s_index = 1723 AND item07 IN (0, 900068000);

UPDATE monster_item
SET item01 = 900068000, percent01 = 100
WHERE s_index = 1773 AND item01 IN (0, 900068000);

-- Quest 802 / exchange 3247: Spoiler mana-remains collection.
-- Rates are copied from the same item IDs' authoritative v2369 drop rows.
UPDATE monster_item
SET item01 = 900331000, percent01 = 1925,
    item02 = 900332000, percent02 = 2500
WHERE s_index = 303
  AND item01 IN (0, 900331000)
  AND item02 IN (0, 900332000);

UPDATE monster_item
SET item06 = 900331000, percent06 = 1925,
    item07 = 900332000, percent07 = 2500
WHERE s_index = 353
  AND item06 IN (0, 900331000)
  AND item07 IN (0, 900332000);

-- Quest 809 / exchange 1222: Smilodon collection components.
UPDATE monster_item
SET item06 = 508211000, percent06 = 300,
    item07 = 508212000, percent07 = 300,
    item08 = 508213000, percent08 = 875
WHERE s_index = 600
  AND item06 IN (0, 508211000)
  AND item07 IN (0, 508212000)
  AND item08 IN (0, 508213000);

UPDATE monster_item
SET item06 = 508211000, percent06 = 300,
    item07 = 508212000, percent07 = 300,
    item08 = 508213000, percent08 = 875
WHERE s_index = 650
  AND item06 IN (0, 508211000)
  AND item07 IN (0, 508212000)
  AND item08 IN (0, 508213000);

-- Quest 812 / exchange 1225: Wild Smilodon / Tarantula's teeth.
UPDATE monster_item
SET item01 = 900060000, percent01 = 725
WHERE s_index = 602 AND item01 IN (0, 900060000);

UPDATE monster_item
SET item06 = 900060000, percent06 = 725
WHERE s_index = 652 AND item06 IN (0, 900060000);

-- Abort migration if any TBL-derived mapping was not installed. This catches
-- an unexpected source snapshot instead of silently leaving quests broken.
DO $$
DECLARE
    missing_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO missing_count
    FROM (VALUES
        (1723, 900068000), (1773, 900068000),
        (303, 900331000), (303, 900332000),
        (353, 900331000), (353, 900332000),
        (600, 508211000), (600, 508212000), (600, 508213000),
        (650, 508211000), (650, 508212000), (650, 508213000),
        (602, 900060000), (652, 900060000)
    ) AS expected(s_index, item_id)
    WHERE NOT EXISTS (
        SELECT 1
        FROM monster_item AS mi
        CROSS JOIN LATERAL (VALUES
            (mi.item01), (mi.item02), (mi.item03), (mi.item04),
            (mi.item05), (mi.item06), (mi.item07), (mi.item08),
            (mi.item09), (mi.item10), (mi.item11), (mi.item12)
        ) AS slots(item_id)
        WHERE mi.s_index = expected.s_index
          AND slots.item_id = expected.item_id
    );

    IF missing_count <> 0 THEN
        RAISE EXCEPTION 'v2615 quest item drop installation failed for % mappings', missing_count;
    END IF;
END $$;
