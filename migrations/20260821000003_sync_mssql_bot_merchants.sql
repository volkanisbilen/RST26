-- Synchronise the active saved merchant stalls from KO_DATABASE_SERVER_001.
-- The older bulk migration contained obsolete rows and omitted the saved
-- position/direction/type columns, which caused those bots to open as buyers.
DELETE FROM bot_merchant_data;

INSERT INTO bot_merchant_data (
    n_index, advert_message,
    n_num1, n_price1, s_count1, s_duration1, is_kc1,
    px, pz, py, minute, zone, s_direction, merchant_type
) VALUES
    (633, 'DENEME',    389205000, 1000000, 199, 6000, false, 80300, 52100, 0, 9999, 21, 312, 0),
    (634, 'TEST',      389201000, 1000000, 199, 6000, false, 80800, 52100, 0, 9999, 21, 312, 0),
    (635, 'BURADAYIM', 389199000, 1000000, 199, 6000, false, 81300, 52100, 0, 9999, 21, 312, 0),
    (636, '1',         389198000, 1000000, 199, 6000, false, 81800, 52100, 0, 9999, 21, 312, 0),
    (637, '2',         389197000, 1000000, 199, 6000, false, 82300, 52100, 0, 9999, 21, 312, 0),
    (638, '3',         389196000, 1000000, 199, 6000, false, 82800, 52100, 0, 9999, 21, 312, 0),
    (639, '4',         389075000, 1000000,   1, 6000, false, 80300, 52400, 0, 9999, 21, 312, 0),
    (640, '5',         389075000, 1000000,   1, 6000, false, 80800, 52400, 0, 9999, 21, 312, 0),
    (641, '6',         389075000, 1000000,   1, 6000, false, 81300, 52400, 0, 9999, 21, 312, 0),
    (642, '7',         389075000, 1000000,   1, 6000, false, 81800, 52400, 0, 9999, 21, 312, 0),
    (643, '8',         389075000, 1000000,   1, 6000, false, 82300, 52400, 0, 9999, 21, 312, 0),
    (644, '9',         389075000, 1000000,   1, 6000, false, 82800, 52400, 0, 9999, 21, 312, 0),
    (645, '0',         389075000, 1000000,   1, 6000, false, 80300, 52700, 0, 9999, 21, 312, 0);
