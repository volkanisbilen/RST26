-- Juraid Mountain follow-up fixes for the v2615 event flow.

-- The attached event reference describes Juraid as a 75+ / 35-minute event.
UPDATE quest_helper
SET b_level = 75
WHERE n_index IN (692, 693, 694, 695)
  AND s_npc_id IN (24438, 15438);

UPDATE event_opt_vroom
SET play = 35,
    attackclose = 35
WHERE zoneid = 87;

UPDATE event_room_play_timer
SET event_sign_time = 10,
    event_play_time = 35,
    event_attack_close = 35,
    event_finish_time = 20
WHERE event_zone_id = 87
   OR event_name = 'Juraid Mountain';

UPDATE quest_talk
SET str_talk = replace(str_talk, '50 minutes', '35 minutes')
WHERE i_num IN (804, 808);
