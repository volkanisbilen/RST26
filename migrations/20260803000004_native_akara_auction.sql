-- Server-owned Akara Altar catalogue for the native v2615
-- CUISpecialAuction panel. Item numbers are existing Akara accessories from
-- the repository's checked item table seed.

CREATE TABLE IF NOT EXISTS native_akara_auction (
    slot SMALLINT PRIMARY KEY CHECK (slot BETWEEN 0 AND 15),
    item_id INTEGER NOT NULL,
    item_ext SMALLINT NOT NULL DEFAULT 0,
    current_bid BIGINT NOT NULL CHECK (current_bid >= 0),
    min_increment BIGINT NOT NULL CHECK (min_increment > 0),
    current_bidder VARCHAR(21),
    bid_count INTEGER NOT NULL DEFAULT 0 CHECK (bid_count >= 0),
    starts_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ends_at TIMESTAMPTZ NOT NULL DEFAULT (CURRENT_TIMESTAMP + INTERVAL '7 days'),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO native_akara_auction
    (slot,item_id,item_ext,current_bid,min_increment,ends_at)
VALUES
    (0,810889000,0,1000000,100000,CURRENT_TIMESTAMP + INTERVAL '7 days'),
    (1,810891000,0,1000000,100000,CURRENT_TIMESTAMP + INTERVAL '7 days'),
    (2,810892000,0,1000000,100000,CURRENT_TIMESTAMP + INTERVAL '7 days'),
    (3,810893000,0,1000000,100000,CURRENT_TIMESTAMP + INTERVAL '7 days'),
    (4,810894000,0,1000000,100000,CURRENT_TIMESTAMP + INTERVAL '7 days'),
    (5,810895000,0,1000000,100000,CURRENT_TIMESTAMP + INTERVAL '7 days'),
    (6,810896000,0,1000000,100000,CURRENT_TIMESTAMP + INTERVAL '7 days'),
    (7,810897000,0,1000000,100000,CURRENT_TIMESTAMP + INTERVAL '7 days')
ON CONFLICT (slot) DO NOTHING;

CREATE INDEX IF NOT EXISTS idx_native_akara_auction_active
    ON native_akara_auction (enabled, ends_at, slot);
