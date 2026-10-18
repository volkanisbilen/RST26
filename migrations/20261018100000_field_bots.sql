-- Field bots: server-controlled players that hunt monsters in the NORMAL (non-PK) maps and can be
-- invited to a party (crates/ko-game/src/systems/field_bots.rs, admin panel page "Saha Botlari").
--
-- One row per supported zone. Everything is OFF by default (enabled = FALSE); the game server only
-- spawns bots for rows an admin has switched on. The columns the game server WRITES back are
-- live_karus / live_elmorad / live_updated_at (current population, shown in the panel).
--
-- Idempotent: safe to apply repeatedly and on a live database. The ko-admin panel carries an
-- identical copy of this statement in crates/ko-admin/schema.sql and creates the table itself on
-- startup, so applying this file by hand is optional but harmless.
--
-- Hard caps (also enforced by the game server and the panel, whatever is stored here):
--   max 40 bots per nation per zone, max 150 field bots in the whole world.
--
-- Supported zones: 1 Luferson (Karus), 2 El Morad, 11 Karus Eslant, 12 El Morad Eslant,
-- 21 Moradon (both nations). A zone belongs to one nation (Moradon to both); the game server
-- ignores a count for a nation that does not own the zone.

CREATE TABLE IF NOT EXISTS field_bot_zone (
    zone_id         SMALLINT    PRIMARY KEY,
    enabled         BOOLEAN     NOT NULL DEFAULT FALSE,
    karus_count     SMALLINT    NOT NULL DEFAULT 0,
    elmorad_count   SMALLINT    NOT NULL DEFAULT 0,
    level_min       SMALLINT    NOT NULL DEFAULT 1,
    level_max       SMALLINT    NOT NULL DEFAULT 83,
    mix_warrior     SMALLINT    NOT NULL DEFAULT 30,
    mix_rogue       SMALLINT    NOT NULL DEFAULT 20,
    mix_mage        SMALLINT    NOT NULL DEFAULT 25,
    mix_priest      SMALLINT    NOT NULL DEFAULT 25,
    mix_kurian      SMALLINT    NOT NULL DEFAULT 0,
    live_karus      SMALLINT    NOT NULL DEFAULT 0,
    live_elmorad    SMALLINT    NOT NULL DEFAULT 0,
    live_updated_at TIMESTAMPTZ,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT field_bot_zone_zone_chk   CHECK (zone_id IN (1, 2, 11, 12, 21)),
    CONSTRAINT field_bot_zone_count_chk  CHECK (karus_count BETWEEN 0 AND 40 AND elmorad_count BETWEEN 0 AND 40),
    CONSTRAINT field_bot_zone_level_chk  CHECK (level_min BETWEEN 1 AND 83 AND level_max BETWEEN 1 AND 83 AND level_min <= level_max),
    CONSTRAINT field_bot_zone_mix_chk    CHECK (mix_warrior BETWEEN 0 AND 100 AND mix_rogue BETWEEN 0 AND 100
                                               AND mix_mage BETWEEN 0 AND 100 AND mix_priest BETWEEN 0 AND 100
                                               AND mix_kurian BETWEEN 0 AND 100)
);

-- Sensible starting points, all disabled. Levels follow each map's monster levels
-- (Moradon 1-60, Luferson/El Morad 24-110, Eslants 68+; a bot hunts monsters from 8 levels
-- below to 4 levels above its own level) and the zone's own minimum level.
INSERT INTO field_bot_zone (zone_id, enabled, karus_count, elmorad_count, level_min, level_max)
VALUES
    (21, FALSE, 6, 6, 8, 45),
    (1,  FALSE, 10, 0, 35, 60),
    (2,  FALSE, 0, 10, 35, 60),
    (11, FALSE, 8, 0, 66, 80),
    (12, FALSE, 0, 8, 66, 80)
ON CONFLICT (zone_id) DO NOTHING;
