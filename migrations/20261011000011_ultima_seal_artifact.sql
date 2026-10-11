-- The Ultima seal is the client's "[Artifact]" NPC (proto 10101, type 61): an attackable, targetable
-- object (like the CSW artifacts). Nation 0 so everybody can attack it; 150000 HP, each hit removes 500.
DELETE FROM npc_template WHERE s_sid = 10101 AND is_monster = TRUE;
UPDATE npc_template SET by_group = 0, i_hp_point = 150000, s_ac = 0, s_atk = 0, s_damage = 0
WHERE s_sid = 10101 AND is_monster = FALSE;
