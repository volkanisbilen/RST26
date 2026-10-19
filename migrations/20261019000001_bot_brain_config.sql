-- Bot brain knobs (admin panel "Bot Zekâsı" page; game server systems/bot_brain.rs).
--
-- One row (id = 1). The game server reads it at start-up and again whenever the panel queues a
-- `bot_brain_reload` admin_command. Missing table / row = built-in defaults. Design: docs/BOT_AI.md.
--
--   enabled                 master switch (FALSE = legacy fight-the-nearest bot loop)
--   patrol_percent          share of PK-zone bots that patrol the whole map; the rest gather at
--                           known fight spots (bowl, bridges, base fronts)
--   reaction_min/max_ms     human-like reaction delay before the first hit on a new target
--   target_mistake_percent  chance to take the second-best target
--   kite_miss_percent       chance that a ranged class fails to kite this tick
--   hp_potions .. buff_scrolls  virtual stock per bot life (no inventory items)
--   potion_cooldown_ms      players: 2400
--   np_percent              NP for a PK-zone bot kill, % of the zone's player-kill NP
--   antifarm_secs           same killer, same bot: nothing again inside this window
--   spawn_grace_secs        a bot killed this soon after respawn pays nothing
--   party_max_bots / party_max_per_class   bot party composition
--   retreat_hp_percent      own HP below this: retreat / heal up
--   outnumbered_percent     retreat when enemies*100 >= allies*this (200 = twice as many)
--   spacing_min/max         natural party spacing (m), varied per member
CREATE TABLE IF NOT EXISTS bot_brain_config (
    id                      SMALLINT    PRIMARY KEY DEFAULT 1,
    enabled                 BOOLEAN     NOT NULL DEFAULT TRUE,
    patrol_percent          SMALLINT    NOT NULL DEFAULT 40,
    reaction_min_ms         SMALLINT    NOT NULL DEFAULT 300,
    reaction_max_ms         SMALLINT    NOT NULL DEFAULT 900,
    target_mistake_percent  SMALLINT    NOT NULL DEFAULT 10,
    kite_miss_percent       SMALLINT    NOT NULL DEFAULT 25,
    hp_potions              SMALLINT    NOT NULL DEFAULT 30,
    mp_potions              SMALLINT    NOT NULL DEFAULT 30,
    ts_scrolls              SMALLINT    NOT NULL DEFAULT 2,
    buff_scrolls            SMALLINT    NOT NULL DEFAULT 4,
    potion_cooldown_ms      SMALLINT    NOT NULL DEFAULT 2400,
    np_percent              SMALLINT    NOT NULL DEFAULT 33,
    antifarm_secs           SMALLINT    NOT NULL DEFAULT 300,
    spawn_grace_secs        SMALLINT    NOT NULL DEFAULT 20,
    party_max_bots          SMALLINT    NOT NULL DEFAULT 6,
    party_max_per_class     SMALLINT    NOT NULL DEFAULT 2,
    retreat_hp_percent      SMALLINT    NOT NULL DEFAULT 35,
    outnumbered_percent     SMALLINT    NOT NULL DEFAULT 200,
    spacing_min             SMALLINT    NOT NULL DEFAULT 3,
    spacing_max             SMALLINT    NOT NULL DEFAULT 8,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT bot_brain_config_single_chk   CHECK (id = 1),
    CONSTRAINT bot_brain_config_percent_chk  CHECK (patrol_percent BETWEEN 0 AND 100 AND target_mistake_percent BETWEEN 0 AND 100
                                                  AND kite_miss_percent BETWEEN 0 AND 100 AND np_percent BETWEEN 0 AND 100
                                                  AND retreat_hp_percent BETWEEN 0 AND 90),
    CONSTRAINT bot_brain_config_reaction_chk CHECK (reaction_min_ms BETWEEN 0 AND 5000 AND reaction_max_ms BETWEEN 0 AND 5000
                                                  AND reaction_min_ms <= reaction_max_ms),
    CONSTRAINT bot_brain_config_stock_chk    CHECK (hp_potions BETWEEN 0 AND 999 AND mp_potions BETWEEN 0 AND 999
                                                  AND ts_scrolls BETWEEN 0 AND 99 AND buff_scrolls BETWEEN 0 AND 99
                                                  AND potion_cooldown_ms BETWEEN 500 AND 30000),
    CONSTRAINT bot_brain_config_party_chk    CHECK (party_max_bots BETWEEN 2 AND 8 AND party_max_per_class BETWEEN 1 AND 8),
    CONSTRAINT bot_brain_config_window_chk   CHECK (antifarm_secs BETWEEN 0 AND 7200 AND spawn_grace_secs BETWEEN 0 AND 600
                                                  AND outnumbered_percent BETWEEN 100 AND 1000),
    CONSTRAINT bot_brain_config_spacing_chk  CHECK (spacing_min BETWEEN 1 AND 20 AND spacing_max BETWEEN 1 AND 30 AND spacing_min <= spacing_max)
);

INSERT INTO bot_brain_config (id) VALUES (1) ON CONFLICT (id) DO NOTHING;

-- The game server and the panel run as the koserver role: they must own what this file creates.
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'koserver') THEN
    ALTER TABLE bot_brain_config OWNER TO koserver;
  END IF;
END $$;
