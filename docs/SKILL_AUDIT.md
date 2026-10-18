# Skill audit (all classes)

Scope: Warrior, Rogue, Mage, Priest and the Kurian/Porutu line, every tier (beginner / novice / master, both
nations), `magic` + `magic_type1..9` on the live DB (read-only export) against the client tables
`Skill_Magic_Main_us/tk` and `Skill_Magic_1..9`.

## Method

`scripts/skill_audit.py` (read-only) takes CSV exports of the magic tables, optionally the client `Data` folder
(through `tbl_dump`) and reports findings by id:

| id | check |
|----|-------|
| S1/S1b/S2/S2b/S3/S4 | missing/orphan/unused type-table rows, class skills with no executable type, NULL magic rows |
| C1..C7 | server-only / client-only skills, main and detail-table value diffs, client main row without client detail row |
| V1..V11 | negative values, missing use_item, id/class/nation consistency (sSkill), DOT without duration, heal on enemy moral, ... |
| N1..N5 | Karus vs El Morad twin comparison (fields and detail rows) |

Client column map (established empirically, every mapped pair agrees on >= 99 % of rows): Main c11 flying_effect,
c12 target_effect, c14 moral, c15 skill_level, c16 skill, c17 msp, c18 hp, c21 use_item, c22 cast_time,
c23 recast_time, c27 success_rate, c28 type1, c29 = server `etc` (the bType[1] second effect), c30 range, c32 use_standing.

Result: server and client data are 99.9 % identical (3899 vs 3899 main rows, 18 ids differ on each side). Description
numbers match the data for 266 of 274 melee skills. The real defects are few and listed below.

## Fixed

| # | Defect | Fix |
|---|--------|-----|
| 1 | Necromancer 110820 / Enchanter 210820 "Instantly Magic" (master) are rows with every numeric column NULL: skill could never be cast | migration, values from the client row |
| 2 | Kurian/Porutu passives with another class's `sSkill`: 115805 Boldness, 115815 Axid Break (1068 = Guardian), 115745/115770/214745/215745/215770 (1147 = Karus novice Kurian) | migration + client `Skill_Magic_Main_us/tk` |
| 3 | Items whose magic row was never seeded (client has it): Prayer of Priest x6 (501160-501165), Continuance items 512010/512011 | migration (magic + magic_type4) |
| 4 | 105710 Rise (Karus Berserker) cost 0 MP while 106710/205710/206710 cost 50 | migration + client c17 |
| 5 | 106550 Mangling (Karus Guardian) range 15, all siblings and the client 0 | migration |
| 6 | The server never checked the learned-skill requirement (`sSkillLevel`): any class skill id of your class could be cast without the level / skill points | `skill_requirement_met` gate in `magic_process::handle` (+3 tests); GMs and Chaos Dungeon / Dungeon Defence / Knight Royale exempt as in the C++ reference |

## Client files (not shipped by this change)

`client_patch/skill_fixes/Data/Skill_Magic_Main_us.tbl` and `Skill_Magic_Main_tk.tbl`, built from the current
`D:\ai\JstKO Client\Data` with `tbl_rawedit skillfix <Data> <out>` (byte-exact except the 8 listed cells).

## Unresolved / for the owner

* Pathos tree skills (sSkill 4106, 7106 ... = tree*1000+class; 106018-106024 etc.): the class gate ignores them
  (class code 410..921), buff types 167/168 (damage-immune area) are not implemented anywhere, and Traina's Wisdom /
  Logos' Strength are magic type 10, which the dispatcher does not know ("unknown skill type"). Moral-11 Type 4 area
  buffs do not include the caster (C++ `UserRegionCheck` does). Check in game whether masters can reach these skills.
* Kurian Devil mechanics are visual only: no damage absorption into the devil gauge (C++ `ABSORBED_TOTAL`, 1/3.1
  of player damage, gauge -20000 ends the form), Spirit Strengthening (-50 % MP while devil) and the +50 % magic
  damage on a devil target are missing.
* Awakening (115802) is unlocked at 2 master points in the client table but `calculate_max_sp` grants +50 SP at 3.
* Mastered Warrior passive percentages (20/34/40/50/60/80 with shield) are double the skill descriptions
  (10/15/20/25/30/40); 34 for Arrest looks like a typo for 30. Ported from the C++ reference, left unchanged.
* Smoke Screen 108780/208780: server moral 12 / recast 600 / duration 10 vs client 10 / 1800 / 3 (deliberate tuning?).
* Sword Aura 105557 "250 %" and Critical Thunder 114557 "150 %" descriptions vs hit 200 / 100.
* Frost Nova, Static Nova, Chain Lightning carry a -420 tick with duration 0 (no DOT happens).
* Client-only rows without a server twin and without an item (304000/1, 495148, 503010, 512020, 610162), Bulk-Up
  scrolls 491423/4 (size buff semantics unclear) and Pathos festival Rush/Blink scrolls 520201/2 were not added.
* Orphan Kurian detail rows (114707, 114717 ... 115818 etc.) have no magic row on either side; unreachable data.
* Beginner priest skill order differs between nations (104005 Holy Attack vs 204005 Light Healing): names match
  effects, no functional defect.

## Verify in game

Necromancer/Enchanter master with 15 master points: Instantly Magic casts, next spell has no cast time. Kurian
master skill window: Boldness/Axid Break/Spirit Strengthening/Contract Of Soul appear in the Kurian tree (needs the
client files). Karus Berserker Rise costs 50 MP. Casting still works for all learned skills of each class
(watch for skills that fail right after the update: that would mean a `sSkillLevel` that exceeds the real
requirement).
