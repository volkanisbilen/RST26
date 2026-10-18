-- Solo Cape Scroll (item 810521000, client row exists) and its skill 495147: the server DB did not know them
-- ("item does not exist"). Apply once:
--   sudo -u postgres psql -d ko_server -f migrations/20261014000004_solo_cape_scroll.sql

CREATE TEMP TABLE sc_magic AS SELECT * FROM magic WHERE magic_num = 500131;
UPDATE sc_magic SET magic_num = 495147, en_name = 'Solo Cape', kr_name = 'Solo Cape',
    description = 'Right click to choose a Solo Cape.', use_item = 810521000;
INSERT INTO magic SELECT * FROM sc_magic ON CONFLICT DO NOTHING;
DROP TABLE sc_magic;

CREATE TEMP TABLE sc_t4 AS SELECT * FROM magic_type4 WHERE i_num = 500131;
UPDATE sc_t4 SET i_num = 495147, buff_type = 197, duration = 1;
INSERT INTO magic_type4 SELECT * FROM sc_t4 ON CONFLICT DO NOTHING;
DROP TABLE sc_t4;

CREATE TEMP TABLE sc_item AS SELECT * FROM item WHERE num = 810171000;
UPDATE sc_item SET num = 810521000, str_name = 'Solo Cape Scroll',
    description = 'Right click to choose a Solo Cape.| The cape becomes a cospre emblem item.',
    item_icon_id1 = 81052000, item_icon_id2 = 81052000, effect1 = 495147, effect2 = 0, countable = 1, duration = 1;
INSERT INTO item SELECT * FROM sc_item ON CONFLICT DO NOTHING;
DROP TABLE sc_item;
