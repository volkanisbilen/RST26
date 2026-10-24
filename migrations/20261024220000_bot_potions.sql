-- Which potions the (virtual) bots drink. Admin page "Bot Potları" edits both tables.
CREATE TABLE IF NOT EXISTS bot_potion_setting (
    kind        VARCHAR(2) PRIMARY KEY CHECK (kind IN ('hp', 'mp')),
    cap_percent SMALLINT NOT NULL DEFAULT 60 CHECK (cap_percent BETWEEN 5 AND 100),
    fixed_skill INTEGER                       -- magic number the bots always drink; NULL = automatic
);
CREATE TABLE IF NOT EXISTS bot_potion_disabled (
    skill_id INTEGER PRIMARY KEY
);
INSERT INTO bot_potion_setting (kind, cap_percent) VALUES ('hp', 60), ('mp', 60) ON CONFLICT (kind) DO NOTHING;
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'koserver') THEN
        EXECUTE 'ALTER TABLE bot_potion_setting OWNER TO koserver';
        EXECUTE 'ALTER TABLE bot_potion_disabled OWNER TO koserver';
    END IF;
END $$;
