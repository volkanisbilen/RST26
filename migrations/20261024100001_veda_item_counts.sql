-- Veda / Beda (24424, Karus Eslant): same fix as Tabeth, item quests ask what the client's Item_Exchange says.
UPDATE item_exchange SET origin_item_count1 = 5  WHERE n_index IN (1388, 1389) AND origin_item_count1 = 10;
UPDATE item_exchange SET origin_item_count1 = 10 WHERE n_index IN (1390, 1391, 1392, 1393, 1394, 1396, 1397) AND origin_item_count1 = 20;
