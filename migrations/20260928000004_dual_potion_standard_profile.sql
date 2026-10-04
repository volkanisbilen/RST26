-- Match the regular item-potion cast/visual profile and consume the exact item.
UPDATE magic
SET t_1 = -1,
    target_effect = 602,
    use_item = CASE magic_num
        WHEN 811182 THEN 811182000
        WHEN 811183 THEN 811183000
    END,
    cast_time = 5,
    recast_time = 20,
    "range" = 25
WHERE magic_num IN (811182, 811183);
