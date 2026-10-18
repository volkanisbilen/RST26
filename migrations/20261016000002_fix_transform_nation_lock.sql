-- Monster/NPC/Special/MamaPag transformation scrolls (magic_type6.user_skill_use 1, 3, 4, 7)
-- were imported with nation = 1 almost across the board (139/139 monster rows, 62/62 NPC rows,
-- 20/20 special rows, 10/10 MamaPag rows all nation=1, with not a single nation=2 or nation=0
-- counterpart) -- an import artifact, not a deliberate Karus-only design: these are cosmetic
-- disguise scrolls, the same item is usable by both nations, and execute_type6's nation check
-- (crates/ko-game/src/handler/magic_process.rs) was rejecting every one of them for Elmorad
-- players ("works for Karus, not for humans").
-- Siege transformations (user_skill_use 0, 5, 6) are left untouched: those are already
-- zone-locked (Delos/Battle zones) and their nation values are genuinely mixed, not a flat 1.
BEGIN;

UPDATE magic_type6
SET nation = 0
WHERE user_skill_use IN (1, 3, 4, 7)
  AND nation = 1;

COMMIT;
