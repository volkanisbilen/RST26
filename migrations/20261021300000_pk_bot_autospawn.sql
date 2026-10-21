-- PK bots that appear by themselves a few seconds after the game server starts (admin panel
-- "PK Otomatik Bot"). count_per_nation bots of EACH nation are topped up per enabled zone.
CREATE TABLE IF NOT EXISTS pk_bot_autospawn (
    zone_id          SMALLINT PRIMARY KEY,
    enabled          BOOLEAN  NOT NULL DEFAULT FALSE,
    count_per_nation SMALLINT NOT NULL DEFAULT 100,
    level            SMALLINT,              -- NULL = zone default (83, Ardream 59)
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
INSERT INTO pk_bot_autospawn (zone_id, enabled, count_per_nation) VALUES
    (71, TRUE, 100),
    (72, FALSE, 100),
    (73, FALSE, 100)
ON CONFLICT (zone_id) DO NOTHING;
