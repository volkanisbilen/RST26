-- Iruta EXP jar exchanges and the v2615 Moradon fixed-price scroll merchant.
-- Client authority: docs/data/Item_Org_us.tbl and itemsell_table.tbl.

-- Iruta: consume one sealed jar and grant its exact stored EXP value.
DELETE FROM item_exchange WHERE n_index IN (990001, 990002);

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
) VALUES
    (990001, 0, 810355000, 1, 0, 0, 0, 0, 0, 0, 0, 0,
     900001000, 50000000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0),
    (990002, 0, 810356000, 1, 0, 0, 0, 0, 0, 0, 0, 0,
     900001000, 100000000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);

-- Reuse the client-defined, currently unspawned [Scroll Merchant] proto.
-- Keeping s_sid/s_pid client-native avoids an extra NPC.tbl patch.
UPDATE npc_template
SET str_name = '[Scroll Merchant]',
    by_group = 3,
    by_act_type = 1,
    by_type = 21,
    i_selling_group = 280000
WHERE s_sid = 29514 AND is_monster = FALSE;

INSERT INTO npc_spawn (
    zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
    special_type, trap_number, left_x, top_z, num_npc, spawn_range,
    regen_time, direction, dot_cnt, path, room
)
SELECT 21, 29514, FALSE, 0, 0, 0, 0, 0, 816, 553, 1, 0, 60, 90, 0, NULL, 0
WHERE NOT EXISTS (
    SELECT 1 FROM npc_spawn
    WHERE zone_id = 21 AND npc_id = 29514 AND is_monster = FALSE
      AND left_x = 816 AND top_z = 553
);

-- First row after the stock v2615 itemsell_table.tbl (rows 1..396).
DELETE FROM item_sell_table WHERE n_index = 397 OR i_selling_group = 280000;

INSERT INTO item_sell_table (
    n_index, i_selling_group,
    item1, item2, item3, item4, item5, item6, item7, item8,
    item9, item10, item11, item12, item13, item14, item15,
    item16, item17, item18, item19, item20, item21, item22, item23, item24
) VALUES (
    397, 280000,
    379063000, 379064000, 379065000, 379066000, 379116000,
    800076000, 800028000, 800078000,
    800091000, 800092000, 800093000, 800094000,
    399127000, 399128000, 399129000,
    0, 0, 0, 0, 0, 0, 0, 0, 0
);

-- Server-side menu labels matching the supplied client patch.
INSERT INTO quest_menu (i_num, str_menu) VALUES
    (60001, 'Exchange 50,000,000 EXP Jar'),
    (60002, 'Exchange 100,000,000 EXP Jar')
ON CONFLICT (i_num) DO UPDATE SET str_menu = EXCLUDED.str_menu;
