-- Julia Manes' Orb exchange values verified against the v2615 client TBLs.
-- Achievement 471 / title 139 is Solar Messenger (Attack +16, Defence +16).

INSERT INTO quest_menu (i_num, str_menu) VALUES
    (45330, 'Manes'' Orb Exchange'),
    (45054, '7 - Event Emblem (3 Days)'),
    (40852, '15 - Crisis/Ibexs Transformation Scroll'),
    (45331, '15 - Magpie Mom Transformation Scroll'),
    (40906, '20 - VIP Vault Key (7 Days)'),
    (45332, '45 - Oreads (1 Day)'),
    (45357, '50 - Solar Pendant'),
    (40855, '100 - Pathos Glove (7 Days)'),
    (45055, '150 - Magic Hammer (10)'),
    (45359, '300 - <Solar Messenger> Achievement'),
    (45333, '450 - Lunar Tattoo (15 Days)')
ON CONFLICT (i_num) DO UPDATE SET str_menu = EXCLUDED.str_menu;

UPDATE quest_talk
SET str_talk = 'You have brought Manes'' Orbs! Which reward would you like to exchange for?'
WHERE i_num = 45422;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM achieve_main
        WHERE s_index = 471 AND title_id = 139
    ) THEN
        RAISE EXCEPTION 'Solar Messenger definition mismatch: expected achievement 471 / title 139';
    END IF;
END $$;
