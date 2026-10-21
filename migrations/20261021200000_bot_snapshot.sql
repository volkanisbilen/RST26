-- Live bot dump for the admin panel ("Bot Envanteri"): refreshed by the `bot_snapshot` admin command.
CREATE TABLE IF NOT EXISTS bot_snapshot (
    bot_id      BIGINT       PRIMARY KEY,
    name        VARCHAR(40)  NOT NULL,
    class       INT          NOT NULL,
    level       INT          NOT NULL,
    nation      INT          NOT NULL,
    zone_id     INT          NOT NULL,
    kind        VARCHAR(10)  NOT NULL,
    max_hp      INT          NOT NULL,
    formula_hp  INT          NOT NULL,
    item_hp     INT          NOT NULL,
    item_ac     INT          NOT NULL,
    items       JSONB        NOT NULL DEFAULT '[]',
    taken_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
