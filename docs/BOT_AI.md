# Bot brain (bot AI decision layer)

`crates/ko-game/src/systems/bot_brain.rs` is the shared, player-like decision layer for runtime
bots. `bot_ai.rs` keeps the mechanics (packets, damage, skill tables, navigation), `field_bots.rs`
the PvE hunters and `event_bots*.rs` the event rooms; the brain owns the *decisions* and the
human imperfections that all of them share. Knobs live in `bot_brain_config`
(`migrations/20261019000001_bot_brain_config.sql`), panel page **Etkinlikler -> Bot Zekâsı**
(`crates/ko-admin/src/bot_brain.rs`, `assets/bot_brain.js`); a save queues `bot_brain_reload`.

The target difficulty is an **average player**, not a pro: reaction delay, occasional wrong
target, forgotten kiting, finite potion stock.

## Architecture (one PK bot, one 250 ms tick)

```
tick_bots -> tick_fighting -> bot_brain::tick_pk        (legacy loop only when disabled)
event_bots::tick -> bot_brain::tick_event             (BDW / Chaos / Juraid, same core)
   1 perception    zone Snapshot (players+bots, 16 m grid), built once per zone per tick
   2 threat        assess(): hp/mp ratio, enemies vs allies within 30 m, "hit recently", ally HP
   3 consumables   virtual HP/MP potion, buff scroll, TS (never blocks movement)
   4 cast lock     a cast with cast_time pins the bot for cast_time*100 ms
   5 target        pick_target(): utility score, 1.5 s stickiness, mistake roll
   6 decision      decide() -> Mode {Objective, Engage, Kite, Retreat, HealUp, Regroup}
   7 execution     engage()/step_to() through bot_path_step_toward (navigation untouched)
   8 objective     Objective trait: PkObjective (hotspots / patrol), Chaos, BDW, Juraid
```

Cost: snapshot `O(units)` per zone per tick, `O(nearby)` per bot. No `bots x bots x players`.

### Threat and modes

| Situation | Mode | Behaviour |
|-----------|------|-----------|
| HP < `retreat_hp_percent` and enemies near | Retreat | run to the nearest friendly group (>= 2 allies away from the enemies) or 30 m away from the enemy centroid |
| HP < threshold, nobody near | HealUp | potions (and priest self-heal via bot_ai), drift to the party; ends at 70 % HP -> back to the fight |
| enemies >= 2 and `enemies*100 >= allies*outnumbered_percent` | Retreat | same as above |
| target, ranged class, enemy within 6.5 m | Kite | back off to 8 m; `kite_miss_percent` chance to forget; cornered -> fight |
| target | Engage | see below |
| party leader > 24 m away | Regroup | class formation point around the leader |
| nothing | Objective | plug-in (below); always moving, never standing in the open |

Hysteresis: a bot in HealUp/Retreat keeps healing until 70 % HP, then returns.

### Target selection (`score_target`)

`100 + (1-hp%)*60 + priest 25 / mage 10 + (isolated ? +30 : -8 per ally within 12 m)
 + 25 if it attacks me + 20 if it is my current target - 1.5 * distance`.
`target_mistake_percent` of the time the runner-up is taken. Candidates pass the same validity
rules as before (enemy nation, alive, PK zone, no GM, no genie, not blinking, not parked as
unreachable). Event bots use `target_score_basic` (distance/HP/class) in `event_bots::find_target`.

### Engage

* skill from `select_bot_attack_skill`, range from `bot_attack_range_for_skill` (magic tables);
  **never casts out of range**; walks to the class formation point instead.
* reaction delay `reaction_min_ms..reaction_max_ms` before the first hit on a *new* target (also
  applied to event bots through `reaction_ready`).
* mana: a caster that cannot afford the skill falls back to the basic attack (101001/201001) and
  drinks an MP potion when below 25 %.
* after a cast with `cast_time`, movement is locked for its duration (`cast_lock_until_ms`).
* existing cooldowns, AC/elemental/mirror damage and AOE paths are untouched (`bot_perform_attack`).

### Formation and spacing

`formation_point`: warrior -> target; rogue -> flank at `spacing` (side from the member's angle);
mage/priest -> stop at `range - 2` (>= 8 m). `spread_from_mates` pushes the goal away from the
nearest party mate closer than the member's own spacing. Each member has its own spacing in
`[spacing_min, spacing_max]` and its own flank side, reshuffled every 12-28 s
(`apply_party_spacing` in bot_ai now uses it), so a group never walks as a train.

### Objective plug-in (`Objective` trait)

| Impl | Behaviour |
|------|-----------|
| `PkObjective` | `patrol_percent` of PK bots are `Role::Patrol` (surveyed waypoint routes over the whole map, `tick_waypoint_patrol`), the rest `Role::Hotspot`: weighted pick among bowl / bridges / base fronts (enemy front x2, own front x0.5), re-picked every 1.5-4 min; at the spot the bot drifts 2-14 m every 2.5-6 s. Spots are projected on walkable ground; unreachable ones are dropped. Zones 71/72/73. |
| `ChaosObjective` | free for all (allies = party mates only); nobody near -> `roam_drive`: random reachable arena points, re-picked every 15-40 s |
| `BdwObjective` | `bdw_ai::objective`: carrier -> home at the Fragment speed (`Drive::GoalScaled`), enemy carrier -> intercept (+60 target bonus), own carrier -> escort, altar takers (`Role::Hotspot`) -> altar / hit it, roamers (`Role::Patrol`) -> sweep altar..enemy base |
| `JuraidObjective` | `event_bots_juraid::objective`: lane chain + stage clearing + Deva (PvE: `fights_players() == false`); closed bridge gates are `dyn_blocks` for the path queries |

PK bots: a real player in the party overrides everything (`move_toward_party_anchor` as before).
Event bots (`tick_event`) never regroup on or follow a player: they assist the party (+15 on an
enemy hitting a mate) and keep acting on their own. Event bots also use the event attack
mechanics (`event_bots::brain_attack`) and room-filtered movement packets
(`event_bots::brain_move`) instead of `bot_perform_attack` / `broadcast_bot_move_ex`.

### Parties

* Bots form their own parties (`tick_pk_party_ai`): at most `party_max_bots` (6), at most
  `party_max_per_class` (2) of one class, a priest first when the party has none
  (`party_accepts`, `recruit_key`).
* Players with the seek flag / Party BBS get invited (unchanged); a real player inviting a PK
  bot by name (`bot_brain::try_party_invite`, hooked in `handler/party.rs` next to the field-bot
  hook) makes the bot leave its bot party and join the player's party, respecting nation, level
  and the 2-per-class rule (politely declined otherwise).

### Consumables (virtual stock)

Per bot life: `hp_potions`, `mp_potions`, `ts_scrolls`, `buff_scrolls`; refilled on respawn
(`on_bot_respawn`). Cooldown `potion_cooldown_ms` (2400 like players). HP potion below 60 %,
MP below 25 % (casters) / 10 % (melee); scrolls only when no enemy is within 30 m. Skill ids come
from the magic table at runtime (`WorldState::potion_skills`, `scroll_buff_skills`,
`transform_scroll_skills`: item_group 9 / self / type 3 potions, type 4 self buffs, type 6 monster
transforms), so the client plays the real animations; the largest potion not exceeding 60 % of
max HP/MP is used. Scroll/TS effects are visual only (no stat change on bots). Field and event
bots share the stock logic (`tick_consumables_for`).

### NP for bot kills

* PK zones only; event bots stay reward free (`event_bots::handle_bot_death` runs first).
* `bot_kill_np`: `np_percent` (33) of the zone's player-kill NP, **0** when the same killer killed
  the same bot within `antifarm_secs` (300) or the bot respawned less than `spawn_grace_secs` (20)
  ago. The verdict is remembered for 2 s and `loyalty::pvp_loyalty_on_bot_kill` (party NP path
  called by attack.rs / magic_process.rs) honours it and applies the same share; the Collection
  Race credit stays as before. Bot-vs-bot in-memory NP and the victim's NP loss use the same share.
* Field bots: still nothing (`player_can_hurt` / `bot_on_death` early-out untouched).

## Knobs (`bot_brain_config`, defaults)

| Knob | Default | Meaning |
|------|---------|---------|
| enabled | true | off = legacy fight-the-nearest loop, no potions, legacy NP |
| patrol_percent | 40 | share of PK bots patrolling the field, rest at hotspots |
| reaction_min_ms / max | 300 / 900 | reaction delay on a new target |
| target_mistake_percent | 10 | second-best target |
| kite_miss_percent | 25 | ranged class forgets to kite |
| hp_potions / mp_potions / ts_scrolls / buff_scrolls | 30 / 30 / 2 / 4 | virtual stock per life |
| potion_cooldown_ms | 2400 | |
| np_percent | 33 | bot kill NP share |
| antifarm_secs / spawn_grace_secs | 300 / 20 | |
| party_max_bots / party_max_per_class | 6 / 2 | |
| retreat_hp_percent | 35 | |
| outnumbered_percent | 200 | |
| spacing_min / spacing_max | 3 / 8 m | |

## Files

* `systems/bot_brain.rs` (new): config, state, snapshot, threat, decision, target scoring,
  formation, hotspots, consumables, NP policy, party invite, Objective trait, tests.
* `systems/bot_ai.rs`: `tick_fighting` hands PK bots to the brain; party rules/priest priority;
  spacing from the brain; NP share + anti-farm in `process_bot_kill_reward` / `bot_on_death`;
  `bot_regene` -> `on_bot_respawn`; `despawn_bot` -> `forget`; a few helpers made `pub(crate)`.
* `systems/loyalty.rs`: `pvp_loyalty_on_bot_kill` honours the verdict and share.
* `systems/event_bots.rs`: scored target pick, reaction delay, consumables (3 tiny hooks).
* `systems/field_bots.rs`: consumables hook.
* `handler/party.rs`: PK bot invite hook (2 lines).
* `world/tables.rs`: potion/scroll skill accessors.
* `world/loading.rs`, `admin_queue.rs`: config load / `bot_brain_reload`.
* ko-admin: `src/bot_brain.rs`, `assets/bot_brain.js`, nav entry, `schema.sql`.

## What to watch in game

* Hotspot coordinates (bridges/fronts) are estimates projected onto walkable ground; adjust
  `RONARK_HOTSPOTS` / `ARDREAM_HOTSPOTS` / `RLB_HOTSPOTS` if bots gather at odd places.
* Potion/scroll animations depend on the live magic table rows (item_group 9); TS is cosmetic.
* Bots standing still: only during the reaction delay, the cast lock or the 2.5-6 s wander pause.
* NP: a player killing the same bot twice within 5 minutes gets nothing the second time (by design).
