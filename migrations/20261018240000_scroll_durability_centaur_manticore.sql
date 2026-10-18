-- 1) EXP 100% Resurrection Scroll (811061000) pointed at magic 494040, which does not exist; the
--    matching Type5 self-resurrection magic is 480018 (use_item 811061000, exp_recover 100).
--    Guarded: only touches the row while it still has the broken value.
UPDATE item SET effect1 = 480018 WHERE num = 811061000 AND effect1 = 494040
  AND EXISTS (SELECT 1 FROM magic WHERE magic_num = 480018);

-- 2) Durability only matters for weapons/armor. Boxes, quest items, consumables ... that were stored
--    with durability 0 (admin panel gives, legacy imports) show up red/broken after a zone change.
--    Restore durability = template duration for every non weapon/armor row. Kinds with another
--    meaning for the field are left alone: 255 (use counter), 252 (cospre timer), 151 (pet),
--    250 (seal scroll). Weapon/armor kinds with duration > 1 are genuinely breakable, untouched.
UPDATE user_items u
   SET durability = i.duration
  FROM item i
 WHERE i.num = u.item_id
   AND u.item_id > 0
   AND u.durability = 0
   AND i.duration > 0
   AND COALESCE(i.kind, 0) NOT IN (255, 252, 151, 250)
   AND (
        i.duration = 1
        OR COALESCE(i.kind, 0) NOT IN (11, 21, 22, 31, 32, 41, 42, 51, 52, 60, 70, 71, 110, 140, 181,
                                       200, 210, 220, 230, 240)
   );

-- 3) Centaur and Manticore (all variants incl. Failed Manticore / Centaurs): 30% less attack damage.
--    The monster damage the server uses is npc_template.s_damage; s_atk (client display value, also
--    the damage of the 10007 variant) is scaled too where it carries real data (> 1).
--    Run-once guard so re-running the migration can never scale twice.
CREATE TABLE IF NOT EXISTS data_fix_applied (
    fix_key    TEXT PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

WITH guard AS (
    INSERT INTO data_fix_applied (fix_key) VALUES ('npc_centaur_manticore_damage_x0.7')
    ON CONFLICT (fix_key) DO NOTHING
    RETURNING 1
)
UPDATE npc_template
   SET s_damage = GREATEST(1, ROUND(s_damage * 0.7))::smallint,
       s_atk    = CASE WHEN s_atk > 1 THEN GREATEST(1, ROUND(s_atk * 0.7))::smallint ELSE s_atk END
 WHERE is_monster
   AND (str_name ILIKE '%centaur%' OR str_name ILIKE '%manticore%')
   AND s_damage > 0
   AND EXISTS (SELECT 1 FROM guard);
