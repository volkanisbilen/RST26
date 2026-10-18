-- Event audit 2026-10-18.
--
-- Fire & Ice (golem war) reward rules pointed at two item ids that do not exist in the item table:
--   1900004000  "Event Emblem"   -> the item is stored with its extension digits: 1900004106
--   340710000   "Lunar War Belt" -> the item is stored as 340710124
-- A rule whose item is missing cannot be given: the winners never got the 24 hour Event Emblem
-- (the server silently dropped it) and the 5% Lunar War Belt could never drop.
--
-- Idempotent and live-safe: it only touches rows that still hold the wrong ids, only when the right
-- item exists, and does nothing when the web-panel table is not there.

DO $$
BEGIN
    IF to_regclass('public.event_reward_rule') IS NOT NULL AND to_regclass('public.item') IS NOT NULL THEN
        UPDATE event_reward_rule
           SET item_id = 1900004106
         WHERE item_id = 1900004000
           AND EXISTS (SELECT 1 FROM item WHERE num = 1900004106);

        UPDATE event_reward_rule
           SET item_id = 340710124
         WHERE item_id = 340710000
           AND EXISTS (SELECT 1 FROM item WHERE num = 340710124);
    END IF;
END $$;
