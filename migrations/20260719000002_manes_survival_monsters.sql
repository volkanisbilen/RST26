-- Manes Survival (zone 96) monster templates and verified spawn layout.
-- New event SIDs clone existing server-side combat/AI templates; only SID/name/PID are overridden.

CREATE TABLE IF NOT EXISTS manes_survival_spawn (
    id              SERIAL PRIMARY KEY,
    npc_id          SMALLINT NOT NULL,
    grade           SMALLINT NOT NULL CHECK (grade BETWEEN 1 AND 4),
    boss_tier       SMALLINT NOT NULL DEFAULT 0 CHECK (boss_tier BETWEEN 0 AND 3),
    spawn_x         INTEGER NOT NULL,
    spawn_z         INTEGER NOT NULL,
    spawn_range     SMALLINT NOT NULL DEFAULT 8,
    spawn_count     SMALLINT NOT NULL DEFAULT 1,
    respawn_seconds SMALLINT NOT NULL DEFAULT 20,
    UNIQUE (npc_id)
);

DROP TABLE IF EXISTS _manes_template_map;
CREATE TEMP TABLE _manes_template_map (
    new_sid SMALLINT PRIMARY KEY,
    source_sid SMALLINT NOT NULL,
    new_name TEXT NOT NULL,
    new_pid SMALLINT NOT NULL
);

INSERT INTO _manes_template_map (new_sid, source_sid, new_name, new_pid) VALUES
(10701,150,'[Lower Grade] Kecoon',100),
(10702,250,'[Lower Grade] Bulcan',200),
(10703,254,'[Lower Grade] Bulture',200),
(10704,550,'[Lower Grade] Werewolf',500),
(10705,353,'[Lower Grade] spoiler',300),
(10706,8646,'[Lower Grade Mid-Boss] Gaboltin',300),
(10707,8648,'[Lower Grade Mid-Boss] Scoride',900),
(10708,8650,'[Lower Grade Mid-Boss] Byzombie',1000),
(10709,8652,'[Lower Grade Mid-Boss] Skelcrown',1100),
(10710,1672,'[Mid Grade] Dust Orc',1638),
(10711,1303,'[Mid Grade] Haunga',1300),
(10712,1180,'[Mid Grade] Skeleton',1100),
(10713,503,'[Mid Grade] Shadow seeker',500),
(10714,507,'[Mid Grade] Lupus',506),
(10715,1114,'[Mid Grade] Dragon Tooth Skeleton',1102),
(10716,8653,'[Mid Grade Mid-Boss] Garukonga',1300),
(10717,8656,'[Mid Grade Mid-Boss] Urukthrone',1636),
(10718,8657,'[Mid Grade Mid-Boss] Alkedrada',1102),
(10719,8658,'[Mid Grade Mid-Boss] Threthonez',5200),
(10720,1721,'[High Grade] Troll',1701),
(10721,9064,'[High Grade] Aionclo',5900),
(10722,2401,'[High Grade] Stone golem',2400),
(10723,2104,'[High Grade] Paramun',2100),
(10724,2402,'[High Grade] Giant golem',2400),
(10725,8007,'[High Grade] Doom Soldier',6000),
(10726,8009,'[High Grade] Evil Wizard',6100),
(10727,1402,'[High Grade] Atross',1410),
(10728,3001,'[High Grade] DARK MARE',2800),
(10729,8401,'[High Grade Mid-Boss] Hell Fire',5701),
(10730,8402,'[High Grade Mid-Boss] Enigma',5802),
(10731,8403,'[High Grade Mid-Boss] Havoc',5801),
(10732,8404,'[High Grade Mid-Boss] Cruel',5805),
(10733,9250,'[Boss] Dark Dragon',32015);

DELETE FROM npc_template WHERE is_monster = true AND s_sid BETWEEN 10701 AND 10733;

INSERT INTO npc_template (
 s_sid,is_monster,str_name,s_pid,s_size,i_weapon_1,i_weapon_2,by_group,by_act_type,
 by_type,by_family,by_rank,by_title,i_selling_group,s_level,i_exp,i_loyalty,
 i_hp_point,s_mp_point,s_atk,s_ac,s_hit_rate,s_evade_rate,s_damage,s_attack_delay,
 by_speed_1,by_speed_2,s_standtime,s_item,i_magic_1,i_magic_2,i_magic_3,s_fire_r,
 s_cold_r,s_lightning_r,s_magic_r,s_disease_r,s_poison_r,s_bulk,by_attack_range,
 by_search_range,by_tracing_range,i_money,by_direct_attack,by_magic_attack,area_range
)
SELECT
 m.new_sid,true,m.new_name,m.new_pid,t.s_size,t.i_weapon_1,t.i_weapon_2,t.by_group,
 t.by_act_type,t.by_type,t.by_family,t.by_rank,t.by_title,t.i_selling_group,t.s_level,
 t.i_exp,t.i_loyalty,t.i_hp_point,t.s_mp_point,t.s_atk,t.s_ac,t.s_hit_rate,
 t.s_evade_rate,t.s_damage,t.s_attack_delay,t.by_speed_1,t.by_speed_2,t.s_standtime,
 t.s_item,t.i_magic_1,t.i_magic_2,t.i_magic_3,t.s_fire_r,t.s_cold_r,t.s_lightning_r,
 t.s_magic_r,t.s_disease_r,t.s_poison_r,t.s_bulk,t.by_attack_range,t.by_search_range,
 t.by_tracing_range,t.i_money,t.by_direct_attack,t.by_magic_attack,t.area_range
FROM _manes_template_map m
JOIN npc_template t ON t.s_sid = m.source_sid AND t.is_monster = true;

DO $$
DECLARE missing_count INTEGER;
BEGIN
 SELECT COUNT(*) INTO missing_count
 FROM _manes_template_map m
 LEFT JOIN npc_template t ON t.s_sid=m.new_sid AND t.is_monster=true
 WHERE t.s_sid IS NULL;
 IF missing_count <> 0 THEN
   RAISE EXCEPTION 'Manes Survival template clone incomplete: % source templates missing', missing_count;
 END IF;
END $$;

DELETE FROM manes_survival_spawn WHERE npc_id BETWEEN 10701 AND 10733;

INSERT INTO manes_survival_spawn
(npc_id,grade,boss_tier,spawn_x,spawn_z,spawn_range,spawn_count,respawn_seconds) VALUES
-- Low-grade monsters: outer map.
(10701,1,0,840,512,18,5,12),(10702,1,0,800,800,18,5,12),
(10703,1,0,512,884,18,5,12),(10704,1,0,179,650,18,4,14),
(10705,1,0,269,269,18,4,14),
-- Low bosses: inner field, visible on the Survival minimap.
(10706,1,1,688,688,6,1,90),(10707,1,1,342,688,6,1,90),
(10708,1,1,342,342,6,1,90),(10709,1,1,688,342,6,1,90),
-- Mid-grade monsters: middle ring.
(10710,2,0,761,512,14,4,16),(10711,2,0,688,688,14,4,16),
(10712,2,0,512,741,14,4,16),(10713,2,0,339,685,14,4,16),
(10714,2,0,267,512,14,4,18),(10715,2,0,342,342,14,4,18),
-- Four mid bosses in the central castle.
(10716,2,2,464,464,5,1,120),(10717,2,2,560,464,5,1,120),
(10718,2,2,464,560,5,1,120),(10719,2,2,560,560,5,1,120),
-- High-grade monsters: inner ring.
(10720,3,0,671,512,10,3,20),(10721,3,0,624,624,10,3,20),
(10722,3,0,512,715,10,3,20),(10723,3,0,431,593,10,3,20),
(10724,3,0,341,512,10,3,22),(10725,3,0,394,394,10,3,22),
(10726,3,0,512,341,10,3,22),(10727,3,0,602,422,10,2,25),
(10728,3,0,602,602,10,2,25),
-- Four high bosses and Dark Dragon in the castle.
(10729,3,2,480,520,4,1,150),(10730,3,2,544,520,4,1,150),
(10731,3,2,512,488,4,1,150),(10732,3,2,512,552,4,1,150),
(10733,4,3,512,520,0,1,0);

DROP TABLE IF EXISTS _manes_template_map;
