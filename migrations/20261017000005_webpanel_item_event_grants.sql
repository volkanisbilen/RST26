-- Grant the webpanel (shared hosting) DB role read access to the static game-data
-- tables needed for the public item search and events schedule pages.
GRANT SELECT ON item, make_item_group, npc_template, monster_item, npc_item,
    event_schedule_main_list, event_schedule_day_list TO webpanel;
