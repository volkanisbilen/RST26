-- The first cape quest (Delaga/Charel EVENT 280) is only available to a
-- Training clan (flag=1) whose point grade is 1..3. AutoRoyalG1 skipped that
-- entire progression by creating every clan as Royal1 (flag=12).
UPDATE server_settings
SET auto_royal_g1 = 0
WHERE auto_royal_g1 <> 0;

-- Repair only untouched clans produced by the old AutoRoyal setting. Existing
-- progressed Royal clans are deliberately excluded. Grade 3 starts at 144000
-- clan points and is the minimum accepted by the official cape quest Lua.
UPDATE knights
SET flag = 1,
    points = 144000,
    s_cape = -1,
    ranking = 0
WHERE flag = 12
  AND points = 0
  AND s_cape = 0;
