# Equipment View opcode + remote emblem rendering (2625 client) — RE notes

Source binary: `docs/reverse-engineering/2625/client_src/KnightOnline.exe`
(disassembly dump alongside it as `KnightOnline.exe.c`). Tooling: `python`
(not `python3`) with `capstone` 5.0.9 / `pefile` 2024.8.26 — no IDA/Ghidra/
objdump/radare2. Same constraints and methodology as
`VOICE_CHAT_PROTOCOL.md`: the dump is Hex-Rays decompiled C with embedded
`"SourceFile.cpp", <line>` debug strings from the packet-send tracer
(`sub_703560`), which is what makes it possible to find real function
boundaries in an otherwise almost entirely `sub_XXXXXX`-named binary (very
few C++ symbols survive — mostly vtable names and template instantiations,
e.g. `CPlayerMySelf::\`vftable'`).

## Part 1 — "Equipment View" context-menu opcode

### What already existed before this pass

`crates/ko-game/src/handler/user_info.rs` already had a `sub_opcode == 5`
branch under `WIZ_USER_INFORMATIN` (`0x98`) named `handle_get_user_info_view`,
with a comment claiming it was the client's "right-click equipment view
request" and that the native handler was called `HandleGetUserInfoView`.
**This pass could find no evidence for that function name or that
sub-opcode assignment anywhere in the decompiled client** — grep for
`HandleGetUserInfoView`, `GetUserInfoView`, `UserInfoView`, and `"Equipment
View"` / `EquipmentView` all come up empty in `KnightOnline.exe.c`. The menu
label itself is never a literal string in the client binary — it is almost
certainly loaded from a string-table resource by numeric ID — so the actual
right-click-menu text could not be correlated to a specific `case` directly.
The sub=5 assignment appears to be inherited from community knowledge of a
leaked GameServer source tree, not from this binary. It has been kept as
the working hypothesis (see "What this pass implemented" below) but is
**not independently confirmed**.

### Confirmed: the client's own incoming-packet dispatch for 0x98

The main S2C packet dispatcher (reached via the opcode lookup table audited
in `KNIGHTONLINE_2625_PROTOCOL_AUDIT.md`) has, for `case 0x98u:`, a single
unconditional call `sub_7DA4F0((_DWORD *)v357, (int)a3, a4);` (decompiled
dump line ~947620). Reading `sub_7DA4F0` directly (it's short) gives the
**real, complete sub-opcode switch the client uses for incoming
`WIZ_USER_INFORMATIN` traffic**:

```c
switch (sub_opcode) {       // byte right after the 0x98 opcode byte
    case 1: /* -> this+120 manager, sub_A13090 */ break;
    case 2: /* -> this+361 manager, sub_BFBF60 */ break;
    case 3: /* -> this+120 manager, sub_A138B0 */ break;
    case 4: /* -> this+120 manager, sub_A14170 */ break;
    case 6: /* -> this+433 manager, sub_734780 */ break;
    default: return;        // <-- sub-opcode 5 falls here
}
```

**This is a confirmed, directly-read fact, not an inference**: a server
reply sent as `WIZ_USER_INFORMATIN` sub-opcode **5** is silently dropped by
the real v2625 client — there is no `case 5` at all. Any implementation
that both sends the sub=5 request *and* expects a sub=5 reply under the
same opcode (the symmetric pattern `user_info.rs` already uses for sub 1/3
list request/refresh) is therefore **known to be unworkable** for the reply
leg, confirmed by direct reading of the client's own switch statement.

### A second, more concrete lead: sub=6 and `UIOtherUserState.cpp`

Searching the whole dump for compile-unit debug strings whose names look
like the right-click "other player" panel turned up exactly one hit:

```c
// decompiled dump, ~line 1616471 (case 0x73 of a large UI-command switch)
case 0x73:
    ...
    v273 = 1688;                    // bytes: 0x98, 0x06  (opcode 0x98, sub-opcode 6)
    LOBYTE(v274) = v208;            // byte: a length prefix
    ...
    memmove((char*)&v274 + 1, v210, v208);   // SByte-string payload (a name)
    sub_703560(&savedregs, &v273, v209, "UIOtherUserState.cpp", 482);
    sub_7032E0(dword_1114654, &v273, v209);
```

This is a real, traced C2S send: opcode `0x98`, sub-opcode **6**, payload
`[u8 len][name bytes]` (a plain SByte string), tagged with the debug string
`"UIOtherUserState.cpp"` — i.e. genuinely inside the class that manages the
right-click "other player" panel, not inferred from an opcode-numbering
neighborhood. Sub-opcode 6 is also exactly the one real case the client's
own `sub_7DA4F0` S2C dispatcher *does* have that sub=5 lacks, dispatching
into `sub_734780` — a very large (~1,770-line) decompiled function. Its
size argues against it being a simple "render 17 equip slots" reply —
more likely it is a multiplexed hub for several different "something about
another player" notifications (incoming party/duel/trade/friend prompts
routed to the same UI class), not a dedicated equipment-view payload.

**Net assessment**: sub=6/`UIOtherUserState.cpp` is the single
best-attested, directly-observed lead this pass found for *any* right-click
"other player" menu action that sends a raw packet inline (rather than
through one of the other systems' own already-known send paths — Friend
`0x49`, Challenge/Duel `0x75`, etc., which would carry their own source
filenames, not this one). It was **not** conclusively pinned to
specifically "Equipment View" as opposed to e.g. "User Information" or an
incoming-request-notification channel, and chasing it further (tracing
`sub_734780`'s internal field layout) was not completed this pass.

### What this pass implemented

Given the above, this pass made a deliberate, documented choice **not** to
switch the trigger opcode on unconfirmed grounds, and instead:

1. Kept the existing C2S trigger assumption — `WIZ_USER_INFORMATIN` (`0x98`)
   sub-opcode **5**, payload `u16 target_id` — moved as-is from
   `user_info.rs` into a new dedicated file,
   `crates/ko-game/src/handler/equipment_view.rs`, wired the same way
   `item_upgrade.rs` dispatches into `character_seal.rs` (a sub-opcode match
   arm delegating to the new module's `handle()`).
2. Fixed a real, concrete gap in the inherited implementation: it only
   resolved `target_id` as a player `SessionId`
   (`world.get_character_info`), so a GM-spawned bot target (which shares
   the same visual/equip wire format — `bot_ai.rs::write_bot_user_info`)
   silently produced no response at all. The new handler resolves the
   target as either a player or a bot, following the exact pattern already
   used in `attack.rs`: `tid < NPC_BAND && world.get_bot(tid).is_none()`.
3. Fixed a second, more important gap: the inherited implementation replied
   with the *raw 42-slot inventory dump* (`SLOT_MAX`+`HAVE_MAX`, i.e.
   equipped gear **and bag contents**) wrapped in the unrelated
   `WIZ_ITEM_UPGRADE`/`ITEM_CHARACTER_SEAL`/`Preview` wire shape (a real,
   independently-shipped feature — see `character_seal.rs` — for *sealing
   an alt character into a Cypher Ring item and previewing it*). Two
   problems with that: (a) leaking a right-click target's bag contents to
   anyone who inspects them is not what "Equipment View" should do and not
   what the task asked for, and (b) that 42-slot format structurally
   **cannot carry any of the 9 cosplay slots at all** (`INVENTORY_COSP` =
   `SLOT_MAX`(14) + `HAVE_MAX`(28) = 42; cosplay starts at inventory index
   42 and the cosplay-slots array, which is what a 3D-model/"what are they
   wearing" view actually needs, is entirely outside that range) — so it
   could never have shown a worn cosplay item (including the emblem from
   Part 2 below) even if the client parsed it. The new handler instead
   replies with exactly the 17 `equip_visual` slots (8 equipped + 9
   cosplay, `handler/region.rs::VISUAL_SLOT_ORDER`/`get_equipped_visual`),
   using the same per-slot wire encoding (`u32 item_id`, `u16 duration`,
   `u8 flag`) that `region.rs::build_user_inout_with_clan` already uses for
   the broadcast INOUT packet.
4. The S2C reply is sent back under the same opcode/sub (symmetric with the
   sub 1/3 pair), which — per the confirmed finding above — **the real
   client's receive dispatcher does not have a case for**. This is flagged
   explicitly as a known, unresolved gap rather than silently shipped as if
   confirmed; see "Open items" below.

### Confidence summary

| Claim | Confidence | Basis |
|---|---|---|
| C2S trigger is `0x98` sub `5`, `u16` target id | Low-medium | Inherited assumption from a prior pass; not found in this binary by this pass |
| Client's S2C dispatch for `0x98` has no case for sub `5` | **High** | Directly read `sub_7DA4F0`'s switch statement |
| `0x98` sub `6` is used by the right-click "other player" panel (`UIOtherUserState.cpp`), name-based | **High** (opcode/sub/source-file), **Low** (which exact menu action) | Directly read the C2S send site and its debug-trace string |
| Server now resolves target as player-or-bot and sends real `equip_visual` data | **High** | This pass's own code, follows established `attack.rs` pattern and `region.rs` constants |
| The reply shape (`0x98`/sub `5` echoed back) is actually rendered by the client | **Unconfirmed — likely false** | Contradicted by the dispatch-table finding above |

### Open items / what a follow-up pass should try first

- Trace `sub_734780` (the sub=6 S2C handler) field-by-field to determine
  whether it is really the equipment-view renderer or an incoming-request
  notification hub. If it's the former, switch both the C2S trigger and S2C
  reply in `equipment_view.rs` to sub=6, with a name-based target lookup
  instead of (or in addition to) the numeric id.
- A live client + packet capture (this pass had neither) would resolve this
  far faster than further static reading — capture the exact bytes sent
  the moment "Equipment View" is clicked in-game.
- If sub=6 turns out to be the real trigger, note its payload is a
  **name**, not a numeric id — bot targeting would then need a
  name-based bot lookup (`world` does not currently appear to index bots by
  name for this purpose; a linear scan or a small name index would be
  needed).

## Part 2 — Why no one ever sees another player's/bot's emblem

### Confirmed by direct code review (this pass re-verified, did not just trust the summary)

- `crates/ko-game/src/handler/region.rs` (players) and
  `crates/ko-game/src/systems/bot_ai.rs` (bots) both build the full 17-slot
  `equip_visual` array — including index 15 (`CEMBLEM`, inventory slot 47)
  — through the exact same loop as every other slot, with no skip/special
  case for the emblem specifically. Re-read directly: `region.rs` lines
  ~51–58 (`snapshot_broadcast_info`) and ~552–557
  (`build_user_inout_with_clan`'s per-slot write loop); `bot_ai.rs` line
  ~4469 (`write_bot_user_info`'s equivalent loop).
- `item_move.rs`'s `InvenToCosp` handler (~line 1310) has a pre-existing,
  accurate inline comment: the client's lightweight *single-slot*
  `WIZ_USERLOOK_CHANGE`-style update (`send_look_change_packet`) has no
  visual mapping for the emblem slot at all (same as the unused `BAG1`
  cosplay slot) — "the original game never bothered" — so equipping an
  emblem is deliberately special-cased to instead trigger a **full**
  `broadcast_user_appearance_update` (an `INOUT_WARP`-shaped packet, the
  same one Genie toggling and zone-enter use), on the theory that nearby
  players would then receive the emblem's `item_id` in its correct slot of
  a complete, from-scratch equip_visual array. This part of the server is
  internally consistent and, by itself, correctly reasoned.
- Conclusion from (1)+(2): **the correct `item_id`/`durability`/`flag` for
  the emblem genuinely goes out on the wire**, in the right slot, in every
  broadcast/appearance packet, for both players and bots. There is no
  missing flag bit, no skipped slot, and no reachable server-side code path
  that would explain the symptom by itself.

### What this pass could not find in the client binary

The string `"emblem"` (any case) does not appear anywhere in the 58 MB
decompiled dump — not even as a debug/log string — and neither do any of
this project's internal slot names (`CEMBLEM`, `CWING`, `CHELMET`, etc.;
these are server-side/leaked-source naming, not anything the client would
need to spell out, since it indexes equip slots purely by integer). The
client does have genuinely separate `CPlayerMySelf`, `CPlayerOther`, and
`CPlayerBase` classes (confirmed via their surviving vtable symbols —
`CPlayerMySelf::\`vftable'`, `CPlayerOther::\`vftable'`,
`CPlayerBase::\`vftable'` — one of the few places C++ symbol names survive
in this binary at all), which structurally confirms the client *can*
special-case rendering between "this is me" and "this is someone else" at
the class level — but locating the actual per-slot mesh/decal-attachment
function and diffing its behavior between the two classes would require
tracing from those vtables through dozens of unnamed `sub_XXXXXX` functions
with no debug strings to anchor the search (the voice-chat and `0x98`
findings above were only tractable because their call sites happened to sit
inside functions that still carried `"SourceFile.cpp"` packet-trace debug
strings; equipment/model-building code does not send packets, so it has no
equivalent anchor). This was judged out of reach for this pass's remaining
budget, and no further RE on it was attempted beyond this literal-string
search and vtable confirmation.

### Verdict

**This looks like a genuine client-side rendering gap, not a server-side
bug** — stated as a judgment call, not a proven fact, because the exact
client function could not be located this pass:

- Both independent server-side send paths (player and bot) are already
  complete and correct for this slot, re-verified directly rather than
  assumed.
- The item in question (`"[JstKO EMBLEM]"`, item id `931677910`) is named
  after this project, suggesting the `CEMBLEM` cosplay slot is a
  **custom addition built on top of a client-side slot that may never have
  had a real mesh/decal-attachment implementation wired up for any
  character at all** in this client build, rather than a retail KO feature
  that regressed. That would also be consistent with it rendering fine for
  the wearer locally (e.g. drawn through a UI/2D element specific to one's
  own character sheet or the self-only "Cospre review" dialog —
  `CUICospreReview`/`CN3TableBase<__TABLE_REVIEW_COSPRE>`, confirmed real
  classes in this binary, though not confirmed to be *the* path the wearer
  actually sees it through) while never having had the "attach this as a
  worn decal on a `CPlayerOther` instance" code path written at all for
  this specific slot.
- This also lines up with the task's own alternate hypothesis: the
  intended way to see someone else's emblem may always have been through
  an inspection dialog (Part 1's "Equipment View") rather than the 3D
  model — Part 1's new handler already carries the emblem's real
  `item_id`/`flag` in its reply (slot index 15 of the 17), so if that
  opcode/sub ever gets confirmed and the dialog gets built to show it, no
  further server change would be needed for that path specifically.

**Not recommended**: guessing at a server-side "fix" (e.g. flipping a flag
bit) for this. Nothing in the confirmed server code suggests a missing bit
— both send paths already match slot-for-slot. A real fix, if wanted on
the 3D-model-render path, is a client binary patch. This project already
has infrastructure for exactly this kind of patch
(`NTT2625-d3d9proxy/client_patches.cpp` — a table of small byte patches
plus at least one full inline-detour patch, the `CN3CPart` missing-skins
guard, ~100 lines including a logging helper and a `__declspec(naked)`
trampoline). Scope, *not attempted this pass*:
- **If** the gap is a single "skip this for anything that isn't
  `CPlayerMySelf`" branch, the fix would be a small table patch (a few
  bytes), similar to the existing `jne`→`nop` entries in `kPatches[]`.
- **If** the gap is that `CPlayerOther` never allocates/loads the
  emblem's mesh/decal resource at all (no branch to flip, just absent
  code), the fix would need a detour-style patch comparable in size to the
  `CN3CPart` skin-guard one (tens to ~100 lines) to inject the missing
  attach logic.
- Which of these it is was not determined this pass; doing so needs either
  a live client under a debugger (to breakpoint on cosplay-slot processing
  while watching a remote character) or substantially more static tracing
  from the confirmed `CPlayerOther`/`CPlayerBase` vtables than this pass's
  budget allowed.
