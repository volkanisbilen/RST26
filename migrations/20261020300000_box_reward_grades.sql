-- Box reward grades (docs/BOX_REWARDS.md "Round 2"):
--  (1) every plain +0 equipment reward in every breakable box pool (effect2 = 251, random_flag 1..3) becomes its +1 sibling
--      (item num + 1, name '<name> (+1)', same kind/slot/race/class/req_level); no sibling -> unchanged.
--  (2) Wing Weapon Boxes: (+N)Wing Weapon Box (900733..900736 = +7..+4) give only +N wing weapons; Wing Weapon Box (810284000)
--      and Chaos Wing Weapon Box (810329000) spread each wing weapon over +4/+5/+6/+7 = 50/30/15/5 % of its slots.
-- Weights per box are unchanged (validated). Extra rows live in the reserved n_index range 2110000000..2110099999
-- (2200000000+ does not fit the INTEGER column). Idempotent: spread rows are merged back into their +4 row and deleted first.
BEGIN;

-- 0) undo a previous run's spread rows (merge counts back into the +4 row, then delete)
UPDATE item_exchange t SET exchange_item_count1 = t.exchange_item_count1 + s.c
  FROM (SELECT r.origin_item_num1 b, regexp_replace(regexp_replace(ri.str_name, '\s+', ' ', 'g'), ' \(\+[0-9]+\)$', '') nm, sum(r.exchange_item_count1) c
          FROM item_exchange r JOIN item ri ON ri.num = r.exchange_item_num1
         WHERE r.n_index BETWEEN 2110000000 AND 2110099999 GROUP BY 1,2) s
  JOIN item ti ON regexp_replace(regexp_replace(ti.str_name, '\s+', ' ', 'g'), ' \(\+[0-9]+\)$', '') = s.nm
                  AND regexp_replace(ti.str_name, '\s+', ' ', 'g') ~ '\(\+4\)$'
 WHERE t.origin_item_num1 = s.b AND t.exchange_item_num1 = ti.num AND t.random_flag IN (1,2,3)
   AND t.n_index NOT BETWEEN 2110000000 AND 2110099999;
DELETE FROM item_exchange WHERE n_index BETWEEN 2110000000 AND 2110099999;

CREATE TEMP TABLE _bg_before ON COMMIT DROP AS
  SELECT origin_item_num1 box, sum(exchange_item_count1)::bigint tot
    FROM item_exchange WHERE random_flag IN (1,2,3) AND origin_item_num1 IN (SELECT num FROM item WHERE effect2 = 251)
   GROUP BY 1;

-- 1) wing weapon boxes: weapons = own rewards with ids >= 1100000000 (+ the Chaos Wirinom/Baal/Nebiros of the Chaos box)
CREATE TEMP TABLE _bg_wing ON COMMIT DROP AS
  SELECT n_index, origin_item_num1 box, exchange_item_num1 id, exchange_item_count1 cnt
    FROM item_exchange
   WHERE random_flag IN (1,2,3)
     AND ((origin_item_num1 IN (810284000,810329000,900733000,900734000,900735000,900736000) AND exchange_item_num1 >= 1100000000)
          OR (origin_item_num1 = 810329000 AND exchange_item_num1 - exchange_item_num1 % 10 IN (127430000,139430000,159430000)));
-- base +0 id (a previous run may have left +4 or +N)
UPDATE _bg_wing SET id = id - id % 10;

-- grade siblings by name within base+1..base+10 (Dark Knight Jamadar has a duplicated (+1), so ids are NOT always base+k;
-- names with a doubled space are normalised); same slot/kind/race/class required
CREATE TEMP TABLE _bg_grade ON COMMIT DROP AS
  SELECT w.id, g.k, min(n.num) gid
    FROM (SELECT DISTINCT id FROM _bg_wing) w JOIN item b ON b.num = w.id CROSS JOIN generate_series(4,7) g(k)
    JOIN item n ON n.num BETWEEN w.id + 1 AND w.id + 10
     AND regexp_replace(n.str_name, '\s+', ' ', 'g') = regexp_replace(b.str_name, '\s+', ' ', 'g') || ' (+' || g.k || ')'
     AND n.kind = b.kind AND n.slot = b.slot AND n.race = b.race AND n.class = b.class
   GROUP BY w.id, g.k;
DO $$ DECLARE bad int; BEGIN
  SELECT count(*) INTO bad FROM (SELECT DISTINCT id FROM _bg_wing) w CROSS JOIN generate_series(4,7) g(k)
    LEFT JOIN _bg_grade r ON r.id = w.id AND r.k = g.k WHERE r.gid IS NULL;
  IF bad > 0 THEN RAISE EXCEPTION 'wing weapon grade siblings missing: %', bad; END IF;
END $$;

-- (+N) boxes: grade by box
UPDATE item_exchange e SET exchange_item_num1 = r.gid
  FROM _bg_wing w JOIN _bg_grade r ON r.id = w.id
 WHERE e.n_index = w.n_index AND w.box IN (900733000,900734000,900735000,900736000)
   AND r.k = CASE w.box WHEN 900733000 THEN 7 WHEN 900734000 THEN 6 WHEN 900735000 THEN 5 ELSE 4 END;

-- plain / Chaos boxes: row becomes +4, extra +5/+6/+7 rows take 30/15/5 % of the slots (counts are multiples of 5 = one slot)
CREATE TEMP TABLE _bg_split ON COMMIT DROP AS
  SELECT w.n_index, w.box, w.id, w.cnt / 5 s,
         round(w.cnt / 5 * 0.05)::int s7, round(w.cnt / 5 * 0.15)::int s6, round(w.cnt / 5 * 0.30)::int s5
    FROM _bg_wing w WHERE w.box IN (810284000,810329000);
UPDATE item_exchange e SET exchange_item_num1 = r.gid, exchange_item_count1 = (p.s - p.s5 - p.s6 - p.s7) * 5
  FROM _bg_split p JOIN _bg_grade r ON r.id = p.id AND r.k = 4 WHERE e.n_index = p.n_index;
INSERT INTO item_exchange (n_index, random_flag, origin_item_num1, origin_item_count1, exchange_item_num1, exchange_item_count1)
SELECT 2110000000 + (row_number() OVER (ORDER BY p.box, p.id, g.k) - 1)::int, 1, p.box, 1, r.gid,
       CASE g.k WHEN 5 THEN p.s5 WHEN 6 THEN p.s6 ELSE p.s7 END * 5
  FROM _bg_split p CROSS JOIN (VALUES (5),(6),(7)) g(k) JOIN _bg_grade r ON r.id = p.id AND r.k = g.k;

-- 2) +0 equipment -> +1 everywhere else (wing weapon rows of the 6 wing boxes are already handled)
CREATE TEMP TABLE _bg_map ON COMMIT DROP AS
  SELECT i.num id, n.num n1 FROM item i JOIN item n ON n.num = i.num + 1
   WHERE i.str_name !~ '\(\+[0-9]+\)\s*$'
     AND n.str_name ~ '\(\+1\)'
     AND (regexp_replace(n.str_name, '\s*\(\+1\)\s*$', '') = i.str_name
          OR i.num IN (1113347800,1113361900,2113364760,2113364780))   -- name typo variants (Jamadhar/Jamadar, Flaming/Fire Staff)
     AND i.kind = n.kind AND i.slot = n.slot AND i.race = n.race AND i.class = n.class AND i.req_level = n.req_level;
UPDATE item_exchange e SET exchange_item_num1 = m.n1
  FROM _bg_map m
 WHERE e.random_flag IN (1,2,3) AND e.exchange_item_num1 = m.id
   AND e.origin_item_num1 IN (SELECT num FROM item WHERE effect2 = 251)
   AND NOT (e.origin_item_num1 IN (810284000,810329000,900733000,900734000,900735000,900736000)
            AND (e.exchange_item_num1 >= 1100000000 OR e.exchange_item_num1 - exchange_item_num1 % 10 IN (127430000,139430000,159430000)));

-- 3) validation: every reward exists, per-box slot sums unchanged
DO $$ DECLARE bad int; BEGIN
  SELECT count(*) INTO bad FROM item_exchange e LEFT JOIN item i ON i.num = e.exchange_item_num1
   WHERE e.random_flag IN (1,2,3) AND e.origin_item_num1 IN (SELECT num FROM item WHERE effect2 = 251) AND i.num IS NULL;
  IF bad > 0 THEN RAISE EXCEPTION 'unknown reward ids: %', bad; END IF;
  SELECT count(*) INTO bad FROM _bg_before b JOIN (
      SELECT origin_item_num1 box, sum(exchange_item_count1)::bigint tot FROM item_exchange WHERE random_flag IN (1,2,3) GROUP BY 1) a
      ON a.box = b.box WHERE a.tot <> b.tot;
  IF bad > 0 THEN RAISE EXCEPTION 'slot sum changed in % boxes', bad; END IF;
END $$;

COMMIT;
