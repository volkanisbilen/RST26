-- The v2615 client uses two independent version domains:
--   launcher patch/Files version = 2615
--   KnightOnLine game handshake wire version = 2614
-- Sharing game_version with LoginServer made the launcher request missing
-- executable/XIGNCODE patches and close with "Could not download files".
ALTER TABLE server_settings
    ADD COLUMN IF NOT EXISTS launcher_version SMALLINT NOT NULL DEFAULT 2615;

UPDATE server_settings
SET launcher_version = 2615
WHERE server_no = 1;
