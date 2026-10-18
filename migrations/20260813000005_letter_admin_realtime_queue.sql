-- Realtime bridge used by the local Letter Admin GUI.
-- The GUI writes the letter and this queue row in one transaction; the game
-- server consumes it and sends LETTER_UNREAD to an online recipient.
CREATE TABLE IF NOT EXISTS letter_admin_notification (
    notification_id BIGSERIAL PRIMARY KEY,
    letter_id        INTEGER NOT NULL REFERENCES letter(letter_id) ON DELETE CASCADE,
    recipient_name   VARCHAR(21) NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at     TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_letter_admin_notification_pending
    ON letter_admin_notification (notification_id)
    WHERE processed_at IS NULL;
