-- Character `a` belongs to clan 1. Its first-cape progression consumed the
-- key/coins but left the clan on cape 0 (the non-rendering "basic" entry in
-- this v2615 client). Select the first visible, grade-3/flag-2 compatible cape.
UPDATE knights
SET s_cape = 1,
    b_cape_r = 0,
    b_cape_g = 0,
    b_cape_b = 0
WHERE id_num = 1
  AND flag > 1
  AND s_cape = 0;
