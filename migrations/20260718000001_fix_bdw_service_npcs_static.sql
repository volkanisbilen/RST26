-- BDW service NPCs were imported as monsters, which selected their monster
-- templates and registered them with the NPC AI movement loop. Each nation
-- has one inn hostess and two merchants; they must use the non-monster
-- service templates and remain stationary in every event room.
UPDATE npc_spawn
SET is_monster = FALSE
WHERE zone_id = 84
  AND npc_id IN (
      8108, -- Karus Inn Hostess
      8109, -- El Morad Inn Hostess
      8111, -- Karus Potion Merchant
      8112, -- Karus Sundries Merchant
      8161, -- El Morad Potion Merchant
      8162  -- El Morad Sundries Merchant
  );
