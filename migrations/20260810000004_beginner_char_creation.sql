-- C++ LOAD_NEW_CHAR_VALUE parity and optional level-specific starting equipment.
-- beginner_type maps directly to CREATE_NEW_CHAR_VALUE.job_type:
-- 0=legacy/default, 1=59, 2=69, 3=1, 4=83 (server-configured).
CREATE TABLE IF NOT EXISTS create_new_char_set_level (
    id BIGSERIAL PRIMARY KEY,
    class_type SMALLINT NOT NULL,
    beginner_type SMALLINT NOT NULL,
    slot_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL DEFAULT 0,
    item_duration SMALLINT NOT NULL DEFAULT 0,
    item_count SMALLINT NOT NULL DEFAULT 0,
    item_flag SMALLINT NOT NULL DEFAULT 0,
    item_expire_time INTEGER NOT NULL DEFAULT 0,
    UNIQUE (class_type, beginner_type, slot_id)
);
CREATE INDEX IF NOT EXISTS idx_new_char_set_level_lookup
    ON create_new_char_set_level (class_type, beginner_type, slot_id);
