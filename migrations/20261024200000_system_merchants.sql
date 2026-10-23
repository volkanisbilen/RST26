-- System merchants: server-controlled stall bots (Karus/El Morad castle + Eslant: quest items, Moradon: fragments / gems / +5 gear).
-- Admin edits the item pools in ko-admin ("Pazar Botları"); the game server spawns one stall (12 slots) per 12 pool items.
CREATE TABLE IF NOT EXISTS system_merchant_group (
    id          SERIAL PRIMARY KEY,
    key         VARCHAR(32)  NOT NULL UNIQUE,
    title       VARCHAR(80)  NOT NULL,
    kind        VARCHAR(16)  NOT NULL DEFAULT 'custom',      -- 'quest' = pool can be refilled from the quest items of the zone
    zone        INTEGER      NOT NULL,
    nation      SMALLINT     NOT NULL DEFAULT 0,             -- 1 Karus, 2 El Morad, 0 = alternate both
    anchor_x    REAL         NOT NULL,
    anchor_z    REAL         NOT NULL,
    columns     SMALLINT     NOT NULL DEFAULT 6,             -- stalls per row
    spacing     REAL         NOT NULL DEFAULT 3.0,
    name_prefix VARCHAR(14)  NOT NULL,
    advert      VARCHAR(60)  NOT NULL DEFAULT '',
    max_stalls  SMALLINT     NOT NULL DEFAULT 12,
    enabled     BOOLEAN      NOT NULL DEFAULT TRUE,
    status      TEXT         NOT NULL DEFAULT '',            -- written by the game server after a (re)load
    live_stalls INTEGER      NOT NULL DEFAULT 0,
    status_at   TIMESTAMPTZ
);
CREATE TABLE IF NOT EXISTS system_merchant_pool (
    id       SERIAL PRIMARY KEY,
    group_id INTEGER NOT NULL REFERENCES system_merchant_group(id) ON DELETE CASCADE,
    sort     INTEGER NOT NULL DEFAULT 0,
    item_id  INTEGER NOT NULL,
    price    BIGINT  NOT NULL DEFAULT 1,
    stock    INTEGER NOT NULL DEFAULT 1,                     -- count shown in the stall (restocked after every sale)
    is_kc    BOOLEAN NOT NULL DEFAULT FALSE,
    enabled  BOOLEAN NOT NULL DEFAULT TRUE,
    UNIQUE (group_id, item_id)
);
CREATE TABLE IF NOT EXISTS system_merchant_sale (
    id        BIGSERIAL PRIMARY KEY,
    at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    group_key VARCHAR(32) NOT NULL,
    stall     VARCHAR(21) NOT NULL,
    buyer     VARCHAR(21) NOT NULL,
    item_id   INTEGER     NOT NULL,
    count     INTEGER     NOT NULL,
    price     BIGINT      NOT NULL,
    is_kc     BOOLEAN     NOT NULL DEFAULT FALSE
);
CREATE INDEX IF NOT EXISTS system_merchant_sale_at ON system_merchant_sale (at DESC);

INSERT INTO system_merchant_group (key, title, kind, zone, nation, anchor_x, anchor_z, columns, spacing, name_prefix, advert, max_stalls, enabled) VALUES
 ('quest_luferson', 'Luferson Castle - görev itemleri',   'quest', 1,  1, 401,  1739, 6, 3, 'LufGorev',  'Gorev itemleri satilik', 12, TRUE),
 ('quest_elmorad',   'El Morad Castle - görev itemleri',  'quest', 2,  2, 1657, 326,  6, 3, 'ElmGorev',  'Gorev itemleri satilik', 12, TRUE),
 ('quest_karus_esl', 'Karus Eslant - görev itemleri',     'quest', 11, 1, 516,  530,  6, 3, 'KEslGorev', 'Gorev itemleri satilik', 6,  TRUE),
 ('quest_elmo_esl',  'El Morad Eslant - görev itemleri',  'quest', 12, 2, 519,  524,  6, 3, 'EEslGorev', 'Gorev itemleri satilik', 6,  TRUE),
 ('moradon_bifrost', 'Moradon - Bifrost fragmentleri',    'custom', 21, 0, 836,  600,  6, 3, 'MorBifrost','Bifrost fragmentleri',    4,  TRUE),
 ('moradon_juraid',  'Moradon - Juraid Mountain gemleri', 'custom', 21, 0, 836,  612,  6, 3, 'MorJuraid', 'Juraid gemleri',         4,  TRUE),
 ('moradon_gear',    'Moradon - +5 High Class silah/zırh','custom', 21, 0, 836,  624,  6, 3, 'MorEsya',   '+5 High Class esyalar',   12, TRUE)
ON CONFLICT (key) DO NOTHING;

-- Quest groups: every tradable (race <> 20, not virtual) item that a quest NPC of the zone asks for in an exchange.
INSERT INTO system_merchant_pool (group_id, sort, item_id, price, stock, is_kc)
SELECT g.id, ROW_NUMBER() OVER (PARTITION BY g.id ORDER BY it.item_id), it.item_id,
       LEAST(GREATEST(COALESCE(i.buy_price, 1), 1), 2000000000)::bigint,
       CASE WHEN COALESCE(i.countable, 0) = 1 THEN 500 ELSE 1 END, FALSE
FROM system_merchant_group g
JOIN LATERAL (
    SELECT DISTINCT o.it AS item_id
    FROM quest_helper h
    JOIN npc_spawn s ON s.npc_id = h.s_npc_id AND s.is_monster = FALSE AND s.zone_id = g.zone
    JOIN item_exchange e ON e.n_index = h.n_exchange_index
    CROSS JOIN LATERAL unnest(ARRAY[e.origin_item_num1, e.origin_item_num2, e.origin_item_num3, e.origin_item_num4, e.origin_item_num5]) AS o(it)
    WHERE h.n_exchange_index > 0 AND o.it > 0 AND o.it < 900000000
) it ON TRUE
JOIN item i ON i.num = it.item_id AND COALESCE(i.race, 0) <> 20 AND COALESCE(i.countable, 0) <> 2
WHERE g.kind = 'quest'
ON CONFLICT (group_id, item_id) DO NOTHING;

-- Moradon: Bifrost dark fragments, Juraid Mountain gems, +5 unique level 80 weapons
INSERT INTO system_merchant_pool (group_id, sort, item_id, price, stock)
SELECT g.id, ROW_NUMBER() OVER (ORDER BY i.num), i.num, GREATEST(COALESCE(i.buy_price, 1), 1)::bigint, 500
FROM system_merchant_group g JOIN item i ON i.num IN (810943000, 810944000, 810948000, 810949000, 810950000, 810951000, 810952000)
WHERE g.key = 'moradon_bifrost' ON CONFLICT (group_id, item_id) DO NOTHING;

INSERT INTO system_merchant_pool (group_id, sort, item_id, price, stock)
SELECT g.id, ROW_NUMBER() OVER (ORDER BY i.num), i.num, GREATEST(COALESCE(i.buy_price, 1), 1)::bigint, 500
FROM system_merchant_group g JOIN item i ON i.num IN (810241000, 810242000, 810243000, 810244000, 910201000, 910202000, 910203000, 910204000)
WHERE g.key = 'moradon_juraid' ON CONFLICT (group_id, item_id) DO NOTHING;

INSERT INTO system_merchant_pool (group_id, sort, item_id, price, stock)
SELECT g.id, ROW_NUMBER() OVER (ORDER BY i.num), i.num, GREATEST(COALESCE(i.buy_price, 1), 1)::bigint, 1
FROM system_merchant_group g JOIN item i ON i.str_name LIKE '%(+5)%' AND i.item_type = 4 AND i.req_level >= 80
 AND COALESCE(i.race, 0) <> 20 AND i.kind IN (11, 21, 22, 31, 32, 41, 42, 51, 52, 60, 70, 71, 110, 140, 181)
WHERE g.key = 'moradon_gear' ON CONFLICT (group_id, item_id) DO NOTHING;

-- the game server writes status / sales, the web panel and ko-admin read them
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'koserver') THEN
        EXECUTE 'ALTER TABLE system_merchant_group OWNER TO koserver';
        EXECUTE 'ALTER TABLE system_merchant_pool OWNER TO koserver';
        EXECUTE 'ALTER TABLE system_merchant_sale OWNER TO koserver';
        EXECUTE 'ALTER SEQUENCE system_merchant_group_id_seq OWNER TO koserver';
        EXECUTE 'ALTER SEQUENCE system_merchant_pool_id_seq OWNER TO koserver';
        EXECUTE 'ALTER SEQUENCE system_merchant_sale_id_seq OWNER TO koserver';
    END IF;
END $$;
