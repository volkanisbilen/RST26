-- Lets GMs reorder Power Up Store listings (drag-and-drop in the admin panel) independently
-- of their id. Backfilled from the current id order so existing display order is unchanged
-- until someone reorders.
ALTER TABLE pus_items ADD COLUMN IF NOT EXISTS sort_order INTEGER;
UPDATE pus_items SET sort_order = id * 10 WHERE sort_order IS NULL;
ALTER TABLE pus_items ALTER COLUMN sort_order SET DEFAULT 0;
ALTER TABLE pus_items ALTER COLUMN sort_order SET NOT NULL;
