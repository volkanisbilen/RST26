-- Support tickets can be opened without logging in (players who cannot verify their account).
ALTER TABLE web_ticket
    ADD COLUMN IF NOT EXISTS guest boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS contact_email varchar(120) NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS guest_account varchar(40) NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS guest_char varchar(40) NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS client_ip varchar(45) NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS access_token varchar(64) NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_web_ticket_guest ON web_ticket (guest, created_at DESC);
