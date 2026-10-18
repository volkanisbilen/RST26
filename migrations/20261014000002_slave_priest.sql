-- Akara's Slave Priest: scroll item 751740000 (already in the client item table) casts skill 495146
-- (client skill table row exists), a one hour self buff (buff type 196). While it runs the HopeGuard
-- mini window (hook 0xEC) controls the priest.
-- Apply once:  sudo -u postgres psql -d ko_server -f migrations/20261014000002_slave_priest.sql

-- the skill (copy of a plain self-buff row)
CREATE TEMP TABLE sp_magic AS SELECT * FROM magic WHERE magic_num = 500131;
UPDATE sp_magic SET magic_num = 495146, en_name = 'Akara''s Slave Priest', kr_name = 'Akara''s Slave Priest',
    description = 'Summons a priest that follows you and heals, buffs, cures and resurrects.', use_item = 751740000;
INSERT INTO magic SELECT * FROM sp_magic ON CONFLICT DO NOTHING;
DROP TABLE sp_magic;

CREATE TEMP TABLE sp_t4 AS SELECT * FROM magic_type4 WHERE i_num = 500131;
UPDATE sp_t4 SET i_num = 495146, buff_type = 196, duration = 3600, exp_pct = 100;
INSERT INTO magic_type4 SELECT * FROM sp_t4 ON CONFLICT DO NOTHING;
DROP TABLE sp_t4;

-- the scroll item (copy of the old Slave Priest Summon Scroll row)
CREATE TEMP TABLE sp_item AS SELECT * FROM item WHERE num = 810171000;
UPDATE sp_item SET num = 751740000, str_name = 'Akara''s Slave Priest',
    description = 'Slave Priest Scroll | Summons a priest that follows you and| heals, buffs, cures and resurrects.',
    item_icon_id1 = 97801900, item_icon_id2 = 97801900, effect1 = 495146, effect2 = 0, countable = 1;
INSERT INTO item SELECT * FROM sp_item ON CONFLICT DO NOTHING;
DROP TABLE sp_item;
