-- Client 2625 defines Golden Pickaxe as a 4,000-duration tool.  A later bulk
-- import replaced that definition with the 30,000-duration Golden Mattock row.
-- The mismatch makes the client calculate an invalid (negative) repair cost.
UPDATE item
SET str_name = 'Golden Pickaxe (+0)',
    duration = 4000,
    buy_price = 4000000
WHERE num = 389135000;

-- Existing copies created under the incorrect 30,000 durability must be made
-- valid as well; otherwise their repair calculation remains inconsistent.
UPDATE user_items
SET durability = 4000
WHERE item_id = 389135000
  AND durability > 4000;

UPDATE user_warehouse
SET durability = 4000
WHERE item_id = 389135000
  AND durability > 4000;
