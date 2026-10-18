-- EVENT 287 consumed 10M Noah and Brain of Centaur, then the old repository
-- attempted to persist to the nonexistent `knights.cape` column. Complete
-- only the untouched grade-3 Training clans prepared by the preceding
-- AutoRoyal repair whose chief no longer owns the quest key.
UPDATE knights AS k
SET flag = 2,
    s_cape = 0
WHERE k.flag = 1
  AND k.points = 144000
  AND k.s_cape = -1
  AND NOT EXISTS (
      SELECT 1
      FROM user_items AS ui
      WHERE ui.str_user_id = k.chief
        AND ui.item_id = 910045000
        AND ui.count > 0
  );
