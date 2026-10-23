-- Bots currently in the game, refreshed by the game server every 10 s; the web panel counts them (and lists them)
-- as online players so the site never looks empty.
CREATE TABLE IF NOT EXISTS bot_online (
    name       varchar(40) PRIMARY KEY,
    nation     smallint NOT NULL,
    class      smallint NOT NULL,
    level      smallint NOT NULL,
    zone_id    smallint NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
GRANT SELECT ON bot_online TO webpanel;
GRANT SELECT ON item_upgrade_settings TO webpanel;
GRANT SELECT ON event_auto_schedule, admin_event_state TO webpanel;
