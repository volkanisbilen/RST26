-- HopeGuard "Special Store [Loyalty & Manner]" (hook 0xF0): the shop list and the daily manner claim.
-- Apply once:  sudo -u postgres psql -d ko_server -f migrations/20261014000001_manner_store.sql
CREATE TABLE IF NOT EXISTS manner_store_items (
    id        SERIAL PRIMARY KEY,
    tab       SMALLINT NOT NULL DEFAULT 0,      -- page of the store window
    item_id   INTEGER  NOT NULL,
    item_count SMALLINT NOT NULL DEFAULT 1,
    price     INTEGER  NOT NULL,
    currency  SMALLINT NOT NULL DEFAULT 0,      -- 0 = manner points, 1 = loyalty (NP)
    days      INTEGER  NOT NULL DEFAULT 0,      -- >0: the item expires after this many days
    enabled   BOOLEAN  NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS manner_daily_claim (
    char_name  TEXT PRIMARY KEY,
    claimed_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE manner_store_items OWNER TO koserver;
ALTER TABLE manner_daily_claim OWNER TO koserver;
GRANT USAGE, SELECT ON SEQUENCE manner_store_items_id_seq TO koserver;

-- a few starting items (only if they exist in the item table); edit / add rows freely
INSERT INTO manner_store_items (tab, item_id, item_count, price, currency)
SELECT 0, v.item_id, v.cnt, v.price, v.currency
FROM (VALUES
    (379016000, 1, 300, 0),     -- Upgrade Scroll
    (379021000, 1, 600, 0),     -- Blessed Upgrade Scroll
    (389093000, 5, 150, 0),     -- HP Scroll
    (352900000, 1, 2000, 1),    -- Trina's Piece (Middle Class) for loyalty
    (353000000, 1, 1000, 1)     -- Trina's Piece (Low Class) for loyalty
) AS v(item_id, cnt, price, currency)
WHERE EXISTS (SELECT 1 FROM item i WHERE i.num = v.item_id)
  AND NOT EXISTS (SELECT 1 FROM manner_store_items);
