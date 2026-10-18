# Shozin (special part sewing) audit - 2026-10-18

Scope: `WIZ_ITEM_UPGRADE (0x5B)` sub-opcode 11 (`SPECIAL_PART_SEWING`), handler
`shozin_exchange` in `crates/ko-game/src/handler/item_upgrade.rs`, recipe table
`item_special_sewing` (live: 2,468 rows), client table `mixtable_us.tbl`
(497 rows, `MixLargeTbl_us.tbl` = 22 categories).

The other tables named in the task (`item_exchange`, `item_exchange_exp`,
`make_*`) are not used by this flow (they belong to quest/NPC exchanges and the
random item-maker), so they were not changed.

## 1. Client side (KnightOnline 2625, decompile `KnightOnline.exe.c`)

* UI class: `CUICombinateItem` (source tag `UICombinateItem.cpp`), global
  `dword_111463C + 1088`. The recipe list shown to the player comes from the
  client's `mixtable_us.tbl` (row = NPC proto id, category, 10 x (item, count),
  success/fail FX + message, representative result item).
* Request builder `sub_A67A10` (0x00A67A10). For the 2625 build (client product
  code is not 37x/38x/39x/760) it sends:
  `[0x5B][0x0B][u32 npcRuntimeId][u32 shadowPieceId][u8 shadowSlot][u8 matCount]`
  `[matCount x u8 invSlot][u8 strlen][matCount x "%0.9d%0.3d" (item id, count)]`.
  The server parser already matched this exactly (9-digit ids, 10 digits for
  ids >= 10^9, 3-digit counts). No change needed.
* Response handler `sub_A6ED30` (0x00A6ED30), dispatch `case 11` of opcode 0x5B:

  | code | client reads after the code |
  |---|---|
  | 1 success | `u16` mixtable row (1-based), `u32` item id, `u8` inventory slot |
  | 2 failure | `u16` mixtable row |
  | 0 / other | nothing (client shows "Wrong material") |

  The row is used for the NPC chat message (`mixtable` success/fail text) and
  the slot byte for the "... has been received" popup.
* The server sent `[code][u32 npcID]` (+ `[u32 item][u8 slot]`). The client
  therefore read the low 16 bits of the NPC id as a mixtable row (out of range ->
  no NPC message at all) and the third byte of the item id as the slot.
  **Fixed**: `build_sewing_result_packet` now emits the client layout.
  The mixtable row is looked up through the new `item_special_sewing_client_map`.

## 2. Server logic defects (all fixed in `item_upgrade.rs`, with unit tests)

| # | Defect | Impact | Fix |
|---|---|---|---|
| 1 | Recipe set chosen by NPC *type* (`174` -> jeweler 31402, else 19073). Shozin (proto 19073) is **type 174** in the live DB (`by_type=174` since the zone-21 sniffer sync). | Every Shozin craft was evaluated against the 126 jeweler recipes; all 2,323 Shozin recipes (Krowaz, Hepa, Holy Knight, Secret armor, ...) were unreachable ("wrong material"). | Recipe set = NPC template id when it has recipes (19073 / 31402); legacy type 135 falls back to 19073; other NPCs have none. |
| 2 | Result packet layout (see 1). | No NPC message, wrong "received" slot. | Client layout. |
| 3 | Material matching was not injective: every offered stack only had to equal *some* recipe slot. | Offering one required material twice instead of another one crafted the item with fewer/different materials. | Exact multiset equality of (item, count). |
| 4 | Several recipes with the same materials but results for different classes (Krowaz armor: 5 class variants, one material list) were picked uniformly. | 80% of Krowaz armor crafts gave armor of another class. | `filter_recipes_by_class`: when the matching results span >1 class family keep the crafter's family. Same-class random variants (Krowaz weapons, Secret +1..+4) stay random. |
| 5 | Shadow piece removal wiped the whole slot; shadow slot could equal a material slot. | Item loss / double use of one slot. | One shadow piece per craft; shadow slot must be distinct from material slots. |
| 6 | Materials were removed with two independent loops without re-verification. | Possible loss/dup on a concurrent inventory change. | Verify-then-deduct in a single inventory lock (`consume_sewing_items`); grant is reverted if the deduction ever fails after the grant. |
| 7 | Reported result slot came from `find_slot_for_item` *after* the grant (for non-stackable items that is the next free slot). | Wrong slot byte. | Slot computed before the grant (same slot `give_item` uses). |
| 8 | Roll `rand(0..=10000)` (10,001 values), success iff `rate >= roll`. | Every rate was 0.01% too generous (a rate-0 recipe still succeeded 1 in 10,001). | Roll `[0,10000)`, success iff `roll < rate`. |

Unchanged on purpose: Moradon-only, NPC range/dead checks, bound/duplicate/rented
material rejection, weight check, `dr_sh_total_exchange`, NPC effect broadcast.
`is_notice` is stored on every row but not acted on (it is TRUE for 100%-rate
weapon conversions, a world notice would spam); decide separately.

## 3. Data audit (live PostgreSQL, read-only) vs client mixtable

Server rows: 2,323 (Shozin 19073), 126 (jeweler 31402), 19 corrupted (npc_id 1).

Client rows (497) by category and server coverage (same NPC + same material
list + result present among the server variants):

| Cat | Client name | Rows | OK | Notes |
|---|---|---|---|---|
| 1 | Darkness (Shojin) = **Krowaz weapons** | 11 | 10 | Nepilim: client result 139442521 (+1) vs server variants 139442739-42 (base items). Unresolved, see 5. |
| 2 | Armor Set (Shojin) = **Krowaz armor** | 5 | 5 | 5 server recipes per row (class variants), now class-filtered. |
| 9 | Hepa's Weapon | 43 | 19 -> 43 | 24 missing: added (see 4). |
| 10 | Hepa's Armor | 40 | 0 | Unresolved: no rate evidence. |
| 11/12/13/17 | Under the Castle / Secret Armor / Trophy / Dark Knight weapon | 52 | 52 | OK |
| 20 | Dark Knight Armor (Holy Knight) | 50 | 25 | Client uses newer class plate/fusion-stone materials; 25 variants added. |
| 14 | Jewel pieces | 3 | 0 | Client lists a placeholder result; server gems recipes (1210-1212) are consistent, leave. |
| 18 | Lunar Rabbit's Mortar | 3 | 2 | row 276 unresolved. |
| 19 | Draki's Pendant | 155 (9 distinct) | 0 | Client uses the 810593000 pendant line; server only has the 1190029xxx "Philosopher" line. Unresolved, no rates. |
| 21 | Cursed [Craft Weapon Box] | 11 | 0 | Cursed box -> Krowaz weapons. Unresolved: rate unknown. |
| 3/15 | Accessory (jewel) | 110 | 0 | Client rows use the generic "Identical Used Accessory"; the real jeweler recipes (126) use concrete items. Unresolved/ambiguous. |
| 22/23/4/5/6/16 | event/misc | 14 | 2 | Missing items in the item table (see below) or no rate evidence. |

Server-only: ~2,087 rows are not listed in the client mixtable (1,166 "Donusturme"
weapon-transfer rows, Draki "Philosopher" pendants, fragments, ...). They still
work when the player drops the materials in by hand; left untouched.

Integrity checks on all 2,468 rows:

* Every material / result item exists in the server item table and in the
  client `Item_Org_us.tbl`, except: 814063000 (rows 1220 "Bronze Star", 1221
  "Silver Star": material missing in the server item table and in the client
  table -> unreachable) and the corrupted rows below.
* No recipe requires `count > 1` of a non-countable item (would be unreachable),
  none has `rate` outside 1..10000, none exceeds the 3-digit client count.
* **Corrupted rows 5047-5065** (19 rows): column-shifted import (first material
  lost, `give_item_id=1`, `success_rate=19073`, `npc_id=1`). Each is an exact
  duplicate (same result, same materials according to the client row) of a healthy
  row 5028-5046 -> deleted.
* 89 exact duplicate rows (65 groups, same materials, result, rate and flags)
  -> deleted (no probability change, only the lowest `n_index` is kept).
* Krowaz lines: 25 "Krowas Armor Exchange" rows (rate 34.50%), 39 "Krowas Weapon
  Exchange" rows (27.00%), all items exist. Krowaz armor pieces
  (`Krowaz *`, `Pathos Krowaz Chitin *`) are otherwise drops/box items; no other
  Krowaz item has a Shozin recipe in either the client or the server data.

## 4. Migration `migrations/20261018160000_shozin_sewing_audit.sql`

Idempotent, one transaction, only touches `item_special_sewing` and the new
`item_special_sewing_client_map`:

1. creates `item_special_sewing_client_map(n_index, mix_index)`; filled from the
   client mixtable (same NPC + same material multiset, result match preferred);
   `-npc_id` rows hold the per-NPC fallback message row.
2. deletes the 19 corrupted rows and the 89 exact duplicates.
3. adds 49 recipes (`n_index` 6001-6049) that exist in the client UI but not on
   the server: 24 Hepa's Weapon recipes (rate 25.00%, same as the 19 existing
   Hepa weapon rows, shadow-success) and 25 Holy Knight armor variants with the
   client's class-specific stone/plate materials (rate copied from the existing
   recipe for the same result item, 33.50%).

Apply: `psql "$DATABASE_URL" -f migrations/20261018160000_shozin_sewing_audit.sql`,
then restart ko-game (recipes load at startup). It can be applied before or
after the new binary (the map table is optional at load time).

## 5. Unresolved / needs an owner decision

* Hepa's Armor (40), Draki's Pendant 810593000 line (9), Cursed weapon box (11),
  Mithril weapons (2), Lunar Rabbit mortar (1): the client lists them, the server
  has no recipe and there is no source for the success rates. They are not added
  (inventing rates would be arbitrary). Each needs a rate; the structure is in
  the client table.
* Nepilim result id mismatch (client 139442521 vs server 139442739-42).
* Christmas star / Chocolatier / Fortune pouch / Snowman rows reference items
  that do not exist in the item table (900417000.., 508260000.., 814063000).
* Shozin Lua (`19073_Shojin.lua`) menu: the "Manufacturing" branch only closes
  the dialog; whether the client opens the combine window from the NPC click or
  needs a server packet could not be established from the decompile.
* `is_notice` is ignored (see 2).
