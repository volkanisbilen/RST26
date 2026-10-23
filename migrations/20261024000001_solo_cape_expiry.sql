-- Solo Cape lasts 7 days from purchase; existing capes get 7 days from now.
ALTER TABLE solo_cape ADD COLUMN IF NOT EXISTS expires_at timestamptz;
UPDATE solo_cape SET expires_at = now() + interval '7 days' WHERE expires_at IS NULL;
