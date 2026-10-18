# KnightOnline 2625 protocol audit

## Source identity

- `KnightOnline.exe`: 20,094,976 bytes
- SHA-256: `9C2A5309F5DC118D938D5CDB7753E272D4D5169659A71F0DB49043B6E5A285E4`
- `KnightOnline.exe.c`: 58,702,054 bytes
- SHA-256: `86230B5FAD9C288B83AC214B46CB1AD53AB3C36FE91A7C875E8BD85DE7D238F0`
- PE image base: `0x00400000`

The binaries are evidence inputs and are intentionally not committed to Git.

## Confirmed 2625 addresses

| Purpose | 2615 VA | 2625 VA | Result |
|---|---:|---:|---|
| Required GameServer version function | `0x007A87A0` | `0x007AE420` | v2625 returns `2625` |
| Main GameServer packet dispatcher | `0x007AF000` | `0x007B4CE0` | moved |
| Main opcode lookup table | `0x007AF568` | `0x007B5258` | 253 bytes, byte-identical |
| Version response parser | `0x007AF730` | `0x007B5420` | moved; same field order |
| Last received opcode global | `0x01105850` | `0x01114594` | moved |
| Main client/game object | `0x01105834` | `0x01114574` | moved |
| Network/send object | `0x01105914` | `0x01114654` | moved |
| Security cookie/global | `0x010FB680` | `0x010FB680` | unchanged |

## Version packet

The v2625 parser at `sub_7B5420` consumes:

```text
u8  cloak_catalog_selector  (0 = Cloak.tbl, nonzero = Cloak_PVP.tbl)
u16 game_version            (must equal 2625)
u8  aes_key_length          (16)
u8  aes_key[16]
u8  trailer                 (0)
```

The exact server payload is therefore:

```text
00 41 0A 10 <16-byte AES key> 00
```

When the parsed version is not 2625, the client writes `Files=2624` to
`Server.ini`. This is the source of the apparent launcher rollback/update loop
when the launcher and GameServer version domains disagree.

## Opcode audit

The 253-byte compact lookup table used by the main GameServer dispatcher is
byte-for-byte identical between the verified 2615 unpacked executable and the
supplied 2625 executable. The special routes remain:

```text
0x01 -> group 0
0x04 -> group 1
0x2B -> group 2 (version)
0x46 -> group 3
0x72 -> group 4
0x89 -> group 5
0x9F -> group 6
0xA0 -> group 7
other -> group 9
```

Consequently, changing the Rust `Opcode` enum wholesale would be incorrect.
Only version-gated payloads or individually proven sub-opcode changes should be
changed.

## Genie compatibility

The v2625 client still uses Genie marker `0x0197`:

- load/request: `[97 01 01 ...]`, function near `0x00B1EEC0`
- save: `[97 01 03 ...]`, `sub_B1F5A0`
- stop: `[97 01 05 ...]`, function near `0x00B1F7E0`

The save routine still enumerates 32 skill slots. Existing Rust Genie option
serialization therefore remains enabled for v2625; it is not replaced with a
guessed fixed structure.

## WIZ_TARGET_HP field change

The v2625 target HP parser moved from the v2615 address `0x00817F60` to
`sub_81DF70` at `0x0081DF70`. Its six fixed fields are:

```text
u32 target_id
u8  echo
u32 max_hp
u32 current_hp
i32 hp_change
i32 score_delta
u8  reserved
```

At `0x0081E537` the client multiplies the first trailing dword (`hp_change`)
by ten for its damage/heal message path. The second trailing dword is passed to
the Ronark score accumulator through `sub_74A200`. Sending an attacker ID in
the first dword, as the old server did, prevents v2625 from showing the actual
damage number. All Rust combat paths now send the signed HP change in the first
dword and retain the score delta in the second.

## Server changes

- Normal GameServer version mode now reads `server_settings.game_version`.
- The hard fallback is 2625.
- LoginServer and GameServer DB versions are migrated together to 2625.
- Historical 2599/2602 wire modes remain available only when explicitly
  selected with `KO_VERSION_MODE` for diagnostics.

## Deployment guard

Do not apply the 2625 migration to the live 2615 service until the 2625 client
patch and launcher files are ready. A live version switch intentionally rejects
the old client.
