-- Quest audit fixes (see docs/QUEST_AUDIT.md). Idempotent and live-safe: every
-- statement only touches rows that are still in the broken state.

-- 1. item_exchange rows imported with their columns shifted by one.
--
-- Migration 20260325000001 inserted ~1000 rows straight from the 27-column client
-- Item_Exchange.tbl (idx, flag, EXTRA, 5x(origin,count), 5x(output,count), 4 extra)
-- into the 27-column server table (idx, flag, 5x(origin,count), 5x(output,count),
-- 5x time). The EXTRA column landed in origin_item_num1, so every value moved one
-- slot right: the real first origin sits in origin_item_count1 and the real EXP /
-- gold / item rewards in the *count* columns. RunQuestExchange then found no
-- reward item and gave nothing (Magmom 6823-6829, Hill/Rily/Kesar/Julius
-- 16243-16306, Menisia 2450, ...), while the quest was still completed.
--
-- A genuine row never has origin_item_num1 = 0 together with a count >= 100000
-- (that count is an item id), so that is the marker. All right-hand sides read the
-- OLD row, so the shift can be undone in one statement; afterwards the marker no
-- longer matches (re-running is a no-op). The time columns held client-only
-- numbers (100, 36 ...), they are reset to 0.
UPDATE item_exchange SET
    origin_item_num1    = origin_item_count1,
    origin_item_count1  = origin_item_num2,
    origin_item_num2    = origin_item_count2,
    origin_item_count2  = origin_item_num3,
    origin_item_num3    = origin_item_count3,
    origin_item_count3  = origin_item_num4,
    origin_item_num4    = origin_item_count4,
    origin_item_count4  = origin_item_num5,
    origin_item_num5    = origin_item_count5,
    origin_item_count5  = exchange_item_num1,
    exchange_item_num1  = exchange_item_count1,
    exchange_item_count1 = exchange_item_num2,
    exchange_item_num2  = exchange_item_count2,
    exchange_item_count2 = exchange_item_num3,
    exchange_item_num3  = exchange_item_count3,
    exchange_item_count3 = exchange_item_num4,
    exchange_item_num4  = exchange_item_count4,
    exchange_item_count4 = exchange_item_num5,
    exchange_item_num5  = exchange_item_count5,
    exchange_item_count5 = exchange_item_time1,
    exchange_item_time1 = 0,
    exchange_item_time2 = 0,
    exchange_item_time3 = 0,
    exchange_item_time4 = 0,
    exchange_item_time5 = 0
WHERE origin_item_num1 = 0
  AND origin_item_count1 >= 100000;

-- 2. Kill quests that have quest_helper rows but no quest_monster row, so their
-- kills were never counted (values from the client Quest_Monster_Exchange.tbl).
INSERT INTO quest_monster (s_quest_num, s_num1a, s_count1)
SELECT v.q, v.m, v.c
FROM (VALUES (456, 2353, 15),   -- Minerva (24204) lvl 65: fallen Angel x15
             (942, 4067, 1),    -- Ijac (29196): Karus Monument x1
             (1631, 10024, 3)   -- Dason (31743)
     ) AS v(q, m, c)
WHERE EXISTS (SELECT 1 FROM quest_helper h WHERE h.s_event_data_index = v.q)
ON CONFLICT (s_quest_num) DO NOTHING;
