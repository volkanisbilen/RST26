# Daily quests: rebalance (2026-10-18)

Migration: `migrations/20261018140000_daily_quests_rebalance.sql` (idempotent, live-safe; the apply command is in the file header).

## How daily quests work here (code facts)

* Table `daily_quests` (200 rows). Every quest whose `min_level..max_level` fits the player starts automatically at
  login / level-up (`daily_quest::auto_start_all`), there is no slot limit, and every kill of one of the quest's
  `mob_id_1..4` counts for every running quest that lists it and matches zone, level and party rule.
* On completion the four reward slots are paid by `daily_quest_finished`: `900001000` = EXP, `900000000` = gold,
  `900002000` / `900003000` = NP, `900004000` = random item from pool `random_id`, anything else is an item
  (`give_item`; inventory full -> letter). **Non-stackable items (item.countable = 0) are always paid as ONE**
  whatever the count says, so those rewards use count 1.
* The quest repeats after `replay_time` hours (24 for all of them, unchanged).
* The panel sends the kill target as u16, so the largest target (300) is fine.

## Formula for the required kills

`L` = average level of the quest's target monsters (`npc_template.s_level`; two placeholder templates are replaced,
see "Data anomalies"), `min` = the quest's `min_level`.

```
K0(L) = piecewise-linear through (L=1: 300) (30: 200) (60: 120) (83: 75) (110: 30)   # tougher target -> fewer kills
g     = 1 + 0.01 * clamp(min - L, -20, +30)     # target far below the lowest eligible player = trash -> up to +30 %,
                                                # target above the lowest eligible player = tougher -> up to -20 %
K     = K0(L) * g     (x 0.8 if the quest is solo-only)   -> rounded to 5, clamped to 30..300
```

Examples: Worm / Bandicoot (L 4.5, lv 1-20) 280; Lycan (L 35) for lv 20-49: 160, the same Lycan for lv 50-83: 215;
Uruk Blade (L 70) for lv 40-49: 80; Stone / Giant Golem (L 100 / 110): 35 / 30. (Before: 50 for every beginner quest,
500 for almost every other quest, 250 for the 60+ ones.)

## Rewards

Four slots per quest: **EXP**, **gold**, one **consumable** (potion/scroll), one **special** (upgrade material,
accessory scroll, box, NP). Consumable and special rotate through per-tier lists by the quest's position in its tier,
so neighbouring quests pay different things. Tier = the quest's `min_level`.

* EXP = `pct(tier) * exp_to_next_level(ref)`; `ref` = target level clamped to the middle of the quest's level band;
  floored per tier (1 k ... 1 M) and capped at 5 M. pct: T1 10 %, T2 12 %, T3 8 %, T4 6 %, T5 2 %, T6 1.5 %.
  (Killing the monsters gives far more EXP than this on this server; the bonus is deliberately small.)
* Gold = `K * (L + 5) * 40`, x0.4 in T1 and x0.8 in T2, rounded to 10 k, clamped 20 k..500 k (about 30-90 k for the
  beginner quests, 140-340 k for the upper tiers; the old reward was a Gold Coin, NPC value 166 k).
* Consumable: T1-T3 Red/Blue Potion (count = K/5, 20..100) and cheap buff scrolls; T4+ Potion of Draki (S/L), 1000/2000 HP and
  Armor 350 scrolls, Power of Lion, Resurrection Scroll (90); T6 also Ibex / Crisis potion.
* Special: Low/Middle class Upgrade Scrolls (T1-T3), Upgrade Scroll (T4+), **Blessed Upgrade Scroll only in T5/T6** (1-2, about
  one quest in five; the old data paid one Blessed Upgrade Scroll **per quest**), Accessory Compound / Enchant Scrolls,
  Silvery Gems, NP (30 at T2 up to about 150 at T6), and boxes from T2 up: Red / Green / Blue Treasure Chest,
  Akara Accessory Box (T4, T6), Dragon Box (T5, T6), Black Lion Weapon Box (T5, T6; mostly Blessed Upgrade Scrolls inside).
  The only solo-only quest (200) pays 2 Blessed Upgrade Scrolls.
* Old rewards: EXP + Blessed Upgrade Scroll x1 + Gold Coin x1 on every quest, Silver Bar x1 instead of the coin at the zone 11 / 71 quests.


| tier | min_level of its quests | quests |
|---|---|---|
| T1 | 1 | 10 |
| T2 | 20 | 18 |
| T3 | 30 | 26 |
| T4 | 40 | 32 |
| T5 | 50 | 100 |
| T6 | 60, 65 | 14 |

## Data anomalies found (NOT changed by the migration)

* **Mobs without any spawn** in `npc_spawn` (marked `NO SPAWN`): those quests cannot be completed today (for example
  the Elmorad twins 95-103 / 133-141 whose ids 501-505, 900, 903, 1001... never spawn, Snatcher 755, Decayed Zombie 1051, Skeleton
  1100 / 1150, Haunga Warrior 1362, Baron / Cardinal / Scolar 1250-1254, Giant Golem 2452 ...). `other zone only` = the mob spawns,
  but not in the quest's zone, so it only works while the player stands in the listed zone(s). `Moradon (lv<=35)` = the quest zone is
  Luferson / El Morad but the mob only spawns in Moradon, which the daily-quest zone check allows for players up to level 35.
  Fix = add spawns in `npc_spawn` or re-point `mob_id_1..4` (Admin panel -> Odul Merkezi -> Gunluk gorevler). Counts and rewards are
  already set for when that happens.
* Template `1150` ("Skeleton") is a placeholder (level 80, 1000 HP, 1200 exp) and template `5301` ("Ancient") is a custom level-83 /
  900 000 HP boss with no spawn. For the formula they count as level 31 (twin 1100) and level 50 (twin 5351).
  Templates `600` and `1111` (quest 200) are placeholders too (1000-1564 HP, 1200 exp); their stored level is used.

## Doing the same through the admin panel

Admin panel -> **Odul Merkezi -> Gunluk gorevler (Daily Quest)** edits every column of `daily_quests` (`Adet` = kills, `Odul 1..4` +
`Adet 1..4` = rewards), checks item ids and hot-reloads the quest definitions of the running game server when you save.
To tweak one quest, change `Adet` and the reward pairs there. Reload without editing:
`INSERT INTO admin_command(command,args) VALUES ('reload','{"what":"daily_quests"}');`

## Server code change that goes with it

`handler/daily_quest.rs::update_daily_quest_count` used to skip a quest as soon as the stored counter was at or above the target,
so lowering a target under a player's progress left that quest stuck forever. A matching kill now completes it (`next_kill_count`,
unit-tested). The migration also clamps stored progress (step 3) so offline players are fine too.

## Before / after per quest

`Reach` = whether the target spawns where the quest can be done (see above). Kills: old -> new.

| id | quest | players | zone | target L | kills | rewards before | rewards after | reach |
|---|---|---|---|---|---|---|---|---|

| 1 | Worm Hunt | 1-20 | 21 | 4 | 50 -> **280** | 50 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 40,000 Gold, Red Potion x55, Silvery Gem x3 | ok |
| 2 | Snatcher Hunt | 1-20 | 21 | 8 | 50 -> **255** | 75 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 50,000 Gold, Scroll of Health(S) x1, Upgrade Scroll (Low Class item) x5 | NO SPAWN |
| 3 | Bandicoot Hunt | 1-20 | 21 | 2 | 50 -> **295** | 100 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 30,000 Gold, Scroll of Lion Strength x1, Potion of soul x5 | ok |
| 4 | Scavenger Bandicoot Hunt | 1-20 | 21 | 7 | 50 -> **265** | 150 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 50,000 Gold, Scroll of 500HP Up(S) x1, Silvery Gem x5 | ok |
| 5 | Bulcan Hunt | 1-20 | 21 | 11 | 50 -> **240** | 300 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 60,000 Gold, Scroll of Armor class x1, Upgrade Scroll (Low Class item) x4 | ok |
| 6 | Bulture Hunt | 1-20 | 21 | 19 | 50 -> **195** | 500 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 70,000 Gold, Blue Potion x40, Upgrade Scroll (Low Class item) x3 | ok |
| 7 | Giant Bulcan Hunt | 1-20 | 21 | 17 | 50 -> **205** | 600 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 70,000 Gold, Blue Potion x40, Upgrade Scroll (Low Class item) x5 | ok |
| 8 | Werewolf Hunt | 1-20 | 21 | 23 | 50 -> **180** | 700 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 80,000 Gold, Red Potion x35, Potion of soul x5 | ok |
| 9 | Silan Hunt | 1-20 | 21 | 20 | 50 -> **190** | 750 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 80,000 Gold, Scroll of Health(S) x1, Silvery Gem x5 | ok |
| 10 | Orc Watcher Hunt | 1-20 | 21 | 30 | 50 -> **160** | 800 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,000 EXP, 90,000 Gold, Scroll of Lion Strength x1, Upgrade Scroll (Low Class item) x4 | ok |
| 11 | Lycan Hunt | 20-49 | 1 | 35 | 500 -> **160** | 100,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 65,000 EXP, 200,000 Gold, Red Potion x30, Silvery Gem x5 | ok |
| 12 | Loup-Garou Hunt | 20-49 | 1 | 38 | 500 -> **145** | 110,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 65,000 EXP, 200,000 Gold, Scroll of Armor class x1, Upgrade Scroll (Middle Class item) x2 | Moradon (lv<=35) |
| 13 | Dark Eyes Hunt | 20-49 | 1 | 35 | 500 -> **160** | 120,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 65,000 EXP, 200,000 Gold, Speed-Up Potion x1, Upgrade Scroll (Low Class item) x5 | Moradon (lv<=35) |
| 14 | Dire Wolf Hunt | 20-49 | 1 | 40 | 500 -> **140** | 130,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 65,000 EXP, 200,000 Gold, Scroll of 1000 HP Up x1, Potion of soul x10 | Moradon (lv<=35) |
| 15 | Scorpion Hunt | 20-49 | 1 | 31 | 500 -> **175** | 100,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 49,000 EXP, 200,000 Gold, Scroll of Armor 200 x1, Upgrade Scroll (Middle Class item) x3 | Moradon (lv<=35) |
| 16 | Pincer Scorpion Hunt | 20-49 | 1 | 36 | 500 -> **155** | 120,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 65,000 EXP, 200,000 Gold, Blue Potion x30, Upgrade Scroll (Low Class item) x8 | Moradon (lv<=35) |
| 17 | Decayed Zombie Hunt | 20-49 | 1 | 24 | 500 -> **210** | 130,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 11,000 EXP, 190,000 Gold, Blue Potion x40, Red Treasure Chest x1 | NO SPAWN |
| 18 | Undying Hunt | 20-49 | 1 | 33 | 500 -> **165** | 150,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 59,000 EXP, 200,000 Gold, Red Potion x35, 30 NP | ok |
| 19 | Skeleton Hunt | 20-49 | 1 | 31 | 500 -> **175** | 120,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 49,000 EXP, 200,000 Gold, Scroll of Armor class x1, 25 NP | NO SPAWN |
| 20 | Smilodon Hunt | 30-49 | 1 | 40 | 500 -> **155** | 120,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 280,000 Gold, Red Potion x30, Silvery Gem x8 | ok |
| 21 | Sabertooth Hunt | 30-49 | 1 | 45 | 500 -> **135** | 130,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 270,000 Gold, Speed-Up Potion x1, 40 NP | Moradon (lv<=35) |
| 22 | Machirodus Hunt | 30-49 | 1 | 43 | 500 -> **145** | 150,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 280,000 Gold, Scroll of 1000HP Up(L) x1, Accessory Compound Scroll x3 | NO SPAWN |
| 23 | Megantereon Hunt | 30-49 | 1 | 45 | 500 -> **135** | 200,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 270,000 Gold, Scroll of 500HP Up(L) x1, Upgrade Scroll (Middle Class item) x3 | ok |
| 24 | Skeleton Knight Hunt | 30-49 | 1 | 35 | 500 -> **175** | 220,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 65,000 EXP, 280,000 Gold, Potion of Draki (S) x5, Green Treasure Chest x1 | ok |
| 25 | Skeleton Champion Hunt | 30-49 | 1 | 37 | 500 -> **170** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 78,000 EXP, 290,000 Gold, Blue Potion x35, Accessory Compound Scroll x5 | ok |
| 26 | Death Knight Hunt | 30-49 | 1 | 45 | 500 -> **135** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 270,000 Gold, Blue Potion x25, Upgrade Scroll (Middle Class item) x4 | Moradon (lv<=35) |
| 27 | Baron Hunt | 30-49 | 1 | 52 | 500 -> **115** | 220,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 260,000 Gold, Red Potion x25, Red Treasure Chest x1 | NO SPAWN |
| 28 | Cardinal Hunt | 30-49 | 1 | 54 | 500 -> **110** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 260,000 Gold, Speed-Up Potion x1, Red Treasure Chest x1 | NO SPAWN |
| 29 | Scolar Hunt | 30-49 | 1 | 46 | 500 -> **130** | 200,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 270,000 Gold, Scroll of 1000HP Up(L) x1, Silvery Gem x8 | NO SPAWN |
| 30 | Garuna Hunt | 30-49 | 1 | 53 | 500 -> **110** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 260,000 Gold, Scroll of 500HP Up(L) x1, 45 NP | ok |
| 31 | Haunga Hunt | 30-49 | 1 | 55 | 500 -> **105** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 250,000 Gold, Potion of Draki (S) x5, Accessory Compound Scroll x3 | ok |
| 32 | Haunga Warrior Hunt | 30-49 | 1 | 60 | 500 -> **95** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 250,000 Gold, Potion of Draki (S) x5, Upgrade Scroll (Middle Class item) x3 | NO SPAWN |
| 33 | Uruk Hai Hunt | 40-49 | 1 | 60 | 500 -> **95** | 220,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 250,000 Gold, Potion of Draki (S) x10, Green Treasure Chest x1 | ok |
| 34 | Uruk Blade Hunt | 40-49 | 1 | 70 | 500 -> **80** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 240,000 Gold, Potion of soul x10, Accessory Enchant Scroll(HP) x1 | ok |
| 35 | Uruk Tron Hunt | 40-49 | 1 | 80 | 500 -> **65** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 220,000 Gold, Speed-Up Potion x1, 70 NP | ok |
| 36 | Kobold Hunt | 40-49 | 1 | 30 | 500 -> **220** | 150,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 140,000 EXP, 310,000 Gold, Scroll of Armor 350 x1, Upgrade Scroll (Middle Class item) x5 | ok |
| 37 | Bugbear Hunt | 40-49 | 1 | 35 | 500 -> **195** | 170,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 140,000 EXP, 310,000 Gold, Scroll of 1000HP Up(L) x1, Upgrade Scroll x2 | ok |
| 38 | Pooka Hunt | 40-49 | 1 | 40 | 500 -> **175** | 200,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 140,000 EXP, 320,000 Gold, Potion of Draki (L) x5, Akara Accessory Box x1 | ok |
| 39 | Goblin Bouncer Hunt | 40-49 | 1 | 45 | 500 -> **150** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 300,000 Gold, Potion of Draki (L) x5, Accessory Compound Scroll x5 | NO SPAWN |
| 40 | Lamia Hunt | 40-49 | 1 | 60 | 500 -> **95** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 250,000 Gold, Potion of Draki (S) x10, Upgrade Scroll x3 | ok |
| 41 | Lamiros Hunt | 40-49 | 1 | 70 | 500 -> **80** | 330,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 240,000 Gold, Potion of soul x10, Upgrade Scroll x3 | other zone only (31) |
| 42 | Ape Hunt | 40-49 | 1 | 42 | 500 -> **165** | 200,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 170,000 EXP, 310,000 Gold, Speed-Up Potion x1, Green Treasure Chest x1 | ok |
| 43 | Tyon Hunt | 40-49 | 1 | 55 | 500 -> **115** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 280,000 Gold, Scroll of Armor 350 x1, Accessory Enchant Scroll(STR) x1 | ok |
| 44 | Gray Oozy Hunt | 40-49 | 1 | 40 | 500 -> **175** | 180,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 140,000 EXP, 320,000 Gold, Scroll of 1000HP Up(L) x1, 50 NP | ok |
| 45 | Lich Hunt | 40-49 | 1 | 70 | 500 -> **80** | 400,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 240,000 Gold, Scroll of 1000HP Up(L) x1, Upgrade Scroll (Middle Class item) x5 | ok |
| 46 | Grell Hunt | 40-49 | 1 | 60 | 500 -> **95** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 250,000 Gold, Potion of Draki (L) x5, Upgrade Scroll x2 | ok |
| 47 | Ancient Hunt | 40-49 | 1 | 50 | 500 -> **130** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 290,000 Gold, Potion of Draki (S) x10, Akara Accessory Box x1 | NO SPAWN |
| 48 | Treant Hunt | 40-49 | 1 | 55 | 500 -> **115** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 280,000 Gold, Potion of soul x10, Accessory Compound Scroll x5 | NO SPAWN |
| 49 | Lycan Hunt | 50-83 | 1 | 35 | 500 -> **215** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, Blue Treasure Chest x1 | ok |
| 50 | Loup-Garou Hunt | 50-83 | 1 | 38 | 500 -> **200** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (S) x15, Blessed Upgrade Scroll x1 | other zone only (21) |
| 51 | Dark Eyes Hunt | 50-83 | 1 | 35 | 500 -> **215** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Resurrection Scroll(90) x1, Dragon Box x1 | other zone only (21) |
| 52 | Dire Wolf Hunt | 50-83 | 1 | 40 | 500 -> **190** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Power of Lion Scroll(Stat)(L) x1, 90 NP | other zone only (21) |
| 53 | Scorpion Hunt | 50-83 | 1 | 31 | 500 -> **235** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of Armor 350 x1, Accessory Enchant Scroll(MP) x1 | other zone only (21) |
| 54 | Pincer Scorpion Hunt | 50-83 | 1 | 36 | 500 -> **210** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of 2000HP Up x1, Upgrade Scroll x3 | other zone only (21) |
| 55 | Decayed Zombie Hunt | 50-83 | 1 | 24 | 500 -> **280** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 320,000 Gold, Scroll of 2000HP Up x1, Black Lion Weapon Box x1 | NO SPAWN |
| 56 | Undying Hunt | 50-83 | 1 | 33 | 500 -> **225** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, Upgrade Scroll x4 | ok |
| 57 | Skeleton Hunt | 50-83 | 1 | 31 | 500 -> **235** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (S) x15, 65 NP | NO SPAWN |
| 58 | Smilodon Hunt | 50-83 | 1 | 40 | 500 -> **190** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Resurrection Scroll(90) x1, Blessed Upgrade Scroll x1 | ok |
| 59 | Sabertooth Hunt | 50-83 | 1 | 45 | 500 -> **170** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Power of Lion Scroll(Stat)(L) x1, 95 NP | other zone only (21) |
| 60 | Machirodus Hunt | 50-83 | 1 | 43 | 500 -> **175** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of Armor 350 x1, Accessory Enchant Scroll(HP) x1 | NO SPAWN |
| 61 | Megantereon Hunt | 50-83 | 1 | 45 | 500 -> **170** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of Armor 350 x1, Upgrade Scroll x3 | ok |
| 62 | Skeleton Knight Hunt | 50-83 | 1 | 35 | 500 -> **215** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of 2000HP Up x1, Black Lion Weapon Box x1 | ok |
| 63 | Skeleton Champion Hunt | 50-83 | 1 | 37 | 500 -> **205** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, Upgrade Scroll x4 | ok |
| 64 | Death Knight Hunt | 50-83 | 1 | 45 | 500 -> **170** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (S) x15, 75 NP | other zone only (21) |
| 65 | Baron Hunt | 50-83 | 1 | 52 | 500 -> **140** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 320,000 Gold, Resurrection Scroll(90) x1, Blessed Upgrade Scroll x1 | NO SPAWN |
| 66 | Cardinal Hunt | 50-83 | 1 | 54 | 500 -> **130** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 310,000 Gold, Power of Lion Scroll(Stat)(L) x1, Blue Treasure Chest x1 | NO SPAWN |
| 67 | Scolar Hunt | 50-83 | 1 | 46 | 500 -> **165** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Power of Lion Scroll(Stat)(L) x1, Blessed Upgrade Scroll x1 | NO SPAWN |
| 68 | Garuna Hunt | 50-83 | 1 | 53 | 500 -> **135** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 310,000 Gold, Scroll of Armor 350 x1, Dragon Box x1 | ok |
| 69 | Haunga Hunt | 50-83 | 1 | 55 | 500 -> **125** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 670,000 EXP, 300,000 Gold, Scroll of 2000HP Up x1, Black Lion Weapon Box x1 | ok |
| 70 | Haunga Warrior Hunt | 50-83 | 1 | 60 | 500 -> **110** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Potion of Draki (L) x10, Upgrade Scroll x4 | NO SPAWN |
| 71 | Uruk Hai Hunt | 50-83 | 1 | 60 | 500 -> **110** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Potion of Draki (S) x15, 85 NP | ok |
| 72 | Uruk Blade Hunt | 50-83 | 1 | 70 | 500 -> **80** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Resurrection Scroll(90) x1, Blessed Upgrade Scroll x1 | ok |
| 73 | Uruk Tron Hunt | 50-83 | 1 | 80 | 500 -> **65** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 220,000 Gold, Resurrection Scroll(90) x1, Blue Treasure Chest x1 | ok |
| 74 | Kobold Hunt | 50-83 | 1 | 30 | 500 -> **240** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Power of Lion Scroll(Stat)(L) x1, Blessed Upgrade Scroll x1 | ok |
| 75 | Bugbear Hunt | 50-83 | 1 | 35 | 500 -> **215** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of Armor 350 x1, Dragon Box x1 | ok |
| 76 | Pooka Hunt | 50-83 | 1 | 40 | 500 -> **190** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of 2000HP Up x1, 90 NP | ok |
| 77 | Goblin Bouncer Hunt | 50-83 | 1 | 45 | 500 -> **170** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, Accessory Enchant Scroll(INT) x1 | NO SPAWN |
| 78 | Lamia Hunt | 50-83 | 1 | 60 | 500 -> **110** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Potion of Draki (S) x15, Upgrade Scroll x3 | ok |
| 79 | Lamiros Hunt | 50-83 | 1 | 70 | 500 -> **80** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Potion of Draki (S) x15, Blessed Upgrade Scroll x1 | other zone only (31) |
| 80 | Ape Hunt | 50-83 | 1 | 42 | 500 -> **180** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Resurrection Scroll(90) x1, Blue Treasure Chest x1 | ok |
| 81 | Tyon Hunt | 50-83 | 1 | 55 | 500 -> **125** | 2,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 670,000 EXP, 300,000 Gold, Power of Lion Scroll(Stat)(L) x1, Blessed Upgrade Scroll x1 | ok |
| 82 | Gray Oozy Hunt | 50-83 | 1 | 40 | 500 -> **190** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of Armor 350 x1, Dragon Box x1 | ok |
| 83 | Lich Hunt | 50-83 | 1 | 70 | 500 -> **80** | 2,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Scroll of 2000HP Up x1, 115 NP | ok |
| 84 | Grell Hunt | 50-83 | 1 | 60 | 500 -> **110** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Potion of Draki (L) x10, Accessory Enchant Scroll(STR) x1 | ok |
| 85 | Ancient Hunt | 50-83 | 1 | 50 | 500 -> **145** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 320,000 Gold, Potion of Draki (L) x10, Upgrade Scroll x3 | NO SPAWN |
| 86 | Treant Hunt | 50-83 | 1 | 55 | 500 -> **125** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 670,000 EXP, 300,000 Gold, Potion of Draki (S) x15, Black Lion Weapon Box x1 | NO SPAWN |
| 87 | Deruvish Hunt | 50-83 | 1 | 60 | 500 -> **110** | 3,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Resurrection Scroll(90) x1, Upgrade Scroll x4 | ok |
| 88 | Apostle Hunt | 50-83 | 1 | 70 | 500 -> **80** | 3,500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Power of Lion Scroll(Stat)(L) x1, 90 NP | ok |
| 89 | Harpy Hunt | 50-83 | 1 | 70 | 500 -> **80** | 3,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Scroll of Armor 350 x1, Dragon Box x1 | other zone only (31) |
| 90 | Raven Harpy Hunt | 50-83 | 1 | 80 | 500 -> **65** | 3,500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 220,000 Gold, Scroll of 2000HP Up x1, 125 NP | other zone only (11/12) |
| 91 | Stone Golem Hunt | 50-83 | 1 | 100 | 500 -> **35** | 4,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 150,000 Gold, Scroll of 2000HP Up x1, Accessory Enchant Scroll(DEX) x1 | other zone only (11/12) |
| 92 | Giant Golem Hunt | 50-83 | 1 | 110 | 500 -> **30** | 5,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 140,000 Gold, Potion of Draki (L) x10, Upgrade Scroll x3 | NO SPAWN |
| 93 | Troll Hunt | 50-83 | 1 | 70 | 500 -> **80** | 4,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Potion of Draki (S) x15, Black Lion Weapon Box x1 | ok |
| 94 | Troll Warrior Hunt | 50-83 | 1 | 80 | 500 -> **65** | 5,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 220,000 Gold, Resurrection Scroll(90) x1, Upgrade Scroll x4 | ok |
| 95 | Lycan Hunt | 20-49 | 2 | 25 | 500 -> **205** | 100,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 13,000 EXP, 200,000 Gold, Speed-Up Potion x1, Silvery Gem x5 | NO SPAWN |
| 96 | Loup-Garou Hunt | 20-49 | 2 | 27 | 500 -> **195** | 110,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 17,000 EXP, 200,000 Gold, Scroll of 1000 HP Up x1, Upgrade Scroll (Middle Class item) x2 | NO SPAWN |
| 97 | Dark Eyes Hunt | 20-49 | 2 | 31 | 500 -> **175** | 120,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 49,000 EXP, 200,000 Gold, Scroll of Armor 200 x1, Upgrade Scroll (Low Class item) x5 | NO SPAWN |
| 98 | Dire Wolf Hunt | 20-49 | 2 | 33 | 500 -> **165** | 130,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 59,000 EXP, 200,000 Gold, Scroll of Armor 200 x1, Potion of soul x10 | NO SPAWN |
| 99 | Scorpion Hunt | 20-49 | 2 | 26 | 500 -> **200** | 100,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 15,000 EXP, 200,000 Gold, Blue Potion x40, Upgrade Scroll (Middle Class item) x3 | NO SPAWN |
| 100 | Pincer Scorpion Hunt | 20-49 | 2 | 30 | 500 -> **180** | 120,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 44,000 EXP, 200,000 Gold, Red Potion x35, Upgrade Scroll (Low Class item) x8 | NO SPAWN |
| 101 | Decayed Zombie Hunt | 20-49 | 2 | 24 | 500 -> **210** | 130,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 11,000 EXP, 190,000 Gold, Scroll of Armor class x1, Red Treasure Chest x1 | ok |
| 102 | Undying Hunt | 20-49 | 2 | 33 | 500 -> **165** | 150,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 59,000 EXP, 200,000 Gold, Speed-Up Potion x1, Red Treasure Chest x1 | NO SPAWN |
| 103 | Skeleton Hunt | 20-49 | 2 | 31 | 500 -> **175** | 120,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 49,000 EXP, 200,000 Gold, Scroll of 1000 HP Up x1, 25 NP | NO SPAWN |
| 104 | Smilodon Hunt | 30-49 | 2 | 37 | 500 -> **170** | 120,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 78,000 EXP, 290,000 Gold, Blue Potion x35, Green Treasure Chest x1 | NO SPAWN |
| 105 | Sabertooth Hunt | 30-49 | 2 | 41 | 500 -> **150** | 130,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 280,000 Gold, Red Potion x30, Accessory Compound Scroll x5 | ok |
| 106 | Machirodus Hunt | 30-49 | 2 | 43 | 500 -> **145** | 150,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 280,000 Gold, Speed-Up Potion x1, Upgrade Scroll (Middle Class item) x4 | ok |
| 107 | Megantereon Hunt | 30-49 | 2 | 45 | 500 -> **135** | 200,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 270,000 Gold, Scroll of 1000HP Up(L) x1, Upgrade Scroll (Middle Class item) x4 | ok |
| 108 | Skeleton Knight Hunt | 30-49 | 2 | 35 | 500 -> **175** | 220,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 65,000 EXP, 280,000 Gold, Scroll of 500HP Up(L) x1, Red Treasure Chest x1 | NO SPAWN |
| 109 | Skeleton Champion Hunt | 30-49 | 2 | 37 | 500 -> **170** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 78,000 EXP, 290,000 Gold, Scroll of 500HP Up(L) x1, Silvery Gem x8 | NO SPAWN |
| 110 | Death Knight Hunt | 30-49 | 2 | 45 | 500 -> **135** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 270,000 Gold, Potion of Draki (S) x5, 40 NP | ok |
| 111 | Baron Hunt | 30-49 | 2 | 52 | 500 -> **115** | 220,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 260,000 Gold, Blue Potion x25, Accessory Compound Scroll x3 | ok |
| 112 | Cardinal Hunt | 30-49 | 2 | 54 | 500 -> **110** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 260,000 Gold, Red Potion x20, Upgrade Scroll (Middle Class item) x3 | other zone only (71/72) |
| 113 | Scolar Hunt | 30-49 | 2 | 46 | 500 -> **130** | 200,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 270,000 Gold, Speed-Up Potion x1, Green Treasure Chest x1 | ok |
| 114 | Garuna Hunt | 30-49 | 2 | 53 | 500 -> **110** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 260,000 Gold, Scroll of 1000HP Up(L) x1, Accessory Compound Scroll x5 | NO SPAWN |
| 115 | Haunga Hunt | 30-49 | 2 | 55 | 500 -> **105** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 250,000 Gold, Scroll of 1000HP Up(L) x1, Accessory Compound Scroll x5 | other zone only (73) |
| 116 | Haunga Warrior Hunt | 30-49 | 2 | 60 | 500 -> **95** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 95,000 EXP, 250,000 Gold, Scroll of 500HP Up(L) x1, Upgrade Scroll (Middle Class item) x4 | ok |
| 117 | Uruk Hai Hunt | 40-49 | 2 | 60 | 500 -> **95** | 220,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 250,000 Gold, Speed-Up Potion x1, Accessory Compound Scroll x5 | ok |
| 118 | Uruk Blade Hunt | 40-49 | 2 | 70 | 500 -> **80** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 240,000 Gold, Scroll of Armor 350 x1, Upgrade Scroll x3 | ok |
| 119 | Uruk Tron Hunt | 40-49 | 2 | 80 | 500 -> **65** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 220,000 Gold, Scroll of Armor 350 x1, Green Treasure Chest x1 | ok |
| 120 | Kobold Hunt | 40-49 | 2 | 30 | 500 -> **220** | 150,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 140,000 EXP, 310,000 Gold, Scroll of 1000HP Up(L) x1, Accessory Enchant Scroll(MP) x1 | ok |
| 121 | Bugbear Hunt | 40-49 | 2 | 35 | 500 -> **195** | 170,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 140,000 EXP, 310,000 Gold, Potion of Draki (L) x5, 50 NP | ok |
| 122 | Pooka Hunt | 40-49 | 2 | 40 | 500 -> **175** | 200,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 140,000 EXP, 320,000 Gold, Potion of Draki (S) x10, Upgrade Scroll (Middle Class item) x5 | ok |
| 123 | Goblin Bouncer Hunt | 40-49 | 2 | 45 | 500 -> **150** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 300,000 Gold, Potion of soul x10, Upgrade Scroll x2 | ok |
| 124 | Lamia Hunt | 40-49 | 2 | 60 | 500 -> **95** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 250,000 Gold, Speed-Up Potion x1, Akara Accessory Box x1 | NO SPAWN |
| 125 | Lamiros Hunt | 40-49 | 2 | 70 | 500 -> **80** | 330,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 240,000 Gold, Speed-Up Potion x1, Akara Accessory Box x1 | NO SPAWN |
| 126 | Ape Hunt | 40-49 | 2 | 42 | 500 -> **165** | 200,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 170,000 EXP, 310,000 Gold, Scroll of Armor 350 x1, Accessory Compound Scroll x5 | NO SPAWN |
| 127 | Tyon Hunt | 40-49 | 2 | 55 | 500 -> **115** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 280,000 Gold, Scroll of 1000HP Up(L) x1, Upgrade Scroll x3 | NO SPAWN |
| 128 | Gray Oozy Hunt | 40-49 | 2 | 40 | 500 -> **175** | 180,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 140,000 EXP, 320,000 Gold, Potion of Draki (L) x5, Green Treasure Chest x1 | NO SPAWN |
| 129 | Lich Hunt | 40-49 | 2 | 70 | 500 -> **80** | 400,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 240,000 Gold, Potion of Draki (S) x10, Accessory Enchant Scroll(INT) x1 | ok |
| 130 | Grell Hunt | 40-49 | 2 | 60 | 500 -> **95** | 300,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 250,000 Gold, Potion of soul x10, 60 NP | NO SPAWN |
| 131 | Ancient Hunt | 40-49 | 2 | 50 | 500 -> **130** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 290,000 Gold, Potion of soul x10, Upgrade Scroll (Middle Class item) x5 | NO SPAWN |
| 132 | Treant Hunt | 40-49 | 2 | 55 | 500 -> **115** | 250,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 210,000 EXP, 280,000 Gold, Speed-Up Potion x1, Upgrade Scroll x2 | ok |
| 133 | Lycan Hunt | 50-83 | 2 | 25 | 500 -> **270** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 320,000 Gold, Power of Lion Scroll(Stat)(L) x1, 60 NP | NO SPAWN |
| 134 | Loup-Garou Hunt | 50-83 | 2 | 27 | 500 -> **260** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 330,000 Gold, Scroll of Armor 350 x1, Blessed Upgrade Scroll x1 | NO SPAWN |
| 135 | Dark Eyes Hunt | 50-83 | 2 | 31 | 500 -> **235** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of Armor 350 x1, Blue Treasure Chest x1 | NO SPAWN |
| 136 | Dire Wolf Hunt | 50-83 | 2 | 33 | 500 -> **225** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of 2000HP Up x1, Blessed Upgrade Scroll x1 | NO SPAWN |
| 137 | Scorpion Hunt | 50-83 | 2 | 26 | 500 -> **265** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 330,000 Gold, Potion of Draki (L) x10, Upgrade Scroll x3 | NO SPAWN |
| 138 | Pincer Scorpion Hunt | 50-83 | 2 | 30 | 500 -> **240** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (S) x15, Black Lion Weapon Box x1 | NO SPAWN |
| 139 | Decayed Zombie Hunt | 50-83 | 2 | 24 | 500 -> **280** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 320,000 Gold, Resurrection Scroll(90) x1, Upgrade Scroll x4 | ok |
| 140 | Undying Hunt | 50-83 | 2 | 33 | 500 -> **225** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Power of Lion Scroll(Stat)(L) x1, 65 NP | NO SPAWN |
| 141 | Skeleton Hunt | 50-83 | 2 | 31 | 500 -> **235** | 500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Power of Lion Scroll(Stat)(L) x1, Blessed Upgrade Scroll x1 | NO SPAWN |
| 142 | Smilodon Hunt | 50-83 | 2 | 37 | 500 -> **205** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of Armor 350 x1, Blue Treasure Chest x1 | NO SPAWN |
| 143 | Sabertooth Hunt | 50-83 | 2 | 41 | 500 -> **185** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of 2000HP Up x1, Blessed Upgrade Scroll x1 | ok |
| 144 | Machirodus Hunt | 50-83 | 2 | 43 | 500 -> **175** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, Dragon Box x1 | ok |
| 145 | Megantereon Hunt | 50-83 | 2 | 45 | 500 -> **170** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (S) x15, 95 NP | ok |
| 146 | Skeleton Knight Hunt | 50-83 | 2 | 35 | 500 -> **215** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Resurrection Scroll(90) x1, Accessory Enchant Scroll(MP) x1 | NO SPAWN |
| 147 | Skeleton Champion Hunt | 50-83 | 2 | 37 | 500 -> **205** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Resurrection Scroll(90) x1, 70 NP | NO SPAWN |
| 148 | Death Knight Hunt | 50-83 | 2 | 45 | 500 -> **170** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Power of Lion Scroll(Stat)(L) x1, Blessed Upgrade Scroll x1 | ok |
| 149 | Baron Hunt | 50-83 | 2 | 52 | 500 -> **140** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 320,000 Gold, Scroll of Armor 350 x1, Blue Treasure Chest x1 | ok |
| 150 | Cardinal Hunt | 50-83 | 2 | 54 | 500 -> **130** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 310,000 Gold, Scroll of 2000HP Up x1, Blessed Upgrade Scroll x1 | other zone only (71/72) |
| 151 | Scolar Hunt | 50-83 | 2 | 46 | 500 -> **165** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, Dragon Box x1 | ok |
| 152 | Garuna Hunt | 50-83 | 2 | 53 | 500 -> **135** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 310,000 Gold, Potion of Draki (S) x15, 100 NP | NO SPAWN |
| 153 | Haunga Hunt | 50-83 | 2 | 55 | 500 -> **125** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 670,000 EXP, 300,000 Gold, Potion of Draki (S) x15, Accessory Enchant Scroll(HP) x1 | other zone only (73) |
| 154 | Haunga Warrior Hunt | 50-83 | 2 | 60 | 500 -> **110** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Resurrection Scroll(90) x1, Upgrade Scroll x3 | ok |
| 155 | Uruk Hai Hunt | 50-83 | 2 | 60 | 500 -> **110** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Power of Lion Scroll(Stat)(L) x1, Black Lion Weapon Box x1 | ok |
| 156 | Uruk Blade Hunt | 50-83 | 2 | 70 | 500 -> **80** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Scroll of Armor 350 x1, Upgrade Scroll x4 | ok |
| 157 | Uruk Tron Hunt | 50-83 | 2 | 80 | 500 -> **65** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 220,000 Gold, Scroll of 2000HP Up x1, Blessed Upgrade Scroll x1 | ok |
| 158 | Kobold Hunt | 50-83 | 2 | 30 | 500 -> **240** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, Dragon Box x1 | ok |
| 159 | Bugbear Hunt | 50-83 | 2 | 35 | 500 -> **215** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, 85 NP | ok |
| 160 | Pooka Hunt | 50-83 | 2 | 40 | 500 -> **190** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (S) x15, Accessory Enchant Scroll(INT) x1 | ok |
| 161 | Goblin Bouncer Hunt | 50-83 | 2 | 45 | 500 -> **170** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Resurrection Scroll(90) x1, Upgrade Scroll x3 | ok |
| 162 | Lamia Hunt | 50-83 | 2 | 60 | 500 -> **110** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Power of Lion Scroll(Stat)(L) x1, Black Lion Weapon Box x1 | NO SPAWN |
| 163 | Lamiros Hunt | 50-83 | 2 | 70 | 500 -> **80** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Scroll of Armor 350 x1, Upgrade Scroll x4 | NO SPAWN |
| 164 | Ape Hunt | 50-83 | 2 | 42 | 500 -> **180** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Scroll of 2000HP Up x1, 70 NP | NO SPAWN |
| 165 | Tyon Hunt | 50-83 | 2 | 55 | 500 -> **125** | 2,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 670,000 EXP, 300,000 Gold, Scroll of 2000HP Up x1, Blessed Upgrade Scroll x1 | NO SPAWN |
| 166 | Gray Oozy Hunt | 50-83 | 2 | 40 | 500 -> **190** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 340,000 Gold, Potion of Draki (L) x10, Blue Treasure Chest x1 | NO SPAWN |
| 167 | Lich Hunt | 50-83 | 2 | 70 | 500 -> **80** | 2,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Potion of Draki (S) x15, Accessory Enchant Scroll(STR) x1 | ok |
| 168 | Grell Hunt | 50-83 | 2 | 60 | 500 -> **110** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Resurrection Scroll(90) x1, Upgrade Scroll x3 | NO SPAWN |
| 169 | Ancient Hunt | 50-83 | 2 | 50 | 500 -> **145** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 500,000 EXP, 320,000 Gold, Power of Lion Scroll(Stat)(L) x1, Black Lion Weapon Box x1 | NO SPAWN |
| 170 | Treant Hunt | 50-83 | 2 | 55 | 500 -> **125** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 670,000 EXP, 300,000 Gold, Scroll of Armor 350 x1, Upgrade Scroll x4 | ok |
| 171 | Deruvish Hunt | 50-83 | 2 | 60 | 500 -> **110** | 3,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 1,500,000 EXP, 290,000 Gold, Scroll of Armor 350 x1, 85 NP | NO SPAWN |
| 172 | Apostle Hunt | 50-83 | 2 | 70 | 500 -> **80** | 3,500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Scroll of 2000HP Up x1, Blessed Upgrade Scroll x1 | NO SPAWN |
| 173 | Harpy Hunt | 50-83 | 2 | 70 | 500 -> **80** | 3,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Potion of Draki (L) x10, Blue Treasure Chest x1 | ok |
| 174 | Raven Harpy Hunt | 50-83 | 2 | 80 | 500 -> **65** | 3,500,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 220,000 Gold, Potion of Draki (S) x15, Blessed Upgrade Scroll x1 | ok |
| 175 | Stone Golem Hunt | 50-83 | 2 | 100 | 500 -> **35** | 4,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 150,000 Gold, Resurrection Scroll(90) x1, Dragon Box x1 | ok |
| 176 | Giant Golem Hunt | 50-83 | 2 | 110 | 500 -> **30** | 5,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 140,000 Gold, Power of Lion Scroll(Stat)(L) x1, 150 NP | ok |
| 177 | Troll Hunt | 50-83 | 2 | 70 | 500 -> **80** | 4,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 240,000 Gold, Power of Lion Scroll(Stat)(L) x1, Upgrade Scroll x4 | NO SPAWN |
| 178 | Troll Warrior Hunt | 50-83 | 2 | 80 | 500 -> **65** | 5,000,000 EXP, Blessed Upgrade Scroll x1, Gold Coin x1 | 4,300,000 EXP, 220,000 Gold, Scroll of Armor 350 x1, 100 NP | other zone only (11/12) |
| 179 | Deruvish Hunt | 60-83 | 11 | 60 | 250 -> **120** | 3,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 1,100,000 EXP, 310,000 Gold, Potion of Draki (L) x15, 150 NP | other zone only (1/2) |
| 180 | Apostle Hunt | 60-83 | 11 | 70 | 250 -> **90** | 4,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,700,000 EXP, 270,000 Gold, Power of Lion Scroll(Stat)(L) x1, Blue Treasure Chest x1 | other zone only (1/2) |
| 181 | Harpy Hunt | 60-83 | 11 | 70 | 250 -> **90** | 3,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,700,000 EXP, 270,000 Gold, Scroll of Armor 350 x1, Blessed Upgrade Scroll x2 | other zone only (31) |
| 182 | Raven Harpy Hunt | 60-83 | 11 | 80 | 250 -> **65** | 4,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 5,000,000 EXP, 220,000 Gold, Scroll of 2000HP Up x1, Black Lion Weapon Box x2 | ok |
| 183 | Stone Golem Hunt | 60-83 | 11 | 100 | 250 -> **35** | 3,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 5,000,000 EXP, 150,000 Gold, Potion of Crisis [Limited Edition] x1, Blessed Upgrade Scroll x1 | ok |
| 184 | Giant Golem Hunt | 60-83 | 11 | 110 | 250 -> **30** | 4,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 5,000,000 EXP, 140,000 Gold, Ibexs potion [Limited edition] x1, 150 NP | NO SPAWN |
| 185 | Troll Hunt | 60-83 | 11 | 70 | 250 -> **90** | 3,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,700,000 EXP, 270,000 Gold, Ibexs potion [Limited edition] x1, Accessory Enchant Scroll(HP) x2 | NO SPAWN |
| 186 | Troll Warrior Hunt | 60-83 | 11 | 80 | 250 -> **65** | 4,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 5,000,000 EXP, 220,000 Gold, Potion of Draki (L) x15, Akara Accessory Box x1 | ok |
| 187 | Troll Berserker Hunt | 60-83 | 11 | 90 | 250 -> **50** | 4,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 5,000,000 EXP, 190,000 Gold, Power of Lion Scroll(Stat)(L) x1, Upgrade Scroll x5 | ok |
| 188 | Troll Captain Hunt | 60-83 | 11 | 100 | 250 -> **35** | 5,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 5,000,000 EXP, 150,000 Gold, Scroll of Armor 350 x1, Dragon Box x1 | ok |
| 189 | Uruk Hai Hunt | 60-83 | 11 | 60 | 250 -> **120** | 800,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 1,100,000 EXP, 310,000 Gold, Scroll of 2000HP Up x1, Black Lion Weapon Box x2 | other zone only (1/2) |
| 190 | Uruk Blade Hunt | 60-83 | 11 | 70 | 250 -> **90** | 1,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,700,000 EXP, 270,000 Gold, Potion of Crisis [Limited Edition] x1, Blessed Upgrade Scroll x1 | other zone only (1/2) |
| 191 | Uruk Tron Hunt | 60-83 | 11 | 80 | 250 -> **65** | 1,100,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 5,000,000 EXP, 220,000 Gold, Potion of Crisis [Limited Edition] x1, 140 NP | other zone only (1/2) |
| 192 | Deruvish Hunt | 50-83 | 71 | 60 | 250 -> **110** | 6,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 1,500,000 EXP, 290,000 Gold, Scroll of 2000HP Up x1, Blessed Upgrade Scroll x1 | NO SPAWN |
| 193 | Apostle Hunt | 50-83 | 71 | 70 | 250 -> **80** | 8,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,300,000 EXP, 240,000 Gold, Potion of Draki (L) x10, Blue Treasure Chest x1 | other zone only (73) |
| 194 | Harpy Hunt | 50-83 | 71 | 70 | 250 -> **80** | 6,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,300,000 EXP, 240,000 Gold, Potion of Draki (S) x15, Blessed Upgrade Scroll x1 | NO SPAWN |
| 195 | Raven Harpy Hunt | 50-83 | 71 | 80 | 250 -> **65** | 8,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,300,000 EXP, 220,000 Gold, Resurrection Scroll(90) x1, Dragon Box x1 | other zone only (73) |
| 196 | Stone Golem Hunt | 50-83 | 71 | 100 | 250 -> **35** | 7,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,300,000 EXP, 150,000 Gold, Resurrection Scroll(90) x1, 145 NP | NO SPAWN |
| 197 | Giant Golem Hunt | 50-83 | 71 | 110 | 250 -> **30** | 8,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,300,000 EXP, 140,000 Gold, Power of Lion Scroll(Stat)(L) x1, Accessory Enchant Scroll(DEX) x1 | other zone only (12) |
| 198 | Troll Hunt | 50-83 | 71 | 70 | 250 -> **80** | 8,000,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,300,000 EXP, 240,000 Gold, Scroll of Armor 350 x1, Upgrade Scroll x3 | ok |
| 199 | Troll Warrior Hunt | 50-83 | 71 | 80 | 250 -> **65** | 9,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 4,300,000 EXP, 220,000 Gold, Scroll of 2000HP Up x1, Black Lion Weapon Box x1 | ok |
| 200 | Death Knight Hunt | 65-83 | 71 | 45 | 250 -> **155** | 9,500,000 EXP, Blessed Upgrade Scroll x1, Silver bar x1 | 2,900,000 EXP, 310,000 Gold, Ibexs potion [Limited edition] x1, Blessed Upgrade Scroll x2 | NO SPAWN |

## Nation filter and level rules (2026-10-18, dq-nation-patch)

* A quest belongs to a nation by its zone (`daily_quest::quest_nation`): zone 1 / 11 = Karus, zone 2 / 12 = El Morad,
  everything else (Moradon 21, Ronark 71, any) is neutral. Karus players never see, auto-start, accept or count El Morad
  quests and vice versa (quest list, progress list, `auto_start_all`, `quest_accept_check`, kill counting). No migration needed
  (derived from `zone_id`); progress rows already stored for the other nation are simply hidden/ignored.
* Level rule is now only the quest's own `min_level..max_level` (inclusive) everywhere. The old extra rule "a player standing in
  Moradon only counts home-zone quests up to level 35" made level 36-83 players lose quests that the list showed as available
  (their mobs only spawn in Moradon); it is gone. Level-ups (and the GM panel level edit) start the newly eligible quests only after
  the new level is stored.
* Kill counter overlay (client): a solid dark panel with a gold/green border centred just below the target HP bar
  (`target_hp.cpp::TargetHp_BarAnchor`); `JstKO\daily_hud.ini` `[hud] x= y=` are now small nudges from that anchor.
