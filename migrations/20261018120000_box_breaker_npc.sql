-- Moradon "Box Breaker" NPC: breaks Chaotic Generator boxes (item.effect2 = 251) through a Lua
-- menu instead of the generator window. Same items and same reward tables (item_exchange rows
-- with random_flag 1..3 and origin_item_num1 = box id), same weights, no gold.
--
--   * proto 31514 already exists in the client's NPC tables ("[Monster Strategist]", model 30900)
--     and in npc_template, but was never spawned and has no quest helper, so it renders without
--     any client NPC patch. It is made a friendly (by_group 3) type-46 quest NPC here.
--   * Quests/31514_BoxBreaker.lua is bound through quest_helper (trigger 100) and is copied to
--     /opt/ko26xx/Quests by deploy/deploy-game.sh because it is new in git.
--   * quest_menu / quest_talk only mirror the client texts (62101.. / 62201..) for the server's
--     text catalog; the client shows its own Quest_Menu_us.tbl / Quest_Talk_us.tbl
--     (client_patch/box_breaker_npc).
--
-- Idempotent: safe to run more than once. The game server reads npc_template / npc_spawn /
-- quest_helper at startup, so restart ko-server after applying.

-- 1. NPC template: keep the client-known proto, make it friendly and named for the GM debug line.
INSERT INTO npc_template
SELECT (jsonb_populate_record(
            NULL::npc_template,
            to_jsonb(t) || jsonb_build_object('s_sid', 31514, 'is_monster', FALSE, 'str_name', '[Box Breaker]')
        )).*
FROM npc_template t
WHERE t.s_sid = 22301 AND t.is_monster = FALSE
ON CONFLICT (s_sid, is_monster) DO NOTHING;

-- by_group 3 = friendly to everyone (otherwise the client treats the NPC as an attack target and
-- never sends the right-click), by_type 46 = plain quest NPC, no shop / special window.
UPDATE npc_template
SET str_name = '[Box Breaker]', by_group = 3, by_act_type = 1, by_type = 46, i_selling_group = 0
WHERE s_sid = 31514 AND is_monster = FALSE;

-- 2. Spawn in Moradon (zone 21), next to the Chaotic Generator (815, 640).
INSERT INTO npc_spawn
    (zone_id, npc_id, is_monster, act_type, regen_type, dungeon_family,
     special_type, trap_number, left_x, top_z, num_npc, spawn_range,
     regen_time, direction, dot_cnt, path, room)
SELECT 21, 31514, FALSE, 0, 0, 0, 0, 0, 822, 641, 1, 0, 0, 90, 0, NULL, 0
WHERE NOT EXISTS (
    SELECT 1 FROM npc_spawn WHERE zone_id = 21 AND npc_id = 31514 AND is_monster = FALSE
);

-- 3. Lua binding: clicking the NPC runs 31514_BoxBreaker.lua with EVENT 100.
INSERT INTO quest_helper
    (n_index, b_message_type, b_level, n_exp, b_class, b_nation, b_quest_type, b_zone,
     s_npc_id, s_event_data_index, b_event_status, n_event_trigger_index,
     n_event_complete_index, n_exchange_index, n_event_talk_index, str_lua_filename,
     s_quest_menu, s_npc_main, s_quest_solo)
VALUES
    (315140100, 2, 0, 0, 5, 3, 0, 21, 31514, 0, 0, 100, 0, 0, 0, '31514_BoxBreaker.lua', 0, 0, 0)
ON CONFLICT (n_index) DO UPDATE SET
    s_npc_id = EXCLUDED.s_npc_id,
    b_zone = EXCLUDED.b_zone,
    n_event_trigger_index = EXCLUDED.n_event_trigger_index,
    str_lua_filename = EXCLUDED.str_lua_filename;

-- 4. Text catalog (English; the Turkish variant ships in the client patch builder).
INSERT INTO quest_menu (i_num, str_menu) VALUES
    (62101, 'Break 1'),
    (62102, 'Break 10'),
    (62103, 'Break all of this kind'),
    (62104, 'Break all my boxes'),
    (62105, 'Previous page'),
    (62106, 'Next page'),
    (62107, 'Back to the box list'),
    (62108, 'Other box (name in chat)'),
    (62109, 'Yes, break everything'),
    (62110, 'No, go back'),
    (62201, 'Red Treasure Chest'),
    (62202, 'Green Treasure Chest'),
    (62203, 'Blue Treasure Chest'),
    (62204, 'Fragment of Arrogance'),
    (62205, 'Fragment of Gluttony'),
    (62206, 'Fragment of Rage'),
    (62207, 'Fragment of Sloth'),
    (62208, 'Fragment of Lechery'),
    (62209, 'Fragment of Jealousy'),
    (62210, 'Fragment of Avarice'),
    (62211, 'Silvery Gem'),
    (62212, 'Red Gem'),
    (62213, 'Yellow Gem'),
    (62214, 'Blue Gem'),
    (62215, 'Green Gem'),
    (62216, 'Black Gem'),
    (62217, '70 Quest - Dark Fragment Chest'),
    (62218, 'Holy Armor - Legend Dark Star Wapon Box'),
    (62219, 'Krowaz Armor - Weapon Box'),
    (62220, 'Legend Dragon Box'),
    (62221, 'Black - Red Chitin - Weapon Box'),
    (62222, 'Accessory Box'),
    (62223, 'Exp Item - Chitin Shell Box'),
    (62224, 'Dragon Box'),
    (62225, 'Superior Draki Supply Box'),
    (62226, 'Old Draki Supply Box'),
    (62227, 'Dark Fragment of Pride'),
    (62228, 'Dark Fragment of Gluttony'),
    (62229, 'Dark Fragment of Wrath'),
    (62230, 'Dark Fragment of Sloth'),
    (62231, 'Dark Fragment of Lust'),
    (62232, 'Dark Fragment of Envy'),
    (62233, 'Dark Fragment of Greed'),
    (62234, 'Akara Accessory Box'),
    (62235, 'Fortified Sterling Silver Gemstone'),
    (62236, 'Black Lion Weapon Box')
ON CONFLICT (i_num) DO UPDATE SET str_menu = EXCLUDED.str_menu;

INSERT INTO quest_talk (i_num, str_talk) VALUES
    (62101, 'Welcome! I break boxes, chests, gems and fragments for you - the same rewards as the Chaotic Generator, free of charge. Pick one below; your amounts are listed in the chat window.'),
    (62102, 'You carry nothing I can break. Bring boxes, chests, gems or fragments and come back.'),
    (62103, 'How many do you want to break? The chosen kind and your amount are in the chat window.'),
    (62104, 'This breaks everything breakable in your bag - boxes, chests, gems and fragments - up to 200 pieces at a time. This cannot be undone. Are you sure?'),
    (62105, 'Done! The rewards are in your bag. A summary is in the chat window.'),
    (62106, 'I stopped after 200 pieces. There is more left - talk to me again to continue. A summary is in the chat window.'),
    (62107, 'Your bag is full. Free some slots and talk to me again. What was already broken is listed in the chat window.'),
    (62108, 'You carry too much weight. Drop something and try again. What was already broken is listed in the chat window.'),
    (62109, 'I can''t do that right now. Stay next to me and make sure you still own the box.')
ON CONFLICT (i_num) DO UPDATE SET str_talk = EXCLUDED.str_talk;
