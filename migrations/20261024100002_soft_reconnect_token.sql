-- Soft re-connect tokens survive a server restart (handler/soft_reconnect.rs).
CREATE TABLE IF NOT EXISTS soft_reconnect_token (
    token      bytea PRIMARY KEY,
    account    text NOT NULL,
    character  text NOT NULL,
    expires_at timestamptz NOT NULL
);
CREATE INDEX IF NOT EXISTS soft_reconnect_token_account ON soft_reconnect_token (account);
ALTER TABLE soft_reconnect_token OWNER TO koserver;
