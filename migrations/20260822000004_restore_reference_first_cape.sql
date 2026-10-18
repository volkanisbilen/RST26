-- `CKnightsManager::UpdateKnightsGrade()` in the supplied reference C++
-- source assigns cape 0 when a Training clan is promoted to flag 2.
-- Previous experimental migrations changed the active test clan (ID 1) to
-- cape 1, which is not the reference first-cape state. Restore only that
-- known, currently affected clan; purchased capes in other clans are untouched.
UPDATE knights
SET s_cape = 0,
    b_cape_r = 0,
    b_cape_g = 0,
    b_cape_b = 0
WHERE id_num = 1
  AND flag = 2
  AND s_cape = 1;
