-- Persistent equipment for a familiar's four pet-inventory slots.
-- Kept separate from user_items so dismissing, dying, zoning or reconnecting
-- never destroys auto-loot equipment or pet consumables.
CREATE TABLE IF NOT EXISTS pet_user_items (
    n_pet_serial_id BIGINT NOT NULL REFERENCES pet_user_data(n_serial_id) ON DELETE CASCADE,
    slot_index SMALLINT NOT NULL CHECK (slot_index BETWEEN 0 AND 3),
    item_id INTEGER NOT NULL,
    durability SMALLINT NOT NULL DEFAULT 0,
    count SMALLINT NOT NULL DEFAULT 0,
    flag SMALLINT NOT NULL DEFAULT 0,
    original_flag SMALLINT NOT NULL DEFAULT 0,
    serial_num BIGINT NOT NULL DEFAULT 0,
    expire_time INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (n_pet_serial_id, slot_index)
);
