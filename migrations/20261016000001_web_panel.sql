-- Tables for the player web panel (separate PHP hosting, connects to this same Postgres DB).
-- Account/character/clan data is read from the existing tb_user / userdata / knights /
-- currentuser / account_char tables; these are new, panel-specific tables only.

-- Email verification for accounts registered through the panel (account_check starts at 0 for
-- these; the game client's own walk-up auto-registration always sets account_check = 1, so this
-- never affects that flow -- see login.rs).
CREATE TABLE IF NOT EXISTS web_email_verify (
    token         VARCHAR(64)   PRIMARY KEY,
    account_id    VARCHAR(21)   NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ   NOT NULL,
    used_at       TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_web_email_verify_account ON web_email_verify (account_id);

-- Password reset (web panel's "Şifremi Unuttum"), via security question or a mailed link.
CREATE TABLE IF NOT EXISTS web_password_reset (
    token         VARCHAR(64)   PRIMARY KEY,
    account_id    VARCHAR(21)   NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ   NOT NULL,
    used_at       TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_web_password_reset_account ON web_password_reset (account_id);

-- Simple throttle so the register/forgot-password/rescue forms can't be hammered.
CREATE TABLE IF NOT EXISTS web_rate_limit (
    bucket        VARCHAR(40)   NOT NULL,
    ip            VARCHAR(45)   NOT NULL,
    at            TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    PRIMARY KEY (bucket, ip, at)
);
CREATE INDEX IF NOT EXISTS idx_web_rate_limit_lookup ON web_rate_limit (bucket, ip, at);

-- Admin-editable content: key/value, images store the uploaded file's relative path.
-- Seeded with the keys the panel's templates expect; admins add more freely.
CREATE TABLE IF NOT EXISTS web_content (
    key           VARCHAR(60)   PRIMARY KEY,
    value         TEXT          NOT NULL DEFAULT '',
    kind          VARCHAR(10)   NOT NULL DEFAULT 'text', -- 'text' or 'image'
    updated_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    updated_by    VARCHAR(40)   NOT NULL DEFAULT ''
);

INSERT INTO web_content (key, value, kind) VALUES
    ('site_name', 'JstKO', 'text'),
    ('hero_title', 'JstKO''a Hoş Geldin', 'text'),
    ('hero_subtitle', 'Ücretsiz oyna, hemen rekabete katıl.', 'text'),
    ('hero_image', '', 'image'),
    ('logo_image', '', 'image'),
    ('download_url', '', 'text'),
    ('discord_url', '', 'text'),
    ('server_open_date', '', 'text')
ON CONFLICT (key) DO NOTHING;

-- Update notes / news list shown on the home page.
CREATE TABLE IF NOT EXISTS web_news (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    title         VARCHAR(200)  NOT NULL,
    body          TEXT          NOT NULL DEFAULT '',
    image         VARCHAR(255)  NOT NULL DEFAULT '',
    published     BOOLEAN       NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    created_by    VARCHAR(40)   NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_web_news_published ON web_news (published, created_at DESC);

-- Support tickets ("Destek") raised from the panel.
CREATE TABLE IF NOT EXISTS web_ticket (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id    VARCHAR(21)   NOT NULL,
    subject       VARCHAR(200)  NOT NULL,
    body          TEXT          NOT NULL,
    status        VARCHAR(12)   NOT NULL DEFAULT 'open',
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_web_ticket_account ON web_ticket (account_id, created_at DESC);

CREATE TABLE IF NOT EXISTS web_ticket_reply (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    ticket_id     BIGINT        NOT NULL REFERENCES web_ticket(id) ON DELETE CASCADE,
    author        VARCHAR(40)   NOT NULL DEFAULT '',
    is_staff      BOOLEAN       NOT NULL DEFAULT FALSE,
    body          TEXT          NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_web_ticket_reply_ticket ON web_ticket_reply (ticket_id, created_at);
