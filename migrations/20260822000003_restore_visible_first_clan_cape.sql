-- The v2615 client loads Cloak.tbl row 0 as its catalogue placeholder but
-- does not attach a cloak model for it. First-cape quest completion promotes
-- a training clan to flag 2; give that state the first visible entry instead.
UPDATE knights
SET s_cape = 1,
    b_cape_r = 0,
    b_cape_g = 0,
    b_cape_b = 0
WHERE flag = 2
  AND s_cape = 0;
