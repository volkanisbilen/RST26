-- Event plan: Under The Castle once a week (Friday), Fire & Ice 20 times a day, every other
-- event 3 times a day. Registration windows (Manes / Ultima / Rift) never overlap each other
-- and stay clear of the temple events' own times (BDW 09/14/19, Chaos 11/16/21, Juraid 10/15/22).
DELETE FROM event_auto_schedule;
INSERT INTO event_auto_schedule (event_key, enabled, days, hour, minute, arg, duration_min, note) VALUES
    ('manes', TRUE, 127, 0, 30, '', 0, 'Manes Survival 1. seans'),
    ('manes', TRUE, 127, 12, 5, '', 0, 'Manes Survival 2. seans'),
    ('manes', TRUE, 127, 18, 0, '', 0, 'Manes Survival 3. seans'),
    ('ultima', TRUE, 127, 7, 0, '10', 0, 'Ultima 1. seans'),
    ('ultima', TRUE, 127, 17, 0, '10', 0, 'Ultima 2. seans'),
    ('ultima', TRUE, 127, 23, 0, '10', 0, 'Ultima 3. seans'),
    ('rift', TRUE, 127, 8, 0, '10', 0, 'Full Moon Rift 1. seans'),
    ('rift', TRUE, 127, 12, 50, '10', 0, 'Full Moon Rift 2. seans'),
    ('rift', TRUE, 127, 20, 0, '10', 0, 'Full Moon Rift 3. seans'),
    ('bifrost', TRUE, 127, 11, 30, '', 0, 'Bifrost 1. seans'),
    ('bifrost', TRUE, 127, 17, 30, '', 0, 'Bifrost 2. seans'),
    ('bifrost', TRUE, 127, 21, 45, '', 0, 'Bifrost 3. seans'),
    ('ft', TRUE, 127, 13, 30, '', 0, 'Forgotten Temple 1. seans'),
    ('ft', TRUE, 127, 18, 30, '', 0, 'Forgotten Temple 2. seans'),
    ('ft', TRUE, 127, 22, 30, '', 0, 'Forgotten Temple 3. seans'),
    ('golem', TRUE, 127, 4, 40, '', 0, 'Ateş & Buz 1. seans'),
    ('golem', TRUE, 127, 5, 40, '', 0, 'Ateş & Buz 2. seans'),
    ('golem', TRUE, 127, 6, 40, '', 0, 'Ateş & Buz 3. seans'),
    ('golem', TRUE, 127, 7, 40, '', 0, 'Ateş & Buz 4. seans'),
    ('golem', TRUE, 127, 8, 40, '', 0, 'Ateş & Buz 5. seans'),
    ('golem', TRUE, 127, 9, 40, '', 0, 'Ateş & Buz 6. seans'),
    ('golem', TRUE, 127, 10, 40, '', 0, 'Ateş & Buz 7. seans'),
    ('golem', TRUE, 127, 11, 40, '', 0, 'Ateş & Buz 8. seans'),
    ('golem', TRUE, 127, 12, 40, '', 0, 'Ateş & Buz 9. seans'),
    ('golem', TRUE, 127, 13, 40, '', 0, 'Ateş & Buz 10. seans'),
    ('golem', TRUE, 127, 14, 40, '', 0, 'Ateş & Buz 11. seans'),
    ('golem', TRUE, 127, 15, 40, '', 0, 'Ateş & Buz 12. seans'),
    ('golem', TRUE, 127, 16, 40, '', 0, 'Ateş & Buz 13. seans'),
    ('golem', TRUE, 127, 17, 40, '', 0, 'Ateş & Buz 14. seans'),
    ('golem', TRUE, 127, 18, 40, '', 0, 'Ateş & Buz 15. seans'),
    ('golem', TRUE, 127, 19, 40, '', 0, 'Ateş & Buz 16. seans'),
    ('golem', TRUE, 127, 20, 40, '', 0, 'Ateş & Buz 17. seans'),
    ('golem', TRUE, 127, 21, 40, '', 0, 'Ateş & Buz 18. seans'),
    ('golem', TRUE, 127, 22, 40, '', 0, 'Ateş & Buz 19. seans'),
    ('golem', TRUE, 127, 23, 40, '', 0, 'Ateş & Buz 20. seans'),
    ('utc', TRUE, 32, 20, 0, '180', 0, 'Under The Castle (Cuma) 1. seans');

-- BDW, Chaos and Juraid keep three starts a day (their native schedule had 4-5).
UPDATE event_schedule_main_list SET hour4 = -1, minute4 = -1, hour5 = -1, minute5 = -1 WHERE eventid IN (9, 10, 11);
