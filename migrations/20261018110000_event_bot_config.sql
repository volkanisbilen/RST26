-- Event-specific bot system (admin panel "Etkinlik Botları" page).
--
-- One row per event. Everything is OFF by default; the game server reads this table at start-up and
-- again whenever the panel queues an `event_bots_reload` admin_command, so no restart is needed.
-- Design and per-event support level: docs/EVENT_BOTS.md.
--
--   event_key             bdw | chaos | juraid | ultima | rift | ft | manes
--                         (juraid / ft / manes are listed so the panel can show them, but the game
--                         server forces them off: they are not supported yet)
--   enabled               master switch (default FALSE)
--   bots_per_team         bots per team (per room for Chaos, per Ultima room / Rift group)
--   min_level/max_level   level range of the bots (1..83)
--   fill_only_when_short  TRUE: top every team up to bots_per_team participants (humans + bots);
--                         FALSE: always add bots_per_team bots per team
--   min_humans            bots only appear when at least this many humans take part (>= 1, bots
--                         never run an event on their own)
--
-- Idempotent and safe on a live database: CREATE ... IF NOT EXISTS, seed rows ON CONFLICT DO NOTHING.
CREATE TABLE IF NOT EXISTS event_bot_config (
    event_key            TEXT PRIMARY KEY,
    enabled              BOOLEAN     NOT NULL DEFAULT FALSE,
    bots_per_team        SMALLINT    NOT NULL DEFAULT 4,
    min_level            SMALLINT    NOT NULL DEFAULT 60,
    max_level            SMALLINT    NOT NULL DEFAULT 83,
    fill_only_when_short BOOLEAN     NOT NULL DEFAULT TRUE,
    min_humans           SMALLINT    NOT NULL DEFAULT 1,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT event_bot_config_key_chk    CHECK (event_key IN ('bdw', 'chaos', 'juraid', 'ultima', 'rift', 'ft', 'manes')),
    CONSTRAINT event_bot_config_count_chk  CHECK (bots_per_team BETWEEN 0 AND 30),
    CONSTRAINT event_bot_config_level_chk  CHECK (min_level BETWEEN 1 AND 83 AND max_level BETWEEN 1 AND 83 AND min_level <= max_level),
    CONSTRAINT event_bot_config_humans_chk CHECK (min_humans BETWEEN 1 AND 30)
);

INSERT INTO event_bot_config (event_key) VALUES
    ('chaos'), ('bdw'), ('ultima'), ('rift'), ('juraid'), ('ft'), ('manes')
ON CONFLICT (event_key) DO NOTHING;

-- The game server and the panel run as the koserver role: they must own what this file creates
-- (psql runs as postgres here, otherwise "permission denied for table event_bot_config").
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'koserver') THEN
    ALTER TABLE event_bot_config OWNER TO koserver;
  END IF;
END $$;
