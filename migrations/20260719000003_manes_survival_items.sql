-- Manes Survival event items for PostgreSQL.
-- Idempotent: safe to run repeatedly from DBeaver or psql.

INSERT INTO item (
 num,extension,str_name,description,item_plus_id,item_alteration,item_icon_id1,item_icon_id2,
 kind,slot,race,class,damage,min_damage,max_damage,delay,range,weight,duration,buy_price,
 sell_price,sell_npc_type,sell_npc_price,ac,countable,effect1,effect2,req_level,req_level_max,
 req_rank,req_title,req_str,req_sta,req_dex,req_intel,req_cha,selling_group,item_type,
 hitrate,evasionrate,dagger_ac,jamadar_ac,sword_ac,club_ac,axe_ac,spear_ac,bow_ac,
 fire_damage,ice_damage,lightning_damage,poison_damage,hp_drain,mp_damage,mp_drain,
 mirror_damage,droprate,str_b,sta_b,dex_b,intel_b,cha_b,max_hp_b,max_mp_b,fire_r,cold_r,
 lightning_r,magic_r,poison_r,curse_r,item_class,np_buy_price,bound,mace_ac,by_grade,
 drop_notice,upgrade_notice
) VALUES
(978023000,22,'Exclusive Manes'' Survival HP Recovery Potion','', '0',0,97802300,97802300,
 97,17,20,0,0,0,0,0,0,0,1,1,0,0,0,0,1,491410,0,0,100,0,0,0,0,0,0,0,253,0,
 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
(978024000,22,'Exclusive Manes'' Survival MP Recovery Potion','', '0',0,97802400,97802400,
 97,17,20,0,0,0,0,0,0,0,1,1,0,0,0,0,1,491411,0,0,100,0,0,0,0,0,0,0,253,0,
 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
(978025000,22,'Exclusive Manes'' Survival Mid-Boss Summon Ticket','', '0',0,97802500,97802500,
 97,15,0,0,0,0,0,0,0,1,3,2500,0,0,416,0,0,491412,0,0,100,0,0,0,0,0,0,0,0,0,
 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
(978026000,22,'Manes'' Orb','', '0',0,97802600,97802600,
 97,15,0,0,0,0,0,0,0,1,1,50,0,0,0,0,1,0,0,0,100,0,0,0,0,0,0,0,0,0,
 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
ON CONFLICT (num) DO UPDATE SET
 extension=EXCLUDED.extension,str_name=EXCLUDED.str_name,description=EXCLUDED.description,
 item_plus_id=EXCLUDED.item_plus_id,item_alteration=EXCLUDED.item_alteration,
 item_icon_id1=EXCLUDED.item_icon_id1,item_icon_id2=EXCLUDED.item_icon_id2,
 kind=EXCLUDED.kind,slot=EXCLUDED.slot,race=EXCLUDED.race,class=EXCLUDED.class,
 weight=EXCLUDED.weight,duration=EXCLUDED.duration,buy_price=EXCLUDED.buy_price,
 sell_price=EXCLUDED.sell_price,ac=EXCLUDED.ac,countable=EXCLUDED.countable,
 effect1=EXCLUDED.effect1,effect2=EXCLUDED.effect2,req_level=EXCLUDED.req_level,
 req_level_max=EXCLUDED.req_level_max,selling_group=EXCLUDED.selling_group;

DO $$
BEGIN
 IF (SELECT COUNT(*) FROM item WHERE num BETWEEN 978023000 AND 978026000) <> 4 THEN
   RAISE EXCEPTION 'Manes Survival item installation failed';
 END IF;
END $$;
