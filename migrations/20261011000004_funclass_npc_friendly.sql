-- by_group 0 makes the client treat the NPC as an attack target and skip the right-click
-- (WIZ_NPC_EVENT) dispatch; same fix as the Akara statue (nation 3 = friendly to everyone).
UPDATE npc_template
SET by_group = 3, by_act_type = 1
WHERE s_sid = 31998 AND is_monster = FALSE;
