-- Promoted/Accredited/Royal clans must have at least the base cape selected.
-- AutoRoyalG1 previously created flag=12 clans with the training-only
-- s_cape=-1 state, which makes the v2615 cape catalogue open with no entries.
UPDATE knights
SET s_cape = 0
WHERE flag >= 2
  AND s_cape < 0;
