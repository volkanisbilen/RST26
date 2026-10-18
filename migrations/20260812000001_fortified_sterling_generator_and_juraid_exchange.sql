-- Fortified Sterling Silver Gemstone support for the v2615 Chaotic Generator.
--
-- The TBL cross-check rows 1000002068..1000002110 were imported one column
-- to the right: origin_item_num1 became 0 and the real origin item landed in
-- origin_item_count1.  Keep the already-applied migration immutable and repair
-- those rows here.
UPDATE item_exchange
SET origin_item_num1      = origin_item_count1,
    origin_item_count1    = origin_item_num2,
    origin_item_num2      = 0,
    origin_item_count2    = 0,
    exchange_item_num1    = exchange_item_count1,
    exchange_item_count1  = exchange_item_num2,
    exchange_item_num2    = 0,
    exchange_item_count2  = 0
WHERE n_index BETWEEN 1000002068 AND 1000002110
  AND origin_item_num1 = 0
  AND origin_item_count1 = 811137000
  AND origin_item_num2 = 1;

-- Fortified Sterling must contain the complete Silvery Gem reward pool too.
-- Copy every Silvery entry with its original random flag and weight.  A stable
-- n_index range makes this migration deterministic and safe to audit.
INSERT INTO item_exchange (
    n_index, random_flag,
    origin_item_num1, origin_item_count1,
    origin_item_num2, origin_item_count2,
    origin_item_num3, origin_item_count3,
    origin_item_num4, origin_item_count4,
    origin_item_num5, origin_item_count5,
    exchange_item_num1, exchange_item_count1,
    exchange_item_num2, exchange_item_count2,
    exchange_item_num3, exchange_item_count3,
    exchange_item_num4, exchange_item_count4,
    exchange_item_num5, exchange_item_count5,
    exchange_item_time1, exchange_item_time2, exchange_item_time3,
    exchange_item_time4, exchange_item_time5
)
SELECT
    1800120000 + ROW_NUMBER() OVER (ORDER BY n_index), random_flag,
    811137000, 1,
    origin_item_num2, origin_item_count2,
    origin_item_num3, origin_item_count3,
    origin_item_num4, origin_item_count4,
    origin_item_num5, origin_item_count5,
    exchange_item_num1, exchange_item_count1,
    exchange_item_num2, exchange_item_count2,
    exchange_item_num3, exchange_item_count3,
    exchange_item_num4, exchange_item_count4,
    exchange_item_num5, exchange_item_count5,
    exchange_item_time1, exchange_item_time2, exchange_item_time3,
    exchange_item_time4, exchange_item_time5
FROM item_exchange silvery
WHERE silvery.origin_item_num1 = 389196000
  AND silvery.random_flag IN (1, 2, 3)
  AND NOT EXISTS (
      SELECT 1
      FROM item_exchange fortified
      WHERE fortified.origin_item_num1 = 811137000
        AND fortified.random_flag IN (1, 2, 3)
        AND fortified.exchange_item_num1 = silvery.exchange_item_num1
  )
ORDER BY silvery.n_index
ON CONFLICT (n_index) DO NOTHING;

-- v2615 dialog text used by both Juraid registration captains.
INSERT INTO quest_talk (i_num, str_talk) VALUES
    (45608, 'Exchange 3 [Silvery Gem] for 1 [Fortified Sterling Silver Gemstone]?'),
    (45609, 'Three [Silvery Gem] will be consumed. Would you like to proceed?'),
    (45610, 'You need 3 [Silvery Gem] in your inventory.'),
    (45611, 'The exchange has been completed.'),
    (45612, 'There is not enough free inventory space for the exchange reward.')
ON CONFLICT (i_num) DO UPDATE SET str_talk = EXCLUDED.str_talk;

INSERT INTO quest_menu (i_num, str_menu) VALUES
    (45608, 'Silvery Gem Exchange'),
    (45609, 'Exchange 3 Silvery Gems for 1 Fortified Sterling Gemstone')
ON CONFLICT (i_num) DO UPDATE SET str_menu = EXCLUDED.str_menu;
