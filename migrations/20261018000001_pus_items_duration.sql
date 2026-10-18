-- Adds a per-item duration (in days) to the Power Up Store catalog, so a listing can be set
-- up to deliver a time-limited item (e.g. a 1-week version of something) instead of always
-- giving the permanent item. 0 = no expiry (existing behavior, unchanged for all current rows).
ALTER TABLE pus_items ADD COLUMN IF NOT EXISTS duration_days INTEGER NOT NULL DEFAULT 0;
