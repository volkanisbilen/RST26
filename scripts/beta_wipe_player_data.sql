-- Beta-launch reset: wipes every account/character/clan/player-progress table while leaving
-- all static game content (items, NPCs, drop tables, quest/magic/event definitions, PUS
-- catalog, etc.) completely untouched.
--
-- NOT a schema migration -- run by hand, once, right before opening beta:
--   sudo -u postgres pg_dump ko_server > ko_server_pre_beta_wipe_$(date +%Y%m%d_%H%M).sql
--   sudo -u postgres psql -d ko_server -f scripts/beta_wipe_player_data.sql
--
-- RESTART IDENTITY resets every SERIAL/IDENTITY sequence on these tables back to 1 automatically
-- (no need to track down sequence names by hand). CASCADE catches any table with a foreign key
-- into one of these that isn't itself listed below.
BEGIN;

TRUNCATE TABLE
    -- web panel (player-submitted, not admin CMS content)
    web_ticket_reply, web_ticket, web_rate_limit, web_password_reset, web_email_verify,

    -- native event participation (not the event's own reward/board config)
    native_akara_auction, native_marble_user, native_coin_user, native_jigsaw_user,
    native_roulette_history, native_roulette_user,

    -- per-account/char misc progress
    user_block_item_list, solo_cape, rental_item, manner_daily_claim,
    user_saved_magic, user_skill_shortcuts, user_enchant, user_costume,
    user_hermetic_seal, user_soul_data,
    anti_afk_list, letter_admin_notification, game_audit_log,

    -- rankings / leaderboards (store specific character/account/clan names)
    world_boss_ranking, user_daily_rank_stats, moraranker_statue_slot,
    draki_tower_rift_rank, draki_tower_daily_rank, daily_rank,
    knights_rating, user_knights_rank, user_personal_rank,

    -- king election (candidacy/votes/results reference real characters)
    king_nomination_result, king_election_votes, king_candidacy_notice_board,
    king_nomination_list, king_election_list,

    -- pets
    pet_user_items, pet_image_change, pet_user_data,

    -- achievements / quests / titles (progress, not the quest/achieve *definitions*)
    right_top_title, user_daily_quest, user_achieve_summary, user_achieve, user_quest,

    -- daily/return/loot progress
    daily_reward_cumulative, daily_reward_user, user_return_data, user_seal_exp,
    user_loot_settings, user_perks, user_daily_op, user_genie_data,

    -- character item seals
    character_seal_mapping, character_seal_items,

    -- mail
    letter_gift, letter, friend_list,

    -- warehouse / inventory
    vip_warehouse_items, vip_warehouse, user_warehouse_coins, user_warehouse,
    user_deleted_items, user_items,

    -- guild / clan
    guild_bank_log, guild_bank_item, guild_bank,
    clan_warehouse_items, user_knightdata, knights_auto, knights_castellan,
    knights_siege_warfare, knights_alliance,

    -- cash shop / premium (redemption/purchase history, not the PUS catalog)
    pus_refund, ppcard_list, account_premium, check_account,

    -- core: accounts, characters, clans (must be last so CASCADE can still see FKs into them)
    knights, currentuser, userdata, account_char, tb_user
    RESTART IDENTITY CASCADE;

-- knights id_num=0 is not a real clan, it's the "no clan" placeholder every character's
-- userdata.knights FK (fk_userdata_knights) defaults to and requires to exist -- without it,
-- creating any new character fails ("violates foreign key constraint fk_userdata_knights").
-- Re-seed it every time this script truncates knights (see 20260207000011_seed_knights_zero.sql).
INSERT INTO knights (id_num, flag, nation, id_name, chief) VALUES (0, 0, 0, '', '')
ON CONFLICT (id_num) DO NOTHING;

COMMIT;

-- NOT included on purpose (static game content / config, never wipe these):
--   item, npc_template, monster_item, make_item_group, magic*, quest_menu/quest_talk/
--   quest_monster/quest_helper, event_reward_rule, pus_category, pus_items, daily_quests,
--   daily_reward, send_messages, server_settings, damage_settings, perks, knights_cape,
--   knights_csw_opt, cindwar_*, lottery_event_settings, collection_race_*, bot_* (NPC filler),
--   rankbug, mining_exchange, item_smash, item_special_sewing, event_beef_play_timer,
--   event_room_play_timer, web_content, web_news, admin_command, admin_heartbeat,
--   admin_audit_log, admin_users.
--
-- king_system is also left alone: it's a 2-row (one per nation) *scheduling* singleton, not
-- per-account data. If you want the election clock reset too, run separately:
--   UPDATE king_system SET by_type=0, by_im_type=0;
