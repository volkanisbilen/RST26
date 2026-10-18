-- "I love knight online Emblem Exchange Voucher" (1/3/7/15/30 day) -> Kaira "Ticket Exchange use" -> 1113329037
-- ([I Love Knight Online] emblem). Reward lifetime = voucher days (1d=24h, 3d=72h, 7d=168h, 15d=360h, 30d=720h).
-- Must stay in step with the client Data\TICKET_EXCHANGE.tbl rows (selector 0 = everyone).
-- Idempotent: upsert by primary key. Rules are read live per exchange, no reload needed.
CREATE TABLE IF NOT EXISTS native_ticket_exchange_rule (
    ticket_item_id INTEGER NOT NULL,
    reward_item_id INTEGER NOT NULL,
    duration_hours INTEGER NOT NULL DEFAULT 0 CHECK (duration_hours >= 0),
    selector SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (ticket_item_id, reward_item_id)
);

INSERT INTO native_ticket_exchange_rule (ticket_item_id, reward_item_id, duration_hours, selector) VALUES
    (811084000, 1113329037,  24, 0),
    (811085000, 1113329037,  72, 0),
    (811086000, 1113329037, 168, 0),
    (811087000, 1113329037, 360, 0),
    (811088000, 1113329037, 720, 0)
ON CONFLICT (ticket_item_id, reward_item_id)
DO UPDATE SET duration_hours = EXCLUDED.duration_hours, selector = EXCLUDED.selector;
