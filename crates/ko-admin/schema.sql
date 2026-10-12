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
