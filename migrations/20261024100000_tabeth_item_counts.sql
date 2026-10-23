-- Tabeth (14424): the item quests asked double what the client's Item_Exchange table says.
-- Counts set to the client's values (and the Lua thresholds in 14424_dela.lua follow).
UPDATE item_exchange SET origin_item_count1 = 5  WHERE n_index IN (1350, 1351) AND origin_item_count1 = 10;
UPDATE item_exchange SET origin_item_count1 = 10 WHERE n_index IN (1352, 1353, 1354, 1355, 1356, 1358, 1359) AND origin_item_count1 = 20;
