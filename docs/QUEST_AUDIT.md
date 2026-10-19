# Quest audit - why quest rewards / quest items were not given

Scope: all 844 Lua scripts in `Quests/`, the quest tables of the live database (read-only snapshot of
2026-10-18) and the Lua bindings in `crates/ko-game/src/lua_engine/`.
Re-run: `python tools/quest_audit.py --data-dir <csv dir> [--dump]` (the section between the AUTO markers
below is regenerated; see the tool's docstring).

## Root causes found

| # | Cause | Effect | Fix |
|---|---|---|---|
| 1 | 2100+ of 2167 `RunQuestExchange(...)` calls are followed by `SaveEvent(...)` without checking the result. When the exchange fails (bag full) the quest was completed anyway: nothing taken, nothing given, quest finished for good. | "Quest done but no reward" | `QuestRunState`: a refused reward (bag full) blocks the `SaveEvent` that would complete the quest in the same script run; the player keeps the quest and gets the bag-full message (WIZ_QUEST 13/3). Data errors (missing exchange row) do NOT block, so quests cannot get stuck. |
| 2 | Slot check of `RunQuestExchange` tested each output against the *same* free slot, ignored slots freed by the turn-in items, and the origin items were removed before the (failing) delivery. | Items lost / partial rewards | `WorldState::can_hold_after_exchange` dry-runs removal + delivery on a copy of the bag; nothing is touched unless everything fits. |
| 3 | `RunQuestExchange` aborted the whole reward when ONE reward id is not in the item table (gold/EXP lost too; the Lua then completed the quest). | Quest completes without any reward | C++ parity: unknown/virtual ids are skipped with a warning, the rest is paid. The reward the player explicitly picked (item_exchange_exp) still refuses. |
| 4 | `by_selected_reward` was only refreshed by dialog buttons, so a reward index picked in an earlier dialog stayed in the session and made later exchanges without a reward table fail ("invalid or missing selected reward"). | Random "no reward" after using a reward-choice quest | `quest_v2_run_event` sets it for every run (-1 for NPC click / turn-in). |
| 5 | `RobItem` only worked from ONE slot, while `HowmuchItem` adds all slots. Non-stackable turn-in items (e.g. 10x VIP Package, quest cards) passed the check and then could not be taken (and a multi-origin exchange stopped half-way). | Turn-in fails / half consumed | `rob_item` falls back to taking the amount across several slots (all-or-nothing). |
| 6 | `RobItem(UID, id)` (count omitted, optional in C++) raised an mlua conversion error and aborted the script (councilor 14407/24407, Magaret, Pot/FoodBox/Well). | Script dies before the reward | count is `Option<u16>` (default 1). |
| 7 | `GiveItem`: ids above u32 wrapped silently (`8104340970` -> another item), counts above 65535 wrapped, failures were logged to the reward audit log as if delivered, an unknown item id answered "bag full". | Wrong/no item, misleading message | exact id parsing, count clamped to 1..9999, log only on success, bag-full message only for a real bag-full. |
| 8 | 960 `item_exchange` rows (the ones added by migration `20260325000001` from the 27-column client table) are shifted one column, 39 of them are used by Lua (Magmom 6823-6829, Hill/Rily 16243-16270, Kesar/Julius 16267-16306, Menisia 2450 ...). Rewards sit in the count columns, so nothing is given and the origin check is meaningless. | No reward at all for those quests | Migration `20261018130000_quest_audit_fixes.sql` realigns them (idempotent). |
| 9 | `25177_iruta.lua` (tattoo voucher NPC): item id `8104340970` (10 digits), `RobItem(id)` without UID/count, voucher ids that do not exist in the item table. | Tattoo exchange never worked | ids replaced by the vouchers whose names/durations match (see list below); Solar 1d/15d and Stellar 3d have no voucher item in the item table and were left untouched. |
| 10 | 3 kill quests with quest_helper rows but no `quest_monster` row (456, 942, 1631): kills never counted. | Quest cannot be finished | Rows added by the same migration (values from client `Quest_Monster_Exchange.tbl`). |

iruta mapping (by item name): Nimbus 3d 931755000->978127000, 7d 931765000->978128000, 15d 931775000->814029000;
Solar 3d 810922000->978129000, 7d 810923000->978130000; War 1/3/7/15d 814613000/814623000/814633000/814643000 ->
978104000/978105000/978106000/978107000; GiveItem Nimbus id -> 810434973 (Solar 810431970, Lunar 810931953, Stella
810433972 follow the same +970/+971/+972/+973 pattern).

## Not changed on purpose / left for the owner

* `Quests/25072_Sedi.lua`, `25166_Sedi_k.lua`, `7001_dungeun.lua` differ between repo and the live Quest-Studio copy: not touched.
* Live `quest_helper` lacks ~700 rows that exist in the client `Quest_Helper.tbl` (Magmom, Mackin/rebirth, Aset event
  menus, Hill/Rily, Kesar/Julius, Rabbit ...). The client table was patched locally in places (rows 14109+ collide with
  server rows), so it was NOT bulk-imported. Scripts that `SaveEvent` such rows (listed below as SAVEEVENT_UNKNOWN) cannot
  change quest state until the owner decides which of those quests should exist.
* 361 quest NPCs have no `npc_spawn` row (e.g. Della 14426 / Elrod 24426, 75 helper rows each; Magmom 31506; Kesar/Julius).
* Items referenced but absent from the item table (event items 9003xx, 9004xx, 508230000 ... ) cannot be invented; see ITEM_MISSING list.
* Reward correctness vs. the original game (which item/amount) was not changed anywhere; only delivery bugs were fixed.

## Owner checklist (in game)

1. Apply migration, restart server. Complete a Moradon starter/level quest with a full bag: expect the "inventory full"
   message, quest stays open, turn-in items kept; free a slot and turn in again: reward arrives, quest completes.
2. Magmom (31506) / Hill / Rily quests: reward (EXP/items) now arrives (needs the NPC to be spawned).
3. Councilor (14407/24407) turn-ins work (RobItem without count).
4. Iruta tattoo vouchers exchange.

## Round 2: Luferson / El Morad / Eslant line quests (weapon and accessory rewards)

Method: every `RunQuestExchange` id used by NPCs 11xxx/14xxx/21xxx-24xxx was compared (a) against its Karus/El Morad twin
(same quest class-by-class, via `quest_helper`) and (b) against the client `docs/data/Item_Exchange.tbl`
(`tbl_dump docs/data/Item_Exchange.tbl 0 100000 > client_ex.txt`, columns: c0 id, c1 flag, c3..c12 origin, c13..c22 reward).
`quest_helper` for these NPCs is identical to the client `Quest_Helper.tbl` (El Morad has the same structure as Karus; the
only differences are the shared nation-3 "255" gate rows, which the client table lacks too). No column-shifted
`item_exchange` rows remain (0 rows with `origin_item_num1 = 0` and a count >= 100000).

| # | Finding | Fix |
|---|---|---|
| 11 | **Captain Falkwine (11510_Forkwain.lua, El Morad) "Monster Suppression Squad (II)"** called `MonsterStoneQuestJoin(UID,199)` = default Stone 1 / family 4, while Captain Fargo (21510_Hwargo.lua, Karus) was already changed to `(UID,199,82,71)` = Stone 2 spawn set (`monster_stone_respawn_list` zone 82, family 71, 30 spawns). | El Morad script now calls `MonsterStoneQuestJoin(UID,199,82,71)` (identical behaviour). The other 6 differences between the two scripts are text ids / quest numbers only. |
| 12 | **Reward items missing in `item_exchange`** (weapons, rings, earrings, pendants, belts): the server rows pay only EXP (or nothing) where the client table lists the items. Quests: Wolf (Jalk 892-895 / Melverick 888-891), Orc (Cheina 953-955 / Kape 957-959), Clarence weapon quest (214-221), accessory quests Teils/Rucel (earring 1122-1129, pendant 1130-1137) and Cheina/Kape (ring 1106-1113, belt 1114-1121). The Quest completed with EXP only. | Migration `20261018250000_quest_reward_items_restore.sql` restores the client reward items per class row (66 rows); EXP/gold the server row paid is kept unless the matching `item_exchange_exp` choice row pays the same EXP (would double). |
| 13 | **Rows with EMPTY rewards** (all `exchange_item_*` = 0): Della/Elrod 148-155 (Outpost Captain weapons), Ascetic Veda 493-496 (Karus Eslant weapons of `<selfname>`). The quest completed and gave nothing. | Same migration, client rewards restored. |

Left unchanged on purpose (server row differs from the client on purpose, not a missing reward): 944/945 (Weapon/Armor
Enchant Scroll ids), 985-992 (`[Quest]` water/potion ids), 1042/1043 (Green Treasure Chest vs Dark Mithril Fragment), and the
origin/amount differences of the class-mastery quests (39-41, 308-315 ...). Rewards of these rows are paid (all slots of
the row are given, `RunQuestExchange` needs one free slot per item, e.g. 4 weapons for the warrior rows).

New checks in `tools/quest_audit.py`: `EXCHANGE_NO_OUTPUT` (used exchange with no reward at all) and, with
`--client-exchange client_ex.txt`, `EXCHANGE_CLIENT_ITEMS` (client lists reward items the server row does not give).
After the migration: 95 -> 21 `EXCHANGE_CLIENT_ITEMS` and 20 -> 7 `EXCHANGE_NO_OUTPUT` (the rest are the intentional rows above and
non-line NPCs: Caval 4000, Hill/Rily 16260-16266, 6165, 6230).

Owner checklist (in game): (1) El Morad Captain Falkwine "Monster Suppression Squad (II)" -> teleports into Monster Stone
2 like the Karus Fargo quest. (2) Do the Wolf (Jalk/Melverick), Orc (Cheina/Kape) weapon quests and a Teils/Rucel or
Cheina/Kape accessory quest with a class: the weapon/ring/earring/pendant/belt arrives together with the quest completing
(keep 4-5 free bag slots). (3) Clarence level-71 weapon quest.

## Round 3: client tables are the ground truth (client-vs-server diff)

Method: the CLIENT tables of `D:\ai\JstKO Client\Data` (Quest_Helper 11629 rows, Quest_Monster_Exchange 859, Item_Exchange 5358,
Item_Exchange_exp 163, Quest_Talk/Menu, NpcMopMap_Info = client NPC placement) were dumped with `tbl_dump` and diffed against a
read-only snapshot of the live DB (2026-10-20: quest_helper 11559, quest_monster 609, item_exchange 12021, npc_spawn, monster_item,
npc_template). The comparison is `tools/quest_client_sync.py`; it also generates the migration and a simulated "after" snapshot,
and `tools/quest_audit.py --client-dump DIR` runs the same comparison as audit findings (`CLIENT_*`).
Note: `docs/data/*.tbl` is NOT the client in use (6 of the 8 quest tables differ from `JstKO Client/Data`); round 3 reads the real client.

Re-run: `tbl_dump <Data>/Quest_Helper.tbl 0 1000000 > dump/Quest_Helper.txt` (same for Quest_Monster_Exchange, Item_Exchange,
NpcMopMap_Info), `python tools/quest_audit.py --data-dir <csv> --dump`, then
`python tools/quest_client_sync.py --client-dump dump --data-dir <csv> --sql out.sql --after <csv_after>`.

Migration: `migrations/20261020100000_quest_client_sync.sql` (one transaction, idempotent: `ON CONFLICT DO NOTHING`, `NOT EXISTS`
guards, every UPDATE re-checks the snapshot value, nothing is deleted; verified idempotent against the model by re-running the
generator on the "after" snapshot: 0 changes).

### Counts before -> after (per category)

| Cat | What | Before | Change | Notes |
|---|---|---|---|---|
| a | `quest_helper` rows in the client but not on the server | 900 client-only rows | **777 added** | 206 `daily_quest_manager` rows (npc 0, server has its own daily quest system) skipped on purpose; 77 rows whose client index is used by other server rows (14109-14187: Moira/Kaira/Hamus/Krevent ...) are inserted at index 40000+ (no Lua `SaveEvent`s these ids). 45 NPC scripts: Aset 192, Mackin 145, Magmom 119, Rabbit 31, Hill/Rily 8 scripts x 10-26, Kesar/Julius 25+25, Akara, Bgirs, Krevent 10, Move 10 ... |
| a | `SAVEEVENT_UNKNOWN` (Lua saves a quest state that has no helper row) | 203 (99 INFO + 104 WARN) | 99 (30 INFO + 69 WARN) | the rest has no helper row in the client either (thethyke prison scripts, Agata/Pablo/Dreak/Skaky old ids: dead branches) |
| a | `quest_monster` rows missing | 265 (219 usable) | **219 added** | 46 more reference monster ids that are not in `npc_template` (player-kill quests use "monster" id 1/2 = opposing nation) |
| a | `item_exchange` ids used by Lua/helper but absent | 34 | 34 | none of them exists in the client table either (0, 705/706, 6250-6254, 6751, 6897-6944 ...): nothing to derive |
| b | `quest_monster` kill target / count differs | 30 | **26 updated** | e.g. quest 78 count 10 -> 3, 342 250 -> 120, 356 60/60/80 -> 40/40/40, 223/224/459/460 targets 9777+1692+11692, 289/290 targets; 4 left (unknown monster ids) |
| b | `quest_helper.b_level` higher on the server than in the client | 19 | **19 lowered** | Menisia 98-104 (12 -> 7), Patric 3331-3346 (8/9 -> 6). The server demanded a level the client does not show. |
| b | `quest_helper` other differences | n_exp 5397, nation 564, level 73, zone 34, npc_main 110, talk 3 | kept | server value is more permissive than the client (nation 3 = both vs client 2 on Agata 26-90, level 1 vs 60 on Rily 9780-9829, exp threshold 0 vs client value), or display-only (zone, npc_main). Not changed: tightening them would block quests that work today. |
| b | `item_exchange` placeholder rows of client-only quests | 11 | **11 replaced by the client row** | 6851-6855, 6858-6861 (Hunt-gate/pet/rebirth quests; the server row demanded 10 x `810035000` and paid a random equipment pool: nobody could ever finish them), 16237/16238 (Dark Mithril 1 -> 2) |
| b | `item_exchange` reward the client lists, server lacks | 30 (all) | **1 appended** (29 Menisia: Speed-Up Potion), 29 kept | kept = Moira random pools (+3 items vs client +1/+2, `16047_Moira.lua`), 944/945, 985-992, 1042/1043, 6242 (judged intentional in rounds 1-2), Kesar 182, server-added Noah bonus (42 rows pay MORE than the client: Forkwain 88/54, Patric 323, Hapa 108, guardsman 1142-1145 ...) |
| b | `item_exchange` origin differs from client | 57 used rows | kept | the Lua script of those rows checks the SERVER item set (e.g. Dreak 39: `HowmuchItem(379098000)`, client row wants 379046000); changing the row would make the turn-in impossible |
| c | `HELPER_EVENT_NO_HANDLER` | 39 | 42 | +3 = new client rows Moira 1695 (trigger 4000). Remaining: Hapa 14301 skill quests 491/514/515 and Marauders 69 (events 241, 554-565, 1555-1765), Krevent 29235 / Keite / Moira / Patric / Burlet dead server rows. They need the original dialog scripts (text ids) - not derivable from the tables. |
| c | Lua order bug: `SaveEvent(complete)` before the reward | 2 | **2 fixed** | `14201_Skaky.lua` / `24201_Skaky.lua` EVENT 537 (class promotion): the quest completed and the player was promoted even when the reward was refused (full bag). Reward first, quest + promotion only if it was delivered. |
| d | quest NPCs without any `npc_spawn` row | 406 (361 with template) | **14 spawned** -> 392 | placed where the client `NpcMopMap_Info` puts them (Della 14426 / Elrod 24426 zone 71, 29012-29014, 29025-29027, 29072/29073 zone 71, Kesar 32557 / Julius 32558, 19001/29001 zone 21). 347 have no client placement (25xxx/16xxx/13xxx help NPCs, Moradon vendors ...), 45 are not in `npc_template` (Rabbit 31770, Hill/Rily 3182x, Bgirs, Cube1-8, Move ...): cannot be created from the client NPC table (it has no stats). |
| d | quest NPC spawned in another zone than `b_zone` | 48 | kept | `b_zone` is only a hint (guide); reported |
| e | quest item required by an exchange, dropped by nobody | 21 slots (quests 802, 809, 812, 939, 940, 942) | **21 drop slots written** | migration `20260817000005` had installed 802/809/812 but a later drop re-import removed them (the live drop tables 303/353/600/650/602/652 do not contain them); bosses 4059/4060/4067 drop their 3 quest items at 100 %. Existing drops are never overwritten (free slots only). 941 (4 items) and 803/805 are chain items produced by another exchange. |
| e | item ids in the client tables but absent from the server item table | 2 | **2 added** | 810921000 / 810924000 (tattoo vouchers used by `25177_iruta.lua`), cloned from the 30-day voucher 810926000 |
| e | item ids absent from BOTH tables | 24 | 24 | 508230000-508233000 (8th cake), 810261000, 890227000, 931750000 (Magmom), 900355762, 9003xx/9004xx (Carlos, KIFA, Ran, Chung/Beak/Hyun/Joo): cannot be invented |

### Engine change (delivery)

* `RunQuestExchange` now honours `exchange_item_timeN` (lifetime in days of a reward, as the C++ `GiveItem` does). The 23 timed rows
  (6876-6883 Moradon event quests, 30-day items) used to hand out permanent items. Unit test `test_exchange_item_days_reads_the_output_slot`.

### Unresolved / needs a decision

* **Player-kill quests** (Quest_Monster_Exchange target id 1/2 = "kill the other nation": Hill/Rily/Kesar/Julius, ~25 quests): the server only counts NPC kills
  (`quest_monster_count_add` is called from the NPC death path), so these quests cannot reach state 3. Needs a hook in the player-kill path.
* `HELPER_EVENT_NO_HANDLER` (42) and the Moira 1695 "Lucky Box" event 4000 need the original quest dialogs (text ids) to be scripted.
* 45 quest NPCs without `npc_template`, 347 without any placement data (see above).
* The 114 differing-on-purpose exchange rows (list in the generator report) - owner decides whether the client numbers should win.
* `BUTTON_NO_HANDLER` 10 on `31737_Fullmoon.lua` is a false positive (table driven `SHOP[EVENT]`).

### Owner checklist (in game, after applying the migration and restarting)

1. **Menisia (Moradon)**: her quests (helper rows 98-104) are offered from level 7 (was 12); the exchange 29 reward now includes the Speed-Up Potion. Patric (13013) quests 3331-3346 from level 6 (was 8/9).
2. **Karus Eslant / El Morad Eslant event NPCs - Magmom (31506)**: talk to him; the event menu quests (e.g. 1662-1668) now start and save state.
3. **Aset / Mackin (rebirth, level 83)**: the Mackin quest chain steps (1718-1723) are now stored; the hunt-gate exchanges 6859/6858 no longer demand 10 x 810035000.
4. **Move NPCs / Knight (16084)**: the 1703-1705 kill-gate quests complete without a bogus item cost.
5. **Hill / Rily (Ronark Land)**: with Della 14426 / Elrod 24426 / Kesar / Julius now standing in Ronark Land (zone 71) talk to them and take a quest; rewards 16237/16238 give 2 Dark Mithril Fragments.
6. **Boss item quests 939/940/942 (Forbidden Love line)**: kill the boss (4059/4060/4067) - the 3 quest items now drop and can be turned in.
7. **Smilodon / Tarantula quests 809 / 812 and the mana remains quest 802**: the collection items drop again from the mobs.
8. **Class promotion at Skaky (Moradon)**: with a full bag the promotion is refused (quest stays open); free a slot and try again.
9. A 30-day timed quest reward (Moradon event quest 6876+): the item shows an expiry date.

<!-- AUTO:BEGIN -->
Scripts scanned: **845** Lua files, **38390** calls, **12336** quest_helper rows, **1644** quests, **12021** item_exchange rows (1350 referenced from Lua), **489357** items.  
Findings: **210 ERROR**, **5930 WARN**, 204 INFO.  
Quests with at least one ERROR on a path they reach: **111** of 1644.

### Findings by check

| Check | Severity | Count |
|---|---|---:|
| `ITEM_MISSING` | ERROR | 59 |
| `MONSTER_NOT_IN_TEMPLATE` | ERROR | 47 |
| `HELPER_EVENT_NO_HANDLER` | ERROR | 42 |
| `EXCHANGE_ORIGIN_ITEM` | ERROR | 26 |
| `EXCHANGE_OUTPUT_ITEM` | ERROR | 24 |
| `BUTTON_NO_HANDLER` | ERROR | 10 |
| `EXCHANGE_NO_OUTPUT` | ERROR | 2 |
| `MONSTER_QUEST_MISSING` | WARN | 3405 |
| `HELPER_NPC_UNSPAWNED` | WARN | 2020 |
| `HELPER_NPC_MISSING` | WARN | 277 |
| `QUEST_ID_UNKNOWN` | WARN | 71 |
| `SAVEEVENT_UNKNOWN` | WARN | 69 |
| `BAD_ITEM_ID` | WARN | 34 |
| `HELPER_EXCHANGE_MISSING` | WARN | 29 |
| `EXCHANGE_CLIENT_ITEMS` | WARN | 17 |
| `EXCHANGE_MISSING` | WARN | 5 |
| `ITEM_MISSING` | WARN | 2 |
| `EXCHANGE_ZERO_COUNT` | WARN | 1 |
| `EXCHANGE_ZERO_COUNT` | INFO | 153 |
| `SAVEEVENT_UNKNOWN` | INFO | 30 |
| `ITEM_MISSING` | INFO | 9 |
| `EXCHANGE_NO_OUTPUT` | INFO | 9 |
| `EVENT_ORDER` | INFO | 1 |
| `CLIENT_EXCHANGE_DIFF` | INFO | 1 |
| `CLIENT_NPC_UNSPAWNED` | INFO | 1 |

### item_exchange rows that are misaligned (shifted one column): 0 total, 0 used by Lua

### Main progression (levels 1-83): quests with an ERROR on a reachable path

| Lvl | Quest | Nation | NPC (Lua) | Problem |
|---:|---:|---|---|---|
| 1 | 0 | both | 18005 (`18005_Bilbor.lua`) | `ITEM_MISSING` L11 `HowmuchItem` item id 910553000 is not in the item table<br>`ITEM_MISSING` L20 `HowmuchItem` item id 910553000 is not in the item table<br>`ITEM_MISSING` L26 `RobItem` item id 910553000 is not in the item table<br>`ITEM_MISSING` L563 `HowmuchItem` item id 800386000 is not in the item table |
| 1 | 837 | both | 29098 (`29098_Naya.lua`) | `EXCHANGE_OUTPUT_ITEM` L33 `RunQuestExchange(240)`: reward item 900355762 (x1) not in item table |
| 1 | 839 | both | 29123 (`29123_CarlosK.lua`) | `ITEM_MISSING` L63 `HowmuchItem` item id 900371000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L56 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table |
| 1 | 840 | both | 29122 (`29122_CarlosE.lua`) | `ITEM_MISSING` L63 `HowmuchItem` item id 900371000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L56 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table |
| 1 | 841 | both | 29123 (`29123_CarlosK.lua`) | `ITEM_MISSING` L99 `HowmuchItem` item id 900375000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L92 `RunQuestExchange(245)`: origin item 900375000 (x1) not in item table |
| 1 | 842 | both | 29122 (`29122_CarlosE.lua`) | `ITEM_MISSING` L99 `HowmuchItem` item id 900375000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L92 `RunQuestExchange(245)`: origin item 900375000 (x1) not in item table |
| 1 | 843 | both | 29123 (`29123_CarlosK.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table |
| 1 | 844 | both | 29122 (`29122_CarlosE.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table |
| 1 | 845 | both | 29123 (`29123_CarlosK.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table |
| 1 | 846 | both | 29122 (`29122_CarlosE.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table |
| 1 | 849 | both | 29123 (`29123_CarlosK.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table |
| 1 | 850 | both | 29122 (`29122_CarlosE.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table |
| 1 | 872 | both | 29138 (`29138_KIFA.lua`) | `ITEM_MISSING` L51 `HowmuchItem` item id 900378000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L44 `RunQuestExchange(2453)`: origin item 900378000 (x5) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L44 `RunQuestExchange(2453)`: origin item 900379000 (x5) not in item table |
| 1 | 887 | both | 29142 (`29142_8th_cake.lua`) | `ITEM_MISSING` L57 `HowmuchItem` item id 508230000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table |
| 1 | 888 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 889 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 890 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 891 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 892 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 893 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 894 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 895 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 896 | both | 29142 (`29142_8th_cake.lua`) | `ITEM_MISSING` L57 `HowmuchItem` item id 508230000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table |
| 1 | 897 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 914 | both | 29171 (`29171_Chung.lua`) | `EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(5000)`: reward item 900421000 (x1) not in item table |
| 1 | 915 | both | 29172 (`29172_Beak.lua`) | `ITEM_MISSING` L41 `HowmuchItem` item id 900427000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(5020)`: origin item 900427000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(5020)`: reward item 900422000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L55 `RunQuestExchange(5020)`: origin item 900427000 (x1) not in item table |
| 1 | 916 | both | 29173 (`29173_HyunE.lua`) | `ITEM_MISSING` L41 `HowmuchItem` item id 900428000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(5002)`: origin item 900428000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(5002)`: reward item 900423000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L55 `RunQuestExchange(5002)`: origin item 900428000 (x1) not in item table |
| 1 | 920 | both | 29174 (`29174_HyunK.lua`) | `ITEM_MISSING` L41 `HowmuchItem` item id 900428000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(5006)`: origin item 900428000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(5006)`: reward item 900423000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L55 `RunQuestExchange(5006)`: origin item 900428000 (x1) not in item table |
| 1 | 924 | both | 29177 (`29177_JooE.lua`) | `ITEM_MISSING` L41 `HowmuchItem` item id 900429000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(5010)`: origin item 900429000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(5010)`: reward item 900424000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L55 `RunQuestExchange(5010)`: origin item 900429000 (x1) not in item table |
| 1 | 925 | both | 29178 (`29178_JooK.lua`) | `ITEM_MISSING` L41 `HowmuchItem` item id 900429000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(5011)`: origin item 900429000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(5011)`: reward item 900424000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L55 `RunQuestExchange(5011)`: origin item 900429000 (x1) not in item table |
| 1 | 929 | both | 29188 (`29188_Ran.lua`) | `ITEM_MISSING` L51 `HowmuchItem` item id 900417000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L65 `RunQuestExchange(5013)`: origin item 900417000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L65 `RunQuestExchange(5013)`: origin item 900418000 (x1) not in item table<br>`EXCHANGE_ORIGIN_ITEM` L65 `RunQuestExchange(5013)`: origin item 900419000 (x1) not in item table |
| 1 | 985 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 986 | both | 29142 (`29142_8th_cake.lua`) | `ITEM_MISSING` L57 `HowmuchItem` item id 508230000 is not in the item table<br>`EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table |
| 1 | 987 | both | 29142 (`29142_8th_cake.lua`) | `EXCHANGE_ORIGIN_ITEM` L24 `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table<br>`EXCHANGE_OUTPUT_ITEM` L24 `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table |
| 1 | 1124 | both | 16085 (`16085_Hamus.lua`) | `ITEM_MISSING` L8 `HowmuchItem` item id 910334000 is not in the item table<br>`ITEM_MISSING` L21 `RobItem` item id 910334000 is not in the item table |
| 1 | 1383 | both | 32530 (`32530_Parpe.lua`) | `ITEM_MISSING` L124 `HowmuchItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L133 `HowmuchItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L137 `RobItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L145 `HowmuchItem` item id 914032852 is not in the item table |
| 1 | 1384 | both | 32530 (`32530_Parpe.lua`) | `ITEM_MISSING` L124 `HowmuchItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L133 `HowmuchItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L137 `RobItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L145 `HowmuchItem` item id 914032852 is not in the item table |
| 1 | 1474 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_ORIGIN_ITEM` L115 `RunQuestExchange(6225)`: origin item 810261000 (x10) not in item table |
| 1 | 1475 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_ORIGIN_ITEM` L127 `RunQuestExchange(6226)`: origin item 810261000 (x2015) not in item table |
| 1 | 1476 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_OUTPUT_ITEM` L139 `RunQuestExchange(6227)`: reward item 810261000 (x30) not in item table |
| 1 | 1506 | both | 19073 (`19073_Shojin.lua`) | `EXCHANGE_NO_OUTPUT` L101 `RunQuestExchange(6230)`: the row has no reward at all (all exchange_item columns empty) - the quest completes and pays nothing<br>`EXCHANGE_NO_OUTPUT` L128 `RunQuestExchange(6230)`: the row has no reward at all (all exchange_item columns empty) - the quest completes and pays nothing |
| 1 | 1531 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_ORIGIN_ITEM` L127 `RunQuestExchange(6226)`: origin item 810261000 (x2015) not in item table |
| 1 | 1666 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_OUTPUT_ITEM` L260 `RunQuestExchange(6825)`: reward item 890227000 (x1) not in item table |
| 1 | 1667 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_OUTPUT_ITEM` L297 `RunQuestExchange(6826)`: reward item 890227000 (x1) not in item table |
| 1 | 1668 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_OUTPUT_ITEM` L334 `RunQuestExchange(6827)`: reward item 931750000 (x1) not in item table |
| 1 | 1669 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_OUTPUT_ITEM` L371 `RunQuestExchange(6828)`: reward item 931750000 (x1) not in item table |
| 1 | 1670 | both | 31506 (`31506_Magmom.lua`) | `EXCHANGE_OUTPUT_ITEM` L408 `RunQuestExchange(6829)`: reward item 931750000 (x1) not in item table |
| 1 | 1695 | both | 16047 (`16047_Moira.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14237 (quest 1695, lvl 1) trigger EVENT 4000 has no handler in `16047_Moira.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 14238 (quest 1695, lvl 1) trigger EVENT 4000 has no handler in `16047_Moira.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 14239 (quest 1695, lvl 1) trigger EVENT 4000 has no handler in `16047_Moira.lua` |
| 1 | 1702 | both | 32530 (`32530_Parpe.lua`) | `ITEM_MISSING` L124 `HowmuchItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L133 `HowmuchItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L137 `RobItem` item id 914022851 is not in the item table<br>`ITEM_MISSING` L145 `HowmuchItem` item id 914032852 is not in the item table |
| 1 | 2601 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14159 (quest 2601, lvl 1) trigger EVENT 600 has no handler in `29235_Krevent.lua` |
| 1 | 6893 | both | 29056 (`29056_Kaira.lua`) | `ITEM_MISSING` L563 `HowmuchItem` item id 800386000 is not in the item table |
| 1 | 6924 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14160 (quest 6924, lvl 1) trigger EVENT 3004 has no handler in `29235_Krevent.lua` |
| 1 | 6925 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14161 (quest 6925, lvl 1) trigger EVENT 4004 has no handler in `29235_Krevent.lua` |
| 1 | 6926 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14162 (quest 6926, lvl 1) trigger EVENT 5004 has no handler in `29235_Krevent.lua` |
| 1 | 6927 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14163 (quest 6927, lvl 1) trigger EVENT 6004 has no handler in `29235_Krevent.lua` |
| 1 | 6928 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14164 (quest 6928, lvl 1) trigger EVENT 7004 has no handler in `29235_Krevent.lua` |
| 1 | 6929 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14165 (quest 6929, lvl 1) trigger EVENT 1031 has no handler in `29235_Krevent.lua` |
| 1 | 6930 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14166 (quest 6930, lvl 1) trigger EVENT 1034 has no handler in `29235_Krevent.lua` |
| 1 | 6931 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14167 (quest 6931, lvl 1) trigger EVENT 1556 has no handler in `29235_Krevent.lua` |
| 1 | 6932 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14168 (quest 6932, lvl 1) trigger EVENT 1041 has no handler in `29235_Krevent.lua` |
| 1 | 6933 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14169 (quest 6933, lvl 1) trigger EVENT 1051 has no handler in `29235_Krevent.lua` |
| 1 | 6934 | both | 29235 (`29235_Krevent.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14170 (quest 6934, lvl 1) trigger EVENT 850 has no handler in `29235_Krevent.lua` |
| 1 | 6937 | both | 13016 (`13016_Keite.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14173 (quest 6937, lvl 1) trigger EVENT 108 has no handler in `13016_Keite.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 14174 (quest 6938, lvl 1) trigger EVENT 108 has no handler in `13016_Keite.lua` |
| 1 | 6938 | both | 13016 (`13016_Keite.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14173 (quest 6937, lvl 1) trigger EVENT 108 has no handler in `13016_Keite.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 14174 (quest 6938, lvl 1) trigger EVENT 108 has no handler in `13016_Keite.lua` |
| 9 | 69 | Karus | 14301 (`14301_Hapa.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 10217 (quest 69, lvl 9) trigger EVENT 554 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 10218 (quest 69, lvl 9) trigger EVENT 554 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 10219 (quest 69, lvl 9) trigger EVENT 555 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 10220 (quest 69, lvl 9) trigger EVENT 565 has no handler in `14301_Hapa.lua` |
| 18 | 95 | both | 19002 (`19002_Burlet.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 533 (quest 95, lvl 18) trigger EVENT 190 has no handler in `19002_Burlet.lua` |
| 23 | 111 | both | 19003 (`13013_Patric.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 8144 (quest 111, lvl 23) trigger EVENT 8630 has no handler in `13013_Patric.lua` |
| 35 | 1633 | both | 16047 (`16047_Moira.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 10839 (quest 1633, lvl 35) trigger EVENT 3000 has no handler in `16047_Moira.lua` |
| 35 | 6873 | both | 16047 (`16047_Moira.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 14110 (quest 6873, lvl 35) trigger EVENT 3002 has no handler in `16047_Moira.lua` |
| 45 | 439 | Karus | 21510 (`21510_Hwargo.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 439 group 1: monster 2 is not a monster in npc_template - kills never count |
| 50 | 491 | both | 14301 (`14301_Hapa.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 2405 (quest 491, lvl 50) trigger EVENT 1565 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2406 (quest 491, lvl 50) trigger EVENT 241 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2407 (quest 491, lvl 50) trigger EVENT 1565 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2605 (quest 514, lvl 60) trigger EVENT 241 has no handler in `14301_Hapa.lua` |
| 51 | 223 | Karus | 24431 (`24431_Hashan.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 223 group 1: monster 11692 is not a monster in npc_template - kills never count |
| 51 | 224 | El Morad | 14431 (`14431_Bess.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 224 group 1: monster 11692 is not a monster in npc_template - kills never count |
| 51 | 459 | Karus | 24431 (`24431_Hashan.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 459 group 1: monster 11692 is not a monster in npc_template - kills never count |
| 51 | 460 | El Morad | 14431 (`14431_Bess.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 460 group 1: monster 11692 is not a monster in npc_template - kills never count |
| 59 | 246 | Karus | 24425 (`24425_Jamed.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 246 group 1: monster 2 is not a monster in npc_template - kills never count |
| 59 | 248 | El Morad | 14425 (`14425_Raek.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 248 group 1: monster 1 is not a monster in npc_template - kills never count |
| 60 | 282 | Karus | 24426 (`24426_Eload.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 282 group 2: monster 2223 is not a monster in npc_template - kills never count |
| 60 | 443 | Karus | 21510 (`21510_Hwargo.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 443 group 1: monster 1 is not a monster in npc_template - kills never count |
| 60 | 514 | both | 14301 (`14301_Hapa.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 2406 (quest 491, lvl 50) trigger EVENT 241 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2603 (quest 514, lvl 60) trigger EVENT 1655 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2604 (quest 514, lvl 60) trigger EVENT 1665 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2605 (quest 514, lvl 60) trigger EVENT 241 has no handler in `14301_Hapa.lua` |
| 61 | 302 | Karus | 24424 (`24424_beda.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 302 group 1: monster 18007 is not a monster in npc_template - kills never count |
| 61 | 303 | both | 14424 (`14424_dela.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 303 group 1: monster 18007 is not a monster in npc_template - kills never count |
| 61 | 1007 | Karus | 24424 (`24424_beda.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 1007 group 1: monster 18007 is not a monster in npc_template - kills never count |
| 61 | 1008 | both | 14424 (`14424_dela.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 1008 group 1: monster 18007 is not a monster in npc_template - kills never count |
| 63 | 515 | both | 14301 (`14301_Hapa.lua`) | `HELPER_EVENT_NO_HANDLER` L0 quest_helper 2406 (quest 491, lvl 50) trigger EVENT 241 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2605 (quest 514, lvl 60) trigger EVENT 241 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2608 (quest 515, lvl 63) trigger EVENT 1755 has no handler in `14301_Hapa.lua`<br>`HELPER_EVENT_NO_HANDLER` L0 quest_helper 2609 (quest 515, lvl 63) trigger EVENT 1765 has no handler in `14301_Hapa.lua` |
| 64 | 315 | Karus | 24424 (`24424_beda.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 315 group 1: monster 17017 is not a monster in npc_template - kills never count |
| 64 | 316 | both | 14424 (`14424_dela.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 316 group 1: monster 17017 is not a monster in npc_template - kills never count |
| 64 | 1013 | Karus | 24424 (`24424_beda.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 1013 group 1: monster 17017 is not a monster in npc_template - kills never count |
| 64 | 1014 | both | 14424 (`14424_dela.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 1014 group 1: monster 17017 is not a monster in npc_template - kills never count |
| 70 | 340 | Karus | 24426 (`24426_Eload.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 340 group 1: monster 2 is not a monster in npc_template - kills never count |
| 70 | 341 | El Morad | 14426 (`14426_Dela.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 341 group 1: monster 1 is not a monster in npc_template - kills never count |
| 70 | 1286 | El Morad | 25006 (`25006_Uranus.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 1286 group 1: monster 1 is not a monster in npc_template - kills never count |
| 70 | 1292 | El Morad | 25066 (`25066_Pina.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 1292 group 1: monster 1 is not a monster in npc_template - kills never count |
| 70 | 1348 | Karus | 25020 (`25020_Pluto.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 1348 group 1: monster 2 is not a monster in npc_template - kills never count |
| 70 | 1352 | Karus | 25164 (`25164_Pina_k.lua`) | `MONSTER_NOT_IN_TEMPLATE` L0 quest 1352 group 1: monster 2 is not a monster in npc_template - kills never count |

### All ERROR findings by script

<details><summary><code>13013_Patric.lua</code> - 1 error(s)</summary>

- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 8144 (quest 111, lvl 23) trigger EVENT 8630 has no handler in `13013_Patric.lua` (quests 111)

</details>

<details><summary><code>13016_Keite.lua</code> - 2 error(s)</summary>

- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14173 (quest 6937, lvl 1) trigger EVENT 108 has no handler in `13016_Keite.lua` (quests 6937, 6938)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14174 (quest 6938, lvl 1) trigger EVENT 108 has no handler in `13016_Keite.lua` (quests 6937, 6938)

</details>

<details><summary><code>14301_Hapa.lua</code> - 21 error(s)</summary>

- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2405 (quest 491, lvl 50) trigger EVENT 1565 has no handler in `14301_Hapa.lua` (quests 491)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2406 (quest 491, lvl 50) trigger EVENT 241 has no handler in `14301_Hapa.lua` (quests 491, 514, 515)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2407 (quest 491, lvl 50) trigger EVENT 1565 has no handler in `14301_Hapa.lua` (quests 491)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2603 (quest 514, lvl 60) trigger EVENT 1655 has no handler in `14301_Hapa.lua` (quests 514)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2604 (quest 514, lvl 60) trigger EVENT 1665 has no handler in `14301_Hapa.lua` (quests 514)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2605 (quest 514, lvl 60) trigger EVENT 241 has no handler in `14301_Hapa.lua` (quests 491, 514, 515)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2606 (quest 514, lvl 60) trigger EVENT 1665 has no handler in `14301_Hapa.lua` (quests 514)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2607 (quest 514, lvl 60) trigger EVENT 1655 has no handler in `14301_Hapa.lua` (quests 514)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2608 (quest 515, lvl 63) trigger EVENT 1755 has no handler in `14301_Hapa.lua` (quests 515)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2609 (quest 515, lvl 63) trigger EVENT 1765 has no handler in `14301_Hapa.lua` (quests 515)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2610 (quest 515, lvl 63) trigger EVENT 241 has no handler in `14301_Hapa.lua` (quests 491, 514, 515)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2611 (quest 515, lvl 63) trigger EVENT 1765 has no handler in `14301_Hapa.lua` (quests 515)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 2612 (quest 515, lvl 63) trigger EVENT 1755 has no handler in `14301_Hapa.lua` (quests 515)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10217 (quest 69, lvl 9) trigger EVENT 554 has no handler in `14301_Hapa.lua` (quests 69)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10218 (quest 69, lvl 9) trigger EVENT 554 has no handler in `14301_Hapa.lua` (quests 69)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10219 (quest 69, lvl 9) trigger EVENT 555 has no handler in `14301_Hapa.lua` (quests 69)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10220 (quest 69, lvl 9) trigger EVENT 565 has no handler in `14301_Hapa.lua` (quests 69)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10220 (quest 69, lvl 9) complete EVENT 559 has no handler in `14301_Hapa.lua` (quests 69)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10221 (quest 69, lvl 9) trigger EVENT 563 has no handler in `14301_Hapa.lua` (quests 69)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10222 (quest 69, lvl 9) trigger EVENT 565 has no handler in `14301_Hapa.lua` (quests 69)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10223 (quest 69, lvl 9) trigger EVENT 555 has no handler in `14301_Hapa.lua` (quests 69)

</details>

<details><summary><code>16047_Moira.lua</code> - 5 error(s)</summary>

- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 10839 (quest 1633, lvl 35) trigger EVENT 3000 has no handler in `16047_Moira.lua` (quests 1633)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14110 (quest 6873, lvl 35) trigger EVENT 3002 has no handler in `16047_Moira.lua` (quests 6873)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14237 (quest 1695, lvl 1) trigger EVENT 4000 has no handler in `16047_Moira.lua` (quests 1695)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14238 (quest 1695, lvl 1) trigger EVENT 4000 has no handler in `16047_Moira.lua` (quests 1695)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14239 (quest 1695, lvl 1) trigger EVENT 4000 has no handler in `16047_Moira.lua` (quests 1695)

</details>

<details><summary><code>19002_Burlet.lua</code> - 1 error(s)</summary>

- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 533 (quest 95, lvl 18) trigger EVENT 190 has no handler in `19002_Burlet.lua` (quests 95)

</details>

<details><summary><code>19073_Shojin.lua</code> - 2 error(s)</summary>

- L101 `EXCHANGE_NO_OUTPUT`: `RunQuestExchange(6230)`: the row has no reward at all (all exchange_item columns empty) - the quest completes and pays nothing (quests 1506)
- L128 `EXCHANGE_NO_OUTPUT`: `RunQuestExchange(6230)`: the row has no reward at all (all exchange_item columns empty) - the quest completes and pays nothing (quests 1506)

</details>

<details><summary><code>25174_Pulcino.lua</code> - 3 error(s)</summary>

- L11 `ITEM_MISSING`: `HowmuchItem` item id 910553000 is not in the item table (quests 0)
- L20 `ITEM_MISSING`: `HowmuchItem` item id 910553000 is not in the item table (quests 0)
- L26 `ITEM_MISSING`: `RobItem` item id 910553000 is not in the item table (quests 0)

</details>

<details><summary><code>29056_Kaira.lua</code> - 1 error(s)</summary>

- L563 `ITEM_MISSING`: `HowmuchItem` item id 800386000 is not in the item table (quests 0, 6893)

</details>

<details><summary><code>29098_Naya.lua</code> - 1 error(s)</summary>

- L33 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(240)`: reward item 900355762 (x1) not in item table (quests 837)

</details>

<details><summary><code>29122_CarlosE.lua</code> - 5 error(s)</summary>

- L24 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table (quests 840, 842, 844, 846, 850)
- L56 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table (quests 840)
- L63 `ITEM_MISSING`: `HowmuchItem` item id 900371000 is not in the item table (quests 840)
- L92 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(245)`: origin item 900375000 (x1) not in item table (quests 842)
- L99 `ITEM_MISSING`: `HowmuchItem` item id 900375000 is not in the item table (quests 842)

</details>

<details><summary><code>29123_CarlosK.lua</code> - 5 error(s)</summary>

- L24 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table (quests 839, 841, 843, 845, 849)
- L56 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(243)`: origin item 900371000 (x1) not in item table (quests 839)
- L63 `ITEM_MISSING`: `HowmuchItem` item id 900371000 is not in the item table (quests 839)
- L92 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(245)`: origin item 900375000 (x1) not in item table (quests 841)
- L99 `ITEM_MISSING`: `HowmuchItem` item id 900375000 is not in the item table (quests 841)

</details>

<details><summary><code>29138_KIFA.lua</code> - 3 error(s)</summary>

- L44 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(2453)`: origin item 900378000 (x5) not in item table (quests 872)
- L44 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(2453)`: origin item 900379000 (x5) not in item table (quests 872)
- L51 `ITEM_MISSING`: `HowmuchItem` item id 900378000 is not in the item table (quests 872)

</details>

<details><summary><code>29142_8th_cake.lua</code> - 9 error(s)</summary>

- L24 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table (quests 887, 888, 889, 890, 891, 892, 893, 894...)
- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table (quests 887, 888, 889, 890, 891, 892, 893, 894...)
- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table (quests 887, 888, 889, 890, 891, 892, 893, 894...)
- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table (quests 887, 888, 889, 890, 891, 892, 893, 894...)
- L50 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(2454)`: origin item 508230000 (x10) not in item table (quests 887, 896, 986)
- L50 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(2454)`: reward item 508231000 (x1) not in item table (quests 887, 896, 986)
- L50 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(2454)`: reward item 508232000 (x1) not in item table (quests 887, 896, 986)
- L50 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(2454)`: reward item 508233000 (x1) not in item table (quests 887, 896, 986)
- L57 `ITEM_MISSING`: `HowmuchItem` item id 508230000 is not in the item table (quests 887, 896, 986)

</details>

<details><summary><code>29171_Chung.lua</code> - 1 error(s)</summary>

- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5000)`: reward item 900421000 (x1) not in item table (quests 914)

</details>

<details><summary><code>29172_Beak.lua</code> - 5 error(s)</summary>

- L24 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5020)`: origin item 900427000 (x1) not in item table (quests 915)
- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5020)`: reward item 900422000 (x1) not in item table (quests 915)
- L41 `ITEM_MISSING`: `HowmuchItem` item id 900427000 is not in the item table (quests 915)
- L55 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5020)`: origin item 900427000 (x1) not in item table (quests 915)
- L55 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5020)`: reward item 900422000 (x1) not in item table (quests 915)

</details>

<details><summary><code>29173_HyunE.lua</code> - 5 error(s)</summary>

- L24 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5002)`: origin item 900428000 (x1) not in item table (quests 916)
- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5002)`: reward item 900423000 (x1) not in item table (quests 916)
- L41 `ITEM_MISSING`: `HowmuchItem` item id 900428000 is not in the item table (quests 916)
- L55 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5002)`: origin item 900428000 (x1) not in item table (quests 916)
- L55 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5002)`: reward item 900423000 (x1) not in item table (quests 916)

</details>

<details><summary><code>29174_HyunK.lua</code> - 5 error(s)</summary>

- L24 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5006)`: origin item 900428000 (x1) not in item table (quests 920)
- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5006)`: reward item 900423000 (x1) not in item table (quests 920)
- L41 `ITEM_MISSING`: `HowmuchItem` item id 900428000 is not in the item table (quests 920)
- L55 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5006)`: origin item 900428000 (x1) not in item table (quests 920)
- L55 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5006)`: reward item 900423000 (x1) not in item table (quests 920)

</details>

<details><summary><code>29177_JooE.lua</code> - 5 error(s)</summary>

- L24 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5010)`: origin item 900429000 (x1) not in item table (quests 924)
- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5010)`: reward item 900424000 (x1) not in item table (quests 924)
- L41 `ITEM_MISSING`: `HowmuchItem` item id 900429000 is not in the item table (quests 924)
- L55 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5010)`: origin item 900429000 (x1) not in item table (quests 924)
- L55 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5010)`: reward item 900424000 (x1) not in item table (quests 924)

</details>

<details><summary><code>29178_JooK.lua</code> - 5 error(s)</summary>

- L24 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5011)`: origin item 900429000 (x1) not in item table (quests 925)
- L24 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5011)`: reward item 900424000 (x1) not in item table (quests 925)
- L41 `ITEM_MISSING`: `HowmuchItem` item id 900429000 is not in the item table (quests 925)
- L55 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5011)`: origin item 900429000 (x1) not in item table (quests 925)
- L55 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(5011)`: reward item 900424000 (x1) not in item table (quests 925)

</details>

<details><summary><code>29188_Ran.lua</code> - 5 error(s)</summary>

- L51 `ITEM_MISSING`: `HowmuchItem` item id 900417000 is not in the item table (quests 929)
- L65 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5013)`: origin item 900417000 (x1) not in item table (quests 929)
- L65 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5013)`: origin item 900418000 (x1) not in item table (quests 929)
- L65 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5013)`: origin item 900419000 (x1) not in item table (quests 929)
- L65 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(5013)`: origin item 900420000 (x1) not in item table (quests 929)

</details>

<details><summary><code>29235_Krevent.lua</code> - 12 error(s)</summary>

- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14159 (quest 2601, lvl 1) trigger EVENT 600 has no handler in `29235_Krevent.lua` (quests 2601)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14160 (quest 6924, lvl 1) trigger EVENT 3004 has no handler in `29235_Krevent.lua` (quests 6924)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14161 (quest 6925, lvl 1) trigger EVENT 4004 has no handler in `29235_Krevent.lua` (quests 6925)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14162 (quest 6926, lvl 1) trigger EVENT 5004 has no handler in `29235_Krevent.lua` (quests 6926)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14163 (quest 6927, lvl 1) trigger EVENT 6004 has no handler in `29235_Krevent.lua` (quests 6927)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14164 (quest 6928, lvl 1) trigger EVENT 7004 has no handler in `29235_Krevent.lua` (quests 6928)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14165 (quest 6929, lvl 1) trigger EVENT 1031 has no handler in `29235_Krevent.lua` (quests 6929)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14166 (quest 6930, lvl 1) trigger EVENT 1034 has no handler in `29235_Krevent.lua` (quests 6930)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14167 (quest 6931, lvl 1) trigger EVENT 1556 has no handler in `29235_Krevent.lua` (quests 6931)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14168 (quest 6932, lvl 1) trigger EVENT 1041 has no handler in `29235_Krevent.lua` (quests 6932)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14169 (quest 6933, lvl 1) trigger EVENT 1051 has no handler in `29235_Krevent.lua` (quests 6933)
- L0 `HELPER_EVENT_NO_HANDLER`: quest_helper 14170 (quest 6934, lvl 1) trigger EVENT 850 has no handler in `29235_Krevent.lua` (quests 6934)

</details>

<details><summary><code>29519_Face.lua</code> - 2 error(s)</summary>

- L8 `ITEM_MISSING`: `HowmuchItem` item id 910334000 is not in the item table (quests 0, 1124)
- L21 `ITEM_MISSING`: `RobItem` item id 910334000 is not in the item table (quests 0, 1124)

</details>

<details><summary><code>29999_Pontus.lua</code> - 2 error(s)</summary>

- L123 `ITEM_MISSING`: `HowmuchItem` item id 700088000 is not in the item table (quests 0)
- L128 `ITEM_MISSING`: `RobItem` item id 700088000 is not in the item table (quests 0)

</details>

<details><summary><code>31506_Magmom.lua</code> - 8 error(s)</summary>

- L115 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(6225)`: origin item 810261000 (x10) not in item table (quests 1474)
- L127 `EXCHANGE_ORIGIN_ITEM`: `RunQuestExchange(6226)`: origin item 810261000 (x2015) not in item table (quests 1475, 1531)
- L139 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(6227)`: reward item 810261000 (x30) not in item table (quests 1476)
- L260 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(6825)`: reward item 890227000 (x1) not in item table (quests 1666)
- L297 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(6826)`: reward item 890227000 (x1) not in item table (quests 1667)
- L334 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(6827)`: reward item 931750000 (x1) not in item table (quests 1668)
- L371 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(6828)`: reward item 931750000 (x1) not in item table (quests 1669)
- L408 `EXCHANGE_OUTPUT_ITEM`: `RunQuestExchange(6829)`: reward item 931750000 (x1) not in item table (quests 1670)

</details>

<details><summary><code>31737_Fullmoon.lua</code> - 10 error(s)</summary>

- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 110 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 111 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 112 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 113 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 114 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 115 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 116 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 117 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 118 has no handler in this script (dead button) (quests 0)
- L24 `BUTTON_NO_HANDLER`: SelectMsg button -> EVENT 119 has no handler in this script (dead button) (quests 0)

</details>

<details><summary><code>32530_Parpe.lua</code> - 39 error(s)</summary>

- L124 `ITEM_MISSING`: `HowmuchItem` item id 914022851 is not in the item table (quests 0, 1383, 1384, 1702)
- L133 `ITEM_MISSING`: `HowmuchItem` item id 914022851 is not in the item table (quests 0, 1383, 1384, 1702)
- L137 `ITEM_MISSING`: `RobItem` item id 914022851 is not in the item table (quests 0, 1383, 1384, 1702)
- L145 `ITEM_MISSING`: `HowmuchItem` item id 914032852 is not in the item table (quests 0, 1383, 1384, 1702)
- L154 `ITEM_MISSING`: `HowmuchItem` item id 914032852 is not in the item table (quests 0, 1383, 1384, 1702)
- L158 `ITEM_MISSING`: `RobItem` item id 914032852 is not in the item table (quests 0, 1383, 1384, 1702)
- L170 `ITEM_MISSING`: `HowmuchItem` item id 399381883 is not in the item table (quests 0, 1383, 1384, 1702)
- L179 `ITEM_MISSING`: `HowmuchItem` item id 399381883 is not in the item table (quests 0, 1383, 1384, 1702)
- L183 `ITEM_MISSING`: `RobItem` item id 399381883 is not in the item table (quests 0, 1383, 1384, 1702)
- L191 `ITEM_MISSING`: `HowmuchItem` item id 399481745 is not in the item table (quests 0, 1383, 1384, 1702)
- L200 `ITEM_MISSING`: `HowmuchItem` item id 399481745 is not in the item table (quests 0, 1383, 1384, 1702)
- L204 `ITEM_MISSING`: `RobItem` item id 399481745 is not in the item table (quests 0, 1383, 1384, 1702)
- L212 `ITEM_MISSING`: `HowmuchItem` item id 399581746 is not in the item table (quests 0, 1383, 1384, 1702)
- L221 `ITEM_MISSING`: `HowmuchItem` item id 399581746 is not in the item table (quests 0, 1383, 1384, 1702)
- L225 `ITEM_MISSING`: `RobItem` item id 399581746 is not in the item table (quests 0, 1383, 1384, 1702)
- L236 `ITEM_MISSING`: `HowmuchItem` item id 399294738 is not in the item table (quests 0, 1383, 1384, 1702)
- L245 `ITEM_MISSING`: `HowmuchItem` item id 399294738 is not in the item table (quests 0, 1383, 1384, 1702)
- L249 `ITEM_MISSING`: `RobItem` item id 399294738 is not in the item table (quests 0, 1383, 1384, 1702)
- L259 `ITEM_MISSING`: `HowmuchItem` item id 399299739 is not in the item table (quests 0, 1383, 1384, 1702)
- L268 `ITEM_MISSING`: `HowmuchItem` item id 399299739 is not in the item table (quests 0, 1383, 1384, 1702)
- L272 `ITEM_MISSING`: `RobItem` item id 399299739 is not in the item table (quests 0, 1383, 1384, 1702)
- L282 `ITEM_MISSING`: `HowmuchItem` item id 399297740 is not in the item table (quests 0, 1383, 1384, 1702)
- L291 `ITEM_MISSING`: `HowmuchItem` item id 399297740 is not in the item table (quests 0, 1383, 1384, 1702)
- L295 `ITEM_MISSING`: `RobItem` item id 399297740 is not in the item table (quests 0, 1383, 1384, 1702)
- L310 `ITEM_MISSING`: `HowmuchItem` item id 504122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L319 `ITEM_MISSING`: `HowmuchItem` item id 504122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L327 `ITEM_MISSING`: `RobItem` item id 504122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L335 `ITEM_MISSING`: `HowmuchItem` item id 505122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L344 `ITEM_MISSING`: `HowmuchItem` item id 505122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L352 `ITEM_MISSING`: `RobItem` item id 505122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L360 `ITEM_MISSING`: `HowmuchItem` item id 506122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L369 `ITEM_MISSING`: `HowmuchItem` item id 506122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L377 `ITEM_MISSING`: `RobItem` item id 506122000 is not in the item table (quests 0, 1383, 1384, 1702)
- L390 `ITEM_MISSING`: `HowmuchItem` item id 901387000 is not in the item table (quests 0, 1383, 1384, 1702)
- L398 `ITEM_MISSING`: `RobItem` item id 901387000 is not in the item table (quests 0, 1383, 1384, 1702)
- L405 `ITEM_MISSING`: `HowmuchItem` item id 902387000 is not in the item table (quests 0, 1383, 1384, 1702)
- L413 `ITEM_MISSING`: `RobItem` item id 902387000 is not in the item table (quests 0, 1383, 1384, 1702)
- L420 `ITEM_MISSING`: `HowmuchItem` item id 903387000 is not in the item table (quests 0, 1383, 1384, 1702)
- L428 `ITEM_MISSING`: `RobItem` item id 903387000 is not in the item table (quests 0, 1383, 1384, 1702)

</details>

<details><summary><code>quest_monster</code> - 47 error(s)</summary>

- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 223 group 1: monster 11692 is not a monster in npc_template - kills never count (quests 223)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 224 group 1: monster 11692 is not a monster in npc_template - kills never count (quests 224)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 246 group 1: monster 2 is not a monster in npc_template - kills never count (quests 246)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 248 group 1: monster 1 is not a monster in npc_template - kills never count (quests 248)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 282 group 2: monster 2223 is not a monster in npc_template - kills never count (quests 282)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 302 group 1: monster 18007 is not a monster in npc_template - kills never count (quests 302)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 303 group 1: monster 18007 is not a monster in npc_template - kills never count (quests 303)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 315 group 1: monster 17017 is not a monster in npc_template - kills never count (quests 315)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 316 group 1: monster 17017 is not a monster in npc_template - kills never count (quests 316)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 340 group 1: monster 2 is not a monster in npc_template - kills never count (quests 340)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 341 group 1: monster 1 is not a monster in npc_template - kills never count (quests 341)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 439 group 1: monster 2 is not a monster in npc_template - kills never count (quests 439)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 443 group 1: monster 1 is not a monster in npc_template - kills never count (quests 443)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 459 group 1: monster 11692 is not a monster in npc_template - kills never count (quests 459)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 460 group 1: monster 11692 is not a monster in npc_template - kills never count (quests 460)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1007 group 1: monster 18007 is not a monster in npc_template - kills never count (quests 1007)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1008 group 1: monster 18007 is not a monster in npc_template - kills never count (quests 1008)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1013 group 1: monster 17017 is not a monster in npc_template - kills never count (quests 1013)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1014 group 1: monster 17017 is not a monster in npc_template - kills never count (quests 1014)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1286 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1286)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1292 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1292)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1348 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1348)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1352 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1352)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1463 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1463)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1467 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1467)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1468 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1468)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1468 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1468)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1469 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1469)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1469 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1469)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1470 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1470)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1470 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1470)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1471 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1471)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1471 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1471)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1472 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1472)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1472 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1472)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1473 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1473)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1473 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1473)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1601 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1601)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1602 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1602)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1603 group 1: monster 1 is not a monster in npc_template - kills never count (quests 1603)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1604 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1604)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1605 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1605)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 1606 group 1: monster 2 is not a monster in npc_template - kills never count (quests 1606)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 2300 group 1: monster 2 is not a monster in npc_template - kills never count (quests 2300)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 2300 group 1: monster 61 is not a monster in npc_template - kills never count (quests 2300)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 2301 group 1: monster 1 is not a monster in npc_template - kills never count (quests 2301)
- L0 `MONSTER_NOT_IN_TEMPLATE`: quest 2301 group 1: monster 61 is not a monster in npc_template - kills never count (quests 2301)

</details>

### Lua files no quest_helper row points to (52)

`14201_SkakySPELLI.lua`, `14202_ClarenceSPELLI.lua`, `14203_DreakSPELLI.lua`, `14204_MinervaSPELLI.lua`, `14410_Chef.lua`, `15424_dela.lua`, `15438_Beldan.lua`, `15439_Agata.lua`, `15725_thethyke_kill.lua`, `15895_thethyke_kill.lua`, `18030_Envoy.lua`, `1881_NTSJOB.lua`, `19073_Shozin.lua`, `19074_Shojin.lua`, `23500_ZindanWarExit.lua`, `23523_shield.lua`, `24201_SkakySPELLLI.lua`, `24202_ClarenceSPELLI.lua`, `24203_DreakSPELLI.lua`, `24204_MinervaSPELLI.lua`, `25075_2_Prison1.lua`, `25076_2_Prison2.lua`, `25077_2_Prison3.lua`, `25078_2_Prison4.lua`, `25079_2_Prison5.lua`, `25080_1_Prison1.lua`, `25081_1_Prison2.lua`, `25082_1_Prison3.lua`, `25083_1_Prison4.lua`, `25084_1_Prison5.lua`, `25282_thethyke_prison_kill.lua`, `25283_thethyke_prison_kill.lua`, `26005_Earth.lua`, `26276_Rily.lua`, `29447_Voyeur.lua`, `29448_Voyeur.lua`, `29449_Voyeur.lua`, `29450_Voyeur.lua`, `29451_Voyeur.lua`, `29452_Voyeur.lua`, `29453_Voyeur.lua`, `29454_Voyeur.lua`, `29455_Voyeur.lua`, `29456_Voyeur.lua`, `29457_Voyeur.lua`, `29458_Voyeur.lua`, `29459_Voyeur.lua`, `29460_Voyeur.lua`, `29461_Voyeur.lua`, `29462_Voyeur.lua`, `30001_Akara.lua`, `31720_(JAPKO) LvL Jump.lua`

<!-- AUTO:END -->
