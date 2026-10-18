-- Repair the active test clan's cape state for the v2615 client.
--
-- The clan already owns a valid cape (201), but it was left on an
-- Accredited5 flag with a partial/invalid mark payload. The v2615 client
-- builds the mantle catalogue and cape render state from the clan flag,
-- grade, cape id and mark metadata sent in MyInfo/UserInfo. Keep the
-- reference cape, promote the clan state to Royal1, and replace the broken
-- mark with a valid empty 2400-byte emblem so KNIGHTS_MARK_REQ remains
-- well-formed until a real emblem is registered in-game.
UPDATE knights
SET flag = 12,
    s_cape = 201,
    b_cape_r = 0,
    b_cape_g = 0,
    b_cape_b = 0,
    s_mark_version = 1,
    s_mark_len = 2400,
    mark = decode(repeat('00', 2400), 'hex')
WHERE id_num = 1;
