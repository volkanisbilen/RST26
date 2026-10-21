-- Web panel CMS: manual patch downloads, home slider, banners, free pages and an editable menu.
-- All editable from the panel's admin area (web-panel/public_html/admin/*), used by the player pages.

-- Manual patch downloads (/patches.php). `url` is either an external link or a /uploads/... file.
CREATE TABLE IF NOT EXISTS web_patch (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    version     VARCHAR(30)  NOT NULL,
    title       VARCHAR(160) NOT NULL DEFAULT '',
    notes       TEXT         NOT NULL DEFAULT '',
    url         VARCHAR(500) NOT NULL,
    size_mb     VARCHAR(20)  NOT NULL DEFAULT '',
    active      BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    created_by  VARCHAR(40)  NOT NULL DEFAULT ''
);

-- Home page slider.
CREATE TABLE IF NOT EXISTS web_slide (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    title       VARCHAR(160) NOT NULL DEFAULT '',
    subtitle    VARCHAR(300) NOT NULL DEFAULT '',
    image       VARCHAR(255) NOT NULL,
    link_url    VARCHAR(500) NOT NULL DEFAULT '',
    button_text VARCHAR(60)  NOT NULL DEFAULT '',
    sort_order  INT          NOT NULL DEFAULT 0,
    active      BOOLEAN      NOT NULL DEFAULT TRUE
);

-- Image banners; `position` picks the slot (home_top, home_mid, home_side, page_top).
CREATE TABLE IF NOT EXISTS web_banner (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    position    VARCHAR(30)  NOT NULL,
    image       VARCHAR(255) NOT NULL,
    link_url    VARCHAR(500) NOT NULL DEFAULT '',
    alt         VARCHAR(160) NOT NULL DEFAULT '',
    sort_order  INT          NOT NULL DEFAULT 0,
    active      BOOLEAN      NOT NULL DEFAULT TRUE
);

-- Free-form pages (/page.php?slug=...). `body` is trusted HTML written by a GM.
CREATE TABLE IF NOT EXISTS web_page (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slug        VARCHAR(60)  NOT NULL UNIQUE,
    title       VARCHAR(160) NOT NULL,
    body        TEXT         NOT NULL DEFAULT '',
    header_image VARCHAR(255) NOT NULL DEFAULT '',
    published   BOOLEAN      NOT NULL DEFAULT TRUE,
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Top menu. parent_id groups dropdown children. Seeded with the current fixed menu.
CREATE TABLE IF NOT EXISTS web_menu (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    parent_id   BIGINT REFERENCES web_menu(id) ON DELETE CASCADE,
    label       VARCHAR(60)  NOT NULL,
    url         VARCHAR(500) NOT NULL DEFAULT '',
    sort_order  INT          NOT NULL DEFAULT 0,
    active      BOOLEAN      NOT NULL DEFAULT TRUE
);

DO $$
DECLARE ev BIGINT;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM web_menu) THEN
        INSERT INTO web_menu (label, url, sort_order) VALUES
            ('Anasayfa', '/', 10),
            ('Sıralamalar', '/rankings.php', 20),
            ('Droplar', '/items.php', 30),
            ('Patch İndir', '/patches.php', 35);
        INSERT INTO web_menu (label, url, sort_order) VALUES ('Etkinlikler', '', 40) RETURNING id INTO ev;
        INSERT INTO web_menu (parent_id, label, url, sort_order) VALUES
            (ev, 'Etkinlik Takvimi', '/events.php', 10),
            (ev, 'Günlük Görevler', '/daily_quests.php', 20),
            (ev, 'Coin Etkinliği', '/event_coin.php', 30),
            (ev, 'Puzzle Etkinliği', '/event_puzzle.php', 40),
            (ev, 'Çark Ödülleri', '/event_roulette.php', 50),
            (ev, 'Lottery', '/event_lottery.php', 60);
        INSERT INTO web_menu (label, url, sort_order) VALUES
            ('Online Oyuncular', '/online.php', 50),
            ('Online GM''ler', '/gms.php', 60),
            ('Karakter Kurtar', '/character_rescue.php', 70),
            ('Destek', '/support.php', 80);
    END IF;
END $$;

-- New editable content keys (background and banner images).
INSERT INTO web_content (key, value, kind) VALUES
    ('site_bg_image', '', 'image'),
    ('footer_text', '', 'text'),
    ('patch_page_intro', 'Oyunda sorun yaşıyorsan aşağıdaki en son yamayı indirip oyun klasörüne çıkart.', 'text')
ON CONFLICT (key) DO NOTHING;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'webpanel') THEN
        EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON web_patch, web_slide, web_banner, web_page, web_menu TO webpanel';
        EXECUTE 'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO webpanel';
    END IF;
END $$;
