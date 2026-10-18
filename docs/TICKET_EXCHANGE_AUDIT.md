# TICKET_EXCHANGE.tbl audit (Kaira "Ticket Exchange Use")

Scope: every row of the shipped client table `Data\TICKET_EXCHANGE.tbl` (SHA-256 `2f80ac8b9ecf0798be18390adc75096f429da19dcdaa44b562713a64c9b80430`,
the hash pinned in `NTT2625-d3d9proxy/manifest_hashes.h`) through the whole server flow. 330 tickets, 930 ticket->reward pairs.
`docs/data/TICKET_EXCHANGE.tbl` was a stale copy (324 tickets / 906 pairs) and has been replaced by the shipped file; `client_patch/` needs no change.

## 1. Table format (decoded from the 2625 client)

`CN3TableBase<_TABLE_TIKET_EXCHANGE>` (loader `sub_523450`, global `dword_1114564`, UI `CUITiketExchange`, `ui\re_TiketExchange.uif`).
62 columns: `c0` ticket item id (u32, key) | `c1` reward lifetime in **hours** (u32, 0 = permanent) | 30 x (`selector` u8, `reward item id` u32), 0 item = unused slot.

Selector, from the client list builder `sub_722740`/`sub_7A5060` and client texts 44849-44851:

| selector | meaning | client text when nothing is listable |
|---|---|---|
| 0 | every character, **including** Portu/Kurian | - |
| 1 | Karus characters only | 44850 "Only Karus characters can use this." |
| 2 | El Morad characters only | 44849 "Only El Morad characters can use this." |
| 3 | every class **except** Portu/Kurian (classes 113-115 / 213-215) | 44851 "Portu/Kurian characters cannot use this." |

So a row such as Minerva Helmet Certificate lists the normal-class helmets with selector 3 and the Portu helmets with selector 0.

## 2. Flow

1. Kaira (`29056_Kaira.lua`, quest_helper trigger 100) menu entry `45573 "Ticket Exchange Use"` -> event 900 -> `SelectMsg(UID, 73, ...)`.
   Flag 73 (0x49) makes the client call `sub_724270` and open `CUITiketExchange` (verified in the 2625 decompile; `select_msg.rs` forwards the flag unchanged).
2. The player drags a ticket from the bag onto the window; the client looks the ticket up in the tbl, filters the 30 slots by nation / Portu class (above) and lists the rewards.
3. Confirm sends `WIZ_NPC_EVENT (0x20)`: `[u8 6][u32 ticket item][u32 reward item]` (9 bytes).
4. `client_event.rs::handle_ticket_exchange` validates, consumes one ticket, grants one reward and answers `[u8 6][u8 status]`.
   Client (`sub_80DC30` case 6): 1 = silent success; 2 "Your inventory is full."; 3 "Invalid item."; 4 "You cannot purchase this item using exchange voucher"; 5 "System Error".

## 3. Defects found and fixed

| # | Defect | Effect | Fix |
|---|---|---|---|
| 1 | Server rules mirrored an old copy of the tbl: 6 tickets (808111000-808116000, Holy Knight Wings coupons, 24 pairs) had no rules | exchange answered "Invalid item" | migration upserts all 930 pairs |
| 2 | 14 ticket ids and 8 reward ids have no row in the server `item` table (server item table predates the 2625 `Item_Org_us.tbl`) | ticket cannot exist in a bag / `give_item` fails -> status 5 (ticket restored, nothing granted) | migration creates the 22 rows from `Item_Org_us`/`Item_Ext_*_us` |
| 3 | Status codes were mapped to the wrong client strings (inventory-full sent 4 = "cannot purchase using voucher", missing ticket sent 2 = "inventory is full") | misleading messages | mapped to the real client texts (see flow) |
| 4 | Selector 3 treated as "everybody" | Portu/Kurian could take non-Portu rewards with a modified client | selector 3 rejects classes 113-115/213-215 (status 4) |
| 5 | Ticket was consumed with `rob_item`, which for `kind 255` decrements **durability**, not the stack | ticket `399289000` (EXP 100% recover scroll charging ticket, durability 10) survived 9 of 10 exchanges = repeatable reward | `take_one_item` (stack/slot based) |
| 6 | Duration `hours` was rounded up to whole days | rows with 1h/2h/4h (810506000, 810639000, 900561000, 914052000) granted 1 day | exact seconds (`give_item_with_expiry_secs`) |
| 7 | Room check ignored the slot/weight that the consumed ticket frees | a full bag could not swap a single ticket for its reward | `ticket_exchange_has_room` evaluates the post-consumption state under one session lock |
| 8 | No busy-state guard | exchange possible while dead / trading / merchant / mining / fishing | same guard as an NPC click (status 3) |
| 9 | Only the reward was audit-logged | ticket consumption untraceable | `log_rob_item` + `log_give_item` |

Also checked and found fine: all 930 rewards and 330 tickets resolve in the client tables (`Item_Org_us` + `Item_Ext_*`, `_tk` fallback for ext 43/44), no duplicate pairs, no ticket is itself a reward (no chains),
every reward is non-stackable (`countable = 0`, so one reward = one free slot), no `countable = 2` reward, rewards of timed rows have a lifetime, anti-dupe order is consume -> grant -> restore ticket on failure with no `await` in between.

Newly seeded items follow the existing convention `race = 20` (server "untradeable" marker, `RACE_UNTRADEABLE`) for timed rewards (697 of 732 already-present timed rewards have it) and for tickets named Non-tradable/Untradable.

## 4. Row-by-row result

Columns: **tbl hours** = reward lifetime; **rewards** = number of reward slots; **sel** = selector set (0 all, 1 Karus, 2 El Morad, 3 non-Portu);
**before** = state of the live server before this change; **after** = state with this change + migration `20261018210000_ticket_exchange_full_sync.sql` applied.

Summary: 16 of 330 tickets could not (fully) work before for a data reason (6 had no server rules, 14 ticket item ids and 8 reward item ids had no `item` row); the other 314 worked end-to-end apart from the generic defects of section 3. All 330 are OK after.
Rows flagged `Portu restriction not enforced` were functionally working but only trusted the client.

| # | ticket id | ticket name | hrs | rewards | sel | before | after |
|---|---|---|---|---|---|---|---|
| 0 | 508056000 | Minerva Helmet Certificate | 1440 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 1 | 508057000 | Minerva Armor Certificate | 1440 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 2 | 508058000 | Minerva Helmet Certificate (30 days) | 720 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 3 | 508059000 | Minerva Armor Certificate (30 days) | 720 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 4 | 508060000 | Pathos' Glove Certificate (30 days) | 720 | 8 | all | OK | OK |
| 5 | 508116000 | Yeniceri Helmet Vouchers | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 6 | 508117000 | Yeniceri armor Vouchers | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 7 | 508118000 | Yeniceri Helmet Vouchers (30 Day) | 720 | 3 | non-Portu | Portu restriction not enforced | OK |
| 8 | 508119000 | Yeniceri armor Vouchers (30 Day) | 720 | 3 | non-Portu | Portu restriction not enforced | OK |
| 9 | 508121000 | Golden Fishing Rod Voucher | 720 | 1 | all | OK | OK |
| 10 | 508122000 | Golden Pickaxe Voucher | 720 | 1 | all | OK | OK |
| 11 | 800170000 | Valkyrie Helmet Certificate | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 12 | 800180000 | Valkyrie Armor Certificate | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 13 | 800230000 | Gryphon's Helmet Certificate | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 14 | 800240000 | Gryphon's armor Certificate | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 15 | 800250000 | Certificate of Pathos' Glove | 720 | 8 | all | OK | OK |
| 16 | 800260000 | Bahamut Helmet Certificate | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 17 | 800270000 | Bahamut Armor Certificate | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 18 | 800290000 | Noah Bahamut Armor Certificate | 1440 | 3 | all | OK | OK |
| 19 | 800380000 | Noah Bahamut Helmet Certificate | 1440 | 3 | all | OK | OK |
| 20 | 800440000 | Voucher for Magic Bag | 720 | 1 | all | OK | OK |
| 21 | 800450000 | Automatic Pet Looting Voucher | 720 | 1 | all | OK | OK |
| 22 | 800451000 | Helmet of Wrath Exchange Certificate | 720 | 1 | all | OK | OK |
| 23 | 810072000 | Magic Bag Exchange Coupon | 720 | 1 | all | OK | OK |
| 24 | 810111000 | Pathos' Glove Certificate (30 days) | 720 | 8 | all | OK | OK |
| 25 | 810112000 | Automatic Looting Exchange Coupon (30 day) | 720 | 1 | all | OK | OK |
| 26 | 810115000 | HP Maestro Voucher | 720 | 1 | all | OK | OK |
| 27 | 810116000 | MP Maestro Voucher | 720 | 1 | all | OK | OK |
| 28 | 810162000 | Kaisyan's trade certified coupon  | 720 | 1 | all | OK | OK |
| 29 | 810163000 | Menissiah's official list coupon | 720 | 1 | all | OK | OK |
| 30 | 810164000 | Wing of Dragon Exchange Coupon | 720 | 8 | Karus/El Morad | OK | OK |
| 31 | 810170000 | Noah Valkyrie Helmet Certificate | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 32 | 810180000 | Noah Valkyrie Armor Certificate | 1440 | 3 | non-Portu | Portu restriction not enforced | OK |
| 33 | 810240000 | Noah Gryphon's armor Certificate | 1440 | 3 | all | OK | OK |
| 34 | 810250000 | Noah Pathos's Glove Certificate | 720 | 8 | all | OK | OK |
| 35 | 810251000 | Battle Hero’s Wing Exchange Coupon (15day) | 360 | 2 | Karus/El Morad | OK | OK |
| 36 | 810344000 | Battle Hero's Wing Exchange Coupon (30day) | 720 | 4 | Karus/El Morad | OK | OK |
| 37 | 810350000 | GNGWC Emblem Exchange Coupon | 360 | 1 | all | OK | OK |
| 38 | 810352000 | GNGWC Winning Emblem Exchange Coupon | 720 | 1 | all | OK | OK |
| 39 | 810452000 | [Reward for Maintenance] Valkyrie Helmet Certificate (1 day) | 24 | 3 | non-Portu | Portu restriction not enforced | OK |
| 40 | 810453000 | [Reward for Maintenance] Bahamut Helmet Certificate (1 day) | 24 | 3 | non-Portu | Portu restriction not enforced | OK |
| 41 | 810454000 | [Reward for Maintenance] Clear Minerva Helmet (1 day) | 24 | 3 | all | OK | OK |
| 42 | 810455000 | [Reward for Maintenance] Griffon Helmet Certificate (1 day) | 24 | 3 | non-Portu | Portu restriction not enforced | OK |
| 43 | 810456000 | [Reward for Maintenance] Yeniceri Helmet Certificate (1 day) | 24 | 3 | non-Portu | Portu restriction not enforced | OK |
| 44 | 810457000 | [Reward for Maintenance] Valkyrie Armor Top Certiciate (1day) | 24 | 3 | non-Portu | Portu restriction not enforced | OK |
| 45 | 810458000 | [Reward for Maintenance] Bahamut Armor Top Certificate (1 day) | 24 | 3 | non-Portu | Portu restriction not enforced | OK |
| 46 | 810459000 | [Reward for Maintenance] Minerva Armor Top Certificate (1 day) | 24 | 3 | all | OK | OK |
| 47 | 810460000 | [Reward for Maintenance] Griffon Armor Top Certificate (1 day) | 24 | 3 | non-Portu | Portu restriction not enforced | OK |
| 48 | 810461000 | [Reward for Maintenance] Yeniceri Pauldron Certifficate (1 day) | 24 | 3 | all | OK | OK |
| 49 | 810462000 | [Reward for Maintenance] Pathos Glove Certificate (1 day) | 24 | 8 | all | OK | OK |
| 50 | 810463000 | [Reward for Maintenance] Gold Dragon Wing Certificate (1 day) | 24 | 8 | Karus/El Morad | OK | OK |
| 51 | 810464000 | [Reward for Maintenance] Red Dragon Wing Certificate (1 day) | 24 | 8 | Karus/El Morad | OK | OK |
| 52 | 810465000 | [Reward for Maintenance] Dryad Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 53 | 810466000 | [Reward for Maintenance] Oreads Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 54 | 810467000 | [Reward for Maintenance] Alseids Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 55 | 810468000 | [Reward for Maintenance] Strength Recovery Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 56 | 810469000 | [Reward for Maintenance] Intelligence Recovery Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 57 | 810470000 | [Reward for Maintenance] Automatic LootingExchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 58 | 810471000 | [Reward for Maintenance] Magic Bag Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 59 | 810472000 | [Reward for Maintenance] Golden Fishing Rod Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 60 | 810473000 | [Reward for Maintenance] Golden Pickaxe Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 61 | 810474000 | [Reward for Maintenance] Kaishan's Trade Certificate Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 62 | 810475000 | [Reward for Maintenance] Mennissiah Trading Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 63 | 810502000 | Alencia Wing Exchange Coupon -Blue (30day) | 720 | 4 | all | OK | OK |
| 64 | 810503000 | Alencia Wing Exchange Coupon -Blue (15day) | 360 | 4 | all | OK | OK |
| 65 | 810504000 | Alencia Wing Exchange Coupon -Blue (7day) | 168 | 4 | all | OK | OK |
| 66 | 810505000 | Alencia Wing Exchange Coupon -Blue (1day) | 24 | 4 | all | OK | OK |
| 67 | 810506000 | Alencia Wing Exchange Coupon -Blue (4Hour) | 4 | 4 | all | 4h duration rounded up to 1 day | OK |
| 68 | 810507000 | Alencia Wing Exchange Coupon - Red (30day) | 720 | 4 | all | OK | OK |
| 69 | 810511000 | Alencia Wing Exchange Coupon -Blue (30Days) | 720 | 4 | all | OK | OK |
| 70 | 810630000 | Voucher for Magic Bag | 360 | 1 | all | OK | OK |
| 71 | 810639000 | Wings of Hellfire Dragon Exchange Voucher (2 hours) | 2 | 4 | all | 2h duration rounded up to 1 day | OK |
| 72 | 810640000 | Wings of Hellfire Dragon Exchange Voucher (15 days) | 360 | 4 | all | OK | OK |
| 73 | 810641000 | Wings of Hellfire Dragon Exchange Voucher (30 days) | 720 | 4 | all | OK | OK |
| 74 | 810642000 | Wings of Hellfire Dragon Exchange Voucher (90 days) | 2160 | 4 | all | OK | OK |
| 75 | 810643000 | Wings of Hellfire Dragon Exchange Voucher (180 days) | 4320 | 4 | all | OK | OK |
| 76 | 810672000 | Wings of Hellfire Dragon Exchange Voucher | 720 | 4 | all | OK | OK |
| 77 | 814029000 | Nimbus Tattoo Exchange Voucher (15 Days) | 360 | 1 | all | OK | OK |
| 78 | 931705000 | Nimbus Tattoo Exchange Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 79 | 810715000 | Cupid's Wing Exchange Coupon | 720 | 4 | all | OK | OK |
| 80 | 810871000 | Oreads Voucher (7 day) | 168 | 1 | all | OK | OK |
| 81 | 810913000 | Minerva Helm Voucher (30 Days) - Limited | 720 | 8 | all/non-Portu | 2 reward item(s) missing in DB; Portu restriction not enforced | OK |
| 82 | 810929000 | Nimbus Tattoo Exchange Voucher (30 Day) | 720 | 1 | all | OK | OK |
| 83 | 978127000 | Nimbus Tattoo Exchange Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 84 | 978128000 | Nimbus Tattoo Exchange Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 85 | 810713000 | Lunar Tattoo Exchange Voucher (15 Day) | 360 | 1 | all | OK | OK |
| 86 | 810969000 | Minerva Armor Voucher (30 Days) - Limited | 720 | 8 | all/non-Portu | 2 reward item(s) missing in DB; Portu restriction not enforced | OK |
| 87 | 810998000 | Pathos' Glove Voucher (30 Days) Limited Edition | 720 | 8 | all | OK | OK |
| 88 | 811073000 | Special Nereids (Defense) Voucher - Non-tradable | 720 | 1 | all | OK | OK |
| 89 | 811074000 | Special Nereids (Attack) Voucher - Non-tradable | 720 | 1 | all | OK | OK |
| 90 | 811090000 | User Buff Pet Scroll Exchange Voucher - HP increase (1 day) | 24 | 1 | all | OK | OK |
| 91 | 811091000 | User Buff Pet Scroll Exchange Voucher - HP increase (3 days) | 72 | 1 | all | OK | OK |
| 92 | 811092000 | User Buff Pet Scroll Exchange Voucher - HP increase (7 days) | 168 | 1 | all | OK | OK |
| 93 | 811093000 | User Buff Pet Scroll Exchange Voucher - HP increase (15 days) | 360 | 1 | all | OK | OK |
| 94 | 811094000 | User Buff Pet Scroll Exchange Voucher - HP increase (30 days) | 720 | 1 | all | OK | OK |
| 95 | 811095000 | User Buff Pet Scroll Exchange Voucher - Defense increase (1 day) | 24 | 1 | all | OK | OK |
| 96 | 811096000 | User Buff Pet Scroll Exchange Voucher - Defense increase (3 days) | 72 | 1 | all | OK | OK |
| 97 | 811097000 | User Buff Pet Scroll Exchange Voucher - Defense increase (7 days) | 168 | 1 | all | OK | OK |
| 98 | 811098000 | User Buff Pet Scroll Exchange Voucher - Defense increase (15 days) | 360 | 1 | all | OK | OK |
| 99 | 811099000 | User Buff Pet Scroll Exchange Voucher - Defense increase (30 days) | 720 | 1 | all | OK | OK |
| 100 | 811101000 | User Buff Pet Scroll Exchange Voucher - Attack increase (1 day) | 24 | 1 | all | OK | OK |
| 101 | 811102000 | User Buff Pet Scroll Exchange Voucher - Attack increase (3 days) | 72 | 1 | all | OK | OK |
| 102 | 811103000 | User Buff Pet Scroll Exchange Voucher - Attack increase (7 days) | 168 | 1 | all | OK | OK |
| 103 | 811104000 | User Buff Pet Scroll Exchange Voucher - Attack increase (15 days) | 360 | 1 | all | OK | OK |
| 104 | 811105000 | User Buff Pet Scroll Exchange Voucher - Attack increase (30 days) | 720 | 1 | all | OK | OK |
| 105 | 811135000 | Special Nereids (Defense) Voucher | 720 | 1 | all | OK | OK |
| 106 | 811136000 | Special Nereids (Attack) Voucher | 720 | 1 | all | OK | OK |
| 107 | 931706000 | Lunar Tattoo Exchange Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 108 | 814038000 | Oreads Voucher (1 Days) | 24 | 1 | all | OK | OK |
| 109 | 814039000 | Oreads Voucher (30 Days) | 720 | 1 | all | OK | OK |
| 110 | 814661000 | Nereids Voucher | 720 | 1 | all | OK | OK |
| 111 | 810927000 | Lunar Tattoo Exchange Voucher (30 Day) | 720 | 1 | all | OK | OK |
| 112 | 900120000 | Minerva Helmet Certificate (30 Days) | 720 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 113 | 900121000 | Minerva Armor Certificate (30 Days) | 720 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 114 | 900122000 | Pathos' Glove Certificate (30 Days) | 720 | 8 | all | OK | OK |
| 115 | 900188000 | Auto Looting Exchange Coupon | 168 | 1 | all | OK | OK |
| 116 | 900385000 | Dryad Voucher | 720 | 1 | all | OK | OK |
| 117 | 900386000 | Oreads Voucher | 720 | 1 | all | OK | OK |
| 118 | 900387000 | Alseids coupon | 720 | 1 | all | OK | OK |
| 119 | 900561000 | [Knight Online 10 year Emblem] | 4 | 1 | all | 4h duration rounded up to 1 day | OK |
| 120 | 900580000 | [10 year Emblem] Exchange Coupon | 168 | 1 | all | OK | OK |
| 121 | 900581000 | Chaos Wings Exchange Coupon | 168 | 4 | all | OK | OK |
| 122 | 900582000 | Battle Hero Wing Exchange Coupon (7day) | 168 | 4 | Karus/El Morad | OK | OK |
| 123 | 900593000 | Knight Chaos-Day Wing Coupon (15 days) | 360 | 4 | all | OK | OK |
| 124 | 900594000 | WAR emblem Vouchers | 720 | 1 | all | OK | OK |
| 125 | 900596000 | WAR Championship emblem Vouchers | 720 | 1 | all | OK | OK |
| 126 | 900676000 | OTP Emblem Exchange coupon | 720 | 1 | all | OK | OK |
| 127 | 900700000 | Wing of Neophyte Certificate | 360 | 2 | Karus/El Morad | OK | OK |
| 128 | 900703000 | Guardian Emblem Voucher | 720 | 1 | all | OK | OK |
| 129 | 900706000 | Wing of Dragon Exchange Coupon | 720 | 8 | Karus/El Morad | OK | OK |
| 130 | 900710000 | [Knight Online 11th Anniversary Emblem] | 168 | 4 | all | OK | OK |
| 131 | 900740000 | WAR emblem Vouchers(7 Day) | 168 | 1 | all | OK | OK |
| 132 | 900750000 | Wing of Neophyte Certificate(15 Day) | 360 | 2 | Karus/El Morad | OK | OK |
| 133 | 900773000 | Voucher for Magic Bag | 720 | 1 | all | OK | OK |
| 134 | 900774000 | Automatic Looting Exchange Coupon (30 day) | 720 | 1 | all | OK | OK |
| 135 | 900775000 | Wing of Dragon Exchange Coupon | 720 | 8 | Karus/El Morad | OK | OK |
| 136 | 910080000 | Voucher for Magic Bag | 720 | 1 | all | OK | OK |
| 137 | 910920000 | Alencia Wing Exchange Coupon -Red (7 Day) | 168 | 4 | all | ticket item missing in DB | OK |
| 138 | 910925000 | Lunar Tattoo Exchange Voucher (3 Day) | 72 | 1 | all | OK | OK |
| 139 | 910926000 | Lunar Tattoo Exchange Voucher (7 Day) | 168 | 1 | all | OK | OK |
| 140 | 910935000 | [Knight Online 12th Anniversary Emblem] Exchange Voucher (1 days) | 24 | 1 | all | OK | OK |
| 141 | 910936000 | [Knight Online 12th Anniversary Emblem] Exchange Voucher (7 days) | 168 | 1 | all | OK | OK |
| 142 | 910937000 | [Knight Online 12th Anniversary Emblem] Exchange Voucher | 720 | 1 | all | OK | OK |
| 143 | 910938000 | Wings of Hellfire Dragon Exchange Voucher (1 days) | 24 | 4 | all | OK | OK |
| 144 | 910939000 | Wings of Hellfire Dragon Exchange Voucher (7 days) | 168 | 4 | all | OK | OK |
| 145 | 910940000 | Wings of Hellfire Dragon Exchange Voucher (30 Day) | 720 | 4 | all | ticket item missing in DB | OK |
| 146 | 910947000 | Magic Bag Exchange Coupon (7 day) | 168 | 1 | all | OK | OK |
| 147 | 810686000 | Stellar Tattoo Exchange Voucher (15 days) | 360 | 1 | all | OK | OK |
| 148 | 914024000 | Alencia Wing Exchange Coupon -Red (15Days) | 360 | 4 | all | OK | OK |
| 149 | 914026000 | Magic Bag Exchange Coupon (15 Days) | 360 | 1 | all | OK | OK |
| 150 | 914042000 | [13th Anniversary Emblem] Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 151 | 914043000 | [13th Anniversary Emblem] Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 152 | 914044000 | [13th Anniversary Emblem] Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 153 | 914045000 | [13th Anniversary Emblem] Voucher (15 Days) | 360 | 1 | all | OK | OK |
| 154 | 914046000 | [13th Anniversary Emblem] Voucher (30 Days) | 720 | 1 | all | OK | OK |
| 155 | 914052000 | Bounty Emblem Voucher | 1 | 2 | all | 1h duration rounded up to 1 day | OK |
| 156 | 930521000 | Oreads Voucher (7 day) | 168 | 1 | all | OK | OK |
| 157 | 914009000 | Stellar Tattoo Exchange Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 158 | 931686000 | Rebirth Emblem Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 159 | 931688000 | [14th Anniversary Emblem] Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 160 | 931689000 | [14th Anniversary Emblem] Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 161 | 931690000 | [14th Anniversary Emblem] Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 162 | 931691000 | [14th Anniversary Emblem] Voucher (15 Days) | 360 | 1 | all | OK | OK |
| 163 | 931692000 | [14th Anniversary Emblem] Voucher (30 Days) | 720 | 1 | all | OK | OK |
| 164 | 810928000 | Stellar Tattoo Exchange Voucher (30 Day) | 720 | 1 | all | OK | OK |
| 165 | 931703000 | Battle Hero Wing Exchange Coupon (30 Days) | 720 | 4 | Karus/El Morad | OK | OK |
| 166 | 978018000 | Stellar Tattoo Exchange Voucher (3 days) | 72 | 1 | all | ticket item missing in DB | OK |
| 167 | 931666000 | Stellar Tattoo Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 168 | 931709000 | [Full Moon Emblem] Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 169 | 931710000 | [Full Moon Emblem] Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 170 | 931711000 | [Full Moon Emblem] Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 171 | 931712000 | [Full Moon Emblem] Voucher (15 Days) | 360 | 1 | all | OK | OK |
| 172 | 931713000 | [Full Moon Emblem] Voucher (30 Days) | 720 | 1 | all | OK | OK |
| 173 | 931720000 | [HopeGuard Emblem] Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 174 | 931721000 | [HopeGuard Emblem] Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 175 | 931722000 | [HopeGuard Emblem] Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 176 | 931723000 | [HopeGuard Emblem] Voucher (15 Days) | 360 | 1 | all | OK | OK |
| 177 | 931724000 | [HopeGuard Emblem] Voucher (30 Days) | 720 | 1 | all | OK | OK |
| 178 | 931730000 | [Event Emblem] Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 179 | 931731000 | [Event Emblem] Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 180 | 931732000 | [Event Emblem] Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 181 | 931733000 | [Event Emblem] Voucher (15 Days) | 360 | 1 | all | OK | OK |
| 182 | 931734000 | [Event Emblem] Voucher (30 Days) | 720 | 1 | all | OK | OK |
| 183 | 931743000 | [Empires Champion Emblem] Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 184 | 931744000 | [Empires Champion Emblem] Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 185 | 931745000 | [Empires Champion Emblem] Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 186 | 931746000 | [Empires Champion Emblem] Voucher (15 Days) | 360 | 1 | all | OK | OK |
| 187 | 931747000 | [Empires Champion Emblem] Voucher (30 Days) | 720 | 1 | all | OK | OK |
| 188 | 978107000 | War Tattoo Voucher (15Days) | 360 | 1 | all | OK | OK |
| 189 | 931753000 | [15th Anniversary Emblem] Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 190 | 931754000 | [15th Anniversary Emblem] Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 191 | 931755000 | [15th Anniversary Emblem] Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 192 | 931756000 | [15th Anniversary Emblem] Voucher (15 Days) | 360 | 1 | all | OK | OK |
| 193 | 931757000 | [15th Anniversary Emblem] Voucher (30 Days) | 720 | 1 | all | OK | OK |
| 194 | 931775000 | [Anniversary emblem]Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 195 | 931776000 | [Anniversary emblem]Voucher (3 Day) | 72 | 1 | all | OK | OK |
| 196 | 931777000 | [Anniversary emblem]Voucher (7 Day) | 168 | 1 | all | OK | OK |
| 197 | 931778000 | [Anniversary emblem]Voucher (15 Day) | 360 | 1 | all | OK | OK |
| 198 | 931779000 | [Anniversary emblem]Voucher (30 Day) | 720 | 1 | all | OK | OK |
| 199 | 931782000 | [Reward for Maintenance] Nereids Exchange Coupon (1 day) | 24 | 1 | all | OK | OK |
| 200 | 978017000 | Alencia Wing Exchange Coupon -Blue (3 Day) | 72 | 4 | all | ticket item missing in DB | OK |
| 201 | 978104000 | War Tattoo Voucher (1Day) | 24 | 1 | all | OK | OK |
| 202 | 978027000 | [20th Anniversary Emblem] Exchange Coupon (1day) | 24 | 1 | all | OK | OK |
| 203 | 978028000 | [20th Anniversary Emblem] Exchange Coupon (3days) | 72 | 1 | all | OK | OK |
| 204 | 978029000 | [20th Anniversary Emblem] Exchange Coupon (7days) | 168 | 1 | all | OK | OK |
| 205 | 978030000 | [20th Anniversary Emblem] Exchange Coupon (15days) | 360 | 1 | all | OK | OK |
| 206 | 978031000 | [20th Anniversary Emblem] Exchange Coupon (30days) | 720 | 1 | all | OK | OK |
| 207 | 978060000 | Minerva Helmet Certificate (3 days) | 72 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 208 | 978061000 | Minerva Helmet Certificate (7 days) | 168 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 209 | 978062000 | Minerva Helmet Certificate (15 days) | 360 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 210 | 978063000 | Minerva Armor Certificate (3 days) | 72 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 211 | 978064000 | Minerva Armor Certificate (7 days) | 168 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 212 | 978065000 | Minerva Armor Certificate (15 days) | 360 | 12 | all/non-Portu | Portu restriction not enforced | OK |
| 213 | 978066000 | Pathos' Glove Certificate (3 days) | 72 | 8 | all | OK | OK |
| 214 | 978067000 | Pathos' Glove Certificate (7 days) | 168 | 8 | all | OK | OK |
| 215 | 978068000 | Pathos' Glove Certificate (15 days) | 360 | 8 | all | OK | OK |
| 216 | 978069000 | Automatic Looting Exchange Coupon (3 day) | 72 | 1 | all | OK | OK |
| 217 | 978070000 | Automatic Looting Exchange Coupon (7 day) | 168 | 1 | all | OK | OK |
| 218 | 978071000 | Automatic Looting Exchange Coupon (15 day) | 360 | 1 | all | OK | OK |
| 219 | 978072000 | Alseids Exchange Coupon (3 day) | 72 | 1 | all | OK | OK |
| 220 | 978073000 | Alseids Exchange Coupon (7 day) | 168 | 1 | all | OK | OK |
| 221 | 978074000 | Alseids Exchange Coupon (15 day) | 360 | 1 | all | OK | OK |
| 222 | 978075000 | Nereids Exchange Coupon (3 day) | 72 | 1 | all | OK | OK |
| 223 | 978076000 | Nereids Exchange Coupon (7 day) | 168 | 1 | all | OK | OK |
| 224 | 978077000 | Nereids Exchange Coupon (15 day) | 360 | 1 | all | OK | OK |
| 225 | 978078000 | Dryad Exchange Coupon (3 day) | 72 | 1 | all | OK | OK |
| 226 | 978079000 | Dryad Exchange Coupon (7 day) | 168 | 1 | all | OK | OK |
| 227 | 978080000 | Dryad Exchange Coupon (15 day) | 360 | 1 | all | OK | OK |
| 228 | 978081000 | Oreads Exchange Coupon (15 day) | 360 | 1 | all | OK | OK |
| 229 | 978082000 | Special Oreads (Defense) (1 day) - Untradable | 24 | 1 | all | OK | OK |
| 230 | 978083000 | Special Oreads (Defense) (3 day) - Untradable | 72 | 1 | all | OK | OK |
| 231 | 978084000 | Special Oreads (Defense) (7 day) - Untradable | 168 | 1 | all | OK | OK |
| 232 | 978085000 | Special Oreads (Defense) (15 day) - Untradable | 360 | 1 | all | OK | OK |
| 233 | 978086000 | Special Oreads (Attack) (1 day) - Untradable | 24 | 1 | all | OK | OK |
| 234 | 978087000 | Special Oreads (Attack) (3 day) - Untradable | 72 | 1 | all | OK | OK |
| 235 | 978088000 | Special Oreads (Attack) (7 day) - Untradable | 168 | 1 | all | OK | OK |
| 236 | 978089000 | Special Oreads (Attack) (15 day) - Untradable | 360 | 1 | all | OK | OK |
| 237 | 978090000 | Special Nereids (Defense) (1 day) | 24 | 1 | all | OK | OK |
| 238 | 978091000 | Special Nereids (Defense) (3 day) | 72 | 1 | all | OK | OK |
| 239 | 978092000 | Special Nereids (Defense) (7 day) | 168 | 1 | all | OK | OK |
| 240 | 978093000 | Special Nereids (Defense) (15 day) | 360 | 1 | all | OK | OK |
| 241 | 978094000 | Special Nereids (Attack) (1 day) | 24 | 1 | all | OK | OK |
| 242 | 978095000 | Special Nereids (Attack) (3 day) | 72 | 1 | all | OK | OK |
| 243 | 978096000 | Special Nereids (Attack) (7 day) | 168 | 1 | all | OK | OK |
| 244 | 978097000 | Special Nereids (Attack) (15 day) | 360 | 1 | all | OK | OK |
| 245 | 978098000 | Mennissiah Trading Exchange Coupon (3Days) | 72 | 1 | all | OK | OK |
| 246 | 978099000 | Mennissiah Trading Exchange Coupon (7Days) | 168 | 1 | all | OK | OK |
| 247 | 978100000 | Mennissiah Trading Exchange Coupon (15Days) | 360 | 1 | all | OK | OK |
| 248 | 978101000 | Kaisyan's trade certified coupon (3Days) | 72 | 1 | all | OK | OK |
| 249 | 978102000 | Kaisyan's trade certified coupon (7Days) | 168 | 1 | all | OK | OK |
| 250 | 978103000 | Kaisyan's trade certified coupon (15Days) | 360 | 1 | all | OK | OK |
| 251 | 814663000 | War Tattoo Voucher | 720 | 1 | all | OK | OK |
| 252 | 978105000 | War Tattoo Voucher (3Days) | 72 | 1 | all | OK | OK |
| 253 | 978106000 | War Tattoo Voucher (7Days) | 168 | 1 | all | OK | OK |
| 254 | 931751000 | Solar Tattoo Exchange Voucher (15 Day) | 360 | 1 | all | OK | OK |
| 255 | 978108000 | Gold Dragon Wing Certificate (3 Days) | 72 | 8 | Karus/El Morad | OK | OK |
| 256 | 978109000 | Gold Dragon Wing Certificate (7 Days) | 168 | 8 | Karus/El Morad | OK | OK |
| 257 | 978110000 | Gold Dragon Wing Certificate (15 Days) | 360 | 8 | Karus/El Morad | OK | OK |
| 258 | 978111000 | Red Dragon Wing Certificate (3 Days) | 72 | 8 | Karus/El Morad | OK | OK |
| 259 | 978112000 | Red Dragon Wing Certificate (7 Days) | 168 | 8 | Karus/El Morad | OK | OK |
| 260 | 978113000 | Red Dragon Wing Certificate (15 Days) | 360 | 8 | Karus/El Morad | OK | OK |
| 261 | 978114000 | Strength Recovery Exchange Coupon (3 Days) | 72 | 1 | all | OK | OK |
| 262 | 978115000 | Strength Recovery Exchange Coupon (7 Days) | 168 | 1 | all | OK | OK |
| 263 | 978116000 | Strength Recovery Exchange Coupon (15 Days) | 360 | 1 | all | OK | OK |
| 264 | 978117000 | Intelligence Recovery Exchange Coupon (3 Days) | 72 | 1 | all | OK | OK |
| 265 | 978118000 | Intelligence Recovery Exchange Coupon (7 Days) | 168 | 1 | all | OK | OK |
| 266 | 978119000 | Intelligence Recovery Exchange Coupon (15 Days) | 360 | 1 | all | OK | OK |
| 267 | 978120000 | Magic Bag Exchange Coupon (3 Days) | 72 | 1 | all | OK | OK |
| 268 | 978121000 | Magic Bag Exchange Coupon (7 Days) | 168 | 1 | all | OK | OK |
| 269 | 978122000 | Magic Bag Exchange Coupon (15 Days) | 360 | 1 | all | OK | OK |
| 270 | 978123000 | Battle Hero’s Wing Exchange Coupon (1 Days) | 24 | 4 | Karus/El Morad | OK | OK |
| 271 | 978124000 | Battle Hero’s Wing Exchange Coupon (3 Days) | 72 | 4 | Karus/El Morad | OK | OK |
| 272 | 931696000 | Solar Tattoo Exchange Voucher (1 Day) | 24 | 1 | all | OK | OK |
| 273 | 810926000 | Solar Tattoo Exchange Voucher (30 Day) | 720 | 1 | all | OK | OK |
| 274 | 978129000 | Solar Tattoo Exchange Voucher (3 Days) | 72 | 1 | all | OK | OK |
| 275 | 978130000 | Solar Tattoo Exchange Voucher (7 Days) | 168 | 1 | all | OK | OK |
| 276 | 998015000 | Voucher for Magic Bag | 360 | 1 | all | OK | OK |
| 277 | 810334000 | Like Emblem Exchange Coupon | 168 | 1 | all | OK | OK |
| 278 | 814681000 | Wings of Hellfire Dragon Exchange Voucher (7 days) | 168 | 4 | all | OK | OK |
| 279 | 814062000 | Wings of Hellfire Dragon Exchange Voucher (30 days) | 720 | 4 | all | OK | OK |
| 280 | 810493000 | Bloody Weapon Exchange Coupon | 0 | 9 | all | OK | OK |
| 281 | 810381000 | Bloody Weapon Exchange Coupon (+4) | 0 | 9 | all | OK | OK |
| 282 | 810382000 | Bloody Weapon Exchange Coupon(+5) | 0 | 9 | all | OK | OK |
| 283 | 810383000 | Bloody Weapon Exchange Coupon(+6) | 0 | 9 | all | OK | OK |
| 284 | 810384000 | Bloody Weapon Exchange Coupon(+7) | 0 | 9 | all | OK | OK |
| 285 | 810208000 |  Rosetta Armor Pack | 0 | 20 | all | OK | OK |
| 286 | 900736000 | (+4)Wing Weapon Box | 0 | 9 | all | OK | OK |
| 287 | 810500000 | Garges Weapon Exchange Coupon | 0 | 9 | all | OK | OK |
| 288 | 810501000 | Garges Weapon Exchange Coupon | 0 | 9 | all | OK | OK |
| 289 | 810232000 | Guardian Exchange Coupon | 0 | 2 | Karus/El Morad | OK | OK |
| 290 | 810233000 | Magpie's mother transform coupon | 0 | 2 | Karus/El Morad | OK | OK |
| 291 | 810209000 | [Gold Dragon Egg] Shenlong Weapon Box | 0 | 9 | all | OK | OK |
| 292 | 914049000 | [13th Anniversary Emblem Replica] Voucher | 0 | 1 | all | OK | OK |
| 293 | 931694000 | [14th Anniversary Emblem Replica] Voucher | 0 | 1 | all | OK | OK |
| 294 | 900764000 | Garges Exchange Voucher (+4) | 0 | 9 | all | OK | OK |
| 295 | 900763000 | Garges Exchange Voucher (+5) | 0 | 9 | all | OK | OK |
| 296 | 900762000 | Garges Exchange Voucher (+6) | 0 | 9 | all | OK | OK |
| 297 | 900761000 | Garges Exchange Voucher (+7) | 0 | 9 | all | OK | OK |
| 298 | 810635000 | [Event] Dark Knight Weapon Chest | 0 | 10 | all | OK | OK |
| 299 | 931780000 |  | 0 | 1 | all | OK | OK |
| 300 | 508086000 | Magpie NPC Transformation Manual Coupon | 0 | 2 | Karus/El Morad | OK | OK |
| 301 | 978032000 | [Knight Online 20th Anniversary Decorative Emblem] Exchange Coupon | 0 | 1 | all | OK | OK |
| 302 | 900788000 | Premium Potion Voucher (HP 300) | 0 | 1 | all | OK | OK |
| 303 | 889320000 | Premium Potion Voucher (HP 300) | 0 | 1 | all | OK | OK |
| 304 | 900789000 | Premium Potion Voucher (MP 100) | 0 | 1 | all | OK | OK |
| 305 | 889340000 | Premium Potion Voucher (MP 100) | 0 | 1 | all | OK | OK |
| 306 | 889350000 | Premium Potion Voucher (MP 300) | 0 | 1 | all | OK | OK |
| 307 | 389370000 | Premium İksir Kuponu (HP 300) | 0 | 1 | all | OK | OK |
| 308 | 389380000 | Premium İksir Kuponu (MP 500) | 0 | 1 | all | OK | OK |
| 309 | 508190000 | World Championship Event winning ticket | 0 | 2 | Karus/El Morad | OK | OK |
| 310 | 800163000 | Ticket for praying hit | 168 | 1 | all | OK | OK |
| 311 | 810226000 | Night chaos day wing coupon | 720 | 4 | all | OK | OK |
| 312 | 810255000 | Chaos patron saint of transformation vouchers | 0 | 2 | Karus/El Morad | OK | OK |
| 313 | 931715000 | [Full Moon Emblem Replica] Voucher | 0 | 1 | all | OK | OK |
| 314 | 811148000 | Special Oreads (Defense) Voucher - Untradable | 720 | 1 | all | OK | OK |
| 315 | 811149000 | Special Oreads (Attack) Voucher - Untradable | 720 | 1 | all | OK | OK |
| 316 | 811150000 | Special Oreads (Defense) Voucher | 720 | 1 | all | OK | OK |
| 317 | 811151000 | Special Oreads (Attack) Voucher | 720 | 1 | all | OK | OK |
| 318 | 999916000 | Special Oreads Voucher (1 day) - Untradable | 24 | 2 | all | ticket item missing in DB | OK |
| 319 | 999917000 | Special Oreads Voucher (3 day) - Untradable | 72 | 2 | all | ticket item missing in DB | OK |
| 320 | 999918000 | Special Oreads Voucher (7 day) - Untradable | 168 | 2 | all | ticket item missing in DB | OK |
| 321 | 999919000 | Special Oreads Voucher (15 day) - Untradable | 360 | 2 | all | ticket item missing in DB | OK |
| 322 | 399289000 | EXP 100% recover Scroll chaging ticket | 0 | 1 | all | kind-255 durability 10: ticket survived the exchange | OK |
| 323 | 811153000 | Aset Transform Scroll Untradable Exchange Voucher | 0 | 2 | Karus/El Morad | OK | OK |
| 324 | 808111000 | Holy Knight Wings Exchange Coupon | 720 | 4 | Karus/El Morad | no server rules; ticket item missing in DB; 4 reward item(s) missing in DB | OK |
| 325 | 808112000 | Holy Knight Wings Exchange Coupon (1 Day) - Non-tradable | 24 | 4 | Karus/El Morad | no server rules; ticket item missing in DB; 4 reward item(s) missing in DB | OK |
| 326 | 808113000 | Holy Knight Wings Exchange Coupon (3 Day) - Non-tradable | 72 | 4 | Karus/El Morad | no server rules; ticket item missing in DB; 4 reward item(s) missing in DB | OK |
| 327 | 808114000 | Holy Knight Wings Exchange Coupon (7 Day) - Non-tradable | 168 | 4 | Karus/El Morad | no server rules; ticket item missing in DB; 4 reward item(s) missing in DB | OK |
| 328 | 808115000 | Holy Knight Wings Exchange Coupon (15 Day) - Non-tradable | 360 | 4 | Karus/El Morad | no server rules; ticket item missing in DB; 4 reward item(s) missing in DB | OK |
| 329 | 808116000 | Holy Knight Wings Exchange Coupon (30 Day) - Non-tradable | 720 | 4 | Karus/El Morad | no server rules; ticket item missing in DB; 4 reward item(s) missing in DB | OK |

## 5. Newly created item rows (22)

| item id | name | role |
|---|---|---|
| 808065704 | Transparent Minerva Armor: Helmet | reward |
| 808065705 | Transparent Minerva Armor: Helmet | reward |
| 808066698 | Transparent Minerva Armor | reward |
| 808066699 | Transparent Minerva Armor | reward |
| 808111000 | Holy Knight Wings Exchange Coupon | ticket |
| 808112000 | Holy Knight Wings Exchange Coupon (1 Day) - Non-tradable | ticket |
| 808113000 | Holy Knight Wings Exchange Coupon (3 Day) - Non-tradable | ticket |
| 808114000 | Holy Knight Wings Exchange Coupon (7 Day) - Non-tradable | ticket |
| 808115000 | Holy Knight Wings Exchange Coupon (15 Day) - Non-tradable | ticket |
| 808116000 | Holy Knight Wings Exchange Coupon (30 Day) - Non-tradable | ticket |
| 910920000 | Alencia Wing Exchange Coupon -Red (7 Day) | ticket |
| 910940000 | Wings of Hellfire Dragon Exchange Voucher (30 Day) | ticket |
| 978017000 | Alencia Wing Exchange Coupon -Blue (3 Day) | ticket |
| 978018000 | Stellar Tattoo Exchange Voucher (3 days) | ticket |
| 999916000 | Special Oreads Voucher (1 day) - Untradable | ticket |
| 999917000 | Special Oreads Voucher (3 day) - Untradable | ticket |
| 999918000 | Special Oreads Voucher (7 day) - Untradable | ticket |
| 999919000 | Special Oreads Voucher (15 day) - Untradable | ticket |
| 1808107168 | Holy Knight Wings | reward |
| 1808108169 | Holy Knight Wings | reward |
| 1808109170 | Holy Knight Wings | reward |
| 1808110171 | Holy Knight Wings | reward |

Row values come from `Item_Org_us.tbl` (+ `Item_Ext_13/14/22_us.tbl` for the ext variants). The column mapping was validated against the 5,426 existing rows that exist in both the live `item` table and `Item_Org_us.tbl`:
kind/slot/class/weight/duration/price/countable/effects/requirements match 98-100%; ext-expanded rows follow `name = ext name`, `item_type = ext.c7`, `duration += ext.c12`, `buy_price *= ext.c13`, `sell_npc_price = buy_price / 6` (checked against Wing of Hero 900028683 and Valkyrie Pauldron 808011698).

## 6. How to apply / verify

1. `psql -d ko_server -f migrations/20261018210000_ticket_exchange_full_sync.sql` (idempotent), then deploy `ko-server` (hours-exact expiry + new validation).
2. In game (Moradon, Kaira): talk -> `Ticket Exchange Use` -> drag a ticket into the window -> pick a reward -> confirm.
   - Holy Knight Wings Exchange Coupon (GM `+give` 808111000 / 808112000): El Morad characters see 2 wings, Karus characters see 2 other wings; the wing arrives with the listed lifetime.
   - A Portu/Kurian character with a Minerva Helmet Certificate sees only the Portu helmets.
   - Fill the bag completely: a single-piece ticket is still swapped for its reward in place; with a stack of 2+ tickets (e.g. 808111000 x2) and no free slot you get "Your inventory is full.".
   - Ticket 399289000 is consumed completely on the first exchange.
