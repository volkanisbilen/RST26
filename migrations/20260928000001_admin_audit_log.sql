-- Auditable history for privileged web-panel operations.
CREATE TABLE IF NOT EXISTS admin_audit_log (
    audit_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actor VARCHAR(64) NOT NULL,
    action VARCHAR(80) NOT NULL,
    target VARCHAR(120) NOT NULL DEFAULT '',
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    remote_ip INET
);

CREATE INDEX IF NOT EXISTS idx_admin_audit_log_time
    ON admin_audit_log (occurred_at DESC);
