-- Restore the nation-war zone rows that an older cleanup removed because
-- their SMDs were not present at that time. +open1..+open6 depends on these
-- distinct zone IDs; otherwise every gate/list collapses to zone 61.

INSERT INTO zone_info (
    zone_no, smd_name, zone_name, zone_type, min_level, max_level,
    init_x, init_z, init_y,
    trade_other_nation, talk_other_nation, attack_other_nation, attack_same_nation,
    friendly_npc, war_zone, clan_updates,
    teleport, gate, escape, calling_friend, teleport_friend,
    blink, pet_spawn, exp_lost, give_loyalty, guard_summon,
    military_zone, mining_zone, blink_zone, auto_loot, gold_lose, status
) VALUES
    (62, 'BattleZone_b.smd', 'Alseids Prairie', 1, 35, 83, 1000, 1000, 0, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, TRUE, TRUE, TRUE, TRUE, TRUE, FALSE, FALSE, FALSE, TRUE, FALSE, 1),
    (63, 'BattleZone_d.smd', 'Nieds Triangle', 1, 35, 83, 1000, 1000, 0, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, TRUE, TRUE, TRUE, TRUE, TRUE, FALSE, FALSE, FALSE, TRUE, FALSE, 1),
    (64, 'BattleZone_e.smd', 'Nereids Island', 1, 35, 83, 1000, 1000, 0, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, TRUE, TRUE, TRUE, TRUE, TRUE, FALSE, FALSE, FALSE, TRUE, FALSE, 1),
    (65, 'Msoft_Moradon_war.smd', 'Moradon War', 1, 35, 83, 1000, 1000, 0, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, TRUE, TRUE, TRUE, TRUE, TRUE, FALSE, FALSE, FALSE, TRUE, FALSE, 1),
    (66, 'new_runawar.smd', 'Oreads', 1, 35, 83, 1000, 1000, 0, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, FALSE, FALSE, TRUE, TRUE, TRUE, TRUE, TRUE, TRUE, FALSE, FALSE, FALSE, TRUE, FALSE, 1)
ON CONFLICT (zone_no) DO UPDATE SET
    smd_name = EXCLUDED.smd_name,
    zone_name = EXCLUDED.zone_name,
    zone_type = EXCLUDED.zone_type,
    min_level = EXCLUDED.min_level,
    max_level = EXCLUDED.max_level,
    attack_other_nation = EXCLUDED.attack_other_nation,
    war_zone = EXCLUDED.war_zone,
    gate = EXCLUDED.gate,
    escape = EXCLUDED.escape,
    calling_friend = EXCLUDED.calling_friend,
    teleport_friend = EXCLUDED.teleport_friend,
    blink = EXCLUDED.blink,
    pet_spawn = EXCLUDED.pet_spawn,
    exp_lost = EXCLUDED.exp_lost,
    give_loyalty = EXCLUDED.give_loyalty,
    auto_loot = EXCLUDED.auto_loot,
    gold_lose = EXCLUDED.gold_lose,
    status = 1;

-- El Morad Eslant Belldan uses the 14438 client/server NPC in zone 12.
-- Keep the Juraid registration rows bound to that NPC and Lua file.
UPDATE quest_helper
SET b_zone = 12,
    s_npc_id = 14438,
    str_lua_filename = '14438_Beldan.lua',
    b_nation = 3
WHERE (b_zone = 19 AND s_npc_id = 15438)
   OR (b_zone = 12 AND s_npc_id IN (14438, 15438));
