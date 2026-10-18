-- Solo Cape: one character-bound cape per character (hook 0xF3 icon, scroll 810521000).
-- Apply once:  sudo -u postgres psql -d ko_server -f migrations/20261014000003_solo_cape.sql
CREATE TABLE IF NOT EXISTS solo_cape (
    char_name TEXT PRIMARY KEY,
    cape_id   INTEGER  NOT NULL,
    r         SMALLINT NOT NULL DEFAULT 0,
    g         SMALLINT NOT NULL DEFAULT 0,
    b         SMALLINT NOT NULL DEFAULT 0
);
ALTER TABLE solo_cape OWNER TO koserver;
