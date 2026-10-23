-- Daily quests for every monster of Karus Eslant (11), Elmorad Eslant (12) and Ronark Land (71).
-- Rule: one quest per monster name of the zone (several templates with the same name share the quest, max 4 ids),
-- only plain monsters (by_type 0, no "[boss/quest]" names) that have at least 4 spawns so 50 kills are realistic.
-- Every quest of these three zones (new and old): 50 kills, rewards 1,000,000 gold + 2,500,000 EXP + 1 Gold Coin.
-- Reward codes: 900000000 = gold, 900001000 = EXP, anything else = item. Old rows are kept in daily_quests_backup_20261023.
BEGIN;

CREATE TABLE IF NOT EXISTS daily_quests_backup_20261023 AS SELECT * FROM daily_quests WHERE zone_id IN (11, 12, 71);

WITH m AS (
    SELECT s.zone_id, t.str_name nm, t.s_sid, t.s_level, sum(s.num_npc) n
    FROM npc_spawn s
    JOIN npc_template t ON t.s_sid = s.npc_id AND t.is_monster
    WHERE s.is_monster AND s.zone_id IN (11, 12, 71) AND t.by_type = 0 AND t.str_name !~ '^\['
    GROUP BY s.zone_id, t.str_name, t.s_sid, t.s_level
), g AS (
    SELECT zone_id, nm, (array_agg(s_sid ORDER BY s_sid))[1:4] ids, max(s_level) lvl, sum(n) n
    FROM m GROUP BY zone_id, nm HAVING sum(n) >= 4
), fresh AS (
    -- skip a group when an existing quest of the zone already counts one of its monsters
    SELECT g.* FROM g
    WHERE NOT EXISTS (
        SELECT 1 FROM daily_quests q
        WHERE q.zone_id = g.zone_id AND (q.mob_id_1 = ANY (g.ids) OR q.mob_id_2 = ANY (g.ids) OR q.mob_id_3 = ANY (g.ids) OR q.mob_id_4 = ANY (g.ids))
    )
), numbered AS (
    SELECT f.*, (SELECT COALESCE(max(id), 0) FROM daily_quests) + row_number() OVER (ORDER BY zone_id, lvl, nm) AS new_id FROM fresh f
)
INSERT INTO daily_quests (id, quest_name, quest_id, time_type, kill_type, mob_id_1, mob_id_2, mob_id_3, mob_id_4, kill_count,
                          reward_1, reward_2, reward_3, reward_4, count_1, count_2, count_3, count_4,
                          zone_id, min_level, max_level, replay_time, random_id)
SELECT new_id, left(nm, 90) || ' Hunt', 0, 1, 2,
       COALESCE(ids[1], 0), COALESCE(ids[2], 0), COALESCE(ids[3], 0), COALESCE(ids[4], 0), 50,
       900000000, 900001000, 379107000, 0, 1000000, 2500000, 1, 0,
       zone_id, LEAST(60, GREATEST(30, lvl - 12)), 83, 24, 0
FROM numbered;

UPDATE daily_quests SET kill_count = 50,
    reward_1 = 900000000, count_1 = 1000000,
    reward_2 = 900001000, count_2 = 2500000,
    reward_3 = 379107000, count_3 = 1,
    reward_4 = 0, count_4 = 0
WHERE zone_id IN (11, 12, 71);

COMMIT;
