-- Automatic event schedule (server local time, the same clock as event_schedule_main_list)
-- and the event state the game server publishes for the web panel.
-- days: bit mask, Sunday = 1, Monday = 2, ... Saturday = 64 (127 = every day)
CREATE TABLE IF NOT EXISTS event_auto_schedule (
    id            SERIAL PRIMARY KEY,
    event_key     VARCHAR(20) NOT NULL,
    enabled       BOOLEAN     NOT NULL DEFAULT TRUE,
    days          SMALLINT    NOT NULL DEFAULT 127,
    hour          SMALLINT    NOT NULL,
    minute        SMALLINT    NOT NULL DEFAULT 0,
    arg           VARCHAR(40) NOT NULL DEFAULT '',
    duration_min  INTEGER     NOT NULL DEFAULT 0,
    note          VARCHAR(80) NOT NULL DEFAULT '',
    last_run      TIMESTAMPTZ,
    last_result   VARCHAR(200) NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_event_auto_schedule_time ON event_auto_schedule (hour, minute) WHERE enabled;

CREATE TABLE IF NOT EXISTS admin_event_state (
    event_key   VARCHAR(20) PRIMARY KEY,
    state       VARCHAR(16) NOT NULL DEFAULT 'idle',
    detail      VARCHAR(120) NOT NULL DEFAULT '',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS name VARCHAR(60) NOT NULL DEFAULT '';
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS grp VARCHAR(40) NOT NULL DEFAULT '';
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS arg_label VARCHAR(40) NOT NULL DEFAULT '';
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS arg_required BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS has_now BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS can_stop BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS sort_no INTEGER NOT NULL DEFAULT 0;

-- Default times (the registration windows are spread out so they never overlap; the
-- temple events keep using event_schedule_main_list). Edit them in the web panel.
INSERT INTO event_auto_schedule (event_key, enabled, days, hour, minute, arg, duration_min, note)
SELECT * FROM (VALUES
    ('manes',  TRUE,  127, 17::smallint,  0::smallint, '',    0, 'Manes Survival 1. seans'),
    ('manes',  TRUE,  127, 20::smallint, 30::smallint, '',    0, 'Manes Survival 2. seans'),
    ('ultima', TRUE,  127, 12::smallint, 30::smallint, '10',  0, 'Ultima 1. seans'),
    ('ultima', TRUE,  127, 23::smallint,  0::smallint, '10',  0, 'Ultima 2. seans'),
    ('rift',   TRUE,  127, 13::smallint, 30::smallint, '10',  0, 'Full Moon Rift 1. seans'),
    ('rift',   TRUE,  127, 19::smallint, 30::smallint, '10',  0, 'Full Moon Rift 2. seans'),
    ('golem',  TRUE,  127, 14::smallint, 30::smallint, '',    0, 'Ateş & Buz 1. seans'),
    ('golem',  TRUE,  127, 22::smallint, 30::smallint, '',    0, 'Ateş & Buz 2. seans'),
    ('bifrost',FALSE, 127, 18::smallint, 30::smallint, '',    0, 'Bifrost (kapalı, isteğe bağlı)'),
    ('utc',    FALSE, 127, 16::smallint, 30::smallint, '180', 0, 'Under The Castle (kapalı, isteğe bağlı)'),
    ('ft',     FALSE, 127, 18::smallint,  0::smallint, '',    0, 'Forgotten Temple (kapalı, isteğe bağlı)')
) AS v(event_key, enabled, days, hour, minute, arg, duration_min, note)
WHERE NOT EXISTS (SELECT 1 FROM event_auto_schedule);
