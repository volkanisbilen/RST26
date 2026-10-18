-- GM PK-bot manual loadout corrections (admin panel "Bot Donanımı" page). The server's
-- procedural bot generator (crates/ko-game/src/systems/bot_ai.rs: generate_bot_equipment,
-- gm_bot_stats, select_bot_attack_skill and the priest/rogue support pools) occasionally picks a
-- visually broken item or puts a piece in the wrong slot; these tables let a GM pin exact
-- stats/items/skills per (class, level bracket) instead, with every field optional so an
-- unconfigured bracket/slot/stat falls back to the existing automatic pick unchanged.
--
-- gm_class matches GM_CLASSES in bot_ai.rs: 1=warrior, 2=rogue, 3=mage, 4=priest,
-- 13=kurian/porutu. level_bracket is one of the three gear tiers the project owner asked for
-- (59/69/83); a bot's level resolves to the highest bracket at or below it (ko-game's
-- bot_loadout_bracket), or no override at all below level 59.
CREATE TABLE IF NOT EXISTS bot_loadout_override (
    gm_class      SMALLINT NOT NULL,
    level_bracket SMALLINT NOT NULL,
    str_stat      SMALLINT,
    sta_stat      SMALLINT,
    dex_stat      SMALLINT,
    int_stat      SMALLINT,
    cha_stat      SMALLINT,
    skills        JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (gm_class, level_bracket),
    CONSTRAINT bot_loadout_override_class_chk CHECK (gm_class IN (1, 2, 3, 4, 13)),
    CONSTRAINT bot_loadout_override_bracket_chk CHECK (level_bracket IN (59, 69, 83))
);

-- One row per overridden equip slot. `slot` is the real inventory slot constant
-- (inventory_constants.rs): 1=HEAD, 4=BREAST, 5=SHOULDER, 6=RIGHTHAND, 8=LEFTHAND, 10=LEG,
-- 12=GLOVE, 13=FOOT -- the 8 equip_visual gear slots a GM bot wears (the 9 cosplay/branding
-- slots stay hardcoded in generate_bot_equipment, out of scope for this feature). A missing row
-- for a given (class, bracket, slot) falls back to pick_bot_gear's auto-pick for that slot.
CREATE TABLE IF NOT EXISTS bot_loadout_override_item (
    gm_class      SMALLINT NOT NULL,
    level_bracket SMALLINT NOT NULL,
    slot          SMALLINT NOT NULL,
    item_id       BIGINT NOT NULL,
    durability    SMALLINT NOT NULL DEFAULT 100,
    PRIMARY KEY (gm_class, level_bracket, slot),
    CONSTRAINT bot_loadout_override_item_class_chk CHECK (gm_class IN (1, 2, 3, 4, 13)),
    CONSTRAINT bot_loadout_override_item_bracket_chk CHECK (level_bracket IN (59, 69, 83)),
    CONSTRAINT bot_loadout_override_item_slot_chk CHECK (slot IN (1, 4, 5, 6, 8, 10, 12, 13))
);

-- The game server and the panel run as the koserver role: they must own what this file creates
-- (psql runs as postgres here, otherwise "permission denied for table bot_loadout_override").
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'koserver') THEN
    ALTER TABLE bot_loadout_override OWNER TO koserver;
    ALTER TABLE bot_loadout_override_item OWNER TO koserver;
  END IF;
END $$;
