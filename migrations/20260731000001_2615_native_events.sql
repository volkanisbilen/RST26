-- Native v2615 client events: Roulette, Jigsaw Puzzle, Coin and Knight Marble.
-- All mutable player state is kept server-side; client payloads are never trusted
-- for reward selection or quantities.

CREATE TABLE IF NOT EXISTS native_event_config (
    event_key VARCHAR(32) PRIMARY KEY,
    active BOOLEAN NOT NULL DEFAULT FALSE,
    cost INTEGER NOT NULL DEFAULT 0 CHECK (cost >= 0),
    reset_hour SMALLINT NOT NULL DEFAULT 0 CHECK (reset_hour BETWEEN 0 AND 23),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO native_event_config (event_key, active, cost, reset_hour) VALUES
    ('roulette', FALSE, 0, 0),
    ('jigsaw', FALSE, 0, 0),
    ('coin', FALSE, 0, 0),
    ('marble', FALSE, 0, 0)
ON CONFLICT (event_key) DO NOTHING;

CREATE TABLE IF NOT EXISTS native_roulette_reward (
    roulette_type SMALLINT NOT NULL CHECK (roulette_type IN (7, 8)),
    slot SMALLINT NOT NULL CHECK (slot BETWEEN 0 AND 14),
    item_id INTEGER NOT NULL,
    item_count SMALLINT NOT NULL DEFAULT 1 CHECK (item_count > 0),
    weight INTEGER NOT NULL DEFAULT 100 CHECK (weight > 0),
    PRIMARY KEY (roulette_type, slot)
);

INSERT INTO native_roulette_reward (roulette_type, slot, item_id, item_count, weight) VALUES
    (7,0,508086000,1,100),(7,1,508086000,1,100),(7,2,508086000,1,100),
    (7,3,508086000,1,100),(7,4,508086000,1,100),(7,5,810202000,1,80),
    (7,6,810201000,1,80),(7,7,811094000,1,60),(7,8,810713000,1,50),
    (7,9,379156000,1,100),(7,10,190250251,1,75),(7,11,190250252,1,65),
    (7,12,190250253,1,55),(7,13,811059000,1,40),(7,14,379152000,1,100),
    (8,0,810947000,1,100),(8,1,810947000,1,100),(8,2,900144023,1,80),
    (8,3,900052000,1,80),(8,4,900146000,1,100),(8,5,900145000,1,100),
    (8,6,814038000,1,60),(8,7,914009000,1,50),(8,8,810442000,1,60),
    (8,9,814040000,1,50),(8,10,810504000,1,35),(8,11,810201000,1,60),
    (8,12,811036000,1,25),(8,13,811034000,1,25),(8,14,811035000,1,25)
ON CONFLICT (roulette_type, slot) DO UPDATE SET
    item_id = EXCLUDED.item_id, item_count = EXCLUDED.item_count;

CREATE TABLE IF NOT EXISTS native_roulette_user (
    character_name VARCHAR(21) PRIMARY KEY,
    pending_type SMALLINT,
    pending_slot SMALLINT,
    pending_item_id INTEGER,
    pending_item_count SMALLINT,
    pending_created_at TIMESTAMPTZ,
    last_spin_at TIMESTAMPTZ,
    last_free_spin_date DATE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS native_roulette_history (
    id BIGSERIAL PRIMARY KEY,
    character_name VARCHAR(21) NOT NULL,
    roulette_type SMALLINT NOT NULL,
    item_id INTEGER NOT NULL,
    item_count SMALLINT NOT NULL,
    won_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_native_roulette_history_character
    ON native_roulette_history (character_name, won_at DESC);

CREATE TABLE IF NOT EXISTS native_jigsaw_reward (
    reward_index SMALLINT PRIMARY KEY CHECK (reward_index BETWEEN 0 AND 8),
    required_total SMALLINT NOT NULL CHECK (required_total > 0),
    item_id INTEGER NOT NULL,
    item_count SMALLINT NOT NULL DEFAULT 1 CHECK (item_count > 0)
);
INSERT INTO native_jigsaw_reward VALUES
    (0,1,900145000,1),(1,2,900146000,1),(2,4,910252000,1),
    (3,8,910250000,1),(4,16,910249000,1),(5,24,910251000,1),
    (6,32,910938000,1),(7,48,810504000,1),(8,64,900120000,1)
ON CONFLICT (reward_index) DO NOTHING;

CREATE TABLE IF NOT EXISTS native_jigsaw_user (
    character_name VARCHAR(21) PRIMARY KEY,
    piece_counts SMALLINT[] NOT NULL DEFAULT ARRAY[0,0,0,0,0,0,0,0]::SMALLINT[],
    reward_claimed BOOLEAN[] NOT NULL DEFAULT ARRAY[FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE]::BOOLEAN[],
    pieces_today SMALLINT NOT NULL DEFAULT 0 CHECK (pieces_today BETWEEN 0 AND 64),
    reset_date DATE NOT NULL DEFAULT CURRENT_DATE,
    last_piece_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS native_coin_reward (
    reward_index SMALLINT PRIMARY KEY CHECK (reward_index BETWEEN 0 AND 9),
    required_points INTEGER NOT NULL CHECK (required_points > 0),
    item_id INTEGER NOT NULL,
    item_count SMALLINT NOT NULL DEFAULT 1 CHECK (item_count > 0)
);
INSERT INTO native_coin_reward VALUES
    (0,8,900145000,1),(1,8,900146000,1),(2,8,910252000,1),
    (3,8,910250000,1),(4,8,910249000,1),(5,16,910251000,1),
    (6,24,910938000,1),(7,32,914009000,1),(8,40,810504000,1),
    (9,48,900120000,1)
ON CONFLICT (reward_index) DO NOTHING;

CREATE TABLE IF NOT EXISTS native_coin_user (
    character_name VARCHAR(21) PRIMARY KEY,
    points INTEGER NOT NULL DEFAULT 0 CHECK (points >= 0),
    reward_claimed BOOLEAN[] NOT NULL DEFAULT ARRAY[FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE]::BOOLEAN[],
    last_point_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS native_marble_board (
    board_index SMALLINT PRIMARY KEY,
    item_id INTEGER NOT NULL DEFAULT 0,
    item_count SMALLINT NOT NULL DEFAULT 0,
    tile_type SMALLINT NOT NULL DEFAULT 0
);
INSERT INTO native_marble_board (board_index,item_id,item_count,tile_type) VALUES
    (0,0,0,0),(1,931773000,1,0),(2,910202000,1,0),(3,910203000,1,0),
    (4,910201000,1,0),(5,910202000,1,0),(6,0,0,2),(7,0,0,1),
    (8,900146000,1,0),(9,900145000,1,0),(10,931773000,1,0),(11,811090000,1,0),
    (12,0,0,2),(13,811101000,1,0),(14,811095000,1,0),(15,910202000,1,0),
    (16,910202000,1,0),(17,0,0,1),(18,0,0,2),(19,910201000,1,0),
    (20,910202000,1,0),(21,910204000,1,0),(22,900146000,1,0),(23,900145000,1,0)
ON CONFLICT (board_index) DO UPDATE SET
    item_id=EXCLUDED.item_id,item_count=EXCLUDED.item_count,tile_type=EXCLUDED.tile_type;

CREATE TABLE IF NOT EXISTS native_marble_user (
    character_name VARCHAR(21) PRIMARY KEY,
    position SMALLINT NOT NULL DEFAULT 0 CHECK (position BETWEEN 0 AND 23),
    laps INTEGER NOT NULL DEFAULT 0 CHECK (laps >= 0),
    rolls_today SMALLINT NOT NULL DEFAULT 0 CHECK (rolls_today >= 0),
    treasure_claimed BOOLEAN[] NOT NULL DEFAULT ARRAY[FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE]::BOOLEAN[],
    reset_date DATE NOT NULL DEFAULT CURRENT_DATE,
    last_roll_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE native_event_config IS 'Server-authoritative activation and cost controls for v2615 native events';
COMMENT ON TABLE native_roulette_reward IS 'Exact Roulette.tbl slot mapping for client types 7 and 8';
COMMENT ON TABLE native_marble_board IS 'Exact knight_marble.tbl board mapping for slots 0..23';
