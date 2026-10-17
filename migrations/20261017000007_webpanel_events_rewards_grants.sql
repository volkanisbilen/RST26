-- Grants for the real event schedule (same tables the admin panel's own /#/events page
-- reads) and the public daily-quest / event-reward catalog pages.
GRANT SELECT ON event_auto_schedule, admin_event_state, daily_quests,
    native_coin_reward, native_jigsaw_reward, native_roulette_reward, lottery_event_settings
    TO webpanel;
