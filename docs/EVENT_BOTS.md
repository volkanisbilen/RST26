# Event bots

Event-specific bots let the server owner fill the event rooms so an event can run (and feel
alive) with few human participants. Everything is **off by default** and configured per event in
the admin panel (**Etkinlikler -> Etkinlik Botları**).

Code: `crates/ko-game/src/systems/event_bots.rs` (new module), table `event_bot_config`
(`migrations/20261018110000_event_bot_config.sql`), panel `crates/ko-admin/src/event_bots.rs` +
`assets/event_bots.js`.

## Support matrix

| Event | Support | What the bots do |
|-------|---------|------------------|
| Chaos Dungeon / Expansion (zone 85) | **full** | Join the free-for-all room, hunt every other participant (humans and bots). Kills/deaths are booked in the room exactly like human kills. |
| Border Defence War (zone 84) | **full** | Fill the free slots so that both teams end up the same size (humans + bots, see "Mirror fill"), walk to the central Altar of Manes and fight over it, carry it (slowed) to their base and deliver, fight enemies and the enemy carrier, join the party of a real player of their nation. Kills and deliveries score like players; bots never get a reward (their `EventUser`s are `prize_given = true`). Code: `systems/event_bots/bdw_ai.rs`. |
| Ultima's Dungeon (zone 55) | **full** | Extra raiders in the first room (existing `ultima.rs` bot driver). They count towards the 4-player minimum, so one human can start a raid. |
| Full Moon Rift (zone 89) | **partial** | Support bots hold the Outer Tower and take monsters down to 10 % HP, but never land the killing blow, so every drop/reward stays with the players. |
| Juraid Mountain (zone 87) | **partial** | Follow a real player's party, or walk their nation's lane stage by stage and kill the monsters; mirror fill keeps both teams the same size. Never take chests (see "Juraid bots"). |
| Forgotten Temple (zone 55) | not supported | No registration step, rewards go to whoever stands in the zone. |
| Manes Survival (zones 57-60) | not supported | Individual survival ranking with Manes Orb prizes; bots would only distort the ranking. |

The two unsupported events appear in the panel (so the owner sees why) but cannot be enabled;
the game server also forces them off.

## Rules that hold for every bot

* **No rewards.** Reward loops are session based and skip bot ids. Room bots are created with
  `prize_given = true`, so BDW/Chaos/Juraid reward loops and the Chaos rank ladder (rank rewards)
  ignore them: ranks are computed among humans only. Killing an event bot gives the killer **no
  NP and no gold** (the PK-zone reward path in `bot_on_death` is replaced for event bots).
* **No bot-only events.** Bots only appear in a room with at least `min_humans` (>= 1) humans.
* **Cleanup.** Event end / manual close (`TransitionedToCleanup`, `ManualCloseCleanup`), the
  per-room finish countdown (`temple_event_room_close`), and a 3 s sweep that removes any bot
  whose event or room no longer exists. Runtime bots additionally carry a 150-minute expiry as a
  last backstop. The admin "Botları temizle" button queues `event_bots_clear`.
* **Room isolation.** Event bots are only listed/broadcast to (and attackable by) players of their
  own `event_room`. Players cannot hit same-nation bots (BDW) or bots while combat is closed.
* **Bounded cost.** One registry lock + one pass per 250 ms tick; target rescans every 1.5 s;
  idle when no event bot exists. A panic inside a tick is caught and logged.
* **Fairness.** The result of a room (winner nation, Chaos kills) is computed from real scores,
  bots included. Bots have class-based damage (`calculate_bot_damage`, mitigated by the target's
  armour: `dmg * 240 / (AC + 240)`), a 12 % miss chance and 2 s / 3 s attack cooldowns.

## Configuration (`event_bot_config`, one row per event)

| Column | Meaning |
|--------|---------|
| `enabled` | master switch, default `FALSE` |
| `bots_per_team` | bots per team: per nation in BDW, per room in Chaos, per Ultima room, per Rift group |
| `min_level`/`max_level` | level range of the bots (1-83) |
| `fill_only_when_short` | `TRUE`: top each team up to `bots_per_team` participants (humans + bots). `FALSE`: always add `bots_per_team` bots per team (capped by room capacity) |
| `min_humans` | bots only join when at least this many humans are in the room (min 1) |

Changes are applied by the `event_bots_reload` admin command (queued by the panel on save) and at
server start. A change affects the **next** room that opens; running events are not touched.

Examples (BDW, `bots_per_team = 4`, fill-only-when-short): 1 Karus human + 0 El Morad -> Karus
+3 bots, El Morad +1 bot (a nation without any human gets exactly as many bots as the other side
has players). 6 Karus humans -> no Karus bots, El Morad +6 bots. Both nations human: each tops up
to `bots_per_team` alone.

## How each event runs in this codebase (research notes)

### Temple room events (BDW / Chaos / Juraid) - `event_room.rs`, `event_system.rs`
* Scheduled or GM-opened (`borderopen`, `chaosopen`, `juraidopen`). Players sign up in the
  client's temple window (`WIZ_EVENT TEMPLE_EVENT_JOIN`) into `signed_up_users` (no minimum).
* When sign-up closes (`EventTickAction::TransitionedToActive`) `assign_users_to_rooms` fills
  `EventRoom`s: BDW/Juraid 8 per nation per room, Chaos 18 mixed per room; each room is a logical
  instance (`event_room = room_id`) of one zone (84 / 85 / 87). Humans are teleported to the
  zone's `start_position` of their nation.
* Combat opens at `attack_open`, closes at `attack_close`; the event ends on the play timer or
  early (BDW score threshold, Juraid Deva kill). `send_winner_screen` -> reward distribution ->
  20 s per-room countdown -> kick -> `cleanup_event`.
* Scoring: BDW kills `+1 * nation_user_count`, altar delivery `+10 * nation_user_count`
  (thresholds 130/300/400/600 by head count). Chaos: per-player kills/deaths, rank by kills,
  EXP `level^3 * 0.15 * (5*kills - deaths)` plus rank rewards. Juraid: monsters in per-nation
  lanes open bridges (4+20 / 8+40 / 12+60 kills), the Deva bird or monument kills decide.

### Single-room events
* **Ultima** (`handler/ultima.rs`): GM-opened registration, rooms of up to 30 (event_room 200+),
  seal then boss, rewards by destructive attacks survived. Min 4 players (a GM waives it).
* **Full Moon Rift** (`handler/rift_entry.rs` + `dungeon_defence.rs`): registration -> groups of
  <= 8 per nation -> a rift room; difficulty by group size (2-3 easy, 4-7 normal, 8+ hard); waves
  walk a fixed lane to the towers; rewards per monster kill.
* **Forgotten Temple** (`handler/forgotten_temple.rs`): open entry into zone 55, staged monster
  waves, reward for everybody alive in the zone at the end.
* **Manes Survival** (`systems/manes_survival.rs`): zones 57-60, individual progression/ranking.

The real game's rules are what the C++ reference port encodes (scoring formulas above). Public
web sources on these private-server-era event mechanics are thin; the code is the authority.

## Architecture

```
admin panel --save--> event_bot_config --event_bots_reload--> WorldState.event_bots (config)
event_system tick: TransitionedToActive -> fill_temple_rooms()  (before teleport_users_to_event)
                   cleanup arms / temple_event_room_close / sweep -> cleanup_*()
driver task (250 ms): tick() -> per bot: respawn | pick enemy | walk (A* on the SMD) | attack
death of an event bot (bot_on_death hook) -> handle_bot_death(): WIZ_DEAD to the room, kill
   credit by name (dead.rs track_*_kill_by_name), respawn timer, no NP/gold
```

* Bots are normal `BotInstance`s (`spawn_gm_bot`, `ai_state = Afk`, so `tick_bots` only regenerates
  them) registered in a per-world `EventBotRegistry` (also holds the config). Room bots are also
  `EventUser`s of their room (name keyed, session-band id >= 20000), so team counts, BDW score
  multipliers and Chaos kill books work unchanged. Auto-party creation skips them (no session).
* Bot-driven fighting is NOT done through `bot_ai`'s PK AI (it is hard-wired to zones 71-73);
  `event_bots.rs` has its own small driver and reuses `spawn_gm_bot`, `select_bot_skill_with_weapon`,
  `calculate_bot_damage`, `build_bot_inout_packet` and the pathfinder. `bot_ai.rs` itself only got
  a 4-line death hook and room-aware broadcasts in `event_bot_swing` / `event_bot_place`.
* Hooks outside the new module (all minimal): `bot_ai::bot_on_death`, `attack.rs` / `magic_process.rs`
  bot branches (attack gate), `region.rs` (room-aware bot listing), `event_system.rs` (fill,
  cleanup, driver start), `event_room.rs` (room close), `dead.rs` (name-based `track_*` cores),
  `ultima.rs` / `rift_entry.rs` / `dungeon_defence.rs` (helper counts), `admin_queue.rs`,
  `world/loading.rs`, `world/mod.rs`.

## Juraid bots (`systems/event_bots_juraid.rs`)

Gated by the same config row (`juraid`). Bots are ordinary event bots (`EventBotKind::Juraid`,
room members flagged `prize_given`), so spawn, sweep, cleanup and the "no humans, no bots" rule are
shared with the other events.

```
Karus     spawn (224,272) -> (224,558) -> (224,848) -> (511,848) -> Deva room (508,513)
El Morad  spawn (799,748) -> (799,459) -> (799,171) -> (511,172) -> Deva room (508,513)
```

(Karus is the x = 224 side, confirmed in a live test; the human respawn points in
`handler::regene` use the same sides.)

* **Mirror fill (equal teams).** Shared with BDW (`event_bots::plan_nation_fill`): every nation is
  topped up by the usual config (`bots_per_team`, `fill_only_when_short`), then the smaller team
  gets extra bots until both teams have the same number of members (humans + bots), max 8 per
  nation. Examples with `bots_per_team = 4`: 3 Karus players, 0 El Morad -> 1 Karus bot + 4 El
  Morad bots (4 vs 4); a 5-man Karus party -> 0 Karus bots + 5 El Morad bots (5 vs 5).
* **Follow.** While a real player of the bot's nation is alive in the room the bot joins his party
  (a lone player gets a party created around him, max 8 members) and stays within ~6 m, attacking
  stage monsters within 40 m of him.
* **Lane runner.** With no living real player of its nation the bot walks the chain by itself.
  A stage is cleared when no alive main/child monster (summon types 2/3) is within 60 m of the
  waypoint; only then does the nation's team move to the next waypoint (several stages can pass at
  once when humans cleared them). The last leg ends at the Deva Bird. The tracker only arms after
  the room's monsters were seen once, so the first ticks (bots spawn just before the monsters) do
  not count as "cleared". Per-nation stage state lives in `JuraidRt` (registry), rebuilt per room.
* **Movement** reuses the event-bot walker (A* over the zone's SMD walkability, the 40-degree /
  clearance `bot_blocked` rules via `pathfind`), bots respawn at their nation spawn after 8 s.
* **Killing.** Bots never attack players or monuments (the lanes are PvE; humans may still hit
  enemy bots in the Deva room). A bot swing deals class damage x 8; the killing blow calls
  `dead::track_juraid_monster_kill_by_name` (the name based core that the player path now also
  uses): child monsters, per-nation main/sub counters, bridge opening and Deva finish work as for
  players. Then the monster is removed with `kill_npc` - no loot roll, no exp, no NP.
* **Chests / rewards.** The only chest is the Deva Bird drop, rolled in the player death path.
  Bot kills never roll loot. As long as any real player belongs to the room, bots stop attacking
  the Deva at 10 % HP, so the killing blow and the chest always go to a real player; in a room
  with no real player left they may finish it. Party loot routing (`get_item_routing_user`) skips
  bot ids explicitly (bots have no session anyway), reward loops skip them (`prize_given`).
* **Cost.** One monster/player scan per room per second (not per bot), bot decisions use the
  cached snapshot; party check every 2 s per bot.

Not verified against a live Juraid run: damage balance (x8 multiplier), lane geometry on the SMD
(bots use the same walkability rules as the other event bots) and the bridge objects (bots do not
wait for a bridge to open, they only wait for the stage to be cleared).

## Why not Forgotten Temple / Manes (yet)

* **Forgotten Temple** has no registration/room assignment to hook, and its rewards go to everyone
  alive in the zone. **Manes** ranks individuals for Orb prizes.

## Applying / rolling back

1. Apply `migrations/20261018110000_event_bot_config.sql` (idempotent; creates the table and seeds
   seven OFF rows). `ko-admin` also creates the table at start-up (`schema.sql`).
2. Deploy `ko-server` (game) and `ko-admin`. Without the table the game server logs a warning and
   keeps every event OFF.
3. Switch an event on in the panel. Rollback = untick "etkin" (or "Botları temizle").

## In-game verification checklist

* Chaos: enable with 6 bots, join alone: the room has 6 extra named players in zone 85 who walk
  to you and fight; your kills/deaths and the Chaos rank EXP follow normally; you receive no NP or
  gold for bot kills; after the event the bots are gone (check `+where` / the minimap).
* BDW: enable, join as Karus alone: 3 Karus + 1 El Morad bots appear and the 3 Karus bots are in your
  party (the room's event party; players' own parties get the room's free bots too, 2 slots stay
  free); bots walk to the altar, one kills it and carries it slowly to its base; the scoreboard rises
  with kills and deliveries; you cannot hit Karus bots; bots vanish after the 20 s countdown.
* Two rooms open at once (e.g. 17+ Chaos players): players of room 1 never see room 2's bots.
* Ultima: with the config on, a solo level-80 GM/player can start a raid (bots join room 1).
* Rift: a solo group sees 3 support bots; monsters stop at ~10 % HP until you finish them.
* Juraid: enable, join as El Morad alone: Karus gets `bots_per_team` mirror bots that walk (224,272) -> (224,558) ... clearing
  each stage; your own bots (if `bots_per_team` > 1) join your party and follow you. Bots never loot the Deva
  chest; they leave the Deva at ~10 % HP while you are in the room.
* Turn everything off: no bot appears in any event.

## Limitations

* Forgotten Temple and Manes are not bot-driven (see above).
* BDW altar: bot carriers have no client session, so the carrier slow-down/effect is simulated by the
  driver (shorter steps + a MAGIC_EFFECTING broadcast); players get the real buff.
* Bots fight with a simplified model (no skill rotation, no buffs, no potions; mages/priests only
  use the ranged attack animation). The kill feed is zone-wide, as for human PvP.
* Ultima/Rift helper bots reuse the existing GM test-bot drivers; their swing animations are now
  room-aware, but they remain simple "stand near the target and hit" bots.
* Bot ids come from the shared runtime id counter (base 20000, session band is 16-bit): after
  roughly 45 000 bot spawns without a restart new bots cannot be used as room members. Event bots
  refuse (and log) ids above 65535 instead of aliasing a real session.
* A GM standing in an event zone with `event_room = 0` does not see room bots.
