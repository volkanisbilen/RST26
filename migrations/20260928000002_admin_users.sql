-- Additional authenticated administrators for the web control panel.
CREATE TABLE IF NOT EXISTS admin_users (
    admin_user_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    password_hash TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(64) NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_admin_users_username_ci
    ON admin_users (lower(username));
