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
-- ;;
CREATE TABLE IF NOT EXISTS event_reward_rule (
    id         INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    event_key  VARCHAR(20) NOT NULL,
    cond       VARCHAR(24) NOT NULL,
    rank_from  INTEGER NOT NULL DEFAULT 0,
    rank_to    INTEGER NOT NULL DEFAULT 0,
    pick       INTEGER NOT NULL DEFAULT 0,
    chance     INTEGER NOT NULL DEFAULT 100,
    item_id    INTEGER NOT NULL DEFAULT 0,
    item_count INTEGER NOT NULL DEFAULT 1,
    days       INTEGER NOT NULL DEFAULT 0,
    enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    note       VARCHAR(80) NOT NULL DEFAULT ''
)
-- ;;
INSERT INTO event_reward_rule(event_key,cond,pick,chance,item_id,item_count,days,note)
SELECT * FROM (VALUES
 ('ultima','survive',0,100,389161000,1,0,'Fragment of Gluttony'),
 ('ultima','survive',0,100,389160000,1,0,'Fragment of Arrogance'),
 ('ultima','stage1',0,100,800051000,1,0,'1. safhada elenen: Ascent Scroll'),
 ('ultima','stage2',0,100,389160000,1,0,'2. safhada elenen: Fragment of Arrogance'),
 ('ultima','stage3',0,100,389161000,1,0,'3. ve sonraki safha: Fragment of Gluttony'),
 ('ultima','box',0,60,810432000,1,30,'Boss kutusu: Lunar Tattoo (sans = hasar payi x3, en çok bu yüzde)')
) AS v(event_key,cond,pick,chance,item_id,item_count,days,note)
WHERE NOT EXISTS (SELECT 1 FROM event_reward_rule WHERE event_key='ultima')
-- ;;
INSERT INTO event_reward_rule(event_key,cond,pick,chance,item_id,item_count,days,note)
SELECT * FROM (VALUES
 ('golem','win',0,100,811182000,3,0,'Kazananlar: Dual Potion'),
 ('golem','win',0,100,1900004000,1,1,'Kazananlar: Event Emblem 1 gün'),
 ('golem','win',2,100,811065000,22,0,'Kazananlardan 2 kişi: Hero''s Vow'),
 ('golem','win',1,100,931734000,1,30,'Kazananlardan 1 kişi: Emblem Voucher 30 gün'),
 ('golem','lose',0,100,811182000,1,0,'Kaybedenler: Dual Potion'),
 ('golem','lose',0,100,1900004000,1,1,'Kaybedenler: Event Emblem 1 gün'),
 ('golem','lose',1,100,811065000,22,0,'Kaybedenlerden 1 kişi: Hero''s Vow'),
 ('golem','draw',0,100,811182000,3,0,'Beraberlik: Dual Potion'),
 ('golem','draw',0,100,1900004000,1,1,'Beraberlik: Event Emblem 1 gün'),
 ('golem','draw',3,100,811065000,22,0,'Beraberlikte 3 kişi: Hero''s Vow'),
 ('golem','draw',1,100,931734000,1,30,'Beraberlikte 1 kişi: Emblem Voucher 30 gün'),
 ('golem','belt',1,5,340710000,1,0,'Katılanlardan 1 kişi, %5: Lunar War Belt')
) AS v(event_key,cond,pick,chance,item_id,item_count,days,note)
WHERE NOT EXISTS (SELECT 1 FROM event_reward_rule WHERE event_key='golem')
-- ;;
INSERT INTO event_reward_rule(event_key,cond,rank_from,rank_to,item_id,item_count,note)
SELECT * FROM (VALUES
 ('manes','rank',1,1,978026000,150,'1. sıra: Manes Orb'),
 ('manes','rank',2,2,978026000,100,'2. sıra: Manes Orb'),
 ('manes','rank',3,3,978026000,50,'3. sıra: Manes Orb'),
 ('manes','rank',4,0,978026000,20,'4. ve sonrası: Manes Orb')
) AS v(event_key,cond,rank_from,rank_to,item_id,item_count,note)
WHERE NOT EXISTS (SELECT 1 FROM event_reward_rule WHERE event_key='manes')
-- ;;
-- GM PK-bot manual loadout corrections ("Bot Donanımı" page); see
-- migrations/20261018000003_bot_loadout_override.sql for the full column documentation.
CREATE TABLE IF NOT EXISTS bot_loadout_override (
    gm_class      SMALLINT NOT NULL,
    level_bracket SMALLINT NOT NULL,
    str_stat      SMALLINT,
    sta_stat      SMALLINT,
    dex_stat      SMALLINT,
    int_stat      SMALLINT,
    cha_stat      SMALLINT,
    skills        JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (gm_class, level_bracket),
    CONSTRAINT bot_loadout_override_class_chk CHECK (gm_class IN (1, 2, 3, 4, 13)),
    CONSTRAINT bot_loadout_override_bracket_chk CHECK (level_bracket IN (59, 69, 83))
)
-- ;;
CREATE TABLE IF NOT EXISTS bot_loadout_override_item (
    gm_class      SMALLINT NOT NULL,
    level_bracket SMALLINT NOT NULL,
    slot          SMALLINT NOT NULL,
    item_id       BIGINT NOT NULL,
    durability    SMALLINT NOT NULL DEFAULT 100,
    PRIMARY KEY (gm_class, level_bracket, slot),
    CONSTRAINT bot_loadout_override_item_class_chk CHECK (gm_class IN (1, 2, 3, 4, 13)),
    CONSTRAINT bot_loadout_override_item_bracket_chk CHECK (level_bracket IN (59, 69, 83)),
    CONSTRAINT bot_loadout_override_item_slot_chk CHECK (slot IN (1, 4, 5, 6, 8, 10, 12, 13))
)
-- ;;
-- Field bots (normal-map hunters, "Saha Botlari" page); see
-- migrations/20261018100000_field_bots.sql for the full documentation. Everything starts disabled.
CREATE TABLE IF NOT EXISTS field_bot_zone (
    zone_id         SMALLINT    PRIMARY KEY,
    enabled         BOOLEAN     NOT NULL DEFAULT FALSE,
    karus_count     SMALLINT    NOT NULL DEFAULT 0,
    elmorad_count   SMALLINT    NOT NULL DEFAULT 0,
    level_min       SMALLINT    NOT NULL DEFAULT 1,
    level_max       SMALLINT    NOT NULL DEFAULT 83,
    mix_warrior     SMALLINT    NOT NULL DEFAULT 30,
    mix_rogue       SMALLINT    NOT NULL DEFAULT 20,
    mix_mage        SMALLINT    NOT NULL DEFAULT 25,
    mix_priest      SMALLINT    NOT NULL DEFAULT 25,
    mix_kurian      SMALLINT    NOT NULL DEFAULT 0,
    live_karus      SMALLINT    NOT NULL DEFAULT 0,
    live_elmorad    SMALLINT    NOT NULL DEFAULT 0,
    live_updated_at TIMESTAMPTZ,
    live_note       TEXT        NOT NULL DEFAULT '',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT field_bot_zone_zone_chk   CHECK (zone_id IN (1, 2, 11, 12, 21)),
    CONSTRAINT field_bot_zone_count_chk  CHECK (karus_count BETWEEN 0 AND 1000 AND elmorad_count BETWEEN 0 AND 1000),
    CONSTRAINT field_bot_zone_level_chk  CHECK (level_min BETWEEN 1 AND 83 AND level_max BETWEEN 1 AND 83 AND level_min <= level_max),
    CONSTRAINT field_bot_zone_mix_chk    CHECK (mix_warrior BETWEEN 0 AND 100 AND mix_rogue BETWEEN 0 AND 100
                                               AND mix_mage BETWEEN 0 AND 100 AND mix_priest BETWEEN 0 AND 100
                                               AND mix_kurian BETWEEN 0 AND 100)
)
-- ;;
INSERT INTO field_bot_zone (zone_id, enabled, karus_count, elmorad_count, level_min, level_max)
VALUES (21, FALSE, 6, 6, 8, 45), (1, FALSE, 10, 0, 35, 60), (2, FALSE, 0, 10, 35, 60),
       (11, FALSE, 8, 0, 66, 80), (12, FALSE, 0, 8, 66, 80)
ON CONFLICT (zone_id) DO NOTHING
-- ;;
-- Field bot limits removed (migrations/20261018260000_field_bot_remove_caps.sql): live_note column + 1000 ceiling.
ALTER TABLE field_bot_zone ADD COLUMN IF NOT EXISTS live_note TEXT NOT NULL DEFAULT ''
-- ;;
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'field_bot_zone_count_chk'
               AND conrelid = 'field_bot_zone'::regclass AND pg_get_constraintdef(oid) LIKE '%40%') THEN
        ALTER TABLE field_bot_zone DROP CONSTRAINT field_bot_zone_count_chk;
        ALTER TABLE field_bot_zone ADD CONSTRAINT field_bot_zone_count_chk
            CHECK (karus_count BETWEEN 0 AND 1000 AND elmorad_count BETWEEN 0 AND 1000);
    END IF;
END $$
-- ;;
-- Right-top info list entries ("Bilgi Paneli" page); see migrations/20261018180000_info_panel_entry.sql.
CREATE TABLE IF NOT EXISTS info_panel_entry (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    sort_order            INTEGER      NOT NULL DEFAULT 0,
    title                 VARCHAR(24)  NOT NULL,
    description           VARCHAR(255) NOT NULL,
    enabled               BOOLEAN      NOT NULL DEFAULT TRUE,
    requires_clan_premium BOOLEAN      NOT NULL DEFAULT FALSE,
    updated_at            TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT info_panel_entry_title_chk CHECK (char_length(btrim(title)) BETWEEN 1 AND 24),
    CONSTRAINT info_panel_entry_desc_chk  CHECK (char_length(btrim(description)) BETWEEN 1 AND 255)
)
-- ;;
CREATE INDEX IF NOT EXISTS idx_info_panel_entry_order ON info_panel_entry (sort_order, id)
