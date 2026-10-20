# Box rewards (boxlar.xls + Red Gem pool)

Migration: `migrations/20261018320000_box_rewards.sql` (idempotent, reserved `item_exchange.n_index` range 2100000000..2100007851, 7852 rows). Break logic: `box_breaker.rs` / `bifrost_piece_exchange` build a pool of `exchange_item_count1 / 5` slots per row (rows with `random_flag IN (1,2,3)`); the chance of a row is its slots / total slots.

## Rate model

* Every managed box gets exactly **10000 slots** (50000 weight, the generator's cap): **8500 slots = 85 %** Red Gem rewards, **1500 slots = 15 %** the box's own rewards (largest-remainder rounding, at least 1 slot = 0.01 % per item; weights are multiples of 5).
* Red Gem part: the 74 Red Gem rows (389197000, `random_flag` 1..3) are merged per item (the pool lists some items twice) and keep their relative weights; the 20 upgraded `(+3)` items are replaced by their +0 twin (same weight, summed with an existing entry of the same item). Nothing had to be dropped. The Red Gem box itself is **not modified** (it is the source) -- it still contains its original `(+3)` rows; change it separately if wanted.
* Own part: existing own rows keep their relative weights (rarer = lower); new boxes use tiers (common 4 : uncommon 2 : rare 1 per item) or the box's existing exchange rows (random_flag 101, used as-is by the Kaira/Moira NPCs) as relative weights.
* Only plain +0 items: every `(+N)` reward was replaced by its +0 twin (same family) or removed if no twin exists (listed below). Quest/placeholder boxes are skipped.
* Own rewards of themed boxes may be level/race restricted items (class armour sets, `<selfname>` weapons) exactly as in the existing boxes.

## Red Gem rewards (identical in every managed box except the 10000-slot own-less case, none left)

| Item | id | slots of 10000 | % of box |
|---|---|---|---|
| Blessed Upgrade Scroll | 379021000 | 782 | 7.82 % |
| Raptor | 156210000 | 337 | 3.37 % |
| Shard | 111210000 | 337 | 3.37 % |
| Weapon Enchant Scroll | 389155000 | 303 | 3.03 % |
| Armor Enchant Scroll | 389156000 | 303 | 3.03 % |
| Chitin Shell Pauldron | 206001000 | 270 | 2.70 % |
| Chitin Shell Pads | 206002000 | 270 | 2.70 % |
| Chitin Shell Helmet | 206003000 | 270 | 2.70 % |
| Chitin Shell Gauntlets | 206004000 | 270 | 2.70 % |
| Chitin Shell Boots | 206005000 | 270 | 2.70 % |
| Rogue Chitin Shell Pauldron | 246001000 | 270 | 2.70 % |
| Rogue Chitin Shell Pads | 246002000 | 270 | 2.70 % |
| Rogue Chitin Shell Helmet | 246003000 | 270 | 2.70 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 270 | 2.70 % |
| Rogue Chitin Shell Boots | 246005000 | 270 | 2.70 % |
| Iron Impact | 146210000 | 168 | 1.68 % |
| Iron Bow | 168410000 | 168 | 1.68 % |
| Mirage | 126410000 | 168 | 1.68 % |
| Mirage Dagger | 111110000 | 168 | 1.68 % |
| Totamic Spear | 151210000 | 168 | 1.68 % |
| Graham | 121310000 | 168 | 1.68 % |
| Deep Scar | 131110000 | 168 | 1.68 % |
| Impact | 141110000 | 168 | 1.68 % |
| Priest Impact | 191110000 | 168 | 1.68 % |
| Elixir Staff | 181110000 | 168 | 1.68 % |
| Priest Chitin Shell Pauldron | 286001000 | 121 | 1.21 % |
| Priest Chitin Shell Pads | 286002000 | 121 | 1.21 % |
| Priest Chitin Shell Helmet | 286003000 | 121 | 1.21 % |
| Priest Chitin Shell Gauntlets | 286004000 | 121 | 1.21 % |
| Priest Chitin Shell Boots | 286005000 | 121 | 1.21 % |
| Complete Robe | 266001000 | 121 | 1.21 % |
| Complete Pants | 266002000 | 121 | 1.21 % |
| Complete Helmet | 266003000 | 121 | 1.21 % |
| Complete Glove | 266004000 | 121 | 1.21 % |
| Complete Boots | 266005000 | 121 | 1.21 % |
| Spear of Murky Waters | 159302311 | 101 | 1.01 % |
| Stone Splitter | 199102262 | 101 | 1.01 % |
| Scorching Staff | 189302255 | 88 | 0.88 % |
| Chaotic Staff | 189202257 | 88 | 0.88 % |
| Oasis Staff | 189102256 | 88 | 0.88 % |
| Light Belt of Life | 340110120 | 61 | 0.61 % |
| Dread Shield | 179102191 | 47 | 0.47 % |
| Cold-Hearted Dagger | 119102294 | 47 | 0.47 % |
| Eagle's Eye | 169302345 | 47 | 0.47 % |
| Judicious Belt | 340610122 | 30 | 0.30 % |
| Sash of Sorrows | 340510121 | 30 | 0.30 % |
| Silver bar | 379067000 | 24 | 0.24 % |
| String of Skulls | 340410123 | 14 | 0.14 % |
| Legionnaire Band | 330151461 | 14 | 0.14 % |
| Hero's Valor | 330111280 | 14 | 0.14 % |
| Minotaur's Earrings | 310511119 | 14 | 0.14 % |
| Cockatrice's Earrings | 310512120 | 14 | 0.14 % |
| Lillime's Earrings | 310514122 | 13 | 0.13 % |
| Lich King's Earrings | 310513121 | 13 | 0.13 % |

Red Gem share total: 8500 slots = 85 %.

## Boxes

`kind`: existing = was already breakable (own rewards lowered from their old weight to 15 %), new = made breakable by this migration.

### Red Treasure Chest [379154000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 32 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Iron Bow | 168410000 | 50 | 0.50 % |
| Enion Bow | 169101104 | 50 | 0.50 % |
| Defender of the Lord | 179101110 | 50 | 0.50 % |
| Elixir Staff | 181110000 | 50 | 0.50 % |
| Wrath of Erenion | 189101106 | 50 | 0.50 % |
| Glacier Erenion | 189201107 | 50 | 0.50 % |
| Lightning Erenion | 189301108 | 50 | 0.50 % |
| Ron's Staff | 189401109 | 50 | 0.50 % |
| Priest Impact | 191110000 | 50 | 0.50 % |
| Smite Hammer | 199101105 | 50 | 0.50 % |
| Chitin Shell Pauldron | 206001000 | 50 | 0.50 % |
| Chitin Shell Pads | 206002000 | 50 | 0.50 % |
| Chitin Shell Helmet | 206003000 | 50 | 0.50 % |
| Chitin Shell Gauntlets | 206004000 | 50 | 0.50 % |
| Chitin Shell Boots | 206005000 | 50 | 0.50 % |
| Rogue Chitin Shell Pauldron | 246001000 | 50 | 0.50 % |
| Rogue Chitin Shell Pads | 246002000 | 50 | 0.50 % |
| Rogue Chitin Shell Helmet | 246003000 | 50 | 0.50 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 49 | 0.49 % |
| Rogue Chitin Shell Boots | 246005000 | 49 | 0.49 % |
| Complete Robe | 266001000 | 49 | 0.49 % |
| Complete Pants | 266002000 | 49 | 0.49 % |
| Complete Helmet | 266003000 | 49 | 0.49 % |
| Complete Glove | 266004000 | 49 | 0.49 % |
| Complete Boots | 266005000 | 49 | 0.49 % |
| Priest Chitin Shell Pauldron | 286001000 | 49 | 0.49 % |
| Priest Chitin Shell Pads | 286002000 | 49 | 0.49 % |
| Priest Chitin Shell Helmet | 286003000 | 49 | 0.49 % |
| Priest Chitin Shell Gauntlets | 286004000 | 49 | 0.49 % |
| Priest Chitin Shell Boots | 286005000 | 49 | 0.49 % |
| Shard | 111210000 | 1 | 0.01 % |
| Dark Vane | 119101101 | 1 | 0.01 % |
| Graham | 121310000 | 1 | 0.01 % |
| Mirage | 126410000 | 1 | 0.01 % |
| Hanguk Sword | 129101102 | 1 | 0.01 % |
| Deep Scar | 131110000 | 1 | 0.01 % |
| Blade Axe | 136210000 | 1 | 0.01 % |
| Impact | 141110000 | 1 | 0.01 % |
| Iron Impact | 146210000 | 1 | 0.01 % |
| Hell Breaker | 149101103 | 1 | 0.01 % |
| Totamic Spear | 151210000 | 1 | 0.01 % |
| Raptor | 156210000 | 1 | 0.01 % |

### Green Treasure Chest [379155000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 32 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Enion Bow | 169101104 | 46 | 0.46 % |
| Defender of the Lord | 179101110 | 46 | 0.46 % |
| Elixir Staff | 181110000 | 46 | 0.46 % |
| Wrath of Erenion | 189101106 | 46 | 0.46 % |
| Glacier Erenion | 189201107 | 46 | 0.46 % |
| Lightning Erenion | 189301108 | 46 | 0.46 % |
| Ron's Staff | 189401109 | 46 | 0.46 % |
| Priest Impact | 191110000 | 46 | 0.46 % |
| Smite Hammer | 199101105 | 46 | 0.46 % |
| Chitin Shell Pauldron | 206001000 | 46 | 0.46 % |
| Chitin Shell Pads | 206002000 | 46 | 0.46 % |
| Chitin Shell Helmet | 206003000 | 46 | 0.46 % |
| Chitin Shell Gauntlets | 206004000 | 46 | 0.46 % |
| Chitin Shell Boots | 206005000 | 46 | 0.46 % |
| Rogue Chitin Shell Pauldron | 246001000 | 45 | 0.45 % |
| Rogue Chitin Shell Pads | 246002000 | 45 | 0.45 % |
| Rogue Chitin Shell Helmet | 246003000 | 45 | 0.45 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 45 | 0.45 % |
| Rogue Chitin Shell Boots | 246005000 | 45 | 0.45 % |
| Complete Robe | 266001000 | 45 | 0.45 % |
| Complete Pants | 266002000 | 45 | 0.45 % |
| Complete Helmet | 266003000 | 45 | 0.45 % |
| Complete Glove | 266004000 | 45 | 0.45 % |
| Complete Boots | 266005000 | 45 | 0.45 % |
| Priest Chitin Shell Pauldron | 286001000 | 45 | 0.45 % |
| Priest Chitin Shell Pads | 286002000 | 45 | 0.45 % |
| Priest Chitin Shell Helmet | 286003000 | 45 | 0.45 % |
| Priest Chitin Shell Gauntlets | 286004000 | 45 | 0.45 % |
| Priest Chitin Shell Boots | 286005000 | 45 | 0.45 % |
| Dark Vane | 119101101 | 14 | 0.14 % |
| Graham | 121310000 | 14 | 0.14 % |
| Mirage | 126410000 | 14 | 0.14 % |
| Hanguk Sword | 129101102 | 14 | 0.14 % |
| Deep Scar | 131110000 | 14 | 0.14 % |
| Blade Axe | 136210000 | 14 | 0.14 % |
| Impact | 141110000 | 14 | 0.14 % |
| Iron Impact | 146210000 | 14 | 0.14 % |
| Hell Breaker | 149101103 | 14 | 0.14 % |
| Totamic Spear | 151210000 | 14 | 0.14 % |
| Raptor | 156210000 | 14 | 0.14 % |
| Iron Bow | 168410000 | 14 | 0.14 % |
| Shard | 111210000 | 13 | 0.13 % |

### Blue Treasure Chest [379156000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 32 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Chitin Shell Gauntlets | 206004000 | 60 | 0.60 % |
| Chitin Shell Boots | 206005000 | 60 | 0.60 % |
| Rogue Chitin Shell Pauldron | 246001000 | 60 | 0.60 % |
| Shard | 111210000 | 49 | 0.49 % |
| Dark Vane | 119101101 | 49 | 0.49 % |
| Graham | 121310000 | 49 | 0.49 % |
| Mirage | 126410000 | 49 | 0.49 % |
| Hanguk Sword | 129101102 | 49 | 0.49 % |
| Deep Scar | 131110000 | 49 | 0.49 % |
| Blade Axe | 136210000 | 49 | 0.49 % |
| Impact | 141110000 | 49 | 0.49 % |
| Iron Impact | 146210000 | 49 | 0.49 % |
| Hell Breaker | 149101103 | 49 | 0.49 % |
| Totamic Spear | 151210000 | 49 | 0.49 % |
| Raptor | 156210000 | 49 | 0.49 % |
| Iron Bow | 168410000 | 49 | 0.49 % |
| Rogue Chitin Shell Pads | 246002000 | 44 | 0.44 % |
| Rogue Chitin Shell Helmet | 246003000 | 44 | 0.44 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 44 | 0.44 % |
| Rogue Chitin Shell Boots | 246005000 | 44 | 0.44 % |
| Complete Robe | 266001000 | 44 | 0.44 % |
| Complete Pants | 266002000 | 44 | 0.44 % |
| Complete Helmet | 266003000 | 44 | 0.44 % |
| Complete Glove | 266004000 | 44 | 0.44 % |
| Complete Boots | 266005000 | 44 | 0.44 % |
| Priest Chitin Shell Pauldron | 286001000 | 43 | 0.43 % |
| Priest Chitin Shell Pads | 286002000 | 43 | 0.43 % |
| Priest Chitin Shell Helmet | 286003000 | 43 | 0.43 % |
| Priest Chitin Shell Gauntlets | 286004000 | 43 | 0.43 % |
| Priest Chitin Shell Boots | 286005000 | 43 | 0.43 % |
| Enion Bow | 169101104 | 6 | 0.06 % |
| Defender of the Lord | 179101110 | 6 | 0.06 % |
| Elixir Staff | 181110000 | 6 | 0.06 % |
| Wrath of Erenion | 189101106 | 6 | 0.06 % |
| Glacier Erenion | 189201107 | 6 | 0.06 % |
| Lightning Erenion | 189301108 | 6 | 0.06 % |
| Ron's Staff | 189401109 | 6 | 0.06 % |
| Priest Impact | 191110000 | 6 | 0.06 % |
| Smite Hammer | 199101105 | 6 | 0.06 % |
| Chitin Shell Pauldron | 206001000 | 6 | 0.06 % |
| Chitin Shell Pads | 206002000 | 6 | 0.06 % |
| Chitin Shell Helmet | 206003000 | 6 | 0.06 % |

### Fragment of Arrogance [389160000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 47 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 82 | 0.82 % |
| Weapon Enchant Scroll | 389155000 | 82 | 0.82 % |
| Mirage Dagger | 111110000 | 47 | 0.47 % |
| Cleaver | 111010000 | 47 | 0.47 % |
| Priest Bone crasher | 191010000 | 47 | 0.47 % |
| Priest Cracker | 190910000 | 47 | 0.47 % |
| Salamander Staff | 181010000 | 47 | 0.47 % |
| Tomahawk | 131010000 | 47 | 0.47 % |
| Philum | 151010000 | 47 | 0.47 % |
| Harpoon | 151110000 | 47 | 0.47 % |
| Bardish | 156010000 | 47 | 0.47 % |
| Glave | 156110000 | 46 | 0.46 % |
| Bone Cleaver | 136110000 | 46 | 0.46 % |
| Totamic Club | 146110000 | 46 | 0.46 % |
| Iron Crossbow | 168310000 | 46 | 0.46 % |
| Slayer | 121210000 | 46 | 0.46 % |
| Chitin Armor Pauldron | 205001000 | 29 | 0.29 % |
| Chitin Armor Pads | 205002000 | 29 | 0.29 % |
| Chitin Helmet | 205003000 | 29 | 0.29 % |
| Chitin Gauntlets | 205004000 | 29 | 0.29 % |
| Chitin Boots | 205005000 | 29 | 0.29 % |
| Rogue Chitin Armor Pauldron | 245001000 | 29 | 0.29 % |
| Rogue Chitin Armor Pads | 245002000 | 29 | 0.29 % |
| Rogue Chitin Helmet | 245003000 | 29 | 0.29 % |
| Rogue Chitin Gauntlets | 245004000 | 29 | 0.29 % |
| Rogue Chitin Boots | 245005000 | 29 | 0.29 % |
| Priest Chitin Armor Pauldron | 285001000 | 29 | 0.29 % |
| Priest Chitin Armor Pads | 285002000 | 29 | 0.29 % |
| Priest Chitin Helmet | 285003000 | 29 | 0.29 % |
| Priest Chitin Gauntlets | 285004000 | 29 | 0.29 % |
| Priest Chitin Boots | 285005000 | 29 | 0.29 % |
| Crimson Robe | 265001000 | 29 | 0.29 % |
| Crimson Pants | 265002000 | 29 | 0.29 % |
| Crimson Helmet | 265003000 | 29 | 0.29 % |
| Crimson Gloves | 265004000 | 29 | 0.29 % |
| Crimson Boots | 265005000 | 29 | 0.29 % |
| Bronze Coin | 379110000 | 11 | 0.11 % |
| Blessed Upgrade Scroll | 379021000 | 11 | 0.11 % |
| Silver Coin | 379109000 | 4 | 0.04 % |
| Shard | 111210000 | 3 | 0.03 % |
| Raptor | 156210000 | 3 | 0.03 % |
| Iron Impact | 146210000 | 3 | 0.03 % |
| Iron Bow | 168410000 | 3 | 0.03 % |
| Totamic Spear | 151210000 | 3 | 0.03 % |
| Impact | 141110000 | 3 | 0.03 % |
| Priest Impact | 191110000 | 3 | 0.03 % |
| Elixir Staff | 181110000 | 3 | 0.03 % |
| Durandal | 126310000 | 3 | 0.03 % |
| Mirage | 126410000 | 3 | 0.03 % |
| Deep Scar | 131110000 | 3 | 0.03 % |
| Graham | 121310000 | 3 | 0.03 % |
| Blade Axe | 136210000 | 3 | 0.03 % |
| Fragment of Arrogance | 389160000 | 3 | 0.03 % |
| Fragment of Gluttony | 389161000 | 3 | 0.03 % |
| Fragment of Rage | 389162000 | 3 | 0.03 % |
| Fragment of Sloth | 389163000 | 3 | 0.03 % |
| Fragment of Lechery | 389164000 | 3 | 0.03 % |
| Fragment of Jealousy | 389165000 | 3 | 0.03 % |
| Gold Coin | 379107000 | 3 | 0.03 % |
| Sherion | 119301131 | 2 | 0.02 % |
| Fragment of Avarice | 389166000 | 1 | 0.01 % |
| Gold bar | 379068000 | 1 | 0.01 % |
| Silver bar | 379067000 | 1 | 0.01 % |
| Foverin | 330940143 | 1 | 0.01 % |
| Shio Tears | 330920141 | 1 | 0.01 % |
| Flame Ring | 330910140 | 1 | 0.01 % |
| Avedon | 139301133 | 1 | 0.01 % |
| Helenid | 169301134 | 1 | 0.01 % |
| Hell Blood | 189601135 | 1 | 0.01 % |
| Elysium | 189603137 | 1 | 0.01 % |
| Garp | 189602136 | 1 | 0.01 % |
| Syphioric | 179301139 | 1 | 0.01 % |
| Lugias | 159301132 | 1 | 0.01 % |
| Holy Animor | 199301138 | 1 | 0.01 % |
| Imir Ring | 330930142 | 1 | 0.01 % |

### Fragment of Gluttony [389161000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 47 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 81 | 0.81 % |
| Weapon Enchant Scroll | 389155000 | 81 | 0.81 % |
| Bardish | 156010000 | 47 | 0.47 % |
| Glave | 156110000 | 47 | 0.47 % |
| Bone Cleaver | 136110000 | 47 | 0.47 % |
| Totamic Club | 146110000 | 47 | 0.47 % |
| Iron Crossbow | 168310000 | 46 | 0.46 % |
| Slayer | 121210000 | 46 | 0.46 % |
| Mirage Dagger | 111110000 | 46 | 0.46 % |
| Cleaver | 111010000 | 46 | 0.46 % |
| Priest Bone crasher | 191010000 | 46 | 0.46 % |
| Priest Cracker | 190910000 | 46 | 0.46 % |
| Salamander Staff | 181010000 | 46 | 0.46 % |
| Tomahawk | 131010000 | 46 | 0.46 % |
| Philum | 151010000 | 46 | 0.46 % |
| Harpoon | 151110000 | 46 | 0.46 % |
| Chitin Armor Pauldron | 205001000 | 29 | 0.29 % |
| Chitin Armor Pads | 205002000 | 29 | 0.29 % |
| Chitin Helmet | 205003000 | 29 | 0.29 % |
| Chitin Gauntlets | 205004000 | 29 | 0.29 % |
| Chitin Boots | 205005000 | 29 | 0.29 % |
| Rogue Chitin Armor Pauldron | 245001000 | 29 | 0.29 % |
| Rogue Chitin Armor Pads | 245002000 | 29 | 0.29 % |
| Rogue Chitin Helmet | 245003000 | 29 | 0.29 % |
| Rogue Chitin Gauntlets | 245004000 | 29 | 0.29 % |
| Rogue Chitin Boots | 245005000 | 29 | 0.29 % |
| Priest Chitin Armor Pauldron | 285001000 | 29 | 0.29 % |
| Priest Chitin Armor Pads | 285002000 | 29 | 0.29 % |
| Priest Chitin Helmet | 285003000 | 29 | 0.29 % |
| Priest Chitin Gauntlets | 285004000 | 29 | 0.29 % |
| Priest Chitin Boots | 285005000 | 29 | 0.29 % |
| Crimson Robe | 265001000 | 29 | 0.29 % |
| Crimson Pants | 265002000 | 29 | 0.29 % |
| Crimson Helmet | 265003000 | 29 | 0.29 % |
| Crimson Gloves | 265004000 | 29 | 0.29 % |
| Crimson Boots | 265005000 | 29 | 0.29 % |
| Bronze Coin | 379110000 | 12 | 0.12 % |
| Blessed Upgrade Scroll | 379021000 | 12 | 0.12 % |
| Silver Coin | 379109000 | 5 | 0.05 % |
| Fragment of Arrogance | 389160000 | 3 | 0.03 % |
| Fragment of Gluttony | 389161000 | 3 | 0.03 % |
| Fragment of Rage | 389162000 | 3 | 0.03 % |
| Fragment of Sloth | 389163000 | 3 | 0.03 % |
| Fragment of Lechery | 389164000 | 3 | 0.03 % |
| Fragment of Jealousy | 389165000 | 3 | 0.03 % |
| Gold Coin | 379107000 | 3 | 0.03 % |
| Shard | 111210000 | 3 | 0.03 % |
| Raptor | 156210000 | 3 | 0.03 % |
| Iron Impact | 146210000 | 3 | 0.03 % |
| Iron Bow | 168410000 | 3 | 0.03 % |
| Totamic Spear | 151210000 | 3 | 0.03 % |
| Impact | 141110000 | 3 | 0.03 % |
| Priest Impact | 191110000 | 3 | 0.03 % |
| Elixir Staff | 181110000 | 3 | 0.03 % |
| Durandal | 126310000 | 3 | 0.03 % |
| Mirage | 126410000 | 3 | 0.03 % |
| Deep Scar | 131110000 | 3 | 0.03 % |
| Graham | 121310000 | 3 | 0.03 % |
| Blade Axe | 136210000 | 3 | 0.03 % |
| Silver bar | 379067000 | 2 | 0.02 % |
| Foverin | 330940143 | 2 | 0.02 % |
| Shio Tears | 330920141 | 2 | 0.02 % |
| Imir Ring | 330930142 | 2 | 0.02 % |
| Flame Ring | 330910140 | 2 | 0.02 % |
| Fragment of Avarice | 389166000 | 1 | 0.01 % |
| Gold bar | 379068000 | 1 | 0.01 % |
| Sherion | 119301131 | 1 | 0.01 % |
| Avedon | 139301133 | 1 | 0.01 % |
| Helenid | 169301134 | 1 | 0.01 % |
| Hell Blood | 189601135 | 1 | 0.01 % |
| Elysium | 189603137 | 1 | 0.01 % |
| Garp | 189602136 | 1 | 0.01 % |
| Syphioric | 179301139 | 1 | 0.01 % |
| Lugias | 159301132 | 1 | 0.01 % |
| Holy Animor | 199301138 | 1 | 0.01 % |

### Fragment of Rage [389162000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 47 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 81 | 0.81 % |
| Weapon Enchant Scroll | 389155000 | 81 | 0.81 % |
| Priest Bone crasher | 191010000 | 46 | 0.46 % |
| Priest Cracker | 190910000 | 46 | 0.46 % |
| Salamander Staff | 181010000 | 46 | 0.46 % |
| Tomahawk | 131010000 | 46 | 0.46 % |
| Philum | 151010000 | 46 | 0.46 % |
| Harpoon | 151110000 | 46 | 0.46 % |
| Bardish | 156010000 | 46 | 0.46 % |
| Glave | 156110000 | 46 | 0.46 % |
| Bone Cleaver | 136110000 | 46 | 0.46 % |
| Totamic Club | 146110000 | 46 | 0.46 % |
| Iron Crossbow | 168310000 | 46 | 0.46 % |
| Slayer | 121210000 | 46 | 0.46 % |
| Mirage Dagger | 111110000 | 46 | 0.46 % |
| Cleaver | 111010000 | 46 | 0.46 % |
| Crimson Gloves | 265004000 | 29 | 0.29 % |
| Crimson Boots | 265005000 | 29 | 0.29 % |
| Chitin Gauntlets | 205004000 | 29 | 0.29 % |
| Chitin Boots | 205005000 | 29 | 0.29 % |
| Rogue Chitin Armor Pauldron | 245001000 | 29 | 0.29 % |
| Rogue Chitin Armor Pads | 245002000 | 29 | 0.29 % |
| Rogue Chitin Helmet | 245003000 | 29 | 0.29 % |
| Rogue Chitin Gauntlets | 245004000 | 29 | 0.29 % |
| Rogue Chitin Boots | 245005000 | 29 | 0.29 % |
| Priest Chitin Armor Pauldron | 285001000 | 29 | 0.29 % |
| Priest Chitin Armor Pads | 285002000 | 29 | 0.29 % |
| Priest Chitin Helmet | 285003000 | 29 | 0.29 % |
| Priest Chitin Gauntlets | 285004000 | 29 | 0.29 % |
| Priest Chitin Boots | 285005000 | 29 | 0.29 % |
| Crimson Robe | 265001000 | 29 | 0.29 % |
| Crimson Pants | 265002000 | 29 | 0.29 % |
| Crimson Helmet | 265003000 | 29 | 0.29 % |
| Chitin Armor Pauldron | 205001000 | 29 | 0.29 % |
| Chitin Armor Pads | 205002000 | 29 | 0.29 % |
| Chitin Helmet | 205003000 | 29 | 0.29 % |
| Bronze Coin | 379110000 | 12 | 0.12 % |
| Blessed Upgrade Scroll | 379021000 | 12 | 0.12 % |
| Silver Coin | 379109000 | 5 | 0.05 % |
| Fragment of Arrogance | 389160000 | 3 | 0.03 % |
| Fragment of Gluttony | 389161000 | 3 | 0.03 % |
| Fragment of Rage | 389162000 | 3 | 0.03 % |
| Fragment of Sloth | 389163000 | 3 | 0.03 % |
| Fragment of Lechery | 389164000 | 3 | 0.03 % |
| Fragment of Jealousy | 389165000 | 3 | 0.03 % |
| Gold Coin | 379107000 | 3 | 0.03 % |
| Shard | 111210000 | 3 | 0.03 % |
| Raptor | 156210000 | 3 | 0.03 % |
| Iron Impact | 146210000 | 3 | 0.03 % |
| Iron Bow | 168410000 | 3 | 0.03 % |
| Totamic Spear | 151210000 | 3 | 0.03 % |
| Impact | 141110000 | 3 | 0.03 % |
| Priest Impact | 191110000 | 3 | 0.03 % |
| Elixir Staff | 181110000 | 3 | 0.03 % |
| Durandal | 126310000 | 3 | 0.03 % |
| Mirage | 126410000 | 3 | 0.03 % |
| Deep Scar | 131110000 | 3 | 0.03 % |
| Graham | 121310000 | 3 | 0.03 % |
| Blade Axe | 136210000 | 3 | 0.03 % |
| Sherion | 119301131 | 2 | 0.02 % |
| Avedon | 139301133 | 2 | 0.02 % |
| Helenid | 169301134 | 2 | 0.02 % |
| Hell Blood | 189601135 | 2 | 0.02 % |
| Elysium | 189603137 | 2 | 0.02 % |
| Garp | 189602136 | 2 | 0.02 % |
| Syphioric | 179301139 | 2 | 0.02 % |
| Lugias | 159301132 | 2 | 0.02 % |
| Holy Animor | 199301138 | 2 | 0.02 % |
| Fragment of Avarice | 389166000 | 1 | 0.01 % |
| Gold bar | 379068000 | 1 | 0.01 % |
| Silver bar | 379067000 | 1 | 0.01 % |
| Foverin | 330940143 | 1 | 0.01 % |
| Shio Tears | 330920141 | 1 | 0.01 % |
| Imir Ring | 330930142 | 1 | 0.01 % |
| Flame Ring | 330910140 | 1 | 0.01 % |

### Fragment of Sloth [389163000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 48 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 81 | 0.81 % |
| Weapon Enchant Scroll | 389155000 | 81 | 0.81 % |
| Mirage Dagger | 111110000 | 47 | 0.47 % |
| Cleaver | 111010000 | 46 | 0.46 % |
| Priest Bone crasher | 191010000 | 46 | 0.46 % |
| Priest Cracker | 190910000 | 46 | 0.46 % |
| Salamander Staff | 181010000 | 46 | 0.46 % |
| Tomahawk | 131010000 | 46 | 0.46 % |
| Philum | 151010000 | 46 | 0.46 % |
| Harpoon | 151110000 | 46 | 0.46 % |
| Bardish | 156010000 | 46 | 0.46 % |
| Glave | 156110000 | 46 | 0.46 % |
| Bone Cleaver | 136110000 | 46 | 0.46 % |
| Totamic Club | 146110000 | 46 | 0.46 % |
| Iron Crossbow | 168310000 | 46 | 0.46 % |
| Slayer | 121210000 | 46 | 0.46 % |
| Chitin Armor Pauldron | 205001000 | 29 | 0.29 % |
| Chitin Armor Pads | 205002000 | 29 | 0.29 % |
| Chitin Helmet | 205003000 | 29 | 0.29 % |
| Chitin Gauntlets | 205004000 | 29 | 0.29 % |
| Chitin Boots | 205005000 | 29 | 0.29 % |
| Rogue Chitin Armor Pauldron | 245001000 | 29 | 0.29 % |
| Rogue Chitin Armor Pads | 245002000 | 29 | 0.29 % |
| Rogue Chitin Helmet | 245003000 | 29 | 0.29 % |
| Rogue Chitin Gauntlets | 245004000 | 29 | 0.29 % |
| Rogue Chitin Boots | 245005000 | 29 | 0.29 % |
| Priest Chitin Armor Pauldron | 285001000 | 29 | 0.29 % |
| Priest Chitin Armor Pads | 285002000 | 29 | 0.29 % |
| Priest Chitin Helmet | 285003000 | 29 | 0.29 % |
| Priest Chitin Gauntlets | 285004000 | 29 | 0.29 % |
| Priest Chitin Boots | 285005000 | 29 | 0.29 % |
| Crimson Robe | 265001000 | 29 | 0.29 % |
| Crimson Pants | 265002000 | 29 | 0.29 % |
| Crimson Helmet | 265003000 | 29 | 0.29 % |
| Crimson Gloves | 265004000 | 29 | 0.29 % |
| Crimson Boots | 265005000 | 29 | 0.29 % |
| Bronze Coin | 379110000 | 12 | 0.12 % |
| Blessed Upgrade Scroll | 379021000 | 12 | 0.12 % |
| Silver Coin | 379109000 | 5 | 0.05 % |
| Shard | 111210000 | 4 | 0.04 % |
| Fragment of Arrogance | 389160000 | 4 | 0.04 % |
| Fragment of Gluttony | 389161000 | 4 | 0.04 % |
| Fragment of Rage | 389162000 | 4 | 0.04 % |
| Fragment of Sloth | 389163000 | 4 | 0.04 % |
| Fragment of Lechery | 389164000 | 4 | 0.04 % |
| Fragment of Jealousy | 389165000 | 4 | 0.04 % |
| Gold Coin | 379107000 | 4 | 0.04 % |
| Raptor | 156210000 | 3 | 0.03 % |
| Iron Impact | 146210000 | 3 | 0.03 % |
| Iron Bow | 168410000 | 3 | 0.03 % |
| Totamic Spear | 151210000 | 3 | 0.03 % |
| Impact | 141110000 | 3 | 0.03 % |
| Priest Impact | 191110000 | 3 | 0.03 % |
| Elixir Staff | 181110000 | 3 | 0.03 % |
| Durandal | 126310000 | 3 | 0.03 % |
| Mirage | 126410000 | 3 | 0.03 % |
| Deep Scar | 131110000 | 3 | 0.03 % |
| Graham | 121310000 | 3 | 0.03 % |
| Blade Axe | 136210000 | 3 | 0.03 % |
| Silver bar | 379067000 | 2 | 0.02 % |
| Gold bar | 379068000 | 1 | 0.01 % |
| Foverin | 330940143 | 1 | 0.01 % |
| Shio Tears | 330920141 | 1 | 0.01 % |
| Imir Ring | 330930142 | 1 | 0.01 % |
| Flame Ring | 330910140 | 1 | 0.01 % |
| Sherion | 119301131 | 1 | 0.01 % |
| Avedon | 139301133 | 1 | 0.01 % |
| Helenid | 169301134 | 1 | 0.01 % |
| Hell Blood | 189601135 | 1 | 0.01 % |
| Elysium | 189603137 | 1 | 0.01 % |
| Garp | 189602136 | 1 | 0.01 % |
| Syphioric | 179301139 | 1 | 0.01 % |
| Lugias | 159301132 | 1 | 0.01 % |
| Holy Animor | 199301138 | 1 | 0.01 % |

### Fragment of Lechery [389164000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 47 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 77 | 0.77 % |
| Weapon Enchant Scroll | 389155000 | 77 | 0.77 % |
| Mirage Dagger | 111110000 | 44 | 0.44 % |
| Cleaver | 111010000 | 44 | 0.44 % |
| Priest Bone crasher | 191010000 | 44 | 0.44 % |
| Priest Cracker | 190910000 | 44 | 0.44 % |
| Salamander Staff | 181010000 | 44 | 0.44 % |
| Tomahawk | 131010000 | 44 | 0.44 % |
| Philum | 151010000 | 44 | 0.44 % |
| Harpoon | 151110000 | 44 | 0.44 % |
| Bardish | 156010000 | 44 | 0.44 % |
| Glave | 156110000 | 44 | 0.44 % |
| Bone Cleaver | 136110000 | 44 | 0.44 % |
| Totamic Club | 146110000 | 44 | 0.44 % |
| Iron Crossbow | 168310000 | 44 | 0.44 % |
| Slayer | 121210000 | 44 | 0.44 % |
| Crimson Pants | 265002000 | 28 | 0.28 % |
| Crimson Helmet | 265003000 | 28 | 0.28 % |
| Crimson Gloves | 265004000 | 28 | 0.28 % |
| Crimson Boots | 265005000 | 27 | 0.27 % |
| Priest Chitin Helmet | 285003000 | 27 | 0.27 % |
| Priest Chitin Gauntlets | 285004000 | 27 | 0.27 % |
| Priest Chitin Boots | 285005000 | 27 | 0.27 % |
| Crimson Robe | 265001000 | 27 | 0.27 % |
| Chitin Armor Pauldron | 205001000 | 27 | 0.27 % |
| Chitin Armor Pads | 205002000 | 27 | 0.27 % |
| Chitin Helmet | 205003000 | 27 | 0.27 % |
| Chitin Gauntlets | 205004000 | 27 | 0.27 % |
| Chitin Boots | 205005000 | 27 | 0.27 % |
| Rogue Chitin Armor Pauldron | 245001000 | 27 | 0.27 % |
| Rogue Chitin Armor Pads | 245002000 | 27 | 0.27 % |
| Rogue Chitin Helmet | 245003000 | 27 | 0.27 % |
| Rogue Chitin Gauntlets | 245004000 | 27 | 0.27 % |
| Rogue Chitin Boots | 245005000 | 27 | 0.27 % |
| Priest Chitin Armor Pauldron | 285001000 | 27 | 0.27 % |
| Priest Chitin Armor Pads | 285002000 | 27 | 0.27 % |
| Bronze Coin | 379110000 | 11 | 0.11 % |
| Blessed Upgrade Scroll | 379021000 | 11 | 0.11 % |
| Shard | 111210000 | 8 | 0.08 % |
| Raptor | 156210000 | 8 | 0.08 % |
| Iron Impact | 146210000 | 8 | 0.08 % |
| Iron Bow | 168410000 | 8 | 0.08 % |
| Totamic Spear | 151210000 | 8 | 0.08 % |
| Impact | 141110000 | 8 | 0.08 % |
| Priest Impact | 191110000 | 8 | 0.08 % |
| Elixir Staff | 181110000 | 8 | 0.08 % |
| Durandal | 126310000 | 8 | 0.08 % |
| Mirage | 126410000 | 8 | 0.08 % |
| Deep Scar | 131110000 | 8 | 0.08 % |
| Graham | 121310000 | 8 | 0.08 % |
| Blade Axe | 136210000 | 8 | 0.08 % |
| Silver Coin | 379109000 | 5 | 0.05 % |
| Sherion | 119301131 | 3 | 0.03 % |
| Avedon | 139301133 | 3 | 0.03 % |
| Helenid | 169301134 | 3 | 0.03 % |
| Hell Blood | 189601135 | 3 | 0.03 % |
| Elysium | 189603137 | 3 | 0.03 % |
| Garp | 189602136 | 3 | 0.03 % |
| Syphioric | 179301139 | 3 | 0.03 % |
| Lugias | 159301132 | 3 | 0.03 % |
| Holy Animor | 199301138 | 3 | 0.03 % |
| Fragment of Arrogance | 389160000 | 3 | 0.03 % |
| Fragment of Gluttony | 389161000 | 3 | 0.03 % |
| Fragment of Rage | 389162000 | 3 | 0.03 % |
| Fragment of Sloth | 389163000 | 3 | 0.03 % |
| Fragment of Lechery | 389164000 | 3 | 0.03 % |
| Fragment of Jealousy | 389165000 | 3 | 0.03 % |
| Gold Coin | 379107000 | 3 | 0.03 % |
| Silver bar | 379067000 | 2 | 0.02 % |
| Foverin | 330940143 | 1 | 0.01 % |
| Shio Tears | 330920141 | 1 | 0.01 % |
| Imir Ring | 330930142 | 1 | 0.01 % |
| Flame Ring | 330910140 | 1 | 0.01 % |
| Fragment of Avarice | 389166000 | 1 | 0.01 % |
| Gold bar | 379068000 | 1 | 0.01 % |

### Fragment of Jealousy [389165000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 47 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 75 | 0.75 % |
| Weapon Enchant Scroll | 389155000 | 75 | 0.75 % |
| Mirage Dagger | 111110000 | 43 | 0.43 % |
| Cleaver | 111010000 | 43 | 0.43 % |
| Priest Bone crasher | 191010000 | 43 | 0.43 % |
| Priest Cracker | 190910000 | 43 | 0.43 % |
| Salamander Staff | 181010000 | 43 | 0.43 % |
| Tomahawk | 131010000 | 43 | 0.43 % |
| Philum | 151010000 | 43 | 0.43 % |
| Harpoon | 151110000 | 43 | 0.43 % |
| Bardish | 156010000 | 43 | 0.43 % |
| Glave | 156110000 | 43 | 0.43 % |
| Bone Cleaver | 136110000 | 43 | 0.43 % |
| Totamic Club | 146110000 | 43 | 0.43 % |
| Iron Crossbow | 168310000 | 43 | 0.43 % |
| Slayer | 121210000 | 43 | 0.43 % |
| Rogue Chitin Armor Pauldron | 245001000 | 27 | 0.27 % |
| Rogue Chitin Armor Pads | 245002000 | 27 | 0.27 % |
| Rogue Chitin Helmet | 245003000 | 27 | 0.27 % |
| Rogue Chitin Gauntlets | 245004000 | 27 | 0.27 % |
| Rogue Chitin Boots | 245005000 | 27 | 0.27 % |
| Priest Chitin Armor Pauldron | 285001000 | 27 | 0.27 % |
| Priest Chitin Armor Pads | 285002000 | 27 | 0.27 % |
| Priest Chitin Helmet | 285003000 | 27 | 0.27 % |
| Priest Chitin Gauntlets | 285004000 | 27 | 0.27 % |
| Priest Chitin Boots | 285005000 | 27 | 0.27 % |
| Crimson Robe | 265001000 | 27 | 0.27 % |
| Crimson Pants | 265002000 | 27 | 0.27 % |
| Crimson Helmet | 265003000 | 27 | 0.27 % |
| Crimson Gloves | 265004000 | 27 | 0.27 % |
| Crimson Boots | 265005000 | 27 | 0.27 % |
| Chitin Boots | 205005000 | 27 | 0.27 % |
| Chitin Armor Pauldron | 205001000 | 27 | 0.27 % |
| Chitin Armor Pads | 205002000 | 27 | 0.27 % |
| Chitin Helmet | 205003000 | 27 | 0.27 % |
| Chitin Gauntlets | 205004000 | 27 | 0.27 % |
| Silver bar | 379067000 | 11 | 0.11 % |
| Blessed Upgrade Scroll | 379021000 | 11 | 0.11 % |
| Bronze Coin | 379110000 | 11 | 0.11 % |
| Shard | 111210000 | 8 | 0.08 % |
| Raptor | 156210000 | 8 | 0.08 % |
| Iron Impact | 146210000 | 8 | 0.08 % |
| Iron Bow | 168410000 | 8 | 0.08 % |
| Totamic Spear | 151210000 | 8 | 0.08 % |
| Impact | 141110000 | 8 | 0.08 % |
| Priest Impact | 191110000 | 8 | 0.08 % |
| Elixir Staff | 181110000 | 8 | 0.08 % |
| Durandal | 126310000 | 8 | 0.08 % |
| Mirage | 126410000 | 8 | 0.08 % |
| Deep Scar | 131110000 | 8 | 0.08 % |
| Graham | 121310000 | 8 | 0.08 % |
| Blade Axe | 136210000 | 8 | 0.08 % |
| Lugias | 159301132 | 4 | 0.04 % |
| Sherion | 119301131 | 4 | 0.04 % |
| Avedon | 139301133 | 4 | 0.04 % |
| Helenid | 169301134 | 4 | 0.04 % |
| Hell Blood | 189601135 | 4 | 0.04 % |
| Elysium | 189603137 | 4 | 0.04 % |
| Garp | 189602136 | 4 | 0.04 % |
| Syphioric | 179301139 | 4 | 0.04 % |
| Holy Animor | 199301138 | 4 | 0.04 % |
| Silver Coin | 379109000 | 4 | 0.04 % |
| Fragment of Arrogance | 389160000 | 3 | 0.03 % |
| Fragment of Gluttony | 389161000 | 3 | 0.03 % |
| Fragment of Rage | 389162000 | 3 | 0.03 % |
| Fragment of Sloth | 389163000 | 3 | 0.03 % |
| Fragment of Lechery | 389164000 | 3 | 0.03 % |
| Fragment of Jealousy | 389165000 | 3 | 0.03 % |
| Gold Coin | 379107000 | 3 | 0.03 % |
| Imir Ring | 330930142 | 2 | 0.02 % |
| Flame Ring | 330910140 | 2 | 0.02 % |
| Foverin | 330940143 | 2 | 0.02 % |
| Shio Tears | 330920141 | 2 | 0.02 % |
| Fragment of Avarice | 389166000 | 1 | 0.01 % |
| Gold bar | 379068000 | 1 | 0.01 % |

### Fragment of Avarice [389166000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 53 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 62 | 0.62 % |
| Weapon Enchant Scroll | 389155000 | 62 | 0.62 % |
| Chitin Shell Pauldron | 206001000 | 35 | 0.35 % |
| Chitin Shell Pads | 206002000 | 35 | 0.35 % |
| Chitin Shell Helmet | 206003000 | 35 | 0.35 % |
| Chitin Shell Gauntlets | 206004000 | 35 | 0.35 % |
| Chitin Shell Boots | 206005000 | 35 | 0.35 % |
| Rogue Chitin Shell Pauldron | 246001000 | 35 | 0.35 % |
| Rogue Chitin Shell Pads | 246002000 | 35 | 0.35 % |
| Rogue Chitin Shell Helmet | 246003000 | 35 | 0.35 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 35 | 0.35 % |
| Rogue Chitin Shell Boots | 246005000 | 35 | 0.35 % |
| Priest Chitin Shell Pauldron | 286001000 | 35 | 0.35 % |
| Priest Chitin Shell Pads | 286002000 | 35 | 0.35 % |
| Priest Chitin Shell Helmet | 286003000 | 35 | 0.35 % |
| Priest Chitin Shell Gauntlets | 286004000 | 35 | 0.35 % |
| Priest Chitin Shell Boots | 286005000 | 35 | 0.35 % |
| Complete Robe | 266001000 | 35 | 0.35 % |
| Complete Pants | 266002000 | 35 | 0.35 % |
| Complete Helmet | 266003000 | 35 | 0.35 % |
| Complete Glove | 266004000 | 35 | 0.35 % |
| Complete Boots | 266005000 | 35 | 0.35 % |
| Shard | 111210000 | 35 | 0.35 % |
| Raptor | 156210000 | 35 | 0.35 % |
| Iron Impact | 146210000 | 35 | 0.35 % |
| Iron Bow | 168410000 | 35 | 0.35 % |
| Totamic Spear | 151210000 | 35 | 0.35 % |
| Impact | 141110000 | 35 | 0.35 % |
| Priest Impact | 191110000 | 35 | 0.35 % |
| Elixir Staff | 181110000 | 35 | 0.35 % |
| Durandal | 126310000 | 35 | 0.35 % |
| Mirage | 126410000 | 35 | 0.35 % |
| Deep Scar | 131110000 | 35 | 0.35 % |
| Graham | 121310000 | 35 | 0.35 % |
| Blade Axe | 136210000 | 35 | 0.35 % |
| Sherion | 119301131 | 12 | 0.12 % |
| Avedon | 139301133 | 12 | 0.12 % |
| Helenid | 169301134 | 12 | 0.12 % |
| Hell Blood | 189601135 | 12 | 0.12 % |
| Elysium | 189603137 | 12 | 0.12 % |
| Garp | 189602136 | 12 | 0.12 % |
| Syphioric | 179301139 | 12 | 0.12 % |
| Lugias | 159301132 | 12 | 0.12 % |
| Holy Animor | 199301138 | 12 | 0.12 % |
| Blessed Upgrade Scroll | 379021000 | 9 | 0.09 % |
| Foverin | 330940143 | 5 | 0.05 % |
| Shio Tears | 330920141 | 5 | 0.05 % |
| Imir Ring | 330930142 | 5 | 0.05 % |
| Flame Ring | 330910140 | 5 | 0.05 % |
| Dark Fragment of Pride | 810943000 | 2 | 0.02 % |
| Dark Fragment of Gluttony | 810944000 | 2 | 0.02 % |
| Dark Fragment of Wrath | 810948000 | 2 | 0.02 % |
| Dark Fragment of Sloth | 810949000 | 2 | 0.02 % |
| Dark Fragment of Lust | 810950000 | 2 | 0.02 % |
| Dark Fragment of Envy | 810951000 | 2 | 0.02 % |
| Dark Fragment of Greed | 810952000 | 2 | 0.02 % |
| Cold-Hearted Dagger | 119102294 | 2 | 0.02 % |
| Eagle's Eye | 169302345 | 2 | 0.02 % |
| Spear of Murky Waters | 159302311 | 2 | 0.02 % |
| Scorching Staff | 189302255 | 2 | 0.02 % |
| Oasis Staff | 189102256 | 2 | 0.02 % |
| Chaotic Staff | 189202257 | 2 | 0.02 % |
| Stone Splitter | 199102262 | 2 | 0.02 % |
| Dread Shield | 179102191 | 2 | 0.02 % |
| Dragon Scale Armor Top | 507001000 | 2 | 0.02 % |
| Dragon Scale Armor Pants | 507002000 | 2 | 0.02 % |
| Legionnaire Band | 330151461 | 2 | 0.02 % |
| Hero's Valor | 330111280 | 2 | 0.02 % |
| Minotaur's Earrings | 310511119 | 2 | 0.02 % |
| Cockatrice's Earrings | 310512120 | 2 | 0.02 % |
| Lillime's Earrings | 310514122 | 2 | 0.02 % |
| Lich King's Earrings | 310513121 | 2 | 0.02 % |
| Judicious Belt | 340610122 | 2 | 0.02 % |
| String of Skulls | 340410123 | 2 | 0.02 % |
| Light Belt of Life | 340110120 | 2 | 0.02 % |
| Sash of Sorrows | 340510121 | 2 | 0.02 % |
| Dragon Scale Helmet | 507003000 | 2 | 0.02 % |
| Dragon Scale Gauntlet | 507004000 | 2 | 0.02 % |
| Dragon Scale Boots | 507005000 | 2 | 0.02 % |
| Mythril Pauldron | 547001000 | 2 | 0.02 % |
| Mythril Pad | 547002000 | 2 | 0.02 % |
| Mythril Helmet | 547003000 | 2 | 0.02 % |
| Mythril Gauntlet | 547004000 | 2 | 0.02 % |
| Mythril Boots | 547005000 | 2 | 0.02 % |
| Pauldron of Trial | 587001000 | 2 | 0.02 % |
| Pads of Trial | 587002000 | 2 | 0.02 % |
| Helmet of Trial | 587003000 | 2 | 0.02 % |
| Gauntlet of Trial | 587004000 | 2 | 0.02 % |
| Boots of Trial | 587005000 | 1 | 0.01 % |
| Ron's Robe | 567001000 | 1 | 0.01 % |
| Ron's Pants | 567002000 | 1 | 0.01 % |
| Ron's Helmet | 567003000 | 1 | 0.01 % |
| Ron's Gloves | 567004000 | 1 | 0.01 % |
| Ron's Boots | 567005000 | 1 | 0.01 % |

### Silvery Gem [389196000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 46 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Blessed Upgrade Scroll | 379021000 | 95 | 0.95 % |
| Raptor | 156210000 | 63 | 0.63 % |
| Shard | 111210000 | 58 | 0.58 % |
| Chitin Shell Gauntlets | 206004000 | 47 | 0.47 % |
| Chitin Shell Boots | 206005000 | 47 | 0.47 % |
| Rogue Chitin Shell Pauldron | 246001000 | 47 | 0.47 % |
| Rogue Chitin Shell Pads | 246002000 | 47 | 0.47 % |
| Rogue Chitin Shell Helmet | 246003000 | 47 | 0.47 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 46 | 0.46 % |
| Rogue Chitin Shell Boots | 246005000 | 46 | 0.46 % |
| Chitin Shell Pauldron | 206001000 | 46 | 0.46 % |
| Chitin Shell Pads | 206002000 | 46 | 0.46 % |
| Chitin Shell Helmet | 206003000 | 46 | 0.46 % |
| Weapon Enchant Scroll | 389155000 | 41 | 0.41 % |
| Armor Enchant Scroll | 389156000 | 41 | 0.41 % |
| Iron Impact | 146210000 | 29 | 0.29 % |
| Iron Bow | 168410000 | 29 | 0.29 % |
| Mirage | 126410000 | 29 | 0.29 % |
| Mirage Dagger | 111110000 | 29 | 0.29 % |
| Totamic Spear | 151210000 | 29 | 0.29 % |
| Graham | 121310000 | 29 | 0.29 % |
| Deep Scar | 131110000 | 29 | 0.29 % |
| Impact | 141110000 | 29 | 0.29 % |
| Priest Impact | 191110000 | 29 | 0.29 % |
| Elixir Staff | 181110000 | 29 | 0.29 % |
| Cold-Hearted Dagger | 119102294 | 26 | 0.26 % |
| Spear of Murky Waters | 159302311 | 23 | 0.23 % |
| Stone Splitter | 199102262 | 23 | 0.23 % |
| Scorching Staff | 189302255 | 21 | 0.21 % |
| Chaotic Staff | 189202257 | 21 | 0.21 % |
| Oasis Staff | 189102256 | 21 | 0.21 % |
| Priest Chitin Shell Pauldron | 286001000 | 21 | 0.21 % |
| Priest Chitin Shell Pads | 286002000 | 21 | 0.21 % |
| Priest Chitin Shell Helmet | 286003000 | 21 | 0.21 % |
| Priest Chitin Shell Gauntlets | 286004000 | 21 | 0.21 % |
| Priest Chitin Shell Boots | 286005000 | 21 | 0.21 % |
| Complete Glove | 266004000 | 21 | 0.21 % |
| Complete Boots | 266005000 | 21 | 0.21 % |
| Complete Robe | 266001000 | 21 | 0.21 % |
| Complete Pants | 266002000 | 21 | 0.21 % |
| Complete Helmet | 266003000 | 21 | 0.21 % |
| Light Belt of Life | 340110120 | 17 | 0.17 % |
| Dread Shield | 179102191 | 13 | 0.13 % |
| Eagle's Eye | 169302345 | 13 | 0.13 % |
| Judicious Belt | 340610122 | 9 | 0.09 % |
| Sash of Sorrows | 340510121 | 9 | 0.09 % |
| Silver bar | 379067000 | 6 | 0.06 % |
| String of Skulls | 340410123 | 5 | 0.05 % |
| Legionnaire Band | 330151461 | 5 | 0.05 % |
| Lillime's Earrings | 310514122 | 5 | 0.05 % |
| Lich King's Earrings | 310513121 | 5 | 0.05 % |
| Hero's Valor | 330111280 | 5 | 0.05 % |
| Minotaur's Earrings | 310511119 | 5 | 0.05 % |
| Cockatrice's Earrings | 310512120 | 5 | 0.05 % |

### Yellow Gem [389198000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 46 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Blessed Upgrade Scroll | 379021000 | 272 | 2.72 % |
| Weapon Enchant Scroll | 389155000 | 108 | 1.08 % |
| Armor Enchant Scroll | 389156000 | 108 | 1.08 % |
| Chitin Shell Pauldron | 206001000 | 39 | 0.39 % |
| Chitin Shell Pads | 206002000 | 39 | 0.39 % |
| Chitin Shell Helmet | 206003000 | 39 | 0.39 % |
| Chitin Shell Gauntlets | 206004000 | 39 | 0.39 % |
| Priest Chitin Shell Pauldron | 286001000 | 35 | 0.35 % |
| Priest Chitin Shell Pads | 286002000 | 35 | 0.35 % |
| Priest Chitin Shell Helmet | 286003000 | 35 | 0.35 % |
| Priest Chitin Shell Gauntlets | 286004000 | 35 | 0.35 % |
| Priest Chitin Shell Boots | 286005000 | 35 | 0.35 % |
| Complete Pants | 266002000 | 35 | 0.35 % |
| Complete Helmet | 266003000 | 35 | 0.35 % |
| Complete Glove | 266004000 | 35 | 0.35 % |
| Complete Boots | 266005000 | 35 | 0.35 % |
| Complete Robe | 266001000 | 35 | 0.35 % |
| Iron Impact | 146210000 | 25 | 0.25 % |
| Iron Bow | 168410000 | 25 | 0.25 % |
| Mirage | 126410000 | 25 | 0.25 % |
| Mirage Dagger | 111110000 | 25 | 0.25 % |
| Totamic Spear | 151210000 | 25 | 0.25 % |
| Graham | 121310000 | 24 | 0.24 % |
| Deep Scar | 131110000 | 24 | 0.24 % |
| Impact | 141110000 | 24 | 0.24 % |
| Priest Impact | 191110000 | 24 | 0.24 % |
| Elixir Staff | 181110000 | 24 | 0.24 % |
| Raptor | 156210000 | 24 | 0.24 % |
| Shard | 111210000 | 24 | 0.24 % |
| Chitin Shell Boots | 206005000 | 20 | 0.20 % |
| Rogue Chitin Shell Pauldron | 246001000 | 20 | 0.20 % |
| Rogue Chitin Shell Pads | 246002000 | 20 | 0.20 % |
| Rogue Chitin Shell Helmet | 246003000 | 20 | 0.20 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 20 | 0.20 % |
| Rogue Chitin Shell Boots | 246005000 | 20 | 0.20 % |
| Chaotic Staff | 189202257 | 18 | 0.18 % |
| Oasis Staff | 189102256 | 18 | 0.18 % |
| Spear of Murky Waters | 159302311 | 10 | 0.10 % |
| Stone Splitter | 199102262 | 10 | 0.10 % |
| Scorching Staff | 189302255 | 9 | 0.09 % |
| Silver bar | 379067000 | 6 | 0.06 % |
| Dread Shield | 179102191 | 3 | 0.03 % |
| Cold-Hearted Dagger | 119102294 | 3 | 0.03 % |
| Eagle's Eye | 169302345 | 3 | 0.03 % |
| Light Belt of Life | 340110120 | 2 | 0.02 % |
| Judicious Belt | 340610122 | 2 | 0.02 % |
| Sash of Sorrows | 340510121 | 2 | 0.02 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |

### Blue Gem [389199000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 44 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Blessed Upgrade Scroll | 379021000 | 178 | 1.78 % |
| Weapon Enchant Scroll | 389155000 | 78 | 0.78 % |
| Armor Enchant Scroll | 389156000 | 78 | 0.78 % |
| Raptor | 156210000 | 60 | 0.60 % |
| Shard | 111210000 | 60 | 0.60 % |
| Chitin Shell Pauldron | 206001000 | 48 | 0.48 % |
| Chitin Shell Pads | 206002000 | 48 | 0.48 % |
| Chitin Shell Helmet | 206003000 | 48 | 0.48 % |
| Chitin Shell Gauntlets | 206004000 | 48 | 0.48 % |
| Chitin Shell Boots | 206005000 | 48 | 0.48 % |
| Rogue Chitin Shell Pauldron | 246001000 | 48 | 0.48 % |
| Rogue Chitin Shell Pads | 246002000 | 48 | 0.48 % |
| Rogue Chitin Shell Helmet | 246003000 | 48 | 0.48 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 48 | 0.48 % |
| Rogue Chitin Shell Boots | 246005000 | 48 | 0.48 % |
| Iron Impact | 146210000 | 30 | 0.30 % |
| Iron Bow | 168410000 | 30 | 0.30 % |
| Mirage | 126410000 | 30 | 0.30 % |
| Mirage Dagger | 111110000 | 30 | 0.30 % |
| Totamic Spear | 151210000 | 30 | 0.30 % |
| Graham | 121310000 | 30 | 0.30 % |
| Deep Scar | 131110000 | 30 | 0.30 % |
| Impact | 141110000 | 30 | 0.30 % |
| Priest Impact | 191110000 | 30 | 0.30 % |
| Elixir Staff | 181110000 | 30 | 0.30 % |
| Priest Chitin Shell Pauldron | 286001000 | 22 | 0.22 % |
| Priest Chitin Shell Pads | 286002000 | 22 | 0.22 % |
| Priest Chitin Shell Helmet | 286003000 | 22 | 0.22 % |
| Priest Chitin Shell Gauntlets | 286004000 | 21 | 0.21 % |
| Priest Chitin Shell Boots | 286005000 | 21 | 0.21 % |
| Complete Robe | 266001000 | 21 | 0.21 % |
| Complete Pants | 266002000 | 21 | 0.21 % |
| Complete Helmet | 266003000 | 21 | 0.21 % |
| Complete Glove | 266004000 | 21 | 0.21 % |
| Complete Boots | 266005000 | 21 | 0.21 % |
| Spear of Murky Waters | 159302311 | 8 | 0.08 % |
| Stone Splitter | 199102262 | 8 | 0.08 % |
| Scorching Staff | 189302255 | 6 | 0.06 % |
| Chaotic Staff | 189202257 | 6 | 0.06 % |
| Oasis Staff | 189102256 | 6 | 0.06 % |
| Light Belt of Life | 340110120 | 2 | 0.02 % |
| Dread Shield | 179102191 | 2 | 0.02 % |
| Cold-Hearted Dagger | 119102294 | 2 | 0.02 % |
| Eagle's Eye | 169302345 | 2 | 0.02 % |
| Silver bar | 379067000 | 2 | 0.02 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |

### Green Gem [389201000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 45 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Blessed Upgrade Scroll | 379021000 | 184 | 1.84 % |
| Weapon Enchant Scroll | 389155000 | 83 | 0.83 % |
| Armor Enchant Scroll | 389156000 | 83 | 0.83 % |
| Raptor | 156210000 | 59 | 0.59 % |
| Shard | 111210000 | 59 | 0.59 % |
| Chitin Shell Pauldron | 206001000 | 48 | 0.48 % |
| Chitin Shell Pads | 206002000 | 48 | 0.48 % |
| Chitin Shell Helmet | 206003000 | 48 | 0.48 % |
| Chitin Shell Gauntlets | 206004000 | 48 | 0.48 % |
| Chitin Shell Boots | 206005000 | 48 | 0.48 % |
| Rogue Chitin Shell Pauldron | 246001000 | 48 | 0.48 % |
| Rogue Chitin Shell Pads | 246002000 | 48 | 0.48 % |
| Rogue Chitin Shell Helmet | 246003000 | 48 | 0.48 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 48 | 0.48 % |
| Rogue Chitin Shell Boots | 246005000 | 48 | 0.48 % |
| Totamic Spear | 151210000 | 36 | 0.36 % |
| Iron Impact | 146210000 | 30 | 0.30 % |
| Iron Bow | 168410000 | 30 | 0.30 % |
| Mirage | 126410000 | 30 | 0.30 % |
| Mirage Dagger | 111110000 | 30 | 0.30 % |
| Graham | 121310000 | 30 | 0.30 % |
| Deep Scar | 131110000 | 30 | 0.30 % |
| Impact | 141110000 | 30 | 0.30 % |
| Priest Impact | 191110000 | 30 | 0.30 % |
| Elixir Staff | 181110000 | 30 | 0.30 % |
| Priest Chitin Shell Pauldron | 286001000 | 21 | 0.21 % |
| Priest Chitin Shell Pads | 286002000 | 21 | 0.21 % |
| Priest Chitin Shell Helmet | 286003000 | 21 | 0.21 % |
| Priest Chitin Shell Gauntlets | 286004000 | 21 | 0.21 % |
| Priest Chitin Shell Boots | 286005000 | 21 | 0.21 % |
| Complete Robe | 266001000 | 21 | 0.21 % |
| Complete Pants | 266002000 | 21 | 0.21 % |
| Complete Helmet | 266003000 | 21 | 0.21 % |
| Complete Glove | 266004000 | 21 | 0.21 % |
| Complete Boots | 266005000 | 21 | 0.21 % |
| Spear of Murky Waters | 159302311 | 6 | 0.06 % |
| Stone Splitter | 199102262 | 6 | 0.06 % |
| Scorching Staff | 189302255 | 3 | 0.03 % |
| Chaotic Staff | 189202257 | 3 | 0.03 % |
| Oasis Staff | 189102256 | 3 | 0.03 % |
| Silver bar | 379067000 | 2 | 0.02 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |
| Dread Shield | 179102191 | 1 | 0.01 % |
| Cold-Hearted Dagger | 119102294 | 1 | 0.01 % |
| Eagle's Eye | 169302345 | 1 | 0.01 % |

### Black Gem [389205000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 44 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Blessed Upgrade Scroll | 379021000 | 185 | 1.85 % |
| Weapon Enchant Scroll | 389155000 | 84 | 0.84 % |
| Armor Enchant Scroll | 389156000 | 84 | 0.84 % |
| Raptor | 156210000 | 60 | 0.60 % |
| Shard | 111210000 | 60 | 0.60 % |
| Rogue Chitin Shell Pauldron | 246001000 | 48 | 0.48 % |
| Rogue Chitin Shell Pads | 246002000 | 48 | 0.48 % |
| Rogue Chitin Shell Helmet | 246003000 | 48 | 0.48 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 48 | 0.48 % |
| Rogue Chitin Shell Boots | 246005000 | 48 | 0.48 % |
| Chitin Shell Pauldron | 206001000 | 48 | 0.48 % |
| Chitin Shell Pads | 206002000 | 48 | 0.48 % |
| Chitin Shell Helmet | 206003000 | 48 | 0.48 % |
| Chitin Shell Gauntlets | 206004000 | 48 | 0.48 % |
| Chitin Shell Boots | 206005000 | 48 | 0.48 % |
| Iron Impact | 146210000 | 30 | 0.30 % |
| Iron Bow | 168410000 | 30 | 0.30 % |
| Mirage | 126410000 | 30 | 0.30 % |
| Mirage Dagger | 111110000 | 30 | 0.30 % |
| Totamic Spear | 151210000 | 30 | 0.30 % |
| Graham | 121310000 | 30 | 0.30 % |
| Deep Scar | 131110000 | 30 | 0.30 % |
| Impact | 141110000 | 30 | 0.30 % |
| Priest Impact | 191110000 | 30 | 0.30 % |
| Elixir Staff | 181110000 | 30 | 0.30 % |
| Priest Chitin Shell Pauldron | 286001000 | 21 | 0.21 % |
| Priest Chitin Shell Pads | 286002000 | 21 | 0.21 % |
| Priest Chitin Shell Helmet | 286003000 | 21 | 0.21 % |
| Priest Chitin Shell Gauntlets | 286004000 | 21 | 0.21 % |
| Priest Chitin Shell Boots | 286005000 | 21 | 0.21 % |
| Complete Robe | 266001000 | 21 | 0.21 % |
| Complete Pants | 266002000 | 21 | 0.21 % |
| Complete Helmet | 266003000 | 21 | 0.21 % |
| Complete Glove | 266004000 | 21 | 0.21 % |
| Complete Boots | 266005000 | 21 | 0.21 % |
| Spear of Murky Waters | 159302311 | 6 | 0.06 % |
| Stone Splitter | 199102262 | 6 | 0.06 % |
| Scorching Staff | 189302255 | 3 | 0.03 % |
| Chaotic Staff | 189202257 | 3 | 0.03 % |
| Oasis Staff | 189102256 | 3 | 0.03 % |
| Judicious Belt | 340610122 | 2 | 0.02 % |
| Silver bar | 379067000 | 2 | 0.02 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |
| Dread Shield | 179102191 | 1 | 0.01 % |
| Cold-Hearted Dagger | 119102294 | 1 | 0.01 % |
| Eagle's Eye | 169302345 | 1 | 0.01 % |

### 70 Quest - Dark Fragment Chest [389556000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 28 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Ron's Helmet | 567003000 | 65 | 0.65 % |
| Ron's Robe | 567001000 | 65 | 0.65 % |
| Ron's Pants | 567002000 | 65 | 0.65 % |
| Ron's Gloves | 567004000 | 65 | 0.65 % |
| Ron's Boots | 567005000 | 65 | 0.65 % |
| Dragon Scale Helmet | 507003000 | 55 | 0.55 % |
| Dragon Scale Armor Top | 507001000 | 55 | 0.55 % |
| Dragon Scale Armor Pants | 507002000 | 55 | 0.55 % |
| Dragon Scale Gauntlet | 507004000 | 55 | 0.55 % |
| Dragon Scale Boots | 507005000 | 55 | 0.55 % |
| Mythril Helmet | 547003000 | 55 | 0.55 % |
| Mythril Pauldron | 547001000 | 55 | 0.55 % |
| Mythril Pad | 547002000 | 55 | 0.55 % |
| Mythril Gauntlet | 547004000 | 55 | 0.55 % |
| Mythril Boots | 547005000 | 55 | 0.55 % |
| Helmet of Trial | 587003000 | 55 | 0.55 % |
| Pauldron of Trial | 587001000 | 55 | 0.55 % |
| Pads of Trial | 587002000 | 55 | 0.55 % |
| Gauntlet of Trial | 587004000 | 54 | 0.54 % |
| Boots of Trial | 587005000 | 54 | 0.54 % |
| Dagger of <selfname> | 910119300 | 44 | 0.44 % |
| Two-Handed Sword of <selfname> | 910120350 | 44 | 0.44 % |
| Spear of <selfname> | 910121300 | 44 | 0.44 % |
| Bow of <selfname> | 910122350 | 44 | 0.44 % |
| Staff of <selfname> | 910123000 | 44 | 0.44 % |
| Pole of <selfname> | 910124000 | 44 | 0.44 % |
| Mace of <selfname> | 910125000 | 44 | 0.44 % |
| Shield of <selfname> | 910126000 | 44 | 0.44 % |

### Holy Armor - Legend Dark Star Wapon Box [507184000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 81 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| <selfname>'s Ruahue | 1110434500 | 37 | 0.37 % |
| Holy Knight Warrior Helmet | 231513000 | 19 | 0.19 % |
| Holy Knight Warrior Pauldron | 231511000 | 19 | 0.19 % |
| Holy Knight Warrior Pads | 231512000 | 19 | 0.19 % |
| Holy Knight Warrior Gauntlets | 231514000 | 19 | 0.19 % |
| Holy Knight Warrior Boots | 231515000 | 19 | 0.19 % |
| Holy Knight Assassin Helmet | 232513000 | 19 | 0.19 % |
| Holy Knight Assassin Pauldron | 232511000 | 19 | 0.19 % |
| Holy Knight Assassin Pads | 232512000 | 19 | 0.19 % |
| Holy Knight Assassin Gauntlets | 232514000 | 19 | 0.19 % |
| Holy Knight Assassin Boots | 232515000 | 19 | 0.19 % |
| Holy Knight Mage Helmet | 233513000 | 19 | 0.19 % |
| Holy Knight Mage Robe | 233511000 | 19 | 0.19 % |
| Holy Knight Mage Pants | 233512000 | 19 | 0.19 % |
| Holy Knight Mage Gloves | 233514000 | 19 | 0.19 % |
| Holy Knight Mage Boots | 233515000 | 19 | 0.19 % |
| Holy Knight Priest Helmet | 234513000 | 19 | 0.19 % |
| Holy Knight Priest Pauldron | 234511000 | 19 | 0.19 % |
| Holy Knight Priest Pads | 234512000 | 19 | 0.19 % |
| Holy Knight Priest Gauntlets | 234514000 | 19 | 0.19 % |
| Holy Knight Priest Boots | 234515000 | 19 | 0.19 % |
| <selfname> - Holy Dagger | 1112110540 | 19 | 0.19 % |
| <selfname> - Holy Two-Handed Axe | 1112310560 | 19 | 0.19 % |
| <selfname> - Holy Club | 1112510560 | 19 | 0.19 % |
| <selfname> - Holy Bow | 1112710460 | 19 | 0.19 % |
| <selfname> - Holy Staff | 1112910400 | 19 | 0.19 % |
| <selfname> - Holy Mace | 1113110460 | 19 | 0.19 % |
| <selfname> - Holy Shield | 1113310400 | 19 | 0.19 % |
| Dagger of the - <selfname> - Legend King | 1111810420 | 19 | 0.19 % |
| Sword of the - <selfname> - Legend King | 1111820420 | 19 | 0.19 % |
| Axe of the - <selfname> - Legend King | 1111830420 | 19 | 0.19 % |
| Bow of the - <selfname> - Legend King | 1111840540 | 19 | 0.19 % |
| Staff of the - <selfname> - Legend King | 1111850420 | 19 | 0.19 % |
| Hammer of the - <selfname> - Legend King | 1111860540 | 19 | 0.19 % |
| Shield of the - <selfname> - Legend King | 1111870420 | 19 | 0.19 % |
| Aztec Holy Dagger | 1121810440 | 19 | 0.19 % |
| Aztec Holy Impact | 1121820440 | 19 | 0.19 % |
| Aztec Holy One-Handed Sword | 1121830440 | 19 | 0.19 % |
| Aztec Holy Crossbow | 1121840440 | 19 | 0.19 % |
| Aztec Holy Staff | 1121850440 | 19 | 0.19 % |
| Aztec Holy Mace | 1121860440 | 19 | 0.19 % |
| Aztec Holy Shield | 1121870440 | 19 | 0.19 % |
| Asgard Legion - <selfname> - Dagger | 1110540560 | 18 | 0.18 % |
| Asgard Legion - <selfname> - Short Sword | 1110550300 | 18 | 0.18 % |
| Asgard Legion - <selfname> - Sword | 1110560580 | 18 | 0.18 % |
| Asgard Legion - <selfname> - Bow | 1110570560 | 18 | 0.18 % |
| Asgard Legion - <selfname> - Staff | 1110580460 | 18 | 0.18 % |
| Asgard Legion - <selfname> - Mace | 1110590560 | 18 | 0.18 % |
| Asgard Legion - <selfname> - Shield | 1110600460 | 18 | 0.18 % |
| Star Dagger | 1110440520 | 18 | 0.18 % |
| Star Spear | 1110450520 | 18 | 0.18 % |
| Star Long Spear | 1110460520 | 18 | 0.18 % |
| Star Bow | 1110470520 | 18 | 0.18 % |
| Star Staff | 1110480000 | 18 | 0.18 % |
| Star Mace | 1110490520 | 18 | 0.18 % |
| Star Shield | 1110500570 | 18 | 0.18 % |
| <selfname>'s Khanjar | 1110429500 | 18 | 0.18 % |
| <selfname>'s Moonlight | 1110430500 | 18 | 0.18 % |
| <selfname>'s Buju Polearm | 1110431500 | 18 | 0.18 % |
| <selfname>'s Azagai | 1110432500 | 18 | 0.18 % |
| <selfname> Fire Codex Gigas | 1110433640 | 18 | 0.18 % |
| <selfname> Ice Codex Gigas | 1110433660 | 18 | 0.18 % |
| <selfname> Lightning Codex Gigas | 1110433680 | 18 | 0.18 % |
| Dark Knight Dagger | 1111466000 | 18 | 0.18 % |
| Dark Knight Jamadar | 1111467000 | 18 | 0.18 % |
| Dark Knight Spear | 1111468000 | 18 | 0.18 % |
| Dark Knight Axe | 1111473452 | 18 | 0.18 % |
| Dark Knight Crossbow | 1111469000 | 18 | 0.18 % |
| Fire Dark Knight Staff | 1111470691 | 18 | 0.18 % |
| Ice Dark Knight Staff | 1111470692 | 18 | 0.18 % |
| Lightning Dark Knight Staff | 1111470693 | 18 | 0.18 % |
| Dark Knight Mace | 1111471000 | 18 | 0.18 % |
| Dark Knight Shield | 1111472000 | 18 | 0.18 % |
| The Egyptian Pharaoh's Jamadar | 1110760540 | 18 | 0.18 % |
| The Egyptian Pharaoh's Spear | 1110770540 | 18 | 0.18 % |
| The Egyptian Pharaoh's Breaker | 1110780740 | 18 | 0.18 % |
| The Egyptian Pharaoh's Crossbow | 1110790620 | 18 | 0.18 % |
| The Egyptian Pharaoh's Staff | 1110800780 | 18 | 0.18 % |
| The Egyptian Pharaoh's Mace | 1110810620 | 18 | 0.18 % |
| The Egyptian Pharaoh's Shield | 1110820800 | 18 | 0.18 % |

### Krowaz Armor - Weapon Box [507194000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 55 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Aztec Gold Molok | 183230000 | 55 | 0.55 % |
| Undefeatable Wirinom | 127410000 | 28 | 0.28 % |
| Weak Dwarf Wirinom | 127420285 | 28 | 0.28 % |
| Chaos Wirinom | 127430000 | 28 | 0.28 % |
| Sweet Kiss Wirinom | 127440000 | 28 | 0.28 % |
| Undefeatable Raum | 128410000 | 28 | 0.28 % |
| Raum on Severe Diet | 128420902 | 28 | 0.28 % |
| Weak Dwarf Raum | 128430903 | 28 | 0.28 % |
| Lousy Pickpocket Raum | 128440000 | 28 | 0.28 % |
| Undefeatable Baal | 139410000 | 28 | 0.28 % |
| Weak Dwarf Baal | 139420285 | 28 | 0.28 % |
| Chaos Baal | 139430000 | 28 | 0.28 % |
| Sweet Kiss Baal | 139440000 | 28 | 0.28 % |
| Undefeatable Nebiros | 159410000 | 28 | 0.28 % |
| Weak Dwarf Nebiros | 159420313 | 28 | 0.28 % |
| Chaos Nebiros | 159430000 | 27 | 0.27 % |
| Sweet Kiss Nebiros | 159440000 | 27 | 0.27 % |
| Undefeatable Windforce | 180410000 | 27 | 0.27 % |
| Windforce on Severe Diet | 180420900 | 27 | 0.27 % |
| Warm Downpour Windforce | 180430901 | 27 | 0.27 % |
| Weak Dwarf Windforce | 180440902 | 27 | 0.27 % |
| Lousy Pickpocket Windforce | 180450000 | 27 | 0.27 % |
| Undefeatable Fulitol | 182410000 | 27 | 0.27 % |
| Trainer Kate Fulitol | 182420313 | 27 | 0.27 % |
| Aztec Gold Fulitol | 182430000 | 27 | 0.27 % |
| Trainer Kate Molok | 183420316 | 27 | 0.27 % |
| Undefeatable Faun | 184410000 | 27 | 0.27 % |
| Trainer Kate Faun | 184420319 | 27 | 0.27 % |
| Aztec Gold Faun | 184430000 | 27 | 0.27 % |
| Gab's Adamant | 197410000 | 27 | 0.27 % |
| Gab's Blessing | 197420000 | 27 | 0.27 % |
| Undefeatable Leonar | 198410898 | 27 | 0.27 % |
| Weak Dwarf Leonar | 198420899 | 27 | 0.27 % |
| Mean Leonar | 198430900 | 27 | 0.27 % |
| Krowaz Warrior Helmet | 208003000 | 27 | 0.27 % |
| Krowaz Warrior Upper Garment | 208001000 | 27 | 0.27 % |
| Krowaz Warrior Lower Garment | 208002000 | 27 | 0.27 % |
| Krowaz Warrior Gauntlets | 208004000 | 27 | 0.27 % |
| Krowaz Warrior Boots | 208005000 | 27 | 0.27 % |
| Krowaz Assassin Helmet | 248003000 | 27 | 0.27 % |
| Krowaz Assassin Upper Garment | 248001000 | 27 | 0.27 % |
| Krowaz Assassin Lower Garment | 248002000 | 27 | 0.27 % |
| Krowaz Assassin Gauntlets | 248004000 | 27 | 0.27 % |
| Krowaz Assassin Boots | 248005000 | 27 | 0.27 % |
| Krowaz Mage Helmet | 268003000 | 27 | 0.27 % |
| Krowaz Mage Robe | 268001000 | 27 | 0.27 % |
| Krowaz Mage Pants | 268002000 | 27 | 0.27 % |
| Krowaz Mage Gauntlets | 268004000 | 27 | 0.27 % |
| Krowaz Mage Boots | 268005000 | 27 | 0.27 % |
| Krowaz Priest Helmet | 288003000 | 27 | 0.27 % |
| Krowaz Priest Upper Garment | 288001000 | 27 | 0.27 % |
| Krowaz Priest Lower Garment | 288002000 | 27 | 0.27 % |
| Krowaz Priest Gauntlet | 288004000 | 27 | 0.27 % |
| Krowaz Priest Boots | 288005000 | 27 | 0.27 % |

### Legend Dragon Box [507204000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 35 replaced by their +0 twin, removed (no +0 twin): Dragon Knight Two-Handed Axe (+7) [1190003507].

| Own reward | id | slots | % |
|---|---|---|---|
| Phoenix Dagger | 1111484000 | 57 | 0.57 % |
| Phoenix Short Sword | 1111485000 | 57 | 0.57 % |
| Phoenix Axe | 1111486453 | 57 | 0.57 % |
| Phoenix Bow | 1111487000 | 57 | 0.57 % |
| Fire Phoenix Staff | 1111488774 | 57 | 0.57 % |
| Ice Phoenix Staff | 1111488775 | 57 | 0.57 % |
| Lightning Phoenix Staff | 1111488776 | 57 | 0.57 % |
| Phoenix Mace | 1111489000 | 57 | 0.57 % |
| Phoenix Shield | 1111490000 | 56 | 0.56 % |
| Legend Knight Warrior Helmet | 331003000 | 38 | 0.38 % |
| Legend Knight Warrior Pauldron | 331001000 | 38 | 0.38 % |
| Legend Knight Warrior Pads | 331002000 | 38 | 0.38 % |
| Legend Knight Warrior Gauntlets | 331004000 | 38 | 0.38 % |
| Legend Knight Warrior Boots | 331005000 | 38 | 0.38 % |
| Legend Knight Assassin Helmet | 332003000 | 38 | 0.38 % |
| Legend Knight Assassin Pauldron | 332001000 | 38 | 0.38 % |
| Legend Knight Assassin Pads | 332002000 | 38 | 0.38 % |
| Legend Knight Assassin Gauntlets | 332004000 | 38 | 0.38 % |
| Legend Knight Assassin Boots | 332005000 | 38 | 0.38 % |
| Legend Knight Mage Helmet | 333003000 | 38 | 0.38 % |
| Legend Knight Mage Robe | 333001000 | 38 | 0.38 % |
| Legend Knight Mage Pants | 333002000 | 38 | 0.38 % |
| Legend Knight Mage Gloves | 333004000 | 38 | 0.38 % |
| Legend Knight Mage Boots | 333005000 | 38 | 0.38 % |
| Legend Knight Priest Helmet | 334003000 | 38 | 0.38 % |
| Legend Knight Priest Pauldron | 334001000 | 38 | 0.38 % |
| Legend Knight Priest Pads | 334002000 | 38 | 0.38 % |
| Legend Knight Priest Gauntlets | 334004000 | 38 | 0.38 % |
| Legend Knight Priest Boots | 334005000 | 38 | 0.38 % |
| Dragon Knight Jamadar | 1190001000 | 38 | 0.38 % |
| Dragon Knight One-Handed Axe | 1190009000 | 38 | 0.38 % |
| Dragon Knight Bow | 1190004000 | 38 | 0.38 % |
| Dragon Knight Staff | 1190005000 | 38 | 0.38 % |
| Dragon Knight Hammer | 1190006000 | 38 | 0.38 % |
| Dragon Knight Shield | 1190007000 | 38 | 0.38 % |

### Black - Red Chitin - Weapon Box [507284000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 72 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Chitin Helmet | 205003000 | 21 | 0.21 % |
| Chitin Armor Pauldron | 205001000 | 21 | 0.21 % |
| Chitin Armor Pads | 205002000 | 21 | 0.21 % |
| Chitin Gauntlets | 205004000 | 21 | 0.21 % |
| Chitin Boots | 205005000 | 21 | 0.21 % |
| Rogue Chitin Helmet | 245003000 | 21 | 0.21 % |
| Rogue Chitin Armor Pauldron | 245001000 | 21 | 0.21 % |
| Rogue Chitin Armor Pads | 245002000 | 21 | 0.21 % |
| Rogue Chitin Gauntlets | 245004000 | 21 | 0.21 % |
| Rogue Chitin Boots | 245005000 | 21 | 0.21 % |
| Priest Chitin Helmet | 285003000 | 21 | 0.21 % |
| Priest Chitin Armor Pauldron | 285001000 | 21 | 0.21 % |
| Priest Chitin Armor Pads | 285002000 | 21 | 0.21 % |
| Priest Chitin Gauntlets | 285004000 | 21 | 0.21 % |
| Priest Chitin Boots | 285005000 | 21 | 0.21 % |
| Crimson Helmet | 265003000 | 21 | 0.21 % |
| Crimson Robe | 265001000 | 21 | 0.21 % |
| Crimson Pants | 265002000 | 21 | 0.21 % |
| Crimson Gloves | 265004000 | 21 | 0.21 % |
| Crimson Boots | 265005000 | 21 | 0.21 % |
| Black Chitin Helmet | 505003000 | 21 | 0.21 % |
| Black Chitin Armor Pauldron | 505001000 | 21 | 0.21 % |
| Black Chitin Armor Pads | 505002000 | 21 | 0.21 % |
| Black Chitin Gauntlets | 505004000 | 21 | 0.21 % |
| Black Chitin Boots | 505005000 | 21 | 0.21 % |
| Black Rogue Chitin Helmet | 545003000 | 21 | 0.21 % |
| Black Rogue Chitin Armor Pauldron | 545001000 | 21 | 0.21 % |
| Black Rogue Chitin Armor Pads | 545002000 | 21 | 0.21 % |
| Black Rogue Chitin Gauntlets | 545004000 | 21 | 0.21 % |
| Black Rogue Chitin Boots | 545005000 | 21 | 0.21 % |
| Black Crimson Helmet | 565003000 | 21 | 0.21 % |
| Black Crimson Robe | 565001000 | 21 | 0.21 % |
| Black Crimson Pants | 565002000 | 21 | 0.21 % |
| Black Crimson Gloves | 565004000 | 21 | 0.21 % |
| Black Crimson Boots | 565005000 | 21 | 0.21 % |
| Black Priest Chitin Helmet | 505103000 | 21 | 0.21 % |
| Black Priest Chitin Armor Pauldron | 505101000 | 21 | 0.21 % |
| Black Priest Chitin Armor Pads | 505102000 | 21 | 0.21 % |
| Black Priest Chitin Gauntlets | 505104000 | 21 | 0.21 % |
| Black Priest Chitin Boots | 505105000 | 21 | 0.21 % |
| Red Chitin Armor Helmet | 605003000 | 21 | 0.21 % |
| Red Chitin Armor Pauldron | 605001000 | 21 | 0.21 % |
| Red Chitin Armor Pads | 605002000 | 21 | 0.21 % |
| Red Chitin Armor Gauntlets | 605004000 | 21 | 0.21 % |
| Red Chitin Armor Boots | 605005000 | 21 | 0.21 % |
| Red Rogue Chitin Armor Helmet | 645003000 | 21 | 0.21 % |
| Red Rogue Chitin Armor Pauldron | 645001000 | 21 | 0.21 % |
| Red Rogue Chitin Armor Pads | 645002000 | 21 | 0.21 % |
| Red Rogue Chitin Armor Gauntlets | 645004000 | 21 | 0.21 % |
| Red Rogue Chitin Armor Boots | 645005000 | 21 | 0.21 % |
| Red Crimson Helmet | 685003000 | 21 | 0.21 % |
| Red Crimson Robe | 685001000 | 21 | 0.21 % |
| Red Crimson Pants | 685002000 | 21 | 0.21 % |
| Red Crimson Glove | 685004000 | 21 | 0.21 % |
| Red Crimson Boots | 685005000 | 21 | 0.21 % |
| Red Priest Chitin Armor Helmet | 605103000 | 21 | 0.21 % |
| Red Priest Chitin Armor Pauldron | 605101000 | 21 | 0.21 % |
| Red Priest Chitin Armor Pads | 605102000 | 21 | 0.21 % |
| Red Priest Chitin Armor Gauntlets | 605104000 | 21 | 0.21 % |
| Red Priest Chitin Armor Boots | 605105000 | 21 | 0.21 % |
| Exceptional Raptor | 156310000 | 20 | 0.20 % |
| Exceptional Shard | 111310000 | 20 | 0.20 % |
| Exceptional Iron Bow | 168610000 | 20 | 0.20 % |
| Exceptional Elixir Staff | 181210000 | 20 | 0.20 % |
| Exceptional Iron Impact | 146310000 | 20 | 0.20 % |
| Exceptional Totamic Spear | 151310000 | 20 | 0.20 % |
| Exceptional Impact | 141210000 | 20 | 0.20 % |
| Exceptional Graham | 121410000 | 20 | 0.20 % |
| Exceptional Deep Scar | 131210000 | 20 | 0.20 % |
| Exceptional Priest Impact | 191210000 | 20 | 0.20 % |
| Exceptional Mirage | 126610000 | 20 | 0.20 % |
| Exceptional Blade Axe | 136310000 | 20 | 0.20 % |

### Accessory Box [508194000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Bronze Belt | 340310108 | 44 | 0.44 % |
| Golden Earring | 310110103 | 44 | 0.44 % |
| Mage Earring | 310310004 | 44 | 0.44 % |
| Cleric Earring | 310310007 | 44 | 0.44 % |
| Elf-Metal Earring | 310610106 | 44 | 0.44 % |
| White-Silver Earring | 310610108 | 44 | 0.44 % |
| Elemental Necklace | 320310124 | 44 | 0.44 % |
| Black Dragon Necklace | 320310130 | 44 | 0.44 % |
| White Dragon Necklace | 320510132 | 44 | 0.44 % |
| Crystal Necklace | 320610125 | 44 | 0.44 % |
| Silver Earring | 310610102 | 44 | 0.44 % |
| Belt of Life | 340110101 | 44 | 0.44 % |
| Light Belt of Life | 340110120 | 44 | 0.44 % |
| Kekuri Belt | 340110255 | 44 | 0.44 % |
| Kekuri Ring | 330310014 | 44 | 0.44 % |
| Warrior Pendant | 320110115 | 44 | 0.44 % |
| Elemental Pendant | 320110116 | 44 | 0.44 % |
| Priest Pendant | 320110117 | 44 | 0.44 % |
| Flame Ring | 330910140 | 44 | 0.44 % |
| Platinum Earring | 310410105 | 36 | 0.36 % |
| Secret-Silver Earring | 310410107 | 36 | 0.36 % |
| Agate Earring | 310410109 | 36 | 0.36 % |
| Amulet of Health | 320510123 | 36 | 0.36 % |
| Amulet of Goddess | 320510127 | 36 | 0.36 % |
| Amulet of Strength | 320610119 | 36 | 0.36 % |
| Elf Belt | 340410113 | 36 | 0.36 % |
| Judicious Belt | 340610122 | 36 | 0.36 % |
| Sash of Sorrows | 340510121 | 36 | 0.36 % |
| Diamond Ring | 330410261 | 36 | 0.36 % |
| Elf Ring | 330610265 | 36 | 0.36 % |
| Ring of Magic | 330150256 | 36 | 0.36 % |
| Warrior Earring | 310310005 | 29 | 0.29 % |
| Rogue Earring | 310310006 | 29 | 0.29 % |
| Opal Earring | 310510110 | 29 | 0.29 % |
| Blue Dragon Necklace | 320610128 | 29 | 0.29 % |
| Amulet of Dexterity | 320510120 | 29 | 0.29 % |
| Ring of Life | 330150257 | 29 | 0.29 % |
| Ring of Courage | 330110255 | 29 | 0.29 % |
| Skeleton Belt | 340410115 | 15 | 0.15 % |
| Belt of Dexterity | 340610111 | 14 | 0.14 % |

### Exp Item - Chitin Shell Box [509194000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 32 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Exceptional Raptor | 156310000 | 47 | 0.47 % |
| Exceptional Shard | 111310000 | 47 | 0.47 % |
| Exceptional Iron Bow | 168610000 | 47 | 0.47 % |
| Exceptional Iron Impact | 146310000 | 47 | 0.47 % |
| Exceptional Elixir Staff | 181210000 | 47 | 0.47 % |
| Exceptional Totamic Spear | 151310000 | 47 | 0.47 % |
| Exceptional Impact | 141210000 | 47 | 0.47 % |
| Exceptional Graham | 121410000 | 47 | 0.47 % |
| Exceptional Deep Scar | 131210000 | 47 | 0.47 % |
| Exceptional Priest Impact | 191210000 | 47 | 0.47 % |
| Exceptional Mirage | 126610000 | 47 | 0.47 % |
| Exceptional Blade Axe | 136310000 | 47 | 0.47 % |
| Chitin Shell Helmet | 206003000 | 47 | 0.47 % |
| Chitin Shell Pauldron | 206001000 | 47 | 0.47 % |
| Chitin Shell Pads | 206002000 | 47 | 0.47 % |
| Chitin Shell Gauntlets | 206004000 | 47 | 0.47 % |
| Chitin Shell Boots | 206005000 | 47 | 0.47 % |
| Rogue Chitin Shell Helmet | 246003000 | 47 | 0.47 % |
| Rogue Chitin Shell Pauldron | 246001000 | 47 | 0.47 % |
| Rogue Chitin Shell Pads | 246002000 | 47 | 0.47 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 47 | 0.47 % |
| Rogue Chitin Shell Boots | 246005000 | 47 | 0.47 % |
| Complete Helmet | 266003000 | 47 | 0.47 % |
| Complete Robe | 266001000 | 47 | 0.47 % |
| Complete Pants | 266002000 | 47 | 0.47 % |
| Complete Glove | 266004000 | 47 | 0.47 % |
| Complete Boots | 266005000 | 47 | 0.47 % |
| Priest Chitin Shell Helmet | 286003000 | 47 | 0.47 % |
| Priest Chitin Shell Pauldron | 286001000 | 46 | 0.46 % |
| Priest Chitin Shell Pads | 286002000 | 46 | 0.46 % |
| Priest Chitin Shell Gauntlets | 286004000 | 46 | 0.46 % |
| Priest Chitin Shell Boots | 286005000 | 46 | 0.46 % |

### Dragon Box [810341000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Dragon Scale Armor Pants | 507002000 | 75 | 0.75 % |
| Dragon Scale Armor Pants | 922002000 | 75 | 0.75 % |
| Dragon Scale Armor Pants | 917002000 | 75 | 0.75 % |
| Dragon Scale Armor Pants | 517002000 | 75 | 0.75 % |
| Dragon Scale Armor Top | 922001000 | 75 | 0.75 % |
| Dragon Scale Armor Top | 517001000 | 75 | 0.75 % |
| Dragon Scale Armor Top | 507001000 | 75 | 0.75 % |
| Dragon Scale Armor Top | 917001000 | 75 | 0.75 % |
| Dragon Scale Boots | 507005000 | 75 | 0.75 % |
| Dragon Scale Boots | 917005000 | 75 | 0.75 % |
| Dragon Scale Boots | 517005000 | 75 | 0.75 % |
| Dragon Scale Boots | 922005000 | 75 | 0.75 % |
| Dragon Scale Gauntlet | 922004000 | 75 | 0.75 % |
| Dragon Scale Gauntlet | 517004000 | 75 | 0.75 % |
| Dragon Scale Gauntlet | 917004000 | 75 | 0.75 % |
| Dragon Scale Gauntlet | 507004000 | 75 | 0.75 % |
| Dragon Scale Helmet | 507003000 | 75 | 0.75 % |
| Dragon Scale Helmet | 922003000 | 75 | 0.75 % |
| Dragon Scale Helmet | 917003000 | 75 | 0.75 % |
| Dragon Scale Helmet | 517003000 | 75 | 0.75 % |

### Superior Draki Supply Box [810596000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 110 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Complete Pants | 266002000 | 53 | 0.53 % |
| Priest Bone crasher | 191010000 | 40 | 0.40 % |
| Priest Impact | 191110000 | 40 | 0.40 % |
| Chitin Shell Pads | 206002000 | 40 | 0.40 % |
| Draki's Pendant | 1190029001 | 33 | 0.33 % |
| Mirage Dagger | 111110000 | 27 | 0.27 % |
| Tomahawk | 131010000 | 27 | 0.27 % |
| Bone Crasher | 141010000 | 27 | 0.27 % |
| Philum | 151010000 | 27 | 0.27 % |
| Harpoon | 151110000 | 27 | 0.27 % |
| Glave | 156110000 | 27 | 0.27 % |
| Salamander Staff | 181010000 | 27 | 0.27 % |
| Blade Axe | 136210000 | 27 | 0.27 % |
| Raptor | 156210000 | 26 | 0.26 % |
| Rogue Chitin Shell Pauldron | 246001000 | 26 | 0.26 % |
| Mirage | 126410000 | 26 | 0.26 % |
| Deep Scar | 131110000 | 26 | 0.26 % |
| Impact | 141110000 | 26 | 0.26 % |
| Elixir Staff | 181110000 | 26 | 0.26 % |
| Chitin Bow | 160450255 | 26 | 0.26 % |
| Glacier Erenion | 189201107 | 26 | 0.26 % |
| Smite Hammer | 199101105 | 26 | 0.26 % |
| Hanguk Sword | 129101102 | 24 | 0.24 % |
| Hell Breaker | 149101103 | 24 | 0.24 % |
| Enion Bow | 169101104 | 24 | 0.24 % |
| Lightning Erenion | 189301108 | 24 | 0.24 % |
| Defender of the Lord | 179101110 | 22 | 0.22 % |
| Cleaver | 111010000 | 13 | 0.13 % |
| Slayer | 121210000 | 13 | 0.13 % |
| Sword of the Dead | 125350258 | 13 | 0.13 % |
| Durandal | 126310000 | 13 | 0.13 % |
| Bone Cleaver | 136110000 | 13 | 0.13 % |
| Totamic Club | 146110000 | 13 | 0.13 % |
| Bardish | 156010000 | 13 | 0.13 % |
| Iron Crossbow | 168310000 | 13 | 0.13 % |
| Chitin Armor Pads | 205002000 | 13 | 0.13 % |
| Chitin Gauntlets | 205004000 | 13 | 0.13 % |
| Rogue Chitin Armor Pauldron | 245001000 | 13 | 0.13 % |
| Crimson Gloves | 265004000 | 13 | 0.13 % |
| Chitin Armor Pauldron | 205001000 | 13 | 0.13 % |
| Chitin Helmet | 205003000 | 13 | 0.13 % |
| Chitin Boots | 205005000 | 13 | 0.13 % |
| Rogue Chitin Armor Pads | 245002000 | 13 | 0.13 % |
| Rogue Chitin Gauntlets | 245004000 | 13 | 0.13 % |
| Crimson Robe | 265001000 | 13 | 0.13 % |
| Crimson Boots | 265005000 | 13 | 0.13 % |
| Priest Chitin Armor Pads | 285002000 | 13 | 0.13 % |
| Priest Chitin Gauntlets | 285004000 | 13 | 0.13 % |
| Chitin Shell Pauldron | 206001000 | 13 | 0.13 % |
| Chitin Shell Helmet | 206003000 | 13 | 0.13 % |
| Chitin Shell Gauntlets | 206004000 | 13 | 0.13 % |
| Chitin Shell Boots | 206005000 | 13 | 0.13 % |
| Rogue Chitin Helmet | 245003000 | 13 | 0.13 % |
| Rogue Chitin Boots | 245005000 | 13 | 0.13 % |
| Rogue Chitin Shell Pads | 246002000 | 13 | 0.13 % |
| Rogue Chitin Shell Helmet | 246003000 | 13 | 0.13 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 13 | 0.13 % |
| Rogue Chitin Shell Boots | 246005000 | 13 | 0.13 % |
| Crimson Pants | 265002000 | 13 | 0.13 % |
| Complete Robe | 266001000 | 13 | 0.13 % |
| Complete Helmet | 266003000 | 13 | 0.13 % |
| Complete Glove | 266004000 | 13 | 0.13 % |
| Complete Boots | 266005000 | 13 | 0.13 % |
| Priest Chitin Boots | 285005000 | 13 | 0.13 % |
| Priest Chitin Shell Pauldron | 286001000 | 13 | 0.13 % |
| Priest Chitin Shell Pads | 286002000 | 13 | 0.13 % |
| Priest Chitin Shell Helmet | 286003000 | 13 | 0.13 % |
| Priest Chitin Shell Gauntlets | 286004000 | 13 | 0.13 % |
| Priest Chitin Shell Boots | 286005000 | 13 | 0.13 % |
| Shard | 111210000 | 13 | 0.13 % |
| Graham | 121310000 | 13 | 0.13 % |
| Iron Impact | 146210000 | 13 | 0.13 % |
| Totamic Spear | 151210000 | 13 | 0.13 % |
| Scorpion Bow | 160450256 | 13 | 0.13 % |
| Iron Bow | 168410000 | 13 | 0.13 % |
| Priest Chitin Armor Pauldron | 285001000 | 13 | 0.13 % |
| Ron's Staff | 189401109 | 13 | 0.13 % |
| Syphioric | 179301139 | 11 | 0.11 % |
| Dark Vane | 119101101 | 11 | 0.11 % |
| Weapon Breaker | 1111310000 | 10 | 0.10 % |
| Gold bar | 379068000 | 9 | 0.09 % |
| Jewel of Strength | 810597000 | 5 | 0.05 % |
| Jewel of Dexterity | 810598000 | 5 | 0.05 % |
| Jewel of Intelligence | 810599000 | 5 | 0.05 % |
| Jewel of Magic | 810600000 | 5 | 0.05 % |
| Jewel of Philosopher | 810610000 | 5 | 0.05 % |
| Jewel of Hardness | 810611000 | 5 | 0.05 % |
| Jewel of Absortion | 810612000 | 5 | 0.05 % |

### Old Draki Supply Box [810678000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 99 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Complete Pants | 266002000 | 60 | 0.60 % |
| Priest Bone crasher | 191010000 | 45 | 0.45 % |
| Chitin Shell Pads | 206002000 | 45 | 0.45 % |
| Mirage Dagger | 111110000 | 30 | 0.30 % |
| Tomahawk | 131010000 | 30 | 0.30 % |
| Philum | 151010000 | 30 | 0.30 % |
| Harpoon | 151110000 | 30 | 0.30 % |
| Glave | 156110000 | 30 | 0.30 % |
| Salamander Staff | 181010000 | 30 | 0.30 % |
| Blade Axe | 136210000 | 30 | 0.30 % |
| Raptor | 156210000 | 30 | 0.30 % |
| Priest Impact | 191110000 | 30 | 0.30 % |
| Rogue Chitin Shell Pauldron | 246001000 | 30 | 0.30 % |
| Mirage | 126410000 | 30 | 0.30 % |
| Deep Scar | 131110000 | 30 | 0.30 % |
| Impact | 141110000 | 30 | 0.30 % |
| Hanguk Sword | 129101102 | 27 | 0.27 % |
| Cleaver | 111010000 | 15 | 0.15 % |
| Slayer | 121210000 | 15 | 0.15 % |
| Sword of the Dead | 125350258 | 15 | 0.15 % |
| Durandal | 126310000 | 15 | 0.15 % |
| Bone Cleaver | 136110000 | 15 | 0.15 % |
| Bone Crasher | 141010000 | 15 | 0.15 % |
| Totamic Club | 146110000 | 15 | 0.15 % |
| Bardish | 156010000 | 15 | 0.15 % |
| Iron Crossbow | 168310000 | 15 | 0.15 % |
| Chitin Armor Pads | 205002000 | 15 | 0.15 % |
| Chitin Gauntlets | 205004000 | 15 | 0.15 % |
| Rogue Chitin Armor Pauldron | 245001000 | 15 | 0.15 % |
| Crimson Gloves | 265004000 | 15 | 0.15 % |
| Chitin Armor Pauldron | 205001000 | 15 | 0.15 % |
| Chitin Helmet | 205003000 | 15 | 0.15 % |
| Chitin Boots | 205005000 | 15 | 0.15 % |
| Rogue Chitin Armor Pads | 245002000 | 15 | 0.15 % |
| Rogue Chitin Gauntlets | 245004000 | 15 | 0.15 % |
| Crimson Robe | 265001000 | 15 | 0.15 % |
| Crimson Boots | 265005000 | 15 | 0.15 % |
| Priest Chitin Armor Pads | 285002000 | 15 | 0.15 % |
| Priest Chitin Gauntlets | 285004000 | 15 | 0.15 % |
| Chitin Shell Pauldron | 206001000 | 15 | 0.15 % |
| Chitin Shell Helmet | 206003000 | 15 | 0.15 % |
| Chitin Shell Gauntlets | 206004000 | 15 | 0.15 % |
| Chitin Shell Boots | 206005000 | 15 | 0.15 % |
| Rogue Chitin Helmet | 245003000 | 15 | 0.15 % |
| Rogue Chitin Boots | 245005000 | 15 | 0.15 % |
| Rogue Chitin Shell Pads | 246002000 | 15 | 0.15 % |
| Rogue Chitin Shell Helmet | 246003000 | 15 | 0.15 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 15 | 0.15 % |
| Rogue Chitin Shell Boots | 246005000 | 15 | 0.15 % |
| Crimson Pants | 265002000 | 15 | 0.15 % |
| Complete Robe | 266001000 | 15 | 0.15 % |
| Complete Helmet | 266003000 | 15 | 0.15 % |
| Complete Glove | 266004000 | 15 | 0.15 % |
| Complete Boots | 266005000 | 15 | 0.15 % |
| Priest Chitin Boots | 285005000 | 15 | 0.15 % |
| Priest Chitin Shell Pauldron | 286001000 | 15 | 0.15 % |
| Priest Chitin Shell Pads | 286002000 | 15 | 0.15 % |
| Priest Chitin Shell Helmet | 286003000 | 15 | 0.15 % |
| Priest Chitin Shell Gauntlets | 286004000 | 15 | 0.15 % |
| Priest Chitin Shell Boots | 286005000 | 15 | 0.15 % |
| Shard | 111210000 | 15 | 0.15 % |
| Graham | 121310000 | 15 | 0.15 % |
| Iron Impact | 146210000 | 15 | 0.15 % |
| Totamic Spear | 151210000 | 15 | 0.15 % |
| Scorpion Bow | 160450256 | 15 | 0.15 % |
| Iron Bow | 168410000 | 15 | 0.15 % |
| Elixir Staff | 181110000 | 15 | 0.15 % |
| Priest Chitin Armor Pauldron | 285001000 | 15 | 0.15 % |
| Hell Breaker | 149101103 | 15 | 0.15 % |
| Chitin Bow | 160450255 | 15 | 0.15 % |
| Enion Bow | 169101104 | 15 | 0.15 % |
| Glacier Erenion | 189201107 | 15 | 0.15 % |
| Lightning Erenion | 189301108 | 15 | 0.15 % |
| Ron's Staff | 189401109 | 15 | 0.15 % |
| Smite Hammer | 199101105 | 15 | 0.15 % |
| Syphioric | 179301139 | 12 | 0.12 % |
| Dark Vane | 119101101 | 12 | 0.12 % |
| Defender of the Lord | 179101110 | 12 | 0.12 % |
| Draki's Pendant | 1190029001 | 10 | 0.10 % |
| Jewel of Strength | 810597000 | 3 | 0.03 % |
| Jewel of Dexterity | 810598000 | 2 | 0.02 % |
| Jewel of Intelligence | 810599000 | 2 | 0.02 % |
| Jewel of Magic | 810600000 | 2 | 0.02 % |
| Jewel of Philosopher | 810610000 | 2 | 0.02 % |
| Jewel of Hardness | 810611000 | 2 | 0.02 % |
| Jewel of Absortion | 810612000 | 2 | 0.02 % |
| Silver bar | 379067000 | 2 | 0.02 % |

### Dark Fragment of Pride [810943000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 65 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 147 | 1.47 % |
| Weapon Enchant Scroll | 389155000 | 147 | 1.47 % |
| Blessed Upgrade Scroll | 379021000 | 93 | 0.93 % |
| Nightmare Crystal | 389242000 | 93 | 0.93 % |
| Rogue Chitin Shell Helmet | 246003000 | 29 | 0.29 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 29 | 0.29 % |
| Rogue Chitin Shell Boots | 246005000 | 29 | 0.29 % |
| Priest Chitin Shell Pauldron | 286001000 | 29 | 0.29 % |
| Priest Chitin Shell Pads | 286002000 | 29 | 0.29 % |
| Priest Chitin Shell Gauntlets | 286004000 | 29 | 0.29 % |
| Priest Chitin Shell Boots | 286005000 | 29 | 0.29 % |
| Complete Pants | 266002000 | 29 | 0.29 % |
| Complete Helmet | 266003000 | 29 | 0.29 % |
| Complete Glove | 266004000 | 29 | 0.29 % |
| Complete Boots | 266005000 | 29 | 0.29 % |
| Priest Chitin Shell Helmet | 286003000 | 29 | 0.29 % |
| Chitin Shell Boots | 206005000 | 25 | 0.25 % |
| Rogue Chitin Shell Pauldron | 246001000 | 25 | 0.25 % |
| Complete Robe | 266001000 | 24 | 0.24 % |
| Shard | 111210000 | 24 | 0.24 % |
| Raptor | 156210000 | 24 | 0.24 % |
| Iron Impact | 146210000 | 24 | 0.24 % |
| Iron Bow | 168410000 | 24 | 0.24 % |
| Totamic Spear | 151210000 | 24 | 0.24 % |
| Impact | 141110000 | 24 | 0.24 % |
| Priest Impact | 191110000 | 24 | 0.24 % |
| Elixir Staff | 181110000 | 24 | 0.24 % |
| Durandal | 126310000 | 24 | 0.24 % |
| Mirage | 126410000 | 24 | 0.24 % |
| Deep Scar | 131110000 | 24 | 0.24 % |
| Graham | 121310000 | 24 | 0.24 % |
| Blade Axe | 136210000 | 24 | 0.24 % |
| Chitin Shell Pads | 206002000 | 24 | 0.24 % |
| Chitin Shell Helmet | 206003000 | 24 | 0.24 % |
| Rogue Chitin Shell Pads | 246002000 | 24 | 0.24 % |
| Chitin Shell Pauldron | 206001000 | 24 | 0.24 % |
| Chitin Shell Gauntlets | 206004000 | 24 | 0.24 % |
| Dark Fragment of Gluttony | 810944000 | 20 | 0.20 % |
| Dark Fragment of Wrath | 810948000 | 17 | 0.17 % |
| Moonlight Fusion Stone | 890109000 | 11 | 0.11 % |
| Magic Fusion Stone | 890110000 | 11 | 0.11 % |
| Solar Fusion Stone | 890111000 | 11 | 0.11 % |
| Chaotic Fusion Stone | 890112000 | 11 | 0.11 % |
| Dark Fragment of Sloth | 810949000 | 8 | 0.08 % |
| Avedon | 139301133 | 3 | 0.03 % |
| Sherion | 119301131 | 3 | 0.03 % |
| Helenid | 169301134 | 3 | 0.03 % |
| Hell Blood | 189601135 | 3 | 0.03 % |
| Elysium | 189603137 | 3 | 0.03 % |
| Garp | 189602136 | 3 | 0.03 % |
| Syphioric | 179301139 | 3 | 0.03 % |
| Lugias | 159301132 | 3 | 0.03 % |
| Holy Animor | 199301138 | 3 | 0.03 % |
| Dark Fragment of Pride | 810943000 | 1 | 0.01 % |
| Foverin | 330940143 | 1 | 0.01 % |
| Dark Fragment of Lust | 810950000 | 1 | 0.01 % |
| Dark Fragment of Envy | 810951000 | 1 | 0.01 % |
| Dark Fragment of Greed | 810952000 | 1 | 0.01 % |
| Moonlight Ancient Metal Plate | 890104000 | 1 | 0.01 % |
| Magic Ancient Metal Plate | 890105000 | 1 | 0.01 % |
| Solar Ancient Metal Plate | 890106000 | 1 | 0.01 % |
| Chaotic Ancient Metal Plate | 890107000 | 1 | 0.01 % |
| Shio Tears | 330920141 | 1 | 0.01 % |
| Imir Ring | 330930142 | 1 | 0.01 % |
| Flame Ring | 330910140 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |
| Cold-Hearted Dagger | 119102294 | 1 | 0.01 % |
| Eagle's Eye | 169302345 | 1 | 0.01 % |
| Spear of Murky Waters | 159302311 | 1 | 0.01 % |
| Scorching Staff | 189302255 | 1 | 0.01 % |
| Oasis Staff | 189102256 | 1 | 0.01 % |
| Chaotic Staff | 189202257 | 1 | 0.01 % |
| Stone Splitter | 199102262 | 1 | 0.01 % |
| Dread Shield | 179102191 | 1 | 0.01 % |
| Dragon Scale Armor Top | 507001000 | 1 | 0.01 % |
| Dragon Scale Armor Pants | 507002000 | 1 | 0.01 % |
| Dragon Scale Helmet | 507003000 | 1 | 0.01 % |
| Dragon Scale Gauntlet | 507004000 | 1 | 0.01 % |
| Dragon Scale Boots | 507005000 | 1 | 0.01 % |
| Mythril Pauldron | 547001000 | 1 | 0.01 % |
| Mythril Pad | 547002000 | 1 | 0.01 % |
| Mythril Helmet | 547003000 | 1 | 0.01 % |
| Mythril Gauntlet | 547004000 | 1 | 0.01 % |
| Mythril Boots | 547005000 | 1 | 0.01 % |
| Pauldron of Trial | 587001000 | 1 | 0.01 % |
| Pads of Trial | 587002000 | 1 | 0.01 % |
| Helmet of Trial | 587003000 | 1 | 0.01 % |
| Gauntlet of Trial | 587004000 | 1 | 0.01 % |
| Boots of Trial | 587005000 | 1 | 0.01 % |
| Ron's Robe | 567001000 | 1 | 0.01 % |
| Ron's Pants | 567002000 | 1 | 0.01 % |
| Ron's Helmet | 567003000 | 1 | 0.01 % |
| Ron's Gloves | 567004000 | 1 | 0.01 % |
| Ron's Boots | 567005000 | 1 | 0.01 % |

### Dark Fragment of Gluttony [810944000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 64 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Armor Enchant Scroll | 389156000 | 138 | 1.38 % |
| Weapon Enchant Scroll | 389155000 | 138 | 1.38 % |
| Nightmare Crystal | 389242000 | 97 | 0.97 % |
| Blessed Upgrade Scroll | 379021000 | 77 | 0.77 % |
| Rogue Chitin Shell Pads | 246002000 | 32 | 0.32 % |
| Rogue Chitin Shell Helmet | 246003000 | 32 | 0.32 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 32 | 0.32 % |
| Rogue Chitin Shell Boots | 246005000 | 32 | 0.32 % |
| Priest Chitin Shell Pauldron | 286001000 | 32 | 0.32 % |
| Priest Chitin Shell Pads | 286002000 | 32 | 0.32 % |
| Priest Chitin Shell Helmet | 286003000 | 32 | 0.32 % |
| Priest Chitin Shell Gauntlets | 286004000 | 32 | 0.32 % |
| Priest Chitin Shell Boots | 286005000 | 32 | 0.32 % |
| Complete Robe | 266001000 | 32 | 0.32 % |
| Complete Pants | 266002000 | 32 | 0.32 % |
| Complete Helmet | 266003000 | 32 | 0.32 % |
| Complete Glove | 266004000 | 32 | 0.32 % |
| Complete Boots | 266005000 | 32 | 0.32 % |
| Rogue Chitin Shell Pauldron | 246001000 | 26 | 0.26 % |
| Shard | 111210000 | 25 | 0.25 % |
| Raptor | 156210000 | 25 | 0.25 % |
| Iron Impact | 146210000 | 25 | 0.25 % |
| Iron Bow | 168410000 | 25 | 0.25 % |
| Totamic Spear | 151210000 | 25 | 0.25 % |
| Impact | 141110000 | 25 | 0.25 % |
| Priest Impact | 191110000 | 25 | 0.25 % |
| Elixir Staff | 181110000 | 25 | 0.25 % |
| Durandal | 126310000 | 25 | 0.25 % |
| Mirage | 126410000 | 25 | 0.25 % |
| Deep Scar | 131110000 | 25 | 0.25 % |
| Graham | 121310000 | 25 | 0.25 % |
| Blade Axe | 136210000 | 25 | 0.25 % |
| Chitin Shell Pads | 206002000 | 25 | 0.25 % |
| Chitin Shell Gauntlets | 206004000 | 25 | 0.25 % |
| Dark Fragment of Wrath | 810948000 | 16 | 0.16 % |
| Moonlight Fusion Stone | 890109000 | 13 | 0.13 % |
| Magic Fusion Stone | 890110000 | 13 | 0.13 % |
| Solar Fusion Stone | 890111000 | 13 | 0.13 % |
| Chaotic Fusion Stone | 890112000 | 13 | 0.13 % |
| Dark Fragment of Sloth | 810949000 | 8 | 0.08 % |
| Recipe: Holy Knight Warrior Armor | 810961000 | 6 | 0.06 % |
| Recipe: Holy Knight Assassin Armor | 810962000 | 6 | 0.06 % |
| Recipe: Holy Knight Mage Armor | 810963000 | 6 | 0.06 % |
| Recipe: Holy Knight Priest Armor | 810964000 | 6 | 0.06 % |
| Recipe: Holy Knight Portu Armor | 810965000 | 6 | 0.06 % |
| Sherion | 119301131 | 4 | 0.04 % |
| Avedon | 139301133 | 4 | 0.04 % |
| Helenid | 169301134 | 4 | 0.04 % |
| Hell Blood | 189601135 | 4 | 0.04 % |
| Elysium | 189603137 | 4 | 0.04 % |
| Garp | 189602136 | 4 | 0.04 % |
| Syphioric | 179301139 | 4 | 0.04 % |
| Lugias | 159301132 | 4 | 0.04 % |
| Holy Animor | 199301138 | 4 | 0.04 % |
| Dark Fragment of Lust | 810950000 | 3 | 0.03 % |
| Dark Fragment of Envy | 810951000 | 2 | 0.02 % |
| Dark Fragment of Greed | 810952000 | 2 | 0.02 % |
| Foverin | 330940143 | 2 | 0.02 % |
| Shio Tears | 330920141 | 2 | 0.02 % |
| Imir Ring | 330930142 | 2 | 0.02 % |
| Flame Ring | 330910140 | 2 | 0.02 % |
| Dark Fragment of Pride | 810943000 | 1 | 0.01 % |
| Dark Fragment of Gluttony | 810944000 | 1 | 0.01 % |
| Moonlight Ancient Metal Plate | 890104000 | 1 | 0.01 % |
| Magic Ancient Metal Plate | 890105000 | 1 | 0.01 % |
| Solar Ancient Metal Plate | 890106000 | 1 | 0.01 % |
| Chaotic Ancient Metal Plate | 890107000 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |
| Cold-Hearted Dagger | 119102294 | 1 | 0.01 % |
| Eagle's Eye | 169302345 | 1 | 0.01 % |
| Spear of Murky Waters | 159302311 | 1 | 0.01 % |
| Scorching Staff | 189302255 | 1 | 0.01 % |
| Oasis Staff | 189102256 | 1 | 0.01 % |
| Chaotic Staff | 189202257 | 1 | 0.01 % |
| Stone Splitter | 199102262 | 1 | 0.01 % |
| Dread Shield | 179102191 | 1 | 0.01 % |
| Dragon Scale Armor Top | 507001000 | 1 | 0.01 % |
| Dragon Scale Armor Pants | 507002000 | 1 | 0.01 % |
| Dragon Scale Helmet | 507003000 | 1 | 0.01 % |
| Dragon Scale Gauntlet | 507004000 | 1 | 0.01 % |
| Dragon Scale Boots | 507005000 | 1 | 0.01 % |
| Mythril Pauldron | 547001000 | 1 | 0.01 % |
| Mythril Pad | 547002000 | 1 | 0.01 % |
| Mythril Helmet | 547003000 | 1 | 0.01 % |
| Mythril Gauntlet | 547004000 | 1 | 0.01 % |
| Mythril Boots | 547005000 | 1 | 0.01 % |
| Pauldron of Trial | 587001000 | 1 | 0.01 % |
| Pads of Trial | 587002000 | 1 | 0.01 % |
| Helmet of Trial | 587003000 | 1 | 0.01 % |
| Gauntlet of Trial | 587004000 | 1 | 0.01 % |
| Boots of Trial | 587005000 | 1 | 0.01 % |
| Ron's Robe | 567001000 | 1 | 0.01 % |
| Ron's Pants | 567002000 | 1 | 0.01 % |
| Ron's Helmet | 567003000 | 1 | 0.01 % |
| Ron's Gloves | 567004000 | 1 | 0.01 % |
| Ron's Boots | 567005000 | 1 | 0.01 % |

### Dark Fragment of Wrath [810948000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 65 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Weapon Enchant Scroll | 389155000 | 91 | 0.91 % |
| Armor Enchant Scroll | 389156000 | 52 | 0.52 % |
| Chitin Shell Boots | 206005000 | 44 | 0.44 % |
| Rogue Chitin Shell Pads | 246002000 | 44 | 0.44 % |
| Rogue Chitin Shell Helmet | 246003000 | 44 | 0.44 % |
| Rogue Chitin Shell Boots | 246005000 | 44 | 0.44 % |
| Priest Chitin Shell Pads | 286002000 | 44 | 0.44 % |
| Priest Chitin Shell Helmet | 286003000 | 44 | 0.44 % |
| Priest Chitin Shell Gauntlets | 286004000 | 44 | 0.44 % |
| Priest Chitin Shell Boots | 286005000 | 44 | 0.44 % |
| Complete Robe | 266001000 | 44 | 0.44 % |
| Complete Pants | 266002000 | 44 | 0.44 % |
| Complete Helmet | 266003000 | 44 | 0.44 % |
| Complete Glove | 266004000 | 44 | 0.44 % |
| Complete Boots | 266005000 | 44 | 0.44 % |
| Fragment of Sloth | 389163000 | 33 | 0.33 % |
| Blessed Upgrade Scroll | 379021000 | 30 | 0.30 % |
| Chitin Shell Pauldron | 206001000 | 29 | 0.29 % |
| Chitin Shell Pads | 206002000 | 29 | 0.29 % |
| Chitin Shell Helmet | 206003000 | 29 | 0.29 % |
| Shard | 111210000 | 29 | 0.29 % |
| Raptor | 156210000 | 29 | 0.29 % |
| Iron Impact | 146210000 | 29 | 0.29 % |
| Iron Bow | 168410000 | 29 | 0.29 % |
| Totamic Spear | 151210000 | 29 | 0.29 % |
| Impact | 141110000 | 29 | 0.29 % |
| Priest Impact | 191110000 | 29 | 0.29 % |
| Elixir Staff | 181110000 | 29 | 0.29 % |
| Durandal | 126310000 | 29 | 0.29 % |
| Mirage | 126410000 | 29 | 0.29 % |
| Deep Scar | 131110000 | 29 | 0.29 % |
| Graham | 121310000 | 29 | 0.29 % |
| Blade Axe | 136210000 | 29 | 0.29 % |
| Moonlight Fusion Stone | 890109000 | 16 | 0.16 % |
| Magic Fusion Stone | 890110000 | 16 | 0.16 % |
| Solar Fusion Stone | 890111000 | 16 | 0.16 % |
| Chaotic Fusion Stone | 890112000 | 16 | 0.16 % |
| Rogue Chitin Shell Pauldron | 246001000 | 14 | 0.14 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 14 | 0.14 % |
| Priest Chitin Shell Pauldron | 286001000 | 14 | 0.14 % |
| Recipe: Holy Knight Mage Armor | 810963000 | 8 | 0.08 % |
| Recipe: Holy Knight Priest Armor | 810964000 | 8 | 0.08 % |
| Recipe: Holy Knight Portu Armor | 810965000 | 8 | 0.08 % |
| Sherion | 119301131 | 7 | 0.07 % |
| Lugias | 159301132 | 7 | 0.07 % |
| Dark Fragment of Sloth | 810949000 | 6 | 0.06 % |
| Avedon | 139301133 | 6 | 0.06 % |
| Helenid | 169301134 | 6 | 0.06 % |
| Hell Blood | 189601135 | 6 | 0.06 % |
| Elysium | 189603137 | 6 | 0.06 % |
| Garp | 189602136 | 6 | 0.06 % |
| Syphioric | 179301139 | 6 | 0.06 % |
| Holy Animor | 199301138 | 6 | 0.06 % |
| Foverin | 330940143 | 3 | 0.03 % |
| Dark Fragment of Envy | 810951000 | 3 | 0.03 % |
| Shio Tears | 330920141 | 3 | 0.03 % |
| Imir Ring | 330930142 | 3 | 0.03 % |
| Flame Ring | 330910140 | 3 | 0.03 % |
| Dark Fragment of Pride | 810943000 | 2 | 0.02 % |
| Dark Fragment of Gluttony | 810944000 | 2 | 0.02 % |
| Dark Fragment of Wrath | 810948000 | 2 | 0.02 % |
| Dark Fragment of Lust | 810950000 | 2 | 0.02 % |
| Dark Fragment of Greed | 810952000 | 1 | 0.01 % |
| Moonlight Ancient Metal Plate | 890104000 | 1 | 0.01 % |
| Magic Ancient Metal Plate | 890105000 | 1 | 0.01 % |
| Solar Ancient Metal Plate | 890106000 | 1 | 0.01 % |
| Chaotic Ancient Metal Plate | 890107000 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |
| Cold-Hearted Dagger | 119102294 | 1 | 0.01 % |
| Eagle's Eye | 169302345 | 1 | 0.01 % |
| Spear of Murky Waters | 159302311 | 1 | 0.01 % |
| Scorching Staff | 189302255 | 1 | 0.01 % |
| Oasis Staff | 189102256 | 1 | 0.01 % |
| Chaotic Staff | 189202257 | 1 | 0.01 % |
| Stone Splitter | 199102262 | 1 | 0.01 % |
| Dread Shield | 179102191 | 1 | 0.01 % |
| Dragon Scale Armor Top | 507001000 | 1 | 0.01 % |
| Dragon Scale Armor Pants | 507002000 | 1 | 0.01 % |
| Dragon Scale Helmet | 507003000 | 1 | 0.01 % |
| Dragon Scale Gauntlet | 507004000 | 1 | 0.01 % |
| Dragon Scale Boots | 507005000 | 1 | 0.01 % |
| Mythril Pauldron | 547001000 | 1 | 0.01 % |
| Mythril Pad | 547002000 | 1 | 0.01 % |
| Mythril Helmet | 547003000 | 1 | 0.01 % |
| Mythril Gauntlet | 547004000 | 1 | 0.01 % |
| Mythril Boots | 547005000 | 1 | 0.01 % |
| Pauldron of Trial | 587001000 | 1 | 0.01 % |
| Pads of Trial | 587002000 | 1 | 0.01 % |
| Helmet of Trial | 587003000 | 1 | 0.01 % |
| Gauntlet of Trial | 587004000 | 1 | 0.01 % |
| Boots of Trial | 587005000 | 1 | 0.01 % |
| Ron's Robe | 567001000 | 1 | 0.01 % |
| Ron's Pants | 567002000 | 1 | 0.01 % |
| Ron's Helmet | 567003000 | 1 | 0.01 % |
| Ron's Gloves | 567004000 | 1 | 0.01 % |
| Ron's Boots | 567005000 | 1 | 0.01 % |

### Dark Fragment of Sloth [810949000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 72 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Nightmare Crystal | 389242000 | 110 | 1.10 % |
| Armor Enchant Scroll | 389156000 | 48 | 0.48 % |
| Rogue Chitin Shell Pads | 246002000 | 41 | 0.41 % |
| Rogue Chitin Shell Helmet | 246003000 | 41 | 0.41 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 41 | 0.41 % |
| Rogue Chitin Shell Boots | 246005000 | 41 | 0.41 % |
| Complete Robe | 266001000 | 41 | 0.41 % |
| Complete Pants | 266002000 | 41 | 0.41 % |
| Complete Helmet | 266003000 | 41 | 0.41 % |
| Complete Glove | 266004000 | 41 | 0.41 % |
| Complete Boots | 266005000 | 41 | 0.41 % |
| Priest Chitin Shell Pauldron | 286001000 | 41 | 0.41 % |
| Priest Chitin Shell Pads | 286002000 | 41 | 0.41 % |
| Priest Chitin Shell Gauntlets | 286004000 | 41 | 0.41 % |
| Priest Chitin Shell Boots | 286005000 | 41 | 0.41 % |
| Chitin Shell Pauldron | 206001000 | 40 | 0.40 % |
| Chitin Shell Pads | 206002000 | 40 | 0.40 % |
| Chitin Shell Helmet | 206003000 | 39 | 0.39 % |
| Chitin Shell Gauntlets | 206004000 | 39 | 0.39 % |
| Chitin Shell Boots | 206005000 | 39 | 0.39 % |
| Rogue Chitin Shell Pauldron | 246001000 | 39 | 0.39 % |
| Shard | 111210000 | 27 | 0.27 % |
| Raptor | 156210000 | 27 | 0.27 % |
| Iron Impact | 146210000 | 27 | 0.27 % |
| Iron Bow | 168410000 | 27 | 0.27 % |
| Totamic Spear | 151210000 | 27 | 0.27 % |
| Impact | 141110000 | 27 | 0.27 % |
| Priest Impact | 191110000 | 27 | 0.27 % |
| Elixir Staff | 181110000 | 27 | 0.27 % |
| Durandal | 126310000 | 27 | 0.27 % |
| Mirage | 126410000 | 27 | 0.27 % |
| Deep Scar | 131110000 | 27 | 0.27 % |
| Graham | 121310000 | 27 | 0.27 % |
| Blade Axe | 136210000 | 27 | 0.27 % |
| Priest Chitin Shell Helmet | 286003000 | 27 | 0.27 % |
| Moonlight Fusion Stone | 890109000 | 16 | 0.16 % |
| Magic Fusion Stone | 890110000 | 16 | 0.16 % |
| Solar Fusion Stone | 890111000 | 16 | 0.16 % |
| Chaotic Fusion Stone | 890112000 | 16 | 0.16 % |
| Recipe: Holy Knight Portu Armor | 810965000 | 9 | 0.09 % |
| Sherion | 119301131 | 7 | 0.07 % |
| Lugias | 159301132 | 7 | 0.07 % |
| Avedon | 139301133 | 7 | 0.07 % |
| Helenid | 169301134 | 7 | 0.07 % |
| Hell Blood | 189601135 | 7 | 0.07 % |
| Garp | 189602136 | 7 | 0.07 % |
| Elysium | 189603137 | 7 | 0.07 % |
| Syphioric | 179301139 | 6 | 0.06 % |
| Holy Animor | 199301138 | 6 | 0.06 % |
| Dark Fragment of Lust | 810950000 | 5 | 0.05 % |
| Shio Tears | 330920141 | 3 | 0.03 % |
| Imir Ring | 330930142 | 3 | 0.03 % |
| Flame Ring | 330910140 | 3 | 0.03 % |
| Dark Fragment of Pride | 810943000 | 2 | 0.02 % |
| Dark Fragment of Wrath | 810948000 | 2 | 0.02 % |
| Dark Fragment of Greed | 810952000 | 1 | 0.01 % |
| Moonlight Ancient Metal Plate | 890104000 | 1 | 0.01 % |
| Magic Ancient Metal Plate | 890105000 | 1 | 0.01 % |
| Solar Ancient Metal Plate | 890106000 | 1 | 0.01 % |
| Chaotic Ancient Metal Plate | 890107000 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |
| Cold-Hearted Dagger | 119102294 | 1 | 0.01 % |
| Eagle's Eye | 169302345 | 1 | 0.01 % |
| Spear of Murky Waters | 159302311 | 1 | 0.01 % |
| Scorching Staff | 189302255 | 1 | 0.01 % |
| Oasis Staff | 189102256 | 1 | 0.01 % |
| Chaotic Staff | 189202257 | 1 | 0.01 % |
| Stone Splitter | 199102262 | 1 | 0.01 % |
| Dread Shield | 179102191 | 1 | 0.01 % |
| Dragon Scale Armor Top | 507001000 | 1 | 0.01 % |
| Dragon Scale Armor Pants | 507002000 | 1 | 0.01 % |
| Dragon Scale Helmet | 507003000 | 1 | 0.01 % |
| Dragon Scale Gauntlet | 507004000 | 1 | 0.01 % |
| Dragon Scale Boots | 507005000 | 1 | 0.01 % |
| Mythril Pauldron | 547001000 | 1 | 0.01 % |
| Mythril Pad | 547002000 | 1 | 0.01 % |
| Mythril Helmet | 547003000 | 1 | 0.01 % |
| Mythril Gauntlet | 547004000 | 1 | 0.01 % |
| Mythril Boots | 547005000 | 1 | 0.01 % |
| Pauldron of Trial | 587001000 | 1 | 0.01 % |
| Pads of Trial | 587002000 | 1 | 0.01 % |
| Helmet of Trial | 587003000 | 1 | 0.01 % |
| Gauntlet of Trial | 587004000 | 1 | 0.01 % |
| Boots of Trial | 587005000 | 1 | 0.01 % |
| Ron's Robe | 567001000 | 1 | 0.01 % |
| Ron's Pants | 567002000 | 1 | 0.01 % |
| Ron's Helmet | 567003000 | 1 | 0.01 % |
| Ron's Gloves | 567004000 | 1 | 0.01 % |
| Ron's Boots | 567005000 | 1 | 0.01 % |

### Dark Fragment of Lust [810950000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 64 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Nightmare Crystal | 389242000 | 156 | 1.56 % |
| Weapon Enchant Scroll | 389155000 | 57 | 0.57 % |
| Rogue Chitin Shell Pauldron | 246001000 | 48 | 0.48 % |
| Rogue Chitin Shell Pads | 246002000 | 48 | 0.48 % |
| Rogue Chitin Shell Helmet | 246003000 | 48 | 0.48 % |
| Rogue Chitin Shell Boots | 246005000 | 48 | 0.48 % |
| Complete Pants | 266002000 | 48 | 0.48 % |
| Complete Helmet | 266003000 | 48 | 0.48 % |
| Complete Boots | 266005000 | 48 | 0.48 % |
| Priest Chitin Shell Pads | 286002000 | 48 | 0.48 % |
| Priest Chitin Shell Gauntlets | 286004000 | 48 | 0.48 % |
| Impact | 141110000 | 44 | 0.44 % |
| Chitin Shell Pauldron | 206001000 | 44 | 0.44 % |
| Chitin Shell Pads | 206002000 | 44 | 0.44 % |
| Chitin Shell Helmet | 206003000 | 44 | 0.44 % |
| Chitin Shell Gauntlets | 206004000 | 44 | 0.44 % |
| Chitin Shell Boots | 206005000 | 44 | 0.44 % |
| Graham | 121310000 | 33 | 0.33 % |
| Complete Robe | 266001000 | 33 | 0.33 % |
| Raptor | 156210000 | 33 | 0.33 % |
| Iron Bow | 168410000 | 33 | 0.33 % |
| Priest Impact | 191110000 | 33 | 0.33 % |
| Elixir Staff | 181110000 | 33 | 0.33 % |
| Durandal | 126310000 | 33 | 0.33 % |
| Mirage | 126410000 | 32 | 0.32 % |
| Moonlight Fusion Stone | 890109000 | 21 | 0.21 % |
| Magic Fusion Stone | 890110000 | 21 | 0.21 % |
| Solar Fusion Stone | 890111000 | 21 | 0.21 % |
| Chaotic Fusion Stone | 890112000 | 21 | 0.21 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 16 | 0.16 % |
| Complete Glove | 266004000 | 16 | 0.16 % |
| Priest Chitin Shell Pauldron | 286001000 | 15 | 0.15 % |
| Priest Chitin Shell Helmet | 286003000 | 15 | 0.15 % |
| Iron Impact | 146210000 | 12 | 0.12 % |
| Totamic Spear | 151210000 | 12 | 0.12 % |
| Silver bar | 379067000 | 12 | 0.12 % |
| Blessed Upgrade Scroll | 379021000 | 8 | 0.08 % |
| Sherion | 119301131 | 8 | 0.08 % |
| Avedon | 139301133 | 8 | 0.08 % |
| Helenid | 169301134 | 8 | 0.08 % |
| Hell Blood | 189601135 | 8 | 0.08 % |
| Elysium | 189603137 | 8 | 0.08 % |
| Garp | 189602136 | 8 | 0.08 % |
| Syphioric | 179301139 | 8 | 0.08 % |
| Lugias | 159301132 | 8 | 0.08 % |
| Holy Animor | 199301138 | 8 | 0.08 % |
| Foverin | 330940143 | 4 | 0.04 % |
| Shio Tears | 330920141 | 4 | 0.04 % |
| Imir Ring | 330930142 | 4 | 0.04 % |
| Flame Ring | 330910140 | 4 | 0.04 % |
| Dark Fragment of Envy | 810951000 | 2 | 0.02 % |
| Dark Fragment of Greed | 810952000 | 2 | 0.02 % |
| Dark Fragment of Gluttony | 810944000 | 2 | 0.02 % |
| Dark Fragment of Sloth | 810949000 | 2 | 0.02 % |
| Moonlight Ancient Metal Plate | 890104000 | 1 | 0.01 % |
| Magic Ancient Metal Plate | 890105000 | 1 | 0.01 % |
| Solar Ancient Metal Plate | 890106000 | 1 | 0.01 % |
| Chaotic Ancient Metal Plate | 890107000 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |
| Cold-Hearted Dagger | 119102294 | 1 | 0.01 % |
| Eagle's Eye | 169302345 | 1 | 0.01 % |
| Spear of Murky Waters | 159302311 | 1 | 0.01 % |
| Scorching Staff | 189302255 | 1 | 0.01 % |
| Oasis Staff | 189102256 | 1 | 0.01 % |
| Chaotic Staff | 189202257 | 1 | 0.01 % |
| Stone Splitter | 199102262 | 1 | 0.01 % |
| Dread Shield | 179102191 | 1 | 0.01 % |
| Dragon Scale Armor Top | 507001000 | 1 | 0.01 % |
| Dragon Scale Armor Pants | 507002000 | 1 | 0.01 % |
| Dragon Scale Helmet | 507003000 | 1 | 0.01 % |
| Dragon Scale Gauntlet | 507004000 | 1 | 0.01 % |
| Dragon Scale Boots | 507005000 | 1 | 0.01 % |
| Mythril Pauldron | 547001000 | 1 | 0.01 % |
| Mythril Pad | 547002000 | 1 | 0.01 % |
| Mythril Helmet | 547003000 | 1 | 0.01 % |
| Mythril Gauntlet | 547004000 | 1 | 0.01 % |
| Mythril Boots | 547005000 | 1 | 0.01 % |
| Pauldron of Trial | 587001000 | 1 | 0.01 % |
| Pads of Trial | 587002000 | 1 | 0.01 % |
| Helmet of Trial | 587003000 | 1 | 0.01 % |
| Gauntlet of Trial | 587004000 | 1 | 0.01 % |
| Boots of Trial | 587005000 | 1 | 0.01 % |
| Ron's Robe | 567001000 | 1 | 0.01 % |
| Ron's Pants | 567002000 | 1 | 0.01 % |
| Ron's Helmet | 567003000 | 1 | 0.01 % |
| Ron's Gloves | 567004000 | 1 | 0.01 % |
| Ron's Boots | 567005000 | 1 | 0.01 % |

### Dark Fragment of Envy [810951000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 60 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Nightmare Crystal | 389242000 | 194 | 1.94 % |
| Weapon Enchant Scroll | 389155000 | 61 | 0.61 % |
| Priest Chitin Shell Pauldron | 286001000 | 53 | 0.53 % |
| Complete Pants | 266002000 | 50 | 0.50 % |
| Complete Helmet | 266003000 | 50 | 0.50 % |
| Complete Glove | 266004000 | 50 | 0.50 % |
| Complete Boots | 266005000 | 50 | 0.50 % |
| Chitin Shell Pads | 206002000 | 48 | 0.48 % |
| Priest Chitin Shell Helmet | 286003000 | 48 | 0.48 % |
| Priest Chitin Shell Boots | 286005000 | 48 | 0.48 % |
| Chitin Shell Gauntlets | 206004000 | 43 | 0.43 % |
| Impact | 141110000 | 43 | 0.43 % |
| Raptor | 156210000 | 43 | 0.43 % |
| Iron Bow | 168410000 | 43 | 0.43 % |
| Elixir Staff | 181110000 | 43 | 0.43 % |
| Rogue Chitin Shell Pauldron | 246001000 | 35 | 0.35 % |
| Rogue Chitin Shell Helmet | 246003000 | 35 | 0.35 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 35 | 0.35 % |
| Mirage | 126410000 | 35 | 0.35 % |
| Graham | 121310000 | 35 | 0.35 % |
| Tempered Fusion Stone | 890108000 | 24 | 0.24 % |
| Moonlight Fusion Stone | 890109000 | 24 | 0.24 % |
| Magic Fusion Stone | 890110000 | 24 | 0.24 % |
| Solar Fusion Stone | 890111000 | 24 | 0.24 % |
| Chaotic Fusion Stone | 890112000 | 24 | 0.24 % |
| Priest Chitin Shell Pads | 286002000 | 14 | 0.14 % |
| Priest Chitin Shell Gauntlets | 286004000 | 14 | 0.14 % |
| Chitin Shell Pauldron | 206001000 | 13 | 0.13 % |
| Sherion | 119301131 | 13 | 0.13 % |
| Avedon | 139301133 | 13 | 0.13 % |
| Helenid | 169301134 | 13 | 0.13 % |
| Hell Blood | 189601135 | 13 | 0.13 % |
| Elysium | 189603137 | 13 | 0.13 % |
| Garp | 189602136 | 13 | 0.13 % |
| Syphioric | 179301139 | 13 | 0.13 % |
| Lugias | 159301132 | 13 | 0.13 % |
| Holy Animor | 199301138 | 13 | 0.13 % |
| Blade Axe | 136210000 | 8 | 0.08 % |
| Iron Impact | 146210000 | 8 | 0.08 % |
| Totamic Spear | 151210000 | 8 | 0.08 % |
| Priest Impact | 191110000 | 8 | 0.08 % |
| Silver bar | 379067000 | 8 | 0.08 % |
| Chitin Shell Helmet | 206003000 | 8 | 0.08 % |
| Chitin Shell Boots | 206005000 | 8 | 0.08 % |
| Blessed Upgrade Scroll | 379021000 | 8 | 0.08 % |
| Foverin | 330940143 | 6 | 0.06 % |
| Shio Tears | 330920141 | 6 | 0.06 % |
| Imir Ring | 330930142 | 6 | 0.06 % |
| Flame Ring | 330910140 | 6 | 0.06 % |
| Cold-Hearted Dagger | 119102294 | 3 | 0.03 % |
| Eagle's Eye | 169302345 | 3 | 0.03 % |
| Spear of Murky Waters | 159302311 | 3 | 0.03 % |
| Scorching Staff | 189302255 | 3 | 0.03 % |
| Oasis Staff | 189102256 | 3 | 0.03 % |
| Chaotic Staff | 189202257 | 3 | 0.03 % |
| Stone Splitter | 199102262 | 3 | 0.03 % |
| Dread Shield | 179102191 | 3 | 0.03 % |
| Dark Fragment of Sloth | 810949000 | 2 | 0.02 % |
| Dark Fragment of Lust | 810950000 | 2 | 0.02 % |
| Dark Fragment of Envy | 810951000 | 2 | 0.02 % |
| Dark Fragment of Gluttony | 810944000 | 2 | 0.02 % |
| Dark Fragment of Wrath | 810948000 | 2 | 0.02 % |
| Dark Fragment of Pride | 810943000 | 2 | 0.02 % |
| Dragon Scale Armor Top | 507001000 | 2 | 0.02 % |
| Dragon Scale Armor Pants | 507002000 | 2 | 0.02 % |
| Dragon Scale Helmet | 507003000 | 2 | 0.02 % |
| Dragon Scale Gauntlet | 507004000 | 2 | 0.02 % |
| Dragon Scale Boots | 507005000 | 2 | 0.02 % |
| Mythril Pauldron | 547001000 | 2 | 0.02 % |
| Mythril Pad | 547002000 | 2 | 0.02 % |
| Mythril Helmet | 547003000 | 2 | 0.02 % |
| Mythril Gauntlet | 547004000 | 2 | 0.02 % |
| Mythril Boots | 547005000 | 2 | 0.02 % |
| Pauldron of Trial | 587001000 | 2 | 0.02 % |
| Pads of Trial | 587002000 | 2 | 0.02 % |
| Helmet of Trial | 587003000 | 2 | 0.02 % |
| Gauntlet of Trial | 587004000 | 2 | 0.02 % |
| Boots of Trial | 587005000 | 2 | 0.02 % |
| Ron's Robe | 567001000 | 2 | 0.02 % |
| Ron's Pants | 567002000 | 2 | 0.02 % |
| Ron's Helmet | 567003000 | 2 | 0.02 % |
| Ron's Gloves | 567004000 | 2 | 0.02 % |
| Ron's Boots | 567005000 | 2 | 0.02 % |
| Dark Fragment of Greed | 810952000 | 1 | 0.01 % |
| Tempered Ancient Metal Plate | 890103000 | 1 | 0.01 % |
| Moonlight Ancient Metal Plate | 890104000 | 1 | 0.01 % |
| Magic Ancient Metal Plate | 890105000 | 1 | 0.01 % |
| Solar Ancient Metal Plate | 890106000 | 1 | 0.01 % |
| Chaotic Ancient Metal Plate | 890107000 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |

### Dark Fragment of Greed [810952000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 62 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Nightmare Crystal | 389242000 | 164 | 1.64 % |
| Rogue Chitin Shell Pauldron | 246001000 | 45 | 0.45 % |
| Priest Chitin Shell Pads | 286002000 | 45 | 0.45 % |
| Complete Robe | 266001000 | 45 | 0.45 % |
| Complete Pants | 266002000 | 45 | 0.45 % |
| Armor Enchant Scroll | 389156000 | 44 | 0.44 % |
| Weapon Enchant Scroll | 389155000 | 44 | 0.44 % |
| Chitin Shell Pads | 206002000 | 39 | 0.39 % |
| Priest Impact | 191110000 | 33 | 0.33 % |
| Mirage | 126410000 | 33 | 0.33 % |
| Deep Scar | 131110000 | 33 | 0.33 % |
| Blade Axe | 136210000 | 33 | 0.33 % |
| Iron Impact | 146210000 | 33 | 0.33 % |
| Totamic Spear | 151210000 | 33 | 0.33 % |
| Raptor | 156210000 | 33 | 0.33 % |
| Iron Bow | 168410000 | 33 | 0.33 % |
| Chitin Shell Gauntlets | 206004000 | 25 | 0.25 % |
| Rogue Chitin Shell Helmet | 246003000 | 25 | 0.25 % |
| Rogue Chitin Shell Boots | 246005000 | 25 | 0.25 % |
| Durandal | 126310000 | 25 | 0.25 % |
| Graham | 121310000 | 25 | 0.25 % |
| Priest Chitin Shell Helmet | 286003000 | 25 | 0.25 % |
| Priest Chitin Shell Gauntlets | 286004000 | 25 | 0.25 % |
| Priest Chitin Shell Boots | 286005000 | 25 | 0.25 % |
| Complete Helmet | 266003000 | 25 | 0.25 % |
| Complete Glove | 266004000 | 25 | 0.25 % |
| Complete Boots | 266005000 | 25 | 0.25 % |
| Shard | 111210000 | 25 | 0.25 % |
| Rogue Chitin Shell Pads | 246002000 | 20 | 0.20 % |
| Priest Chitin Shell Pauldron | 286001000 | 20 | 0.20 % |
| Tempered Fusion Stone | 890108000 | 19 | 0.19 % |
| Moonlight Fusion Stone | 890109000 | 19 | 0.19 % |
| Magic Fusion Stone | 890110000 | 19 | 0.19 % |
| Solar Fusion Stone | 890111000 | 19 | 0.19 % |
| Chaotic Fusion Stone | 890112000 | 19 | 0.19 % |
| Recipe: Holy Knight Warrior Armor | 810961000 | 13 | 0.13 % |
| Recipe: Holy Knight Assassin Armor | 810962000 | 13 | 0.13 % |
| Recipe: Holy Knight Mage Armor | 810963000 | 13 | 0.13 % |
| Recipe: Holy Knight Priest Armor | 810964000 | 13 | 0.13 % |
| Recipe: Holy Knight Portu Armor | 810965000 | 13 | 0.13 % |
| Sherion | 119301131 | 13 | 0.13 % |
| Avedon | 139301133 | 13 | 0.13 % |
| Helenid | 169301134 | 13 | 0.13 % |
| Hell Blood | 189601135 | 13 | 0.13 % |
| Elysium | 189603137 | 13 | 0.13 % |
| Garp | 189602136 | 13 | 0.13 % |
| Syphioric | 179301139 | 13 | 0.13 % |
| Lugias | 159301132 | 13 | 0.13 % |
| Holy Animor | 199301138 | 13 | 0.13 % |
| Impact | 141110000 | 7 | 0.07 % |
| Elixir Staff | 181110000 | 7 | 0.07 % |
| Foverin | 330940143 | 7 | 0.07 % |
| Shio Tears | 330920141 | 7 | 0.07 % |
| Imir Ring | 330930142 | 7 | 0.07 % |
| Flame Ring | 330910140 | 7 | 0.07 % |
| Blessed Upgrade Scroll | 379021000 | 6 | 0.06 % |
| Cold-Hearted Dagger | 119102294 | 4 | 0.04 % |
| Eagle's Eye | 169302345 | 4 | 0.04 % |
| Spear of Murky Waters | 159302311 | 4 | 0.04 % |
| Scorching Staff | 189302255 | 4 | 0.04 % |
| Oasis Staff | 189102256 | 4 | 0.04 % |
| Chaotic Staff | 189202257 | 4 | 0.04 % |
| Stone Splitter | 199102262 | 4 | 0.04 % |
| Dread Shield | 179102191 | 4 | 0.04 % |
| Dark Fragment of Pride | 810943000 | 2 | 0.02 % |
| Dark Fragment of Gluttony | 810944000 | 2 | 0.02 % |
| Dark Fragment of Wrath | 810948000 | 2 | 0.02 % |
| Dark Fragment of Sloth | 810949000 | 2 | 0.02 % |
| Dark Fragment of Lust | 810950000 | 2 | 0.02 % |
| Dark Fragment of Envy | 810951000 | 2 | 0.02 % |
| Dragon Scale Armor Top | 507001000 | 2 | 0.02 % |
| Dragon Scale Armor Pants | 507002000 | 2 | 0.02 % |
| Dragon Scale Helmet | 507003000 | 2 | 0.02 % |
| Dragon Scale Gauntlet | 507004000 | 2 | 0.02 % |
| Dragon Scale Boots | 507005000 | 2 | 0.02 % |
| Mythril Pauldron | 547001000 | 2 | 0.02 % |
| Mythril Pad | 547002000 | 2 | 0.02 % |
| Mythril Helmet | 547003000 | 2 | 0.02 % |
| Mythril Gauntlet | 547004000 | 2 | 0.02 % |
| Mythril Boots | 547005000 | 2 | 0.02 % |
| Pauldron of Trial | 587001000 | 2 | 0.02 % |
| Pads of Trial | 587002000 | 2 | 0.02 % |
| Helmet of Trial | 587003000 | 2 | 0.02 % |
| Gauntlet of Trial | 587004000 | 2 | 0.02 % |
| Boots of Trial | 587005000 | 2 | 0.02 % |
| Ron's Robe | 567001000 | 2 | 0.02 % |
| Ron's Pants | 567002000 | 2 | 0.02 % |
| Ron's Helmet | 567003000 | 2 | 0.02 % |
| Ron's Gloves | 567004000 | 2 | 0.02 % |
| Ron's Boots | 567005000 | 2 | 0.02 % |
| Tempered Ancient Metal Plate | 890103000 | 1 | 0.01 % |
| Moonlight Ancient Metal Plate | 890104000 | 1 | 0.01 % |
| Magic Ancient Metal Plate | 890105000 | 1 | 0.01 % |
| Solar Ancient Metal Plate | 890106000 | 1 | 0.01 % |
| Chaotic Ancient Metal Plate | 890107000 | 1 | 0.01 % |
| Dark Fragment of Greed | 810952000 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Minotaur's Earrings | 310511119 | 1 | 0.01 % |
| Cockatrice's Earrings | 310512120 | 1 | 0.01 % |
| Lillime's Earrings | 310514122 | 1 | 0.01 % |
| Lich King's Earrings | 310513121 | 1 | 0.01 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |

### Akara Accessory Box [811053000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Akara's Agate Earrings | 1310410109 | 45 | 0.45 % |
| Akara's Agility Necklace | 1320510810 | 45 | 0.45 % |
| Akara's Black Dragon Necklace | 1320310800 | 45 | 0.45 % |
| Akara's Cockatrice Earrings | 1310512120 | 45 | 0.45 % |
| Akara's Crystal Belt | 1340610106 | 44 | 0.44 % |
| Akara's Crystal Earrings | 1310510104 | 44 | 0.44 % |
| Akara's Elemental Pendant | 1320110780 | 44 | 0.44 % |
| Akara's Elf Belt | 1340410113 | 44 | 0.44 % |
| Akara's Elf Metal Earrings | 1310610106 | 44 | 0.44 % |
| Akara's Flame Ring | 1399310150 | 44 | 0.44 % |
| Akara's Hero's Valor | 1399306110 | 44 | 0.44 % |
| Akara's Imir Ring | 1399312170 | 44 | 0.44 % |
| Akara's Iron Belt | 1340610107 | 44 | 0.44 % |
| Akara's Iron Necklace | 1320310790 | 44 | 0.44 % |
| Akara's Lich King's Earrings | 1310513121 | 44 | 0.44 % |
| Akara's Lillime Earrings | 1310514122 | 44 | 0.44 % |
| Akara's Lunar War Belt | 1340710108 | 44 | 0.44 % |
| Akara's Mage Earrings | 1310310008 | 44 | 0.44 % |
| Akara's Minotaur Earrings | 1310511119 | 44 | 0.44 % |
| Akara's Opal Earrings | 1310510110 | 44 | 0.44 % |
| Akara's Platinum Earrings | 1310410105 | 44 | 0.44 % |
| Akara's Priest Earrings | 1310310007 | 44 | 0.44 % |
| Akara's Ring of Courage | 1399308130 | 44 | 0.44 % |
| Akara's Ring of Life | 1399309140 | 44 | 0.44 % |
| Akara's Rogue Earrings | 1310310006 | 44 | 0.44 % |
| Akara's Secret Silver Earrings | 1310410107 | 44 | 0.44 % |
| Akara's Shio Tears | 1399311160 | 44 | 0.44 % |
| Akara's Silver Earrings | 1310610102 | 44 | 0.44 % |
| Akara's Skeleton Belt | 1340410115 | 44 | 0.44 % |
| Akara's Strength Necklace | 1320610820 | 44 | 0.44 % |
| Akara's String of Skulls | 1340410123 | 44 | 0.44 % |
| Akara's Warrior Earrings | 1310310005 | 44 | 0.44 % |
| Akara's White Silver Earrings | 1310610108 | 44 | 0.44 % |
| Akaras's Legionnaire Band | 1399307120 | 44 | 0.44 % |

### Fortified Sterling Silver Gemstone [811137000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

Upgraded rows: 66 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Chitin Shell Pauldron | 206001000 | 71 | 0.71 % |
| Chitin Shell Pads | 206002000 | 70 | 0.70 % |
| Rogue Chitin Shell Pauldron | 246001000 | 70 | 0.70 % |
| Rogue Chitin Shell Pads | 246002000 | 70 | 0.70 % |
| Raptor | 156210000 | 68 | 0.68 % |
| Shard | 111210000 | 65 | 0.65 % |
| Blessed Upgrade Scroll | 379021000 | 58 | 0.58 % |
| Complete Robe | 266001000 | 55 | 0.55 % |
| Complete Pants | 266002000 | 55 | 0.55 % |
| Priest Chitin Shell Pauldron | 286001000 | 55 | 0.55 % |
| Priest Chitin Shell Pads | 286002000 | 55 | 0.55 % |
| Graham | 121310000 | 48 | 0.48 % |
| Mirage | 126410000 | 47 | 0.47 % |
| Deep Scar | 131110000 | 47 | 0.47 % |
| Impact | 141110000 | 47 | 0.47 % |
| Iron Impact | 146210000 | 47 | 0.47 % |
| Totamic Spear | 151210000 | 47 | 0.47 % |
| Iron Bow | 168410000 | 47 | 0.47 % |
| Elixir Staff | 181110000 | 39 | 0.39 % |
| Priest Impact | 191110000 | 39 | 0.39 % |
| Blade Axe | 136210000 | 30 | 0.30 % |
| Chitin Shell Gauntlets | 206004000 | 28 | 0.28 % |
| Chitin Shell Boots | 206005000 | 28 | 0.28 % |
| Rogue Chitin Shell Helmet | 246003000 | 28 | 0.28 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 28 | 0.28 % |
| Rogue Chitin Shell Boots | 246005000 | 28 | 0.28 % |
| Chitin Shell Helmet | 206003000 | 28 | 0.28 % |
| Weapon Enchant Scroll | 389155000 | 25 | 0.25 % |
| Armor Enchant Scroll | 389156000 | 25 | 0.25 % |
| Mirage Dagger | 111110000 | 18 | 0.18 % |
| Priest Chitin Shell Helmet | 286003000 | 13 | 0.13 % |
| Priest Chitin Shell Gauntlets | 286004000 | 13 | 0.13 % |
| Priest Chitin Shell Boots | 286005000 | 13 | 0.13 % |
| Complete Glove | 266004000 | 13 | 0.13 % |
| Complete Boots | 266005000 | 13 | 0.13 % |
| Complete Helmet | 266003000 | 13 | 0.13 % |
| Dye | 508167782 | 7 | 0.07 % |
| Cold-Hearted Dagger | 119102294 | 3 | 0.03 % |
| Spear of Murky Waters | 159302311 | 3 | 0.03 % |
| Eagle's Eye | 169302345 | 3 | 0.03 % |
| Oasis Staff | 189102256 | 3 | 0.03 % |
| Scorching Staff | 189302255 | 3 | 0.03 % |
| Chaotic Staff | 189202257 | 3 | 0.03 % |
| Stone Splitter | 199102262 | 3 | 0.03 % |
| Dread Shield | 179102191 | 3 | 0.03 % |
| Lillime's Earrings | 310514122 | 3 | 0.03 % |
| Lich King's Earrings | 310513121 | 3 | 0.03 % |
| Silver bar | 379067000 | 3 | 0.03 % |
| Minotaur's Earrings | 310511119 | 3 | 0.03 % |
| Cockatrice's Earrings | 310512120 | 3 | 0.03 % |
| Judicious Belt | 340610122 | 1 | 0.01 % |
| String of Skulls | 340410123 | 1 | 0.01 % |
| Hero's Valor | 330111280 | 1 | 0.01 % |
| Legionnaire Band | 330151461 | 1 | 0.01 % |
| Light Belt of Life | 340110120 | 1 | 0.01 % |
| Sash of Sorrows | 340510121 | 1 | 0.01 % |
| Fortified Minotaur Earrings | 1310515301 | 1 | 0.01 % |
| Fortified Cockatrice Earrings | 1310516302 | 1 | 0.01 % |
| Fortified Lich King's Earrings | 1310517303 | 1 | 0.01 % |
| Fortified Lilime's Earrings | 1310518304 | 1 | 0.01 % |

### Black Lion Weapon Box [999915000] (existing)

Own source: existing r123 rows (+N removed). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Blessed Upgrade Scroll | 379021000 | 1350 | 13.50 % |
| [Black Lion] Axe | 1113367900 | 15 | 0.15 % |
| [Black Lion] Bow | 1113363900 | 15 | 0.15 % |
| [Black Lion] Dagger | 1113360900 | 15 | 0.15 % |
| [Black Lion] Flaming Staff | 2113364760 | 15 | 0.15 % |
| [Black Lion] Frost Staff | 2113364780 | 15 | 0.15 % |
| [Black Lion] Jamadhar | 1113361900 | 15 | 0.15 % |
| [Black Lion] Lightning Staff | 2113364800 | 15 | 0.15 % |
| [Black Lion] Mace | 1113365900 | 15 | 0.15 % |
| [Black Lion] Shield | 1113366900 | 15 | 0.15 % |
| [Black Lion] Spear | 1113362900 | 15 | 0.15 % |

### 70 Quest Box [507294000] (new)

Own source: 70 Quest Dark Fragment Chest pool (+N -> +0). Own 15 %, Red Gem 85 %.

Upgraded rows: 28 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Ron's Helmet | 567003000 | 65 | 0.65 % |
| Ron's Robe | 567001000 | 65 | 0.65 % |
| Ron's Pants | 567002000 | 65 | 0.65 % |
| Ron's Gloves | 567004000 | 65 | 0.65 % |
| Ron's Boots | 567005000 | 65 | 0.65 % |
| Dragon Scale Helmet | 507003000 | 55 | 0.55 % |
| Dragon Scale Armor Top | 507001000 | 55 | 0.55 % |
| Dragon Scale Armor Pants | 507002000 | 55 | 0.55 % |
| Dragon Scale Gauntlet | 507004000 | 55 | 0.55 % |
| Dragon Scale Boots | 507005000 | 55 | 0.55 % |
| Mythril Helmet | 547003000 | 55 | 0.55 % |
| Mythril Pauldron | 547001000 | 55 | 0.55 % |
| Mythril Pad | 547002000 | 55 | 0.55 % |
| Mythril Gauntlet | 547004000 | 55 | 0.55 % |
| Mythril Boots | 547005000 | 55 | 0.55 % |
| Helmet of Trial | 587003000 | 55 | 0.55 % |
| Pauldron of Trial | 587001000 | 55 | 0.55 % |
| Pads of Trial | 587002000 | 55 | 0.55 % |
| Gauntlet of Trial | 587004000 | 54 | 0.54 % |
| Boots of Trial | 587005000 | 54 | 0.54 % |
| Dagger of <selfname> | 910119300 | 44 | 0.44 % |
| Two-Handed Sword of <selfname> | 910120350 | 44 | 0.44 % |
| Spear of <selfname> | 910121300 | 44 | 0.44 % |
| Bow of <selfname> | 910122350 | 44 | 0.44 % |
| Staff of <selfname> | 910123000 | 44 | 0.44 % |
| Pole of <selfname> | 910124000 | 44 | 0.44 % |
| Mace of <selfname> | 910125000 | 44 | 0.44 % |
| Shield of <selfname> | 910126000 | 44 | 0.44 % |

### Trophy box [508164000] (new)

Own source: existing Trophy rows (flag 101, +N -> +0). Own 15 %, Red Gem 85 %.

Upgraded rows: 14 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Full Plate Armor Boots | 204005000 | 59 | 0.59 % |
| Rogue Plate Armor Pauldron | 243001000 | 59 | 0.59 % |
| Rogue Plate Armor Pads | 243002000 | 58 | 0.58 % |
| Rogue Plate Helmet | 243003000 | 58 | 0.58 % |
| Rogue Plate Gauntlet | 243004000 | 58 | 0.58 % |
| Rogue Plate Boots | 243005000 | 58 | 0.58 % |
| Rogue Full Plate Armor Pauldron | 244001000 | 58 | 0.58 % |
| Rogue Full Plate Armor Pads | 244002000 | 58 | 0.58 % |
| Rogue Full Plate Armor Helmet | 244003000 | 58 | 0.58 % |
| Rogue Full Plate Armor Gauntlet | 244004000 | 58 | 0.58 % |
| Rogue Full Plate Armor Boots | 244005000 | 58 | 0.58 % |
| Mage Silk Robe | 263001000 | 58 | 0.58 % |
| Mage Silk Pants | 263002000 | 58 | 0.58 % |
| Mage Helmet | 263003000 | 58 | 0.58 % |
| Old Warrior's Earring | 341230813 | 35 | 0.35 % |
| Old Shio Tears | 343840831 | 23 | 0.23 % |
| Old Mage Earring | 340940812 | 23 | 0.23 % |
| Old Imir Ring | 344130832 | 23 | 0.23 % |
| Old Rogue's Earring | 341520814 | 23 | 0.23 % |
| Old Priest's Earring | 341810815 | 23 | 0.23 % |
| Old Elf Metal Earring | 310650796 | 23 | 0.23 % |
| Old Warrior Pendant | 342100806 | 23 | 0.23 % |
| Old Elemental Pendant | 342390807 | 23 | 0.23 % |
| Old Elf Belt | 344710807 | 23 | 0.23 % |
| Fragment of Avarice | 389166000 | 12 | 0.12 % |
| Old Bronze Earring | 310130791 | 12 | 0.12 % |
| Old Elemental Necklace | 320340791 | 12 | 0.12 % |
| Old Intelligence Necklace | 320370801 | 12 | 0.12 % |
| Old Crystal Ring | 330140811 | 12 | 0.12 % |
| Old Belt of Life | 340120791 | 12 | 0.12 % |
| Old Belt of Curse | 340140801 | 12 | 0.12 % |
| Old Silver Earring | 310620792 | 12 | 0.12 % |
| Old Magic Power Necklace | 320380802 | 12 | 0.12 % |
| Old Goddess Necklace | 320520792 | 12 | 0.12 % |
| Old Mano Ring | 330630812 | 12 | 0.12 % |
| Old Bronze Belt | 340360802 | 12 | 0.12 % |
| Old Belt of Strength | 340520792 | 12 | 0.12 % |
| Old Gold Earring | 310120793 | 12 | 0.12 % |
| Old Green Dragon Necklace | 320530793 | 12 | 0.12 % |
| Old Curse Necklace | 320570803 | 12 | 0.12 % |
| Old Ruby Ring | 330640813 | 12 | 0.12 % |
| Old Belt of Intelligence | 340530793 | 12 | 0.12 % |
| Old Belt of Harpy | 340650803 | 12 | 0.12 % |
| Old Crystal Earring | 310520794 | 12 | 0.12 % |
| Old White Dragon Necklace | 320540794 | 12 | 0.12 % |
| Old Crystal Necklace | 320640804 | 12 | 0.12 % |
| Old Gold Ring | 330660814 | 12 | 0.12 % |
| Old Crystal Belt | 340630794 | 12 | 0.12 % |
| Old Platinum Earring | 310440795 | 12 | 0.12 % |
| Old Blue Dragon Necklace | 320620795 | 12 | 0.12 % |
| Old Silver Ring | 330670815 | 12 | 0.12 % |
| Old Belt of Dexterity | 340640795 | 12 | 0.12 % |
| Old Red Dragon Necklace | 320350796 | 12 | 0.12 % |
| Old Diamond Ring | 330450816 | 12 | 0.12 % |
| Old Mana Belt | 340340796 | 12 | 0.12 % |
| Old Secret Silver Earring | 310450797 | 12 | 0.12 % |
| Old Black Dragon Necklace | 320360797 | 12 | 0.12 % |
| Old Elf Ring | 330650807 | 12 | 0.12 % |
| Old Fire Belt | 340350797 | 12 | 0.12 % |
| Old White Silver Earring | 310660798 | 12 | 0.12 % |
| Old Agility Necklace | 320550798 | 12 | 0.12 % |

### Holy knight's legacy box [508195000] (new)

Own source: Holy Knight armour sets (all classes). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Holy Knight Warrior Pauldron | 314511000 | 60 | 0.60 % |
| Holy Knight Warrior Pads | 314512000 | 60 | 0.60 % |
| Holy Knight Warrior Helmet | 314513000 | 60 | 0.60 % |
| Holy Knight Warrior Gauntlets | 314514000 | 60 | 0.60 % |
| Holy Knight Warrior Boots | 314515000 | 60 | 0.60 % |
| Holy Knight Assassin Pauldron | 314521000 | 60 | 0.60 % |
| Holy Knight Assassin Pads | 314522000 | 60 | 0.60 % |
| Holy Knight Assassin Helmet | 314523000 | 60 | 0.60 % |
| Holy Knight Assassin Gauntlets | 314524000 | 60 | 0.60 % |
| Holy Knight Assassin Boots | 314525000 | 60 | 0.60 % |
| Holy Knight Mage Robe | 314531000 | 60 | 0.60 % |
| Holy Knight Mage Pants | 314532000 | 60 | 0.60 % |
| Holy Knight Mage Helmet | 314533000 | 60 | 0.60 % |
| Holy Knight Mage Gloves | 314534000 | 60 | 0.60 % |
| Holy Knight Mage Boots | 314535000 | 60 | 0.60 % |
| Holy Knight Priest Pauldron | 314541000 | 60 | 0.60 % |
| Holy Knight Priest Pads | 314542000 | 60 | 0.60 % |
| Holy Knight Priest Helmet | 314543000 | 60 | 0.60 % |
| Holy Knight Priest Gauntlets | 314544000 | 60 | 0.60 % |
| Holy Knight Priest Boots | 314545000 | 60 | 0.60 % |
| Holy Knight Portu Pauldron | 314551000 | 60 | 0.60 % |
| Holy Knight Portu Pads | 314552000 | 60 | 0.60 % |
| Holy Knight Portu Helmet | 314553000 | 60 | 0.60 % |
| Holy Knight Portu Gauntlets | 314554000 | 60 | 0.60 % |
| Holy Knight Portu Boots | 314555000 | 60 | 0.60 % |

### Wizard's legacy box [508196000] (new)

Own source: Holy Knight Mage set, Wizard Staff, Wizard Mask. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Holy Knight Mage Robe | 314531000 | 250 | 2.50 % |
| Holy Knight Mage Pants | 314532000 | 250 | 2.50 % |
| Holy Knight Mage Helmet | 314533000 | 250 | 2.50 % |
| Holy Knight Mage Gloves | 314534000 | 250 | 2.50 % |
| Holy Knight Mage Boots | 314535000 | 250 | 2.50 % |
| Wizard Staff | 180310000 | 125 | 1.25 % |
| Wizard Mask | 508113000 | 125 | 1.25 % |

### Castellan wife's legacy box [508197000] (new)

Own source: Castellan's teaching/dignity, Mage set, blessed scrolls. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Holy Knight Mage Robe | 314531000 | 207 | 2.07 % |
| Holy Knight Mage Pants | 314532000 | 207 | 2.07 % |
| Holy Knight Mage Helmet | 314533000 | 207 | 2.07 % |
| Holy Knight Mage Gloves | 314534000 | 207 | 2.07 % |
| Holy Knight Mage Boots | 314535000 | 207 | 2.07 % |
| Castellan's teaching | 191335000 | 103 | 1.03 % |
| Dignity of Castellan | 191336000 | 103 | 1.03 % |
| Blessed Upgrade Scroll | 379021000 | 52 | 0.52 % |
| Blessed Enchant Scroll(STR) | 379022000 | 52 | 0.52 % |
| Blessed Immune Scroll | 379023000 | 52 | 0.52 % |
| Blessed Reduce Scroll | 379024000 | 52 | 0.52 % |
| Blessed Elemental Scroll | 379025000 | 51 | 0.51 % |

### Castellan's legacy box [508198000] (new)

Own source: Castellan weapons, Warrior/Assassin sets, blessed scrolls. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Holy Knight Warrior Pauldron | 314511000 | 113 | 1.13 % |
| Holy Knight Warrior Pads | 314512000 | 113 | 1.13 % |
| Holy Knight Warrior Helmet | 314513000 | 113 | 1.13 % |
| Holy Knight Warrior Gauntlets | 314514000 | 113 | 1.13 % |
| Holy Knight Warrior Boots | 314515000 | 113 | 1.13 % |
| Holy Knight Assassin Pauldron | 314521000 | 113 | 1.13 % |
| Holy Knight Assassin Pads | 314522000 | 113 | 1.13 % |
| Holy Knight Assassin Helmet | 314523000 | 113 | 1.13 % |
| Holy Knight Assassin Gauntlets | 314524000 | 113 | 1.13 % |
| Holy Knight Assassin Boots | 314525000 | 113 | 1.13 % |
| Smart castellan's sword | 191329000 | 57 | 0.57 % |
| Stern castellan's sword | 191330000 | 57 | 0.57 % |
| Castellan's teaching | 191335000 | 57 | 0.57 % |
| Dignity of Castellan | 191336000 | 57 | 0.57 % |
| Blessed Upgrade Scroll | 379021000 | 29 | 0.29 % |
| Blessed Enchant Scroll(STR) | 379022000 | 29 | 0.29 % |
| Blessed Immune Scroll | 379023000 | 28 | 0.28 % |
| Blessed Reduce Scroll | 379024000 | 28 | 0.28 % |
| Blessed Elemental Scroll | 379025000 | 28 | 0.28 % |

### [Gold Dragon Egg] Shenlong Weapon Box [810209000] (new)

Own source: Shenlong Bow + Dragon Knight/Phoenix weapons & Legend Knight armour (Legend Dragon pool, +N -> +0). Own 15 %, Red Gem 85 %.

Upgraded rows: 35 replaced by their +0 twin, removed (no +0 twin): Dragon Knight Two-Handed Axe (+7) [1190003507].

| Own reward | id | slots | % |
|---|---|---|---|
| Phoenix Dagger | 1111484000 | 56 | 0.56 % |
| Phoenix Short Sword | 1111485000 | 56 | 0.56 % |
| Phoenix Axe | 1111486453 | 56 | 0.56 % |
| Phoenix Bow | 1111487000 | 56 | 0.56 % |
| Fire Phoenix Staff | 1111488774 | 56 | 0.56 % |
| Ice Phoenix Staff | 1111488775 | 56 | 0.56 % |
| Lightning Phoenix Staff | 1111488776 | 56 | 0.56 % |
| Phoenix Mace | 1111489000 | 56 | 0.56 % |
| Phoenix Shield | 1111490000 | 56 | 0.56 % |
| Legend Knight Warrior Helmet | 331003000 | 38 | 0.38 % |
| Legend Knight Warrior Pauldron | 331001000 | 38 | 0.38 % |
| Legend Knight Warrior Pads | 331002000 | 38 | 0.38 % |
| Legend Knight Warrior Gauntlets | 331004000 | 38 | 0.38 % |
| Legend Knight Warrior Boots | 331005000 | 38 | 0.38 % |
| Legend Knight Assassin Helmet | 332003000 | 38 | 0.38 % |
| Legend Knight Assassin Pauldron | 332001000 | 38 | 0.38 % |
| Legend Knight Assassin Pads | 332002000 | 38 | 0.38 % |
| Legend Knight Assassin Gauntlets | 332004000 | 38 | 0.38 % |
| Legend Knight Assassin Boots | 332005000 | 38 | 0.38 % |
| Legend Knight Mage Helmet | 333003000 | 38 | 0.38 % |
| Legend Knight Mage Robe | 333001000 | 38 | 0.38 % |
| Legend Knight Mage Pants | 333002000 | 38 | 0.38 % |
| Legend Knight Mage Gloves | 333004000 | 38 | 0.38 % |
| Legend Knight Mage Boots | 333005000 | 38 | 0.38 % |
| Legend Knight Priest Helmet | 334003000 | 38 | 0.38 % |
| Legend Knight Priest Pauldron | 334001000 | 38 | 0.38 % |
| Legend Knight Priest Pads | 334002000 | 38 | 0.38 % |
| Legend Knight Priest Gauntlets | 334004000 | 38 | 0.38 % |
| Legend Knight Priest Boots | 334005000 | 38 | 0.38 % |
| Dragon Knight Jamadar | 1190001000 | 37 | 0.37 % |
| Dragon Knight One-Handed Axe | 1190009000 | 37 | 0.37 % |
| Dragon Knight Bow | 1190004000 | 37 | 0.37 % |
| Dragon Knight Staff | 1190005000 | 37 | 0.37 % |
| Dragon Knight Hammer | 1190006000 | 37 | 0.37 % |
| Dragon Knight Shield | 1190007000 | 37 | 0.37 % |
| Shenlong Bow | 1110444000 | 14 | 0.14 % |

### Accessory Box [810228000] (new)

Own source: existing Old accessory rows (flag 101). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Old Gold Earring | 310120793 | 77 | 0.77 % |
| Old Green Dragon Necklace | 320530793 | 77 | 0.77 % |
| Old Strength Necklace | 320630800 | 77 | 0.77 % |
| Old Belt of Strength | 340520792 | 77 | 0.77 % |
| Old Lightning Belt | 340540799 | 77 | 0.77 % |
| Old Red Dragon Necklace | 320350796 | 77 | 0.77 % |
| Old Rogue's Earring | 341520814 | 77 | 0.77 % |
| Old Mana Belt | 340340796 | 77 | 0.77 % |
| Old Mano Earring | 310460799 | 77 | 0.77 % |
| Old Magic Power Necklace | 320380802 | 77 | 0.77 % |
| Old Mage Earring | 340940812 | 76 | 0.76 % |
| Old Agility Necklace | 320550798 | 76 | 0.76 % |
| Old Skeleton Belt | 345000808 | 66 | 0.66 % |
| Old Priest's Earring | 341810815 | 66 | 0.66 % |
| Old Belt of Harpy | 340650803 | 66 | 0.66 % |
| Old White Dragon Necklace | 320540794 | 66 | 0.66 % |
| Old Belt of Dexterity | 340640795 | 51 | 0.51 % |
| Old Flame Ring | 343550830 | 51 | 0.51 % |
| Old Bronze Belt | 340360802 | 51 | 0.51 % |
| Old Bronze Earring | 310130791 | 51 | 0.51 % |
| Old Black Dragon Necklace | 320360797 | 51 | 0.51 % |
| Old Blue Dragon Necklace | 320620795 | 51 | 0.51 % |
| Belt of Dexterity | 340610111 | 8 | 0.08 % |

### bifrost box [810229000] (new)

Own source: Fragments of the seven sins. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Fragment of Arrogance | 389160000 | 215 | 2.15 % |
| Fragment of Gluttony | 389161000 | 215 | 2.15 % |
| Fragment of Rage | 389162000 | 214 | 2.14 % |
| Fragment of Sloth | 389163000 | 214 | 2.14 % |
| Fragment of Lechery | 389164000 | 214 | 2.14 % |
| Fragment of Jealousy | 389165000 | 214 | 2.14 % |
| Fragment of Avarice | 389166000 | 214 | 2.14 % |

### Juriad Jewel Box [810235000] (new)

Own source: Gems + Draki jewels. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Silvery Gem | 389196000 | 158 | 1.58 % |
| Red Gem | 389197000 | 158 | 1.58 % |
| Yellow Gem | 389198000 | 158 | 1.58 % |
| Blue Gem | 389199000 | 158 | 1.58 % |
| Green Gem | 389201000 | 158 | 1.58 % |
| Black Gem | 389205000 | 157 | 1.57 % |
| Jewel of Strength | 810597000 | 79 | 0.79 % |
| Jewel of Dexterity | 810598000 | 79 | 0.79 % |
| Jewel of Intelligence | 810599000 | 79 | 0.79 % |
| Jewel of Magic | 810600000 | 79 | 0.79 % |
| Jewel of Philosopher | 810610000 | 79 | 0.79 % |
| Jewel of Hardness | 810611000 | 79 | 0.79 % |
| Jewel of Absortion | 810612000 | 79 | 0.79 % |

### Wing Weapon Box [810284000] (new)

Own source: Star + Garges weapons. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Star Dagger | 1110440000 | 108 | 1.08 % |
| Star Spear | 1110450000 | 108 | 1.08 % |
| Star Long Spear | 1110460000 | 107 | 1.07 % |
| Star Bow | 1110470000 | 107 | 1.07 % |
| Star Staff | 1110480000 | 107 | 1.07 % |
| Star Mace | 1110490000 | 107 | 1.07 % |
| Star Shield | 1110500000 | 107 | 1.07 % |
| Garges Dagger | 1110581000 | 107 | 1.07 % |
| Garges Two-handed Sword | 1110582000 | 107 | 1.07 % |
| Garges Scythe | 1110583000 | 107 | 1.07 % |
| Garges Bow | 1110584000 | 107 | 1.07 % |
| Garges Staff | 1110585000 | 107 | 1.07 % |
| Garges Mace | 1110586000 | 107 | 1.07 % |
| Garges Shield | 1110587000 | 107 | 1.07 % |

### Chaos Wing Weapon Box [810329000] (new)

Own source: Chaos + Dark Knight + Bloody weapons. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Chaos Wirinom | 127430000 | 150 | 1.50 % |
| Chaos Baal | 139430000 | 150 | 1.50 % |
| Chaos Nebiros | 159430000 | 150 | 1.50 % |
| Dark Knight Dagger | 1111466000 | 75 | 0.75 % |
| Dark Knight Jamadar | 1111467000 | 75 | 0.75 % |
| Dark Knight Spear | 1111468000 | 75 | 0.75 % |
| Dark Knight Crossbow | 1111469000 | 75 | 0.75 % |
| Dark Knight Staff | 1111470000 | 75 | 0.75 % |
| Dark Knight Mace | 1111471000 | 75 | 0.75 % |
| Dark Knight Shield | 1111472000 | 75 | 0.75 % |
| Bloody Dagger | 1111452000 | 75 | 0.75 % |
| Bloody Sword | 1111453000 | 75 | 0.75 % |
| Bloody Two-handed Sword | 1111454000 | 75 | 0.75 % |
| Bloody Bow | 1111455000 | 75 | 0.75 % |
| Bloody Staff | 1111456000 | 75 | 0.75 % |
| Bloody Mace | 1111457000 | 75 | 0.75 % |
| Bloody Shield | 1111458000 | 75 | 0.75 % |

### High Class Accessory Box [810342000] (new)

Own source: existing High Class accessory rows (flag 101). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Belt of Harpy | 340610116 | 607 | 6.07 % |
| Iron Necklace | 320310126 | 447 | 4.47 % |
| Iron Belt | 340610107 | 446 | 4.46 % |

### Accessory Box [810347000] (new)

Own source: existing Old accessory rows (flag 101). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Old Gold Earring | 310120793 | 77 | 0.77 % |
| Old Green Dragon Necklace | 320530793 | 77 | 0.77 % |
| Old Strength Necklace | 320630800 | 77 | 0.77 % |
| Old Belt of Strength | 340520792 | 77 | 0.77 % |
| Old Lightning Belt | 340540799 | 77 | 0.77 % |
| Old Red Dragon Necklace | 320350796 | 77 | 0.77 % |
| Old Rogue's Earring | 341520814 | 77 | 0.77 % |
| Old Mana Belt | 340340796 | 77 | 0.77 % |
| Old Mano Earring | 310460799 | 77 | 0.77 % |
| Old Magic Power Necklace | 320380802 | 77 | 0.77 % |
| Old Mage Earring | 340940812 | 76 | 0.76 % |
| Old Agility Necklace | 320550798 | 76 | 0.76 % |
| Old Skeleton Belt | 345000808 | 66 | 0.66 % |
| Old Priest's Earring | 341810815 | 66 | 0.66 % |
| Old Belt of Harpy | 340650803 | 66 | 0.66 % |
| Old White Dragon Necklace | 320540794 | 66 | 0.66 % |
| Old Belt of Dexterity | 340640795 | 51 | 0.51 % |
| Old Flame Ring | 343550830 | 51 | 0.51 % |
| Old Bronze Belt | 340360802 | 51 | 0.51 % |
| Old Bronze Earring | 310130791 | 51 | 0.51 % |
| Old Black Dragon Necklace | 320360797 | 51 | 0.51 % |
| Old Blue Dragon Necklace | 320620795 | 51 | 0.51 % |
| Belt of Dexterity | 340610111 | 8 | 0.08 % |

### Bifrost Box [810348000] (new)

Own source: Fragments of the seven sins. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Fragment of Arrogance | 389160000 | 215 | 2.15 % |
| Fragment of Gluttony | 389161000 | 215 | 2.15 % |
| Fragment of Rage | 389162000 | 214 | 2.14 % |
| Fragment of Sloth | 389163000 | 214 | 2.14 % |
| Fragment of Lechery | 389164000 | 214 | 2.14 % |
| Fragment of Jealousy | 389165000 | 214 | 2.14 % |
| Fragment of Avarice | 389166000 | 214 | 2.14 % |

### Juriad Jewel Box [810349000] (new)

Own source: Gems + Draki jewels. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Silvery Gem | 389196000 | 158 | 1.58 % |
| Red Gem | 389197000 | 158 | 1.58 % |
| Yellow Gem | 389198000 | 158 | 1.58 % |
| Blue Gem | 389199000 | 158 | 1.58 % |
| Green Gem | 389201000 | 158 | 1.58 % |
| Black Gem | 389205000 | 157 | 1.57 % |
| Jewel of Strength | 810597000 | 79 | 0.79 % |
| Jewel of Dexterity | 810598000 | 79 | 0.79 % |
| Jewel of Intelligence | 810599000 | 79 | 0.79 % |
| Jewel of Magic | 810600000 | 79 | 0.79 % |
| Jewel of Philosopher | 810610000 | 79 | 0.79 % |
| Jewel of Hardness | 810611000 | 79 | 0.79 % |
| Jewel of Absortion | 810612000 | 79 | 0.79 % |

### Dragon Box [810377000] (new)

Own source: same as Dragon Box [810341000] (Dragon Scale armour). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Dragon Scale Armor Pants | 507002000 | 75 | 0.75 % |
| Dragon Scale Armor Pants | 922002000 | 75 | 0.75 % |
| Dragon Scale Armor Pants | 917002000 | 75 | 0.75 % |
| Dragon Scale Armor Pants | 517002000 | 75 | 0.75 % |
| Dragon Scale Armor Top | 922001000 | 75 | 0.75 % |
| Dragon Scale Armor Top | 517001000 | 75 | 0.75 % |
| Dragon Scale Armor Top | 507001000 | 75 | 0.75 % |
| Dragon Scale Armor Top | 917001000 | 75 | 0.75 % |
| Dragon Scale Boots | 507005000 | 75 | 0.75 % |
| Dragon Scale Boots | 917005000 | 75 | 0.75 % |
| Dragon Scale Boots | 517005000 | 75 | 0.75 % |
| Dragon Scale Boots | 922005000 | 75 | 0.75 % |
| Dragon Scale Gauntlet | 922004000 | 75 | 0.75 % |
| Dragon Scale Gauntlet | 517004000 | 75 | 0.75 % |
| Dragon Scale Gauntlet | 917004000 | 75 | 0.75 % |
| Dragon Scale Gauntlet | 507004000 | 75 | 0.75 % |
| Dragon Scale Helmet | 507003000 | 75 | 0.75 % |
| Dragon Scale Helmet | 922003000 | 75 | 0.75 % |
| Dragon Scale Helmet | 917003000 | 75 | 0.75 % |
| Dragon Scale Helmet | 517003000 | 75 | 0.75 % |

### [Special Monster Staff] Accessory Box [810536000] (new)

Own source: existing Old accessory rows (flag 101). Own 15 %, Red Gem 85 %.

Upgraded rows: 1 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Old Gold Earring | 310120793 | 77 | 0.77 % |
| Old Green Dragon Necklace | 320530793 | 77 | 0.77 % |
| Old Strength Necklace | 320630800 | 77 | 0.77 % |
| Old Belt of Strength | 340520792 | 77 | 0.77 % |
| Old Lightning Belt | 340540799 | 77 | 0.77 % |
| Old Red Dragon Necklace | 320350796 | 77 | 0.77 % |
| Old Rogue's Earring | 341520814 | 77 | 0.77 % |
| Old Mana Belt | 340340796 | 77 | 0.77 % |
| Old Mano Earring | 310460799 | 77 | 0.77 % |
| Old Magic Power Necklace | 320380802 | 77 | 0.77 % |
| Old Mage Earring | 340940812 | 76 | 0.76 % |
| Old Agility Necklace | 320550798 | 76 | 0.76 % |
| Old Skeleton Belt | 345000808 | 66 | 0.66 % |
| Old Priest's Earring | 341810815 | 66 | 0.66 % |
| Old Belt of Harpy | 340650803 | 66 | 0.66 % |
| Old White Dragon Necklace | 320540794 | 66 | 0.66 % |
| Old Belt of Dexterity | 340640795 | 51 | 0.51 % |
| Old Flame Ring | 343550830 | 51 | 0.51 % |
| Old Bronze Belt | 340360802 | 51 | 0.51 % |
| Old Bronze Earring | 310130791 | 51 | 0.51 % |
| Old Black Dragon Necklace | 320360797 | 51 | 0.51 % |
| Old Blue Dragon Necklace | 320620795 | 51 | 0.51 % |
| Belt of Dexterity | 340610111 | 8 | 0.08 % |

### Gold Dragon Box [811038000] (new)

Own source: Dragon necklaces/amulets + Dragon Scale armour. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Dragon Scale Armor Pants | 507002000 | 68 | 0.68 % |
| Dragon Scale Armor Pants | 922002000 | 68 | 0.68 % |
| Dragon Scale Armor Pants | 917002000 | 68 | 0.68 % |
| Dragon Scale Armor Pants | 517002000 | 68 | 0.68 % |
| Dragon Scale Armor Top | 922001000 | 68 | 0.68 % |
| Dragon Scale Armor Top | 517001000 | 68 | 0.68 % |
| Dragon Scale Armor Top | 507001000 | 68 | 0.68 % |
| Dragon Scale Armor Top | 917001000 | 68 | 0.68 % |
| Dragon Scale Boots | 507005000 | 68 | 0.68 % |
| Dragon Scale Boots | 917005000 | 68 | 0.68 % |
| Dragon Scale Boots | 517005000 | 68 | 0.68 % |
| Dragon Scale Boots | 922005000 | 68 | 0.68 % |
| Dragon Scale Gauntlet | 922004000 | 67 | 0.67 % |
| Dragon Scale Gauntlet | 517004000 | 67 | 0.67 % |
| Dragon Scale Gauntlet | 917004000 | 67 | 0.67 % |
| Dragon Scale Gauntlet | 507004000 | 67 | 0.67 % |
| Dragon Scale Helmet | 507003000 | 67 | 0.67 % |
| Dragon Scale Helmet | 922003000 | 67 | 0.67 % |
| Dragon Scale Helmet | 917003000 | 67 | 0.67 % |
| Dragon Scale Helmet | 517003000 | 67 | 0.67 % |
| Red Dragon Amulet | 320310129 | 27 | 0.27 % |
| Black Dragon Necklace | 320310130 | 27 | 0.27 % |
| Green Dragon Amulet | 320510131 | 27 | 0.27 % |
| White Dragon Necklace | 320510132 | 27 | 0.27 % |
| Blue Dragon Necklace | 320610128 | 27 | 0.27 % |
| Dragon's Marble | 389200000 | 13 | 0.13 % |

### Cursed Weapon Material Box [814049000] (new)

Own source: Cursed Bone, Magic Jewel Powder, Iron/Silver bar. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Cursed Bone | 379045000 | 375 | 3.75 % |
| Magic Jewel Powder | 379236000 | 375 | 3.75 % |
| Iron bar | 379076000 | 375 | 3.75 % |
| Silver bar | 379067000 | 375 | 3.75 % |

### Cursed Weapon Box [814050000] (new)

Own source: Cursed weapon frames + Sword of Cursed Knight. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Frame of Cursed Dagger | 814051000 | 134 | 1.34 % |
| Frame of Cursed Two Handed Sword | 814052000 | 134 | 1.34 % |
| Frame of Cursed Two Handed Axe | 814053000 | 134 | 1.34 % |
| Frame of Cursed Spear | 814054000 | 134 | 1.34 % |
| Frame of Cursed Bow | 814055000 | 133 | 1.33 % |
| Frame of Cursed Flame Staff | 814056000 | 133 | 1.33 % |
| Frame of Cursed Frost Staff | 814057000 | 133 | 1.33 % |
| Frame of Cursed Thunder Staff | 814058000 | 133 | 1.33 % |
| Frame of Cursed Priest's Mace | 814059000 | 133 | 1.33 % |
| Frame of Cursed Shield | 814060000 | 133 | 1.33 % |
| Frame of Cursed Longspear | 814061000 | 133 | 1.33 % |
| Sword of Cursed Knight | 910028000 | 33 | 0.33 % |

### 15th Anniversary Fortune Box [890083000] (new)

Own source: 15th Anniversary: blessed scrolls, enchant scrolls, lottery. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Weapon Enchant Scroll | 389155000 | 200 | 2.00 % |
| Armor Enchant Scroll | 389156000 | 200 | 2.00 % |
| Blessed Upgrade Scroll | 379021000 | 100 | 1.00 % |
| Blessed Enchant Scroll(STR) | 379022000 | 100 | 1.00 % |
| Blessed Immune Scroll | 379023000 | 100 | 1.00 % |
| Blessed Reduce Scroll | 379024000 | 100 | 1.00 % |
| Blessed Elemental Scroll | 379025000 | 100 | 1.00 % |
| Accessory Enchant Scroll(STR) | 379160000 | 100 | 1.00 % |
| Accessory Enchant Scroll(HP) | 379161000 | 100 | 1.00 % |
| Accessory Enchant Scroll(DEX) | 379162000 | 100 | 1.00 % |
| Accessory Enchant Scroll(INT) | 379163000 | 100 | 1.00 % |
| Accessory Enchant Scroll(MP) | 379164000 | 100 | 1.00 % |
| Premium Lottery | 379166000 | 50 | 0.50 % |
| Unique Item Lotto Ticket | 379158000 | 50 | 0.50 % |

### Fortune Box I [890084000] (new)

Own source: Fortune I: low scrolls, potions. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll (Low Class item) | 379221000 | 137 | 1.37 % |
| Enchant Scroll(STR) (Low Class item) | 379222000 | 137 | 1.37 % |
| Enchant Scroll(HP) (Low Class item) | 379223000 | 137 | 1.37 % |
| Enchant Scroll(DEX) (Low Class item) | 379224000 | 137 | 1.37 % |
| Enchant Scroll(INT) (Low Class item) | 379225000 | 136 | 1.36 % |
| Enchant Scroll(MAGIC) (Low Class item) | 379226000 | 136 | 1.36 % |
| Immune Scroll (Low Class item) | 379227000 | 136 | 1.36 % |
| Reduce Scroll (Low Class item) | 379228000 | 136 | 1.36 % |
| Elemental Scroll (Low Class item) | 379229000 | 136 | 1.36 % |
| Sweeping Potion | 379098000 | 68 | 0.68 % |
| Alacrity potion | 389054000 | 68 | 0.68 % |
| Potion of recovery | 389059000 | 68 | 0.68 % |
| Speed-Up Potion | 379129000 | 68 | 0.68 % |

### Fortune Box II [890085000] (new)

Own source: Fortune II. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll (Low Class item) | 379221000 | 120 | 1.20 % |
| Enchant Scroll(STR) (Low Class item) | 379222000 | 120 | 1.20 % |
| Enchant Scroll(HP) (Low Class item) | 379223000 | 120 | 1.20 % |
| Enchant Scroll(DEX) (Low Class item) | 379224000 | 120 | 1.20 % |
| Enchant Scroll(INT) (Low Class item) | 379225000 | 120 | 1.20 % |
| Enchant Scroll(MAGIC) (Low Class item) | 379226000 | 120 | 1.20 % |
| Immune Scroll (Low Class item) | 379227000 | 120 | 1.20 % |
| Reduce Scroll (Low Class item) | 379228000 | 120 | 1.20 % |
| Elemental Scroll (Low Class item) | 379229000 | 120 | 1.20 % |
| MP Shell | 389084000 | 60 | 0.60 % |
| Super MP Shell | 389085000 | 60 | 0.60 % |
| Hyper MP Shell | 389086000 | 60 | 0.60 % |
| Sweeping Potion | 379098000 | 60 | 0.60 % |
| Alacrity potion | 389054000 | 60 | 0.60 % |
| Potion of recovery | 389059000 | 60 | 0.60 % |
| Speed-Up Potion | 379129000 | 60 | 0.60 % |

### Fortune Box III [890086000] (new)

Own source: Fortune III. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll (Low Class item) | 379221000 | 91 | 0.91 % |
| Enchant Scroll(STR) (Low Class item) | 379222000 | 91 | 0.91 % |
| Enchant Scroll(HP) (Low Class item) | 379223000 | 91 | 0.91 % |
| Enchant Scroll(DEX) (Low Class item) | 379224000 | 91 | 0.91 % |
| Enchant Scroll(INT) (Low Class item) | 379225000 | 91 | 0.91 % |
| Enchant Scroll(MAGIC) (Low Class item) | 379226000 | 91 | 0.91 % |
| Immune Scroll (Low Class item) | 379227000 | 91 | 0.91 % |
| Reduce Scroll (Low Class item) | 379228000 | 91 | 0.91 % |
| Elemental Scroll (Low Class item) | 379229000 | 91 | 0.91 % |
| Upgrade Scroll (Middle Class item) | 379205000 | 46 | 0.46 % |
| Enchant Scroll(STR) (Middle Class item) | 379206000 | 46 | 0.46 % |
| Enchant Scroll(HP) (Middle Class item) | 379208000 | 46 | 0.46 % |
| Enchant Scroll(DEX) (Middle Class item) | 379209000 | 46 | 0.46 % |
| Enchant Scroll(INT) (Middle Class item) | 379210000 | 46 | 0.46 % |
| Enchant Scroll(MAGIC) (Middle Class item) | 379211000 | 46 | 0.46 % |
| Immune Scroll (Middle Class item) | 379212000 | 45 | 0.45 % |
| Reduce Scroll (Middle Class item) | 379213000 | 45 | 0.45 % |
| Elemental Scroll (Middle Class item) | 379214000 | 45 | 0.45 % |
| MP Shell | 389084000 | 45 | 0.45 % |
| Super MP Shell | 389085000 | 45 | 0.45 % |
| Hyper MP Shell | 389086000 | 45 | 0.45 % |
| HP Shell | 389090000 | 45 | 0.45 % |
| Super HP Shell | 389091000 | 45 | 0.45 % |
| Hyper HP Shell | 389092000 | 45 | 0.45 % |

### Fortune Box IV [890087000] (new)

Own source: Fortune IV. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll (Middle Class item) | 379205000 | 89 | 0.89 % |
| Enchant Scroll(STR) (Middle Class item) | 379206000 | 89 | 0.89 % |
| Enchant Scroll(HP) (Middle Class item) | 379208000 | 89 | 0.89 % |
| Enchant Scroll(DEX) (Middle Class item) | 379209000 | 89 | 0.89 % |
| Enchant Scroll(INT) (Middle Class item) | 379210000 | 88 | 0.88 % |
| Enchant Scroll(MAGIC) (Middle Class item) | 379211000 | 88 | 0.88 % |
| Immune Scroll (Middle Class item) | 379212000 | 88 | 0.88 % |
| Reduce Scroll (Middle Class item) | 379213000 | 88 | 0.88 % |
| Elemental Scroll (Middle Class item) | 379214000 | 88 | 0.88 % |
| MP Shell | 389084000 | 44 | 0.44 % |
| Super MP Shell | 389085000 | 44 | 0.44 % |
| Hyper MP Shell | 389086000 | 44 | 0.44 % |
| HP Shell | 389090000 | 44 | 0.44 % |
| Super HP Shell | 389091000 | 44 | 0.44 % |
| Hyper HP Shell | 389092000 | 44 | 0.44 % |
| Durability Shell | 389087000 | 44 | 0.44 % |
| Super Durability Shell | 389088000 | 44 | 0.44 % |
| Hyper durability Shell | 389089000 | 44 | 0.44 % |
| Scroll of Armor 50 | 379121000 | 44 | 0.44 % |
| Scroll of Armor 100 | 379122000 | 44 | 0.44 % |
| Scroll of Armor 150 | 379123000 | 44 | 0.44 % |
| Scroll of 500 HP Up | 379125000 | 44 | 0.44 % |
| Scroll of 750 HP Up | 379126000 | 44 | 0.44 % |
| Scroll of 1000 HP Up | 379127000 | 44 | 0.44 % |
| Scroll of Attack | 379128000 | 44 | 0.44 % |

### Fortune Box V [890088000] (new)

Own source: Fortune V. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll (Middle Class item) | 379205000 | 91 | 0.91 % |
| Enchant Scroll(STR) (Middle Class item) | 379206000 | 91 | 0.91 % |
| Enchant Scroll(HP) (Middle Class item) | 379208000 | 91 | 0.91 % |
| Enchant Scroll(DEX) (Middle Class item) | 379209000 | 91 | 0.91 % |
| Enchant Scroll(INT) (Middle Class item) | 379210000 | 91 | 0.91 % |
| Enchant Scroll(MAGIC) (Middle Class item) | 379211000 | 91 | 0.91 % |
| Immune Scroll (Middle Class item) | 379212000 | 91 | 0.91 % |
| Reduce Scroll (Middle Class item) | 379213000 | 91 | 0.91 % |
| Elemental Scroll (Middle Class item) | 379214000 | 91 | 0.91 % |
| Scroll of Armor 50 | 379121000 | 46 | 0.46 % |
| Scroll of Armor 100 | 379122000 | 46 | 0.46 % |
| Scroll of Armor 150 | 379123000 | 46 | 0.46 % |
| Scroll of 500 HP Up | 379125000 | 46 | 0.46 % |
| Scroll of 750 HP Up | 379126000 | 46 | 0.46 % |
| Scroll of 1000 HP Up | 379127000 | 45 | 0.45 % |
| Scroll of Attack | 379128000 | 45 | 0.45 % |
| HP Scroll | 389093000 | 45 | 0.45 % |
| Defense Scroll | 389094000 | 45 | 0.45 % |
| Resistance Scroll | 389095000 | 45 | 0.45 % |
| Attack Speed Scroll | 389096000 | 45 | 0.45 % |
| Persistent HP Scroll | 389097000 | 45 | 0.45 % |
| Magic Attack Scroll | 389098000 | 45 | 0.45 % |
| Physical Attack Scroll | 389099000 | 45 | 0.45 % |
| Weapon Enchant Scroll | 389155000 | 23 | 0.23 % |
| Armor Enchant Scroll | 389156000 | 23 | 0.23 % |

### Fortune Box VI [890089000] (new)

Own source: Fortune VI. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll (Middle Class item) | 379205000 | 120 | 1.20 % |
| Enchant Scroll(STR) (Middle Class item) | 379206000 | 120 | 1.20 % |
| Enchant Scroll(HP) (Middle Class item) | 379208000 | 120 | 1.20 % |
| Enchant Scroll(DEX) (Middle Class item) | 379209000 | 120 | 1.20 % |
| Enchant Scroll(INT) (Middle Class item) | 379210000 | 120 | 1.20 % |
| Enchant Scroll(MAGIC) (Middle Class item) | 379211000 | 120 | 1.20 % |
| Immune Scroll (Middle Class item) | 379212000 | 120 | 1.20 % |
| Reduce Scroll (Middle Class item) | 379213000 | 120 | 1.20 % |
| Elemental Scroll (Middle Class item) | 379214000 | 120 | 1.20 % |
| Upgrade Scroll | 379016000 | 60 | 0.60 % |
| Enchant Scroll(STR) | 379017000 | 60 | 0.60 % |
| Immune Scroll | 379018000 | 60 | 0.60 % |
| Reduce Scroll | 379019000 | 60 | 0.60 % |
| Elemental Scroll | 379020000 | 60 | 0.60 % |
| Weapon Enchant Scroll | 389155000 | 60 | 0.60 % |
| Armor Enchant Scroll | 389156000 | 60 | 0.60 % |

### Fortune Box VII [890090000] (new)

Own source: Fortune VII. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll | 379016000 | 207 | 2.07 % |
| Enchant Scroll(STR) | 379017000 | 207 | 2.07 % |
| Immune Scroll | 379018000 | 207 | 2.07 % |
| Reduce Scroll | 379019000 | 207 | 2.07 % |
| Elemental Scroll | 379020000 | 207 | 2.07 % |
| Weapon Enchant Scroll | 389155000 | 103 | 1.03 % |
| Armor Enchant Scroll | 389156000 | 103 | 1.03 % |
| Accessory Enchant Scroll(STR) | 379160000 | 52 | 0.52 % |
| Accessory Enchant Scroll(HP) | 379161000 | 52 | 0.52 % |
| Accessory Enchant Scroll(DEX) | 379162000 | 52 | 0.52 % |
| Accessory Enchant Scroll(INT) | 379163000 | 52 | 0.52 % |
| Accessory Enchant Scroll(MP) | 379164000 | 51 | 0.51 % |

### Fortune Box VIII [890091000] (new)

Own source: Fortune VIII. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll | 379016000 | 154 | 1.54 % |
| Enchant Scroll(STR) | 379017000 | 154 | 1.54 % |
| Immune Scroll | 379018000 | 154 | 1.54 % |
| Reduce Scroll | 379019000 | 154 | 1.54 % |
| Elemental Scroll | 379020000 | 154 | 1.54 % |
| Blessed Upgrade Scroll | 379021000 | 77 | 0.77 % |
| Blessed Enchant Scroll(STR) | 379022000 | 77 | 0.77 % |
| Blessed Immune Scroll | 379023000 | 77 | 0.77 % |
| Blessed Reduce Scroll | 379024000 | 77 | 0.77 % |
| Blessed Elemental Scroll | 379025000 | 77 | 0.77 % |
| Weapon Enchant Scroll | 389155000 | 77 | 0.77 % |
| Armor Enchant Scroll | 389156000 | 77 | 0.77 % |
| Accessory Enchant Scroll(STR) | 379160000 | 39 | 0.39 % |
| Accessory Enchant Scroll(HP) | 379161000 | 38 | 0.38 % |
| Accessory Enchant Scroll(DEX) | 379162000 | 38 | 0.38 % |
| Accessory Enchant Scroll(INT) | 379163000 | 38 | 0.38 % |
| Accessory Enchant Scroll(MP) | 379164000 | 38 | 0.38 % |

### Spirit's Giftbox [890102000] (new)

Own source: Spirit's Giftbox: scrolls. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll (Middle Class item) | 379205000 | 100 | 1.00 % |
| Enchant Scroll(STR) (Middle Class item) | 379206000 | 100 | 1.00 % |
| Enchant Scroll(HP) (Middle Class item) | 379208000 | 100 | 1.00 % |
| Enchant Scroll(DEX) (Middle Class item) | 379209000 | 100 | 1.00 % |
| Enchant Scroll(INT) (Middle Class item) | 379210000 | 100 | 1.00 % |
| Enchant Scroll(MAGIC) (Middle Class item) | 379211000 | 100 | 1.00 % |
| Immune Scroll (Middle Class item) | 379212000 | 100 | 1.00 % |
| Reduce Scroll (Middle Class item) | 379213000 | 100 | 1.00 % |
| Elemental Scroll (Middle Class item) | 379214000 | 100 | 1.00 % |
| Upgrade Scroll | 379016000 | 50 | 0.50 % |
| Enchant Scroll(STR) | 379017000 | 50 | 0.50 % |
| Immune Scroll | 379018000 | 50 | 0.50 % |
| Reduce Scroll | 379019000 | 50 | 0.50 % |
| Elemental Scroll | 379020000 | 50 | 0.50 % |
| Scroll of Armor 50 | 379121000 | 50 | 0.50 % |
| Scroll of Armor 100 | 379122000 | 50 | 0.50 % |
| Scroll of Armor 150 | 379123000 | 50 | 0.50 % |
| Scroll of 500 HP Up | 379125000 | 50 | 0.50 % |
| Scroll of 750 HP Up | 379126000 | 50 | 0.50 % |
| Scroll of 1000 HP Up | 379127000 | 50 | 0.50 % |
| Scroll of Attack | 379128000 | 50 | 0.50 % |

### Pandora's Box [890230000] (new)

Own source: Pandora's Box: scrolls, jewels, premium lottery. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll | 379016000 | 123 | 1.23 % |
| Enchant Scroll(STR) | 379017000 | 123 | 1.23 % |
| Immune Scroll | 379018000 | 123 | 1.23 % |
| Reduce Scroll | 379019000 | 123 | 1.23 % |
| Elemental Scroll | 379020000 | 123 | 1.23 % |
| Blessed Upgrade Scroll | 379021000 | 61 | 0.61 % |
| Blessed Enchant Scroll(STR) | 379022000 | 61 | 0.61 % |
| Blessed Immune Scroll | 379023000 | 61 | 0.61 % |
| Blessed Reduce Scroll | 379024000 | 61 | 0.61 % |
| Blessed Elemental Scroll | 379025000 | 61 | 0.61 % |
| Weapon Enchant Scroll | 389155000 | 61 | 0.61 % |
| Armor Enchant Scroll | 389156000 | 61 | 0.61 % |
| Jewel of Strength | 810597000 | 61 | 0.61 % |
| Jewel of Dexterity | 810598000 | 61 | 0.61 % |
| Jewel of Intelligence | 810599000 | 61 | 0.61 % |
| Jewel of Magic | 810600000 | 61 | 0.61 % |
| Jewel of Philosopher | 810610000 | 61 | 0.61 % |
| Jewel of Hardness | 810611000 | 61 | 0.61 % |
| Jewel of Absortion | 810612000 | 61 | 0.61 % |
| Premium Lottery | 379166000 | 31 | 0.31 % |

### Armor material box [900214000] (new)

Own source: Armor materials. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Goblin Pauldron | 379049000 | 158 | 1.58 % |
| Goblin Pads | 379050000 | 158 | 1.58 % |
| Goblin Helmet | 379051000 | 158 | 1.58 % |
| Goblin Gauntlet | 379052000 | 158 | 1.58 % |
| Goblin Boots | 379053000 | 158 | 1.58 % |
| Iron bar | 379076000 | 158 | 1.58 % |
| Silver bar | 379067000 | 157 | 1.57 % |
| Mythril Pauldron | 379054000 | 79 | 0.79 % |
| Mythril Pads | 379055000 | 79 | 0.79 % |
| Mythril Helmet | 379056000 | 79 | 0.79 % |
| Mythril Gauntlet | 379057000 | 79 | 0.79 % |
| Mythril Boots | 379058000 | 79 | 0.79 % |

### No polluted tree box [900348000] (new)

Own source: existing rows (flag 101, +N -> +0). Own 15 %, Red Gem 85 %.

Upgraded rows: 20 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Durandal | 126310000 | 141 | 1.41 % |
| Tomahawk | 131010000 | 141 | 1.41 % |
| Bone Crasher | 141010000 | 141 | 1.41 % |
| Salamander Staff | 181010000 | 141 | 1.41 % |
| Priest Bone crasher | 191010000 | 141 | 1.41 % |
| Bardish | 156010000 | 85 | 0.85 % |
| Mirage Dagger | 111110000 | 71 | 0.71 % |
| Slayer | 121210000 | 71 | 0.71 % |
| Bone Cleaver | 136110000 | 71 | 0.71 % |
| Totamic Club | 146110000 | 71 | 0.71 % |
| Harpoon | 151110000 | 71 | 0.71 % |
| Glave | 156110000 | 71 | 0.71 % |
| Iron Crossbow | 168310000 | 71 | 0.71 % |
| Philum | 151010000 | 71 | 0.71 % |
| Cleaver | 111010000 | 71 | 0.71 % |
| Sword of the Dead | 125350258 | 71 | 0.71 % |

### Old box [900369000] (new)

Own source: existing rows (flag 101, +N -> +0). Own 15 %, Red Gem 85 %.

Upgraded rows: 94 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Chitin Gauntlets | 205004000 | 72 | 0.72 % |
| Salamander Staff | 181010000 | 62 | 0.62 % |
| Priest Bone crasher | 191010000 | 62 | 0.62 % |
| Rogue Chitin Gauntlets | 245004000 | 62 | 0.62 % |
| Rogue Chitin Boots | 245005000 | 62 | 0.62 % |
| Mirage Dagger | 111110000 | 57 | 0.57 % |
| Slayer | 121210000 | 57 | 0.57 % |
| Durandal | 126310000 | 57 | 0.57 % |
| Tomahawk | 131010000 | 57 | 0.57 % |
| Bone Cleaver | 136110000 | 57 | 0.57 % |
| Bone Crasher | 141010000 | 57 | 0.57 % |
| Totamic Club | 146110000 | 57 | 0.57 % |
| Harpoon | 151110000 | 57 | 0.57 % |
| Glave | 156110000 | 56 | 0.56 % |
| Iron Crossbow | 168310000 | 56 | 0.56 % |
| Crimson Robe | 265001000 | 36 | 0.36 % |
| Crimson Pants | 265002000 | 36 | 0.36 % |
| Crimson Helmet | 265003000 | 36 | 0.36 % |
| Crimson Gloves | 265004000 | 36 | 0.36 % |
| Crimson Boots | 265005000 | 36 | 0.36 % |
| Priest Chitin Armor Pauldron | 285001000 | 36 | 0.36 % |
| Priest Chitin Armor Pads | 285002000 | 36 | 0.36 % |
| Priest Chitin Helmet | 285003000 | 36 | 0.36 % |
| Priest Chitin Gauntlets | 285004000 | 36 | 0.36 % |
| Priest Chitin Boots | 285005000 | 36 | 0.36 % |
| Chitin Armor Pauldron | 205001000 | 36 | 0.36 % |
| Chitin Armor Pads | 205002000 | 36 | 0.36 % |
| Chitin Helmet | 205003000 | 36 | 0.36 % |
| Chitin Boots | 205005000 | 36 | 0.36 % |
| Rogue Chitin Armor Pauldron | 245001000 | 36 | 0.36 % |
| Rogue Chitin Armor Pads | 245002000 | 36 | 0.36 % |
| Rogue Chitin Helmet | 245003000 | 36 | 0.36 % |

### Plausible box [900370000] (new)

Own source: existing rows (flag 101, +N -> +0). Own 15 %, Red Gem 85 %.

Upgraded rows: 46 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Complete Boots | 266005000 | 63 | 0.63 % |
| Deep Scar | 131110000 | 63 | 0.63 % |
| Blade Axe | 136210000 | 63 | 0.63 % |
| Impact | 141110000 | 63 | 0.63 % |
| Chitin Shell Gauntlets | 206004000 | 63 | 0.63 % |
| Chitin Shell Boots | 206005000 | 63 | 0.63 % |
| Shard | 111210000 | 57 | 0.57 % |
| Graham | 121310000 | 57 | 0.57 % |
| Mirage | 126410000 | 57 | 0.57 % |
| Chitin Shell Pauldron | 206001000 | 57 | 0.57 % |
| Chitin Shell Pads | 206002000 | 57 | 0.57 % |
| Chitin Shell Helmet | 206003000 | 57 | 0.57 % |
| Complete Helmet | 266003000 | 57 | 0.57 % |
| Complete Glove | 266004000 | 57 | 0.57 % |
| Rogue Chitin Shell Pauldron | 246001000 | 37 | 0.37 % |
| Priest Chitin Shell Pauldron | 286001000 | 37 | 0.37 % |
| Priest Chitin Shell Pads | 286002000 | 37 | 0.37 % |
| Priest Chitin Shell Helmet | 286003000 | 37 | 0.37 % |
| Iron Impact | 146210000 | 37 | 0.37 % |
| Totamic Spear | 151210000 | 37 | 0.37 % |
| Raptor | 156210000 | 37 | 0.37 % |
| Rogue Chitin Shell Pads | 246002000 | 37 | 0.37 % |
| Rogue Chitin Shell Helmet | 246003000 | 37 | 0.37 % |
| Rogue Chitin Shell Gauntlets | 246004000 | 37 | 0.37 % |
| Priest Chitin Shell Gauntlets | 286004000 | 37 | 0.37 % |
| Priest Chitin Shell Boots | 286005000 | 37 | 0.37 % |
| Iron Bow | 168410000 | 37 | 0.37 % |
| Elixir Staff | 181110000 | 37 | 0.37 % |
| Priest Impact | 191110000 | 37 | 0.37 % |
| Rogue Chitin Shell Boots | 246005000 | 37 | 0.37 % |
| Complete Robe | 266001000 | 37 | 0.37 % |
| Complete Pants | 266002000 | 37 | 0.37 % |

### Premium Box [900577000] (new)

Own source: Premium: blessed + enchant scrolls. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Blessed Upgrade Scroll | 379021000 | 154 | 1.54 % |
| Blessed Enchant Scroll(STR) | 379022000 | 154 | 1.54 % |
| Blessed Immune Scroll | 379023000 | 154 | 1.54 % |
| Blessed Reduce Scroll | 379024000 | 154 | 1.54 % |
| Blessed Elemental Scroll | 379025000 | 154 | 1.54 % |
| Weapon Enchant Scroll | 389155000 | 154 | 1.54 % |
| Armor Enchant Scroll | 389156000 | 153 | 1.53 % |
| Accessory Enchant Scroll(STR) | 379160000 | 77 | 0.77 % |
| Accessory Enchant Scroll(HP) | 379161000 | 77 | 0.77 % |
| Accessory Enchant Scroll(DEX) | 379162000 | 77 | 0.77 % |
| Accessory Enchant Scroll(INT) | 379163000 | 77 | 0.77 % |
| Accessory Enchant Scroll(MP) | 379164000 | 77 | 0.77 % |
| Premium Lottery | 379166000 | 38 | 0.38 % |

### Recruiters Supply Box [900660000] (new)

Own source: Recruiters supply: low scrolls, potions, shells. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Upgrade Scroll (Low Class item) | 379221000 | 104 | 1.04 % |
| Enchant Scroll(STR) (Low Class item) | 379222000 | 104 | 1.04 % |
| Enchant Scroll(HP) (Low Class item) | 379223000 | 104 | 1.04 % |
| Enchant Scroll(DEX) (Low Class item) | 379224000 | 104 | 1.04 % |
| Enchant Scroll(INT) (Low Class item) | 379225000 | 104 | 1.04 % |
| Enchant Scroll(MAGIC) (Low Class item) | 379226000 | 103 | 1.03 % |
| Immune Scroll (Low Class item) | 379227000 | 103 | 1.03 % |
| Reduce Scroll (Low Class item) | 379228000 | 103 | 1.03 % |
| Elemental Scroll (Low Class item) | 379229000 | 103 | 1.03 % |
| Sweeping Potion | 379098000 | 103 | 1.03 % |
| Alacrity potion | 389054000 | 103 | 1.03 % |
| Potion of recovery | 389059000 | 103 | 1.03 % |
| Speed-Up Potion | 379129000 | 103 | 1.03 % |
| MP Shell | 389084000 | 52 | 0.52 % |
| Super MP Shell | 389085000 | 52 | 0.52 % |
| Hyper MP Shell | 389086000 | 52 | 0.52 % |

### Buff Box [900668000] (new)

Own source: Buff scrolls & potions. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Scroll of Armor 50 | 379121000 | 94 | 0.94 % |
| Scroll of Armor 100 | 379122000 | 94 | 0.94 % |
| Scroll of Armor 150 | 379123000 | 94 | 0.94 % |
| Scroll of 500 HP Up | 379125000 | 94 | 0.94 % |
| Scroll of 750 HP Up | 379126000 | 94 | 0.94 % |
| Scroll of 1000 HP Up | 379127000 | 94 | 0.94 % |
| Scroll of Attack | 379128000 | 94 | 0.94 % |
| HP Scroll | 389093000 | 94 | 0.94 % |
| Defense Scroll | 389094000 | 94 | 0.94 % |
| Resistance Scroll | 389095000 | 94 | 0.94 % |
| Attack Speed Scroll | 389096000 | 93 | 0.93 % |
| Persistent HP Scroll | 389097000 | 93 | 0.93 % |
| Magic Attack Scroll | 389098000 | 93 | 0.93 % |
| Physical Attack Scroll | 389099000 | 93 | 0.93 % |
| Sweeping Potion | 379098000 | 47 | 0.47 % |
| Alacrity potion | 389054000 | 47 | 0.47 % |
| Potion of recovery | 389059000 | 47 | 0.47 % |
| Speed-Up Potion | 379129000 | 47 | 0.47 % |

### Halloween Mask Box [900669000] (new)

Own source: Halloween / event masks. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Pumpkin's mask | 508021000 | 215 | 2.15 % |
| Scream's mask | 508031000 | 215 | 2.15 % |
| Scream Mask | 508033000 | 214 | 2.14 % |
| Envoy ambassador's mask | 508041000 | 214 | 2.14 % |
| Wizard Pumpkin's mask | 508042000 | 214 | 2.14 % |
| Devil Mask | 508043000 | 214 | 2.14 % |
| Wizard Mask | 508113000 | 214 | 2.14 % |

### (+7)Wing Weapon Box [900733000] (new)

Own source: Wing weapons tier +7: Star..Aztec Holy. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Star Dagger | 1110440000 | 42 | 0.42 % |
| Star Spear | 1110450000 | 42 | 0.42 % |
| Star Long Spear | 1110460000 | 42 | 0.42 % |
| Star Bow | 1110470000 | 42 | 0.42 % |
| Star Staff | 1110480000 | 42 | 0.42 % |
| Star Mace | 1110490000 | 42 | 0.42 % |
| Star Shield | 1110500000 | 42 | 0.42 % |
| Garges Dagger | 1110581000 | 42 | 0.42 % |
| Garges Two-handed Sword | 1110582000 | 42 | 0.42 % |
| Garges Scythe | 1110583000 | 42 | 0.42 % |
| Garges Bow | 1110584000 | 42 | 0.42 % |
| Garges Staff | 1110585000 | 42 | 0.42 % |
| Garges Mace | 1110586000 | 42 | 0.42 % |
| Garges Shield | 1110587000 | 42 | 0.42 % |
| Bloody Dagger | 1111452000 | 42 | 0.42 % |
| Bloody Sword | 1111453000 | 42 | 0.42 % |
| Bloody Two-handed Sword | 1111454000 | 42 | 0.42 % |
| Bloody Bow | 1111455000 | 42 | 0.42 % |
| Bloody Staff | 1111456000 | 42 | 0.42 % |
| Bloody Mace | 1111457000 | 41 | 0.41 % |
| Bloody Shield | 1111458000 | 41 | 0.41 % |
| Dark Knight Dagger | 1111466000 | 41 | 0.41 % |
| Dark Knight Jamadar | 1111467000 | 41 | 0.41 % |
| Dark Knight Spear | 1111468000 | 41 | 0.41 % |
| Dark Knight Crossbow | 1111469000 | 41 | 0.41 % |
| Dark Knight Staff | 1111470000 | 41 | 0.41 % |
| Dark Knight Mace | 1111471000 | 41 | 0.41 % |
| Dark Knight Shield | 1111472000 | 41 | 0.41 % |
| Phoenix Dagger | 1111484000 | 21 | 0.21 % |
| Phoenix Short Sword | 1111485000 | 21 | 0.21 % |
| Phoenix Bow | 1111487000 | 21 | 0.21 % |
| Phoenix Staff | 1111488000 | 21 | 0.21 % |
| Phoenix Mace | 1111489000 | 21 | 0.21 % |
| Phoenix Shield | 1111490000 | 21 | 0.21 % |
| Dagger of the - <selfname> - Legend King | 1111810000 | 21 | 0.21 % |
| Sword of the - <selfname> - Legend King | 1111820000 | 21 | 0.21 % |
| Bow of the - <selfname> - Legend King | 1111840000 | 21 | 0.21 % |
| Staff of the - <selfname> - Legend King | 1111850000 | 21 | 0.21 % |
| Hammer of the - <selfname> - Legend King | 1111860000 | 21 | 0.21 % |
| Shield of the - <selfname> - Legend King | 1111870000 | 21 | 0.21 % |
| Axe of the - <selfname> - Legend King | 1112830000 | 21 | 0.21 % |
| Aztec Holy Dagger | 1121810000 | 10 | 0.10 % |
| Aztec Holy One-Handed Sword | 1121830000 | 10 | 0.10 % |
| Aztec Holy Crossbow | 1121840000 | 10 | 0.10 % |
| Aztec Holy Staff | 1121850000 | 10 | 0.10 % |
| Aztec Holy Mace | 1121860000 | 10 | 0.10 % |
| Aztec Holy Shield | 1121870000 | 10 | 0.10 % |

### (+6)Wing Weapon Box [900734000] (new)

Own source: Wing weapons tier +6. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Star Dagger | 1110440000 | 46 | 0.46 % |
| Star Spear | 1110450000 | 46 | 0.46 % |
| Star Long Spear | 1110460000 | 46 | 0.46 % |
| Star Bow | 1110470000 | 46 | 0.46 % |
| Star Staff | 1110480000 | 46 | 0.46 % |
| Star Mace | 1110490000 | 46 | 0.46 % |
| Star Shield | 1110500000 | 46 | 0.46 % |
| Garges Dagger | 1110581000 | 46 | 0.46 % |
| Garges Two-handed Sword | 1110582000 | 46 | 0.46 % |
| Garges Scythe | 1110583000 | 46 | 0.46 % |
| Garges Bow | 1110584000 | 46 | 0.46 % |
| Garges Staff | 1110585000 | 46 | 0.46 % |
| Garges Mace | 1110586000 | 46 | 0.46 % |
| Garges Shield | 1110587000 | 46 | 0.46 % |
| Bloody Dagger | 1111452000 | 46 | 0.46 % |
| Bloody Sword | 1111453000 | 46 | 0.46 % |
| Bloody Two-handed Sword | 1111454000 | 46 | 0.46 % |
| Bloody Bow | 1111455000 | 46 | 0.46 % |
| Bloody Staff | 1111456000 | 46 | 0.46 % |
| Bloody Mace | 1111457000 | 46 | 0.46 % |
| Bloody Shield | 1111458000 | 46 | 0.46 % |
| Dark Knight Dagger | 1111466000 | 46 | 0.46 % |
| Dark Knight Jamadar | 1111467000 | 46 | 0.46 % |
| Dark Knight Spear | 1111468000 | 46 | 0.46 % |
| Dark Knight Crossbow | 1111469000 | 46 | 0.46 % |
| Dark Knight Staff | 1111470000 | 45 | 0.45 % |
| Dark Knight Mace | 1111471000 | 45 | 0.45 % |
| Dark Knight Shield | 1111472000 | 45 | 0.45 % |
| Phoenix Dagger | 1111484000 | 23 | 0.23 % |
| Phoenix Short Sword | 1111485000 | 23 | 0.23 % |
| Phoenix Bow | 1111487000 | 23 | 0.23 % |
| Phoenix Staff | 1111488000 | 23 | 0.23 % |
| Phoenix Mace | 1111489000 | 23 | 0.23 % |
| Phoenix Shield | 1111490000 | 23 | 0.23 % |
| Dagger of the - <selfname> - Legend King | 1111810000 | 11 | 0.11 % |
| Sword of the - <selfname> - Legend King | 1111820000 | 11 | 0.11 % |
| Bow of the - <selfname> - Legend King | 1111840000 | 11 | 0.11 % |
| Staff of the - <selfname> - Legend King | 1111850000 | 11 | 0.11 % |
| Hammer of the - <selfname> - Legend King | 1111860000 | 11 | 0.11 % |
| Shield of the - <selfname> - Legend King | 1111870000 | 11 | 0.11 % |
| Axe of the - <selfname> - Legend King | 1112830000 | 11 | 0.11 % |

### (+5)Wing Weapon Box [900735000] (new)

Own source: Wing weapons tier +5. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Star Dagger | 1110440000 | 67 | 0.67 % |
| Star Spear | 1110450000 | 67 | 0.67 % |
| Star Long Spear | 1110460000 | 67 | 0.67 % |
| Star Bow | 1110470000 | 67 | 0.67 % |
| Star Staff | 1110480000 | 67 | 0.67 % |
| Star Mace | 1110490000 | 67 | 0.67 % |
| Star Shield | 1110500000 | 67 | 0.67 % |
| Garges Dagger | 1110581000 | 67 | 0.67 % |
| Garges Two-handed Sword | 1110582000 | 67 | 0.67 % |
| Garges Scythe | 1110583000 | 67 | 0.67 % |
| Garges Bow | 1110584000 | 67 | 0.67 % |
| Garges Staff | 1110585000 | 67 | 0.67 % |
| Garges Mace | 1110586000 | 67 | 0.67 % |
| Garges Shield | 1110587000 | 67 | 0.67 % |
| Bloody Dagger | 1111452000 | 33 | 0.33 % |
| Bloody Sword | 1111453000 | 33 | 0.33 % |
| Bloody Two-handed Sword | 1111454000 | 33 | 0.33 % |
| Bloody Bow | 1111455000 | 33 | 0.33 % |
| Bloody Staff | 1111456000 | 33 | 0.33 % |
| Bloody Mace | 1111457000 | 33 | 0.33 % |
| Bloody Shield | 1111458000 | 33 | 0.33 % |
| Dark Knight Dagger | 1111466000 | 33 | 0.33 % |
| Dark Knight Jamadar | 1111467000 | 33 | 0.33 % |
| Dark Knight Spear | 1111468000 | 33 | 0.33 % |
| Dark Knight Crossbow | 1111469000 | 33 | 0.33 % |
| Dark Knight Staff | 1111470000 | 33 | 0.33 % |
| Dark Knight Mace | 1111471000 | 33 | 0.33 % |
| Dark Knight Shield | 1111472000 | 33 | 0.33 % |
| Phoenix Dagger | 1111484000 | 17 | 0.17 % |
| Phoenix Short Sword | 1111485000 | 17 | 0.17 % |
| Phoenix Bow | 1111487000 | 17 | 0.17 % |
| Phoenix Staff | 1111488000 | 17 | 0.17 % |
| Phoenix Mace | 1111489000 | 16 | 0.16 % |
| Phoenix Shield | 1111490000 | 16 | 0.16 % |

### (+4)Wing Weapon Box [900736000] (new)

Own source: Wing weapons tier +4. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Star Dagger | 1110440000 | 78 | 0.78 % |
| Star Spear | 1110450000 | 78 | 0.78 % |
| Star Long Spear | 1110460000 | 78 | 0.78 % |
| Star Bow | 1110470000 | 78 | 0.78 % |
| Star Staff | 1110480000 | 78 | 0.78 % |
| Star Mace | 1110490000 | 78 | 0.78 % |
| Star Shield | 1110500000 | 78 | 0.78 % |
| Garges Dagger | 1110581000 | 78 | 0.78 % |
| Garges Two-handed Sword | 1110582000 | 78 | 0.78 % |
| Garges Scythe | 1110583000 | 78 | 0.78 % |
| Garges Bow | 1110584000 | 78 | 0.78 % |
| Garges Staff | 1110585000 | 78 | 0.78 % |
| Garges Mace | 1110586000 | 78 | 0.78 % |
| Garges Shield | 1110587000 | 78 | 0.78 % |
| Bloody Dagger | 1111452000 | 39 | 0.39 % |
| Bloody Sword | 1111453000 | 39 | 0.39 % |
| Bloody Two-handed Sword | 1111454000 | 39 | 0.39 % |
| Bloody Bow | 1111455000 | 39 | 0.39 % |
| Bloody Staff | 1111456000 | 39 | 0.39 % |
| Bloody Mace | 1111457000 | 39 | 0.39 % |
| Bloody Shield | 1111458000 | 39 | 0.39 % |
| Dark Knight Dagger | 1111466000 | 20 | 0.20 % |
| Dark Knight Jamadar | 1111467000 | 20 | 0.20 % |
| Dark Knight Spear | 1111468000 | 19 | 0.19 % |
| Dark Knight Crossbow | 1111469000 | 19 | 0.19 % |
| Dark Knight Staff | 1111470000 | 19 | 0.19 % |
| Dark Knight Mace | 1111471000 | 19 | 0.19 % |
| Dark Knight Shield | 1111472000 | 19 | 0.19 % |

### Forgotten Accessory Box [900823000] (new)

Own source: existing Forgotten accessory rows (flag 101). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Old Gold Earring | 310120793 | 77 | 0.77 % |
| Old Green Dragon Necklace | 320530793 | 77 | 0.77 % |
| Old Strength Necklace | 320630800 | 77 | 0.77 % |
| Old Belt of Strength | 340520792 | 77 | 0.77 % |
| Old Lightning Belt | 340540799 | 77 | 0.77 % |
| Old Red Dragon Necklace | 320350796 | 77 | 0.77 % |
| Old Rogue's Earring | 341520814 | 77 | 0.77 % |
| Old Mana Belt | 340340796 | 77 | 0.77 % |
| Old Mano Earring | 310460799 | 77 | 0.77 % |
| Old Magic Power Necklace | 320380802 | 77 | 0.77 % |
| Old Mage Earring | 340940812 | 76 | 0.76 % |
| Old Agility Necklace | 320550798 | 76 | 0.76 % |
| Old Skeleton Belt | 345000808 | 66 | 0.66 % |
| Old Priest's Earring | 341810815 | 66 | 0.66 % |
| Old Belt of Harpy | 340650803 | 66 | 0.66 % |
| Old White Dragon Necklace | 320540794 | 66 | 0.66 % |
| Old Belt of Dexterity | 340640795 | 51 | 0.51 % |
| Old Flame Ring | 343550830 | 51 | 0.51 % |
| Old Bronze Belt | 340360802 | 51 | 0.51 % |
| Old Bronze Earring | 310130791 | 51 | 0.51 % |
| Old Black Dragon Necklace | 320360797 | 51 | 0.51 % |
| Old Blue Dragon Necklace | 320620795 | 51 | 0.51 % |
| Belt of Dexterity | 340610111 | 8 | 0.08 % |

### Advanced Forgotten Accessory Box [900824000] (new)

Own source: High Class accessory rows (Iron Necklace/Belt, Belt of Harpy; own +1 rows have no +0 twin). Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Belt of Harpy | 340610116 | 607 | 6.07 % |
| Iron Necklace | 320310126 | 447 | 4.47 % |
| Iron Belt | 340610107 | 446 | 4.46 % |

### Lucky Box [910078000] (new)

Own source: existing Lucky Box rows (flag 101, +N -> +0). Own 15 %, Red Gem 85 %.

Upgraded rows: 19 replaced by their +0 twin.

| Own reward | id | slots | % |
|---|---|---|---|
| Durandal | 126310000 | 147 | 1.47 % |
| Tomahawk | 131010000 | 147 | 1.47 % |
| Bone Crasher | 141010000 | 147 | 1.47 % |
| Salamander Staff | 181010000 | 147 | 1.47 % |
| Priest Bone crasher | 191010000 | 147 | 1.47 % |
| Bardish | 156010000 | 108 | 1.08 % |
| Mirage Dagger | 111110000 | 73 | 0.73 % |
| Slayer | 121210000 | 73 | 0.73 % |
| Bone Cleaver | 136110000 | 73 | 0.73 % |
| Totamic Club | 146110000 | 73 | 0.73 % |
| Glave | 156110000 | 73 | 0.73 % |
| Iron Crossbow | 168310000 | 73 | 0.73 % |
| Philum | 151010000 | 73 | 0.73 % |
| Cleaver | 111010000 | 73 | 0.73 % |
| Sword of the Dead | 125350258 | 73 | 0.73 % |

### Ruler of the Legends Box [978049000] (new)

Own source: Ruler of the Legend weapons. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Ruler of the Legend - Serpent | 1113312000 | 188 | 1.88 % |
| Ruler of the Legend - White Tiger | 1113313000 | 188 | 1.88 % |
| Ruler of the Legend - Blue Dragon | 1113314000 | 188 | 1.88 % |
| Ruler of the Legend - Phoenix | 1113315000 | 188 | 1.88 % |
| Ruler of the Legend - Pegasus | 1113316000 | 187 | 1.87 % |
| Ruler of the Legend - Diana's Deer | 1113317000 | 187 | 1.87 % |
| Ruler of the Legend - Black Tortoise | 1113318000 | 187 | 1.87 % |
| Ruler of the Legend - Fenrir | 1113319600 | 187 | 1.87 % |

### Rosetta Armor Box [978053000] (new)

Own source: Rosetta armour. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| Rosetta Warrior Top | 521511000 | 75 | 0.75 % |
| Rosetta Warrior Bottom | 521512000 | 75 | 0.75 % |
| Rosetta Warrior Helmet | 521513000 | 75 | 0.75 % |
| Rosetta Warrior Guntlet | 521514000 | 75 | 0.75 % |
| Rosetta Warrior Boots | 521515000 | 75 | 0.75 % |
| Rosetta Assassin Top | 522511000 | 75 | 0.75 % |
| Rosetta Assassin Bottom | 522512000 | 75 | 0.75 % |
| Rosetta Assassin Helmet | 522513000 | 75 | 0.75 % |
| Rosetta Assassin Guntlet | 522514000 | 75 | 0.75 % |
| Rosetta Assassin Boots | 522515000 | 75 | 0.75 % |
| Rosetta Mage Robe | 523511000 | 75 | 0.75 % |
| Rosetta Mage Pants | 523512000 | 75 | 0.75 % |
| Rosetta Mage Helmet | 523513000 | 75 | 0.75 % |
| Rosetta Mage Gloves | 523514000 | 75 | 0.75 % |
| Rosetta Mage Boots | 523515000 | 75 | 0.75 % |
| Rosetta Priest Top | 524511000 | 75 | 0.75 % |
| Rosetta Priest Bottom | 524512000 | 75 | 0.75 % |
| Rosetta Priest Helmet | 524513000 | 75 | 0.75 % |
| Rosetta Priest Guntlet | 524514000 | 75 | 0.75 % |
| Rosetta Priest Boots | 524515000 | 75 | 0.75 % |

### Steel Lion Weapon Box [999914000] (new)

Own source: Steel Lion weapons. Own 15 %, Red Gem 85 %.

| Own reward | id | slots | % |
|---|---|---|---|
| [Steel Lion] Dagger | 1113346800 | 167 | 1.67 % |
| [Steel Lion] Jamadhar | 1113347800 | 167 | 1.67 % |
| [Steel Lion] Spear | 1113348800 | 167 | 1.67 % |
| [Steel Lion] Bow | 1113349800 | 167 | 1.67 % |
| [Steel Lion] Mace | 1113351800 | 167 | 1.67 % |
| [Steel Lion] Shield | 1113352800 | 167 | 1.67 % |
| [Steel Lion] Fire Staff | 2113350700 | 166 | 1.66 % |
| [Steel Lion] Ice Staff | 2113350720 | 166 | 1.66 % |
| [Steel Lion] Lightning Staff | 2113350740 | 166 | 1.66 % |

## Skipped sheet boxes (left untouched, not breakable)

* Gift box which cannot be given [900063000]: placeholder "Gift box which cannot be given" (quest gift without owner)
* [Goblin food box] Fourth sample [900152000]: quest leftover "Goblin food box - Fourth sample"
* [Goblin food box] Fourth sample [900154000]: quest leftover "Goblin food box - Fourth sample"
* [Goblin food box] Fourth sample [900155000]: quest leftover "Goblin food box - Fourth sample"
* [Goblin food box] Fourth sample [900156000]: quest leftover "Goblin food box - Fourth sample"
* Specially made honey box [900196000]: quest/cooking ingredient box (Specially made honey box)
* Specially made fruit box [900198000]: quest/cooking ingredient box (Specially made fruit box)
* Festival Preparation Box [910209000]: quest turn-in item (Festival Preparation Box, handed to the Imperial Chef)

## Existing breakable items that are not boxes (untouched)

Transform scrolls (390000000-398000000), Abyss Gem, Confidential State Document x2, Village Key, Lost Gift have no `random_flag` 1..3 rows. Red Gem itself is the pool source and is not changed.

## Round 2: reward grades (+1 everywhere, wing weapon boxes by name)

Migration: `migrations/20261020300000_box_reward_grades.sql` (single transaction, idempotent; validates reward ids exist and that every box keeps its exact slot sum). Scope: all rows with `random_flag IN (1,2,3)` of every `effect2 = 251` box (the 86 generated boxes incl. the Red Gem box).

1. **Plain +0 equipment becomes +1.** A reward item `N` is replaced by `N+1` when `N+1` is named `<name> (+1)` (a few name typos allow-listed: Jamadhar/Jamadar, Flaming/Fire Staff Black Lion) and has the same kind/slot/race/class/req_level. Result on the live DB: 4454 reward rows switched to a `(+1)` item (0 plain +0 equipment rows with a +1 sibling remain); weights are untouched. Unchanged: consumables, scrolls, gems, materials, and ~200 distinct unique/set pieces that have no `(+1)` variant (e.g. Dark Vane, Cold-Hearted Dagger, Spear of Murky Waters, Dread Shield, Eagle's Eye, Dark Knight/Phoenix Axe, Chitin Bow). The Red Gem box's original `(+3)` rows stay as they are.
2. **Wing Weapon Boxes follow the box name.** "Wing weapons" = the own weapon/shield rewards (ids >= 1100000000: Star, Garges, Bloody, Dark Knight, Phoenix, Legend King, Aztec Holy, plus Chaos Wirinom/Baal/Nebiros in the Chaos box). Their `+N` siblings are looked up by name (not by `id+N`, Dark Knight Jamadar has a duplicated `(+1)` so its grades sit at id+N+1).
   * `(+7)Wing Weapon Box` 900733000 gives only `(+7)`, 900734000 `(+6)`, 900735000 `(+5)`, 900736000 `(+4)`; the weapon slots are unchanged (1500 of 10000 slots).
   * `Wing Weapon Box` 810284000 and `Chaos Wing Weapon Box` 810329000 (names carry no grade): interpreted as "+4..+7 spread". Each wing weapon keeps its slot share, split 50 % +4 / 30 % +5 / 15 % +6 / 5 % +7 (rounded in whole slots, remainder to +4). The existing row becomes the +4 row, +5/+6/+7 are new rows in the reserved range `n_index 2110000000..2110099999` (93 rows; 2200000000+ does not fit the INTEGER column). 810284000: 14 weapons, +4 758 / +5 448 / +6 224 / +7 70 slots; 810329000: 17 weapons, 740 / 457 / 223 / 80.
   * `[Gold Dragon Egg] Shenlong Weapon Box` 810209000 and every other box use rule 1 only (their name encodes no grade).
   * Re-running first merges the spread rows back into the +4 row and deletes them, then re-splits.
