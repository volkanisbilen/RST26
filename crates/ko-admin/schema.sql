-- Queue of commands for a running game server (the server polls it; rows older than 10 minutes are ignored).
CREATE TABLE IF NOT EXISTS admin_command (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    command     VARCHAR(40) NOT NULL,
    args        JSONB NOT NULL DEFAULT '{}'::jsonb,
    status      VARCHAR(12) NOT NULL DEFAULT 'pending',
    result      TEXT NOT NULL DEFAULT ''
)
-- ;;
CREATE INDEX IF NOT EXISTS idx_admin_command_pending ON admin_command (status, id)
-- ;;
ALTER TABLE admin_users ADD COLUMN IF NOT EXISTS role VARCHAR(16) NOT NULL DEFAULT 'admin'
-- ;;
CREATE TABLE IF NOT EXISTS admin_heartbeat (
    id  SMALLINT PRIMARY KEY,
    at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- ;;
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
)
-- ;;
CREATE TABLE IF NOT EXISTS admin_event_state (
    event_key   VARCHAR(20) PRIMARY KEY,
    state       VARCHAR(16) NOT NULL DEFAULT 'idle',
    detail      VARCHAR(120) NOT NULL DEFAULT '',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- ;;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS name VARCHAR(60) NOT NULL DEFAULT ''
-- ;;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS grp VARCHAR(40) NOT NULL DEFAULT ''
-- ;;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS arg_label VARCHAR(40) NOT NULL DEFAULT ''
-- ;;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS arg_required BOOLEAN NOT NULL DEFAULT FALSE
-- ;;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS has_now BOOLEAN NOT NULL DEFAULT FALSE
-- ;;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS can_stop BOOLEAN NOT NULL DEFAULT TRUE
-- ;;
ALTER TABLE admin_event_state ADD COLUMN IF NOT EXISTS sort_no INTEGER NOT NULL DEFAULT 0
-- ;;
CREATE TABLE IF NOT EXISTS pus_code_batch (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    label       VARCHAR(80) NOT NULL DEFAULT '',
    kc          INTEGER NOT NULL DEFAULT 0,
    tl          INTEGER NOT NULL DEFAULT 0,
    quantity    INTEGER NOT NULL DEFAULT 0,
    note        VARCHAR(200) NOT NULL DEFAULT '',
    created_by  VARCHAR(40) NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ
)
-- ;;
CREATE TABLE IF NOT EXISTS pus_code (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    code          VARCHAR(24) NOT NULL UNIQUE,
    batch_id      BIGINT NOT NULL REFERENCES pus_code_batch(id) ON DELETE CASCADE,
    kc            INTEGER NOT NULL DEFAULT 0,
    tl            INTEGER NOT NULL DEFAULT 0,
    note          VARCHAR(200) NOT NULL DEFAULT '',
    disabled      BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at    TIMESTAMPTZ,
    used_at       TIMESTAMPTZ,
    used_account  VARCHAR(40),
    used_char     VARCHAR(40),
    used_ip       VARCHAR(45)
)
-- ;;
CREATE INDEX IF NOT EXISTS idx_pus_code_used ON pus_code (used_account, used_at)
-- ;;
CREATE INDEX IF NOT EXISTS idx_pus_code_batch ON pus_code (batch_id)
-- ;;
CREATE TABLE IF NOT EXISTS pus_code_attempt (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    account    VARCHAR(40) NOT NULL DEFAULT '',
    char_name  VARCHAR(40) NOT NULL DEFAULT '',
    ip         VARCHAR(45) NOT NULL DEFAULT '',
    code       VARCHAR(40) NOT NULL DEFAULT '',
    result     VARCHAR(16) NOT NULL DEFAULT ''
)
-- ;;
CREATE INDEX IF NOT EXISTS idx_pus_code_attempt_acc ON pus_code_attempt (account, at DESC)
-- ;;
CREATE TABLE IF NOT EXISTS kc_ledger (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    account    VARCHAR(40) NOT NULL,
    char_name  VARCHAR(40) NOT NULL DEFAULT '',
    kc_delta   INTEGER NOT NULL DEFAULT 0,
    tl_delta   INTEGER NOT NULL DEFAULT 0,
    kc_after   BIGINT NOT NULL DEFAULT 0,
    tl_after   BIGINT NOT NULL DEFAULT 0,
    reason     VARCHAR(20) NOT NULL DEFAULT '',
    ref        VARCHAR(60) NOT NULL DEFAULT ''
)
-- ;;
CREATE INDEX IF NOT EXISTS idx_kc_ledger_acc ON kc_ledger (account, at DESC)
