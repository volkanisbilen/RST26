-- The active PostgreSQL test clan was left on the provisional first-cape
-- entry (1), while the verified MSSQL reference for JstKO uses cape 201.
-- Cloak.tbl in the v2615 client contains entry 201 (pattern02black), so use
-- the reference cloak without changing the clan's promotion/ranking state.
UPDATE knights
SET s_cape = 201,
    b_cape_r = 0,
    b_cape_g = 0,
    b_cape_b = 0
WHERE id_num = 1
  AND lower(id_name) = 'jstko'
  AND flag >= 2;
