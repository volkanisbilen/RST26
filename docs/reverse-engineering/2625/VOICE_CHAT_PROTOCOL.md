# Party voice chat protocol (2625 client) — reverse-engineering notes

Source binary: `docs/reverse-engineering/2625/client_src/KnightOnline.exe`
(disassembly dump alongside it as `KnightOnline.exe.c`). RE tooling available
in this environment: `python` (NOT `python3`, which resolves to a broken
Windows Store stub) has `capstone` 5.0.9 and `pefile` 2024.8.26 installed.
No IDA/Ghidra/objdump/radare2 available — static analysis only, via
capstone/pefile from Python.

## Summary

The client has real, present (not stubbed-out) voice chat infrastructure:
Opus codec, DirectSound capture/playback, and UI. The server-side protocol
is currently completely unimplemented on our side. This doc records what
was recovered from static analysis so the feature can be built without
redoing the RE from scratch.

## Opcode

The real client-to-server/server-to-client opcode used for voice chat
control messages is **`0xB5`**. This is NOT the same as the `VOICE = 0xC6`
slot reserved (but never sent/handled) in `reference_cpp/shared/packets.h`
— that enum entry is dead/unused in the actual client binary.

In our codebase, `0xB5` is already named `WizSound` in
`crates/ko-protocol/src/opcode.rs:321` (and the opcode<->enum tables around
lines 663 and 881), and is currently only handled as an outbound
server->client stub — see `crates/ko-game/src/handler/mod.rs:466`. There is
no real inbound handler for it yet.

## Client-side class

`CVoiceChatMng` — vftable at `0x1033DB0`, constructor at `0xEBED40`. Owns
mic/speaker state, the Opus encoder/decoder, and the second (voice) TCP
socket described below.

The disassembly dump (`KnightOnline.exe.c`) turned out to be Hex-Rays
*decompiled C*, not raw asm — every reference to `CVoiceChatMng` carries a
`"VoiceChatMng.cpp", <line>` debug string from `sub_703560`, which made it
possible to find and read the two real packet dispatchers directly instead
of tracing vtable pointers by hand:

- **`sub_EC7AF0`** — the client's S2C dispatcher for opcode `0xB5` itself
  (called from the main packet-receive switch's `case 0xB5u:`, right next to
  `case 0xB4u:`/`case 0xB6u:`/`case 0xB7u:`, confirming `0xB5` really is a
  normal opcode dispatched like any other, with the *next* byte as its own
  sub-opcode switch). This is the function that matters most for our server,
  since it defines exactly what the client expects **us to send**.
- **`sub_EC69A0`** — the dispatcher for the *separate* voice-relay socket's
  framed messages. It's reached from a raw-buffer parser that explicitly
  checks `htons(*(u16*)buf) == 0xAA55` then `htons(*(u16*)(buf+len+4)) ==
  0x55AA` — i.e. this is the AA55/55AA framing described below, confirming
  the two-socket architecture directly rather than inferring it.

Three small helpers used throughout both dispatchers were also identified by
reading their (very short) bodies directly, which pins down the exact field
widths below with high confidence: `sub_703080`/`sub_7030A0`/`sub_7030C0`
read a `u16`, `sub_702FC0`/`sub_703020`/`sub_703040` read a `u32`,
`sub_7030E0` reads a single `u8`, and **`sub_702EB0` reads a full `u64`**
(`*(QWORD*)...`, advancing the cursor by 8) — so every "room/token" field
below is a genuine 64-bit value on the wire, not a 32-bit one as might be
guessed from a casual read.

## Two-connection architecture

Voice chat uses **two separate TCP connections**, not one:

1. **Game socket** (the normal, encrypted game connection, via the client's
   existing `0x6FD6C0` send path). Carries ONLY control messages — small
   signaling packets under opcode `0xB5`, sub-opcode in the first data byte.

   **S2C (what `sub_EC7AF0` parses — i.e. what our server must send), now
   decoded field-by-field:**

   | Sub-opcode | Payload | Meaning |
   |---|---|---|
   | 0x02 | `u16 room_id` (`0xFFFF`=fail) `u64 token` `u8 reserved` | Room join/create result. On success the client just stores `room_id`/`token` globals (`dword_1115708`/`qword_1115700`); it sends nothing back for this specific message. On `room_id==0xFFFF` it shows error string table id `0xAC48` and bails without reading further. |
   | 0x03 | `u16 result` (+ 2 more `u16` reads only when `result==1`) | Generic failure/reset. `result==0`: shows error `0xAC3E`. `result==1`: shows error `0xAC3F`, then reads two more `u16` fields (unidentified). `result==2`: shows error `0xAC40` **and resets all voice-chat client state to defaults** (room/token/mic flags) — this is the "you were kicked / room closed" case. |
   | 0x04 | `u8 notice_type` (0-3); for 1-3 also `u8 len` + `len` bytes (SByte string) | Server notice/error with a template id per `notice_type` (`0xAC4E`/`0xAC41`/`0xAC42`/`0xAC43`) and, for 1-3, a text argument (reason/name) substituted into it. |
   | 0x05 | `SByte name` `u32 member_id` `SByte str2` `SByte str3` `u32 val1` `u32 val2` | Per-member roster broadcast (join notice) — adds `name`/`member_id` to the client's voice-chat UI roster. `val1`/`val2`'s exact meaning is **not confirmed** (see Open items) — this implementation's working assumption is documented there. |
   | 0x06 | `u16 room_id` `u64 token` `u8 param` | Same shape as `0x02` (room_id/token), plus one extra byte stored as a per-session "distance/attenuation" value (`this+44`). Looks like a refresh/update variant of the join result rather than a distinct feature. Conditionally triggers a C2S `0x09` ack (see below). |
   | 0x07 | `u8 sub` (1/2/3) then per sub: `1`,`2`: `u32 member_id` `u16 a` `u16 b`; `3`: `u8 count` then `count` × (`u32 member_id` `u16 a` `u16 b`) | Per-member mic/speaker parameter update(s) — single (`1`, also fires a C2S ack via `sub_EC4B60`/`0x09`; `2`, no ack) or batched (`3`). |
   | 0x08 | `u8 member_id_byte` | Sets the "who is currently speaking" indicator (`this+44`) and notifies the roster UI. |
   | 0x09 | `u32` (ignored) | Increments a per-session counter; once it reaches the room's member count, clears the "currently speaking" indicator. Looks like a speaking-timeout tick, not something our server needs to drive proactively. |
   | 0x0A | `u8 enabled` | Global voice-chat enable/disable toggle — clears the roster and shows/hides the voice UI. |

   **C2S (confirmed OUTBOUND sends found at their call sites, i.e. what the
   real client actually transmits to the server):**

   | Sub-opcode | Payload | Trigger |
   |---|---|---|
   | 0x01 | `u16 room_id` `u32 token_lo` `u32 token_hi` `u16 const=256` | Sent immediately after the voice-relay socket confirms login (relay dispatcher `sub_EC69A0` case `4`/`1`) — echoes room_id+token back to the game server as a "I'm live on the relay" confirmation. |
   | 0x02 | `u16 echo=1` `u16 room_id` | Sent when the relay tells the client a *new member joined* and that member is the client itself (self-join ack, relay case `4`/`2`). |
   | 0x03 | *(2-byte header only)* | Sent conditionally from the `0x06` S2C handler (`sub_EC4B60`) — generic ack/ping. |
   | 0x05 | *(2-byte header only)* | **Correction to the previous pass's assumption**: this is sent BY the client (sets a local `byte_11146FA=1` "voice chat active" flag), it is not something the server sends. It is the best-supported candidate found in this pass for the client's "open voice chat" trigger — see Open items, since the exact request opcode that asks the server to create/join a room was not directly located. |
   | 0x07 | `u8 const=2` `u16 a` `u16 b` | Sent whenever the mic/speaker volume (COM `IAudioEndpointVolume`) changes locally; `a`/`b` are the user's saved mic/speaker volume settings (same values persisted to `Voice.ini`). |
   | 0x09 | *(2-byte header only)* | Sent conditionally after processing an S2C `0x06` update. |

2. **Voice relay socket** — a SEPARATE, UNENCRYPTED TCP connection to a
   dedicated voice server, confirmed directly in `sub_EC69A0` via its
   `0xAA55`/`0x55AA` frame-boundary checks (see above). This connection
   carries the actual audio stream, never routed through the main game
   server's encrypted packet pipe. How the client learns this connection's
   address is one of the two biggest open items from this pass — see below.

   Framing on this connection:

   ```text
   AA 55 | u16 len | payload (len bytes) | 55 AA
   ```

   Payload sub-opcodes — these are the client's INBOUND cases (`sub_EC69A0`,
   i.e. what the relay must send), still mostly at the doc's original
   confidence level since this pass focused its RE budget on the
   higher-value game-socket dispatcher above. Confirmed: this dispatcher
   only has cases `{1, 4, 5, 6}` (anything else is silently ignored by the
   client), and case `4` has a nested `{1, 2}` switch on a second byte:

   | Sub-opcode | Purpose |
   |---|---|
   | 1 | Status ack: `u16 status` (must be `1`) `+ dword_11155E4 = <next field>`. Shown as a toast via string id `0xAC45`. |
   | 4 / 1 | Login success: `u16 room_id` `u64 token` `+ name string + 2 more fields` — same room_id/token globals as game-socket `0x02`, this is the relay independently confirming them. Fields beyond token were not fully traced. |
   | 4 / 2 | New member joined (relay-side): `u16 result` then, if `1`: `u16 member_id` `+ string` — when `member_id` equals the client's own id, it replies on the GAME socket with C2S `0x02` (self-join ack). |
   | 5 | **Voice data** — copies `len` bytes into a per-member playback buffer, found by walking a linked list keyed by a member handle, then calls an Opus-decode helper (`sub_EC20B0`). This is the actual audio-receive path; our relay only needs to forward bytes opaquely, never decode them itself. |
   | 6 | Mic/speaker state — looks up a member node in the same linked list and stores two values (`v81[5]`/`v81[6]`) onto it, i.e. a per-member state update, forwarded into the roster UI. |

   This dispatcher's OUTBOUND side (what the real client sends TO the relay
   — i.e. what our relay's login/room/voice-data/mic-state/keepalive
   handlers must accept) was not located in this pass (it requires finding
   where the client builds the AA55-framed send buffer, not just where it
   parses one) — the sub-opcode numbers and rough shapes below remain the
   previous pass's assumption, now additionally cross-checked against the
   confirmed 64-bit token width above:

   | Sub-opcode | Purpose |
   |---|---|
   | 0x01 | login — assumed payload `u64 token` (matches the game socket's confirmed 64-bit token field) |
   | 0x04 | room create/join |
   | 0x05 | voice data |
   | 0x06 | mic/speaker state |
   | 0x07 | keepalive |

   Voice data (sub-opcode `0x05`) payload is a sequence of Opus frame
   records:

   ```text
   u32 opus_frame_len
   u8  opus_frame[opus_frame_len]
   ```

   repeated for however many frames are batched in one send.

## Audio codec parameters

- Opus, 48 kHz, mono, `RESTRICTED_LOWDELAY` application mode.
- Encoder created via `opus_encoder_create(48000, 1, 0x803)` — `0x803` is
  `OPUS_APPLICATION_RESTRICTED_LOWDELAY`.
- Bitrate set via `OPUS_SET_BITRATE(48000)` (48 kbps CBR-ish target).
- 10 ms frames — 480 samples per `opus_encode` call at 48 kHz.

## Server-side implementation (this pass)

Implemented in `crates/ko-game`:

1. `handler/voice_chat.rs` — real inbound `0xB5` handler (wired in
   `handler/mod.rs`, replacing the old outbound-only stub). On a join
   request it looks up the sender's current party (`WorldState::get_party_id`,
   `crates/ko-game/src/world/social.rs`) and uses the **party id directly as
   the voice room id** — voice chat is a party feature, so no separate room
   id space was introduced. It issues a random 64-bit token
   (`issue_token`/`validate_token`, matching the confirmed 64-bit token
   field width), replies with the confirmed `0x02` room-result format, then
   sends an `0x05` roster entry for the requester's own membership whose
   trailing two `u32` fields carry the voice relay's address (see the
   assumption called out below).
2. `systems/voice_relay.rs` — a brand-new, standalone TCP listener (started
   from `GameServer::run` in `server.rs` alongside the other background
   systems, bind address from `VOICE_RELAY_BIND_ADDR`), speaking the
   `AA55 | u16 len | payload | 55AA` framing. Validates each connection's
   login token against `handler::voice_chat::validate_token`, then relays
   voice-data (`0x05`) and mic-state (`0x06`) bytes opaquely to every OTHER
   connection currently in the same room — it never decodes Opus, matching
   the client's own relay (which only decodes on the *receiving* end).
3. Token/session bridging: `handler::voice_chat::{issue_token, validate_token}`
   is the bridge — the relay never touches the encrypted game-socket
   session machinery, it only needs the token to look up a room id.

## Open items / judgment calls for before this ships

- **The C2S "create/join my voice room" trigger opcode is an assumption**
  (`0x04`, see the game-socket C2S table above) — the real trigger was not
  directly located in this pass; only its likely *response* (`0x02`) was
  confirmed from the client's own dispatcher. If live packet capture later
  shows the client actually sends a different sub-opcode (or sends `0x05`
  with no payload, per the correction above), `SUB_JOIN_REQUEST` in
  `voice_chat.rs` is the one constant to change.
- **The relay-address delivery mechanism is the least confident part of this
  pass.** No sub-opcode in the confirmed `0xB5` S2C dispatch unambiguously
  reads an IP/port — the implementation's assumption is that the `0x05`
  roster broadcast's trailing two `u32` fields double as the relay address
  for the joining player's own entry. This needs a live client (or a packet
  capture) to confirm before relying on it; if wrong, voice chat will issue
  a room+token successfully but the client just won't know where to open
  its relay socket.
- **The voice relay's own OUTBOUND-from-client wire format (what the client
  sends INTO sub_EC69A0's sibling send path) was not traced in this pass** —
  implemented per the original assumption (login carries the 64-bit token,
  room/voice-data/mic-state/keepalive as previously documented).
- Byte layout for game-socket sub-opcodes `0x03`'s two extra fields (shown
  only when `result==1`) and `0x07`'s per-member `u16`/`u16` pair semantics
  (likely volume + pan, or mic/speaker mute flags — not disambiguated)
  remain unconfirmed; not required for the minimum room-join flow
  implemented here.
