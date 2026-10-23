-- Mixed market: many stalls, every stall sells its own random selection of the pool (Moradon).
ALTER TABLE system_merchant_group ADD COLUMN IF NOT EXISTS mode VARCHAR(8) NOT NULL DEFAULT 'ordered';
ALTER TABLE system_merchant_group ADD COLUMN IF NOT EXISTS bot_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE system_merchant_group ADD COLUMN IF NOT EXISTS area_w REAL NOT NULL DEFAULT 60;
ALTER TABLE system_merchant_group ADD COLUMN IF NOT EXISTS area_h REAL NOT NULL DEFAULT 60;
ALTER TABLE system_merchant_group ADD COLUMN IF NOT EXISTS per_bot_min SMALLINT NOT NULL DEFAULT 4;
ALTER TABLE system_merchant_group ADD COLUMN IF NOT EXISTS per_bot_max SMALLINT NOT NULL DEFAULT 10;

INSERT INTO system_merchant_group (key, title, kind, zone, nation, anchor_x, anchor_z, columns, spacing, name_prefix, advert, max_stalls, enabled, mode, bot_count, area_w, area_h, per_bot_min, per_bot_max)
VALUES ('moradon_market', 'Moradon pazarı (karışık)', 'custom', 21, 0, 790, 540, 6, 3, 'MorPazar', 'Esya satiyorum', 12, TRUE, 'mixed', 120, 120, 120, 4, 10)
ON CONFLICT (key) DO NOTHING;

UPDATE system_merchant_pool SET group_id = (SELECT id FROM system_merchant_group WHERE key = 'moradon_market')
 WHERE group_id IN (SELECT id FROM system_merchant_group WHERE key IN ('moradon_bifrost', 'moradon_juraid', 'moradon_gear'))
   AND item_id NOT IN (SELECT item_id FROM system_merchant_pool WHERE group_id = (SELECT id FROM system_merchant_group WHERE key = 'moradon_market'));
DELETE FROM system_merchant_group WHERE key IN ('moradon_bifrost', 'moradon_juraid', 'moradon_gear');

DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'koserver') THEN
        EXECUTE 'ALTER TABLE system_merchant_group OWNER TO koserver';
    END IF;
END $$;
