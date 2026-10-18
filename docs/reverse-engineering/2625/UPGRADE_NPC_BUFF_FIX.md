# Upgrade and NPC buff audit (2026-09-19)

Client evidence: supplied KnightOnline2625/KnightOnline.exe.c.

- Main game dispatcher, opcode 0x5B, routes subtypes 2/7/14 to 0x00B99EB0.
- 0x00B99EB0 reads mode then result. Result 0 enters 0x00B95810.
- 0x00B95810 consumes the Logos flag only for subtype 2, AFTER result.
  The previous Rust implementation inserted it BEFORE mode, making a normal
  failure look like mode 0 / result 1. Failed items consequently stayed visible.
- An authoritative inventory slot count follows completed upgrades, including
  destroyed origins and consumed materials. Preview sends no inventory changes.
- High-class standard successful +7/+8/+9 upgrades announce globally, including
  GM upgrades, regardless of missing per-item notice flags.

Live PostgreSQL evidence:

- `new_upgrade` is the runtime recipe source, not legacy `item_upgrade`.
- `item_upgrade_settings` already contains Blessed Upgrade Scroll rates for
  item types 4/5 and grades 1..9. No rate changes are needed.
- Missing grade transitions include Chitin Shell Pauldron 206001546,
  Chitin Shell Pads 206002036, and Krowaz Warrior Upper Garment 208001546.
- Migration 20260919000003 adds only absent Blessed Scroll recipes for existing,
  matching normal high-class item pairs: same name stem, kind, slot, item type,
  class, adjacent item ID and adjacent named grade. Special/rebirth classes,
  unnamed supply items and existing recipes are excluded.

NPC effects:

- Logos skills 500034/492018/510533/504001 and Enchanter skills
  302344/302331/302328/490223 exist as Type4 buffs in live PostgreSQL.
- Lua CastSkill previously stored a buff without recalculating derived stats,
  and sent success=0, duration=0, speed=0. The corrected path refreshes stats
  and sends the actual Type4 duration/speed and success=1.
- Royal/National Enchanter 31508 callbacks validate current level and money;
  levels <=35 are free, 36..60 cost 30,000 per buff, 61+ cost 50,000.
  Existing Victory coupon exemption and bundle prices are preserved.
- Buff callbacks cast only once; the victory-menu speed selection now grants
  speed rather than incorrectly selecting HP.

In-game verification remains necessary: fail an upgrade with the inventory open,
observe a +7 notice from another character, upgrade a repaired +6 pair, select
each Logos effect, and compare Enchanter stats/coins at levels 35, 36 and 61.
