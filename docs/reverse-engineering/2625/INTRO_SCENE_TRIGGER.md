# Pre-login intro cinematic trigger (2625 client) — RE notes

Source binary: `docs/reverse-engineering/2625/client_src/KnightOnline.exe`
(disassembly dump alongside it as `KnightOnline.exe.c`, Hex-Rays decompiled C
with embedded `"SourceFile.cpp", <line>` debug strings — same constraints and
methodology as `VOICE_CHAT_PROTOCOL.md`/`EQUIPMENT_VIEW_AND_EMBLEM_RENDERING.md`).
Tooling: `python` (not `python3`) with `capstone` 5.0.9 / `pefile` 2024.8.26 —
no IDA/Ghidra/objdump/radare2. This pass additionally used `pefile` directly
to read raw vtable arrays out of `.rdata` (file offset == VA − `0x400000`
for every section that matters here, confirmed via `pefile` section headers),
since the decompiled dump only ever shows one `ClassName::`vftable'` pointer
assignment per slot, never the vtable's full contents as a literal array.

## Headline finding — corrects this task's own starting premise

The task briefing (based on an earlier pass this session) assumed the
pre-login intro is driven by `docs/data/movie_scene.tbl` /
`.n3scene`/`.n3scenex` Nebula3 scene files. **This pass found strong,
directly-read evidence that this is wrong**, or at least not the whole
picture:

- `movie_scene.tbl` is real and is loaded at startup into a genuine client
  class, `CN3TableBase<__TABLE_MOVIE_SCENE>` (confirmed via the surviving
  C++ template-instantiation symbol — see below). But the **only** code
  path this pass could find that ever selects a row from it
  (`sub_7879F0`, `SelectMovieScene(row_id)`) is called **exclusively** from
  a byte-code/script interpreter (`sub_7DC620`) that is itself called from
  exactly one place: **the main incoming (server) packet dispatcher** — the
  same dispatcher audited in `KNIGHTONLINE_2625_PROTOCOL_AUDIT.md`. That
  means this whole pipeline (`movie_scene.tbl` →
  `CN3TableBase<__TABLE_MOVIE_SCENE>` → `CCurrentMovieSceneData` →
  `CN3Scene`) is, as far as this pass could determine, a **server-scripted,
  in-world 3D camera-cutscene system** (consistent with the per-row camera
  files it also touches — `Zones\Camera\Zone_%d.n3camera` — and with several
  row names in the table looking like real KO zone names, e.g.
  `st_s_pulreton_3th_2010a01`, `Pulreton` being a real zone). It requires an
  active, logged-in game-server connection to ever fire. It cannot be what
  plays before any login has happened.
- Separately, and **independently of `movie_scene.tbl` entirely**, this pass
  found a second, much simpler system: a dedicated `CGameProcMovie` client
  game-state that plays a single **hardcoded `.wmv` file**
  (`KnightMovie\nuslamF.wmv`) through a completely standard **DirectShow**
  pipeline (`CoCreateInstance(CLSID_FilterGraph)` →
  `IGraphBuilder::RenderFile()` → `QueryInterface(IMediaControl /
  IMediaEventEx / IMediaSeeking / IVideoWindow / IBasicVideo)`), confirmed
  unambiguously via literal error-trace strings naming every one of those
  COM interfaces. **This is almost certainly the real "pre-login intro
  cinematic"** the task is asking about — it is a pre-rendered video, not a
  3D scene graph, which also matches the user's own N3Viewer observation
  that `scene_stage04.n3scene` renders as an empty placeholder scene (because
  it's the wrong system to be looking at).

Both systems are real, both are described below with addresses, but **the
patch target for "play a custom video instead" should be `CGameProcMovie` /
`sub_866630`, not `CN3Scene`/`movie_scene.tbl`.**

## Confirmed: the `movie_scene.tbl` pipeline (in-world camera cutscenes, NOT pre-login)

- `docs/data/movie_scene.tbl` dumped via `target/debug/tbl_dump.exe` has 10
  rows (`row=0..9`), columns
  `[UnsignedInt id, String n3scene, String n3scenex, UnsignedInt, UnsignedByte×4, SignedShort c8, UnsignedByte c9, SignedShort c10, SignedShort c11, SignedShort c12, UnsignedByte c13]`.
  Rows 0 and 6 have empty path strings (placeholder/no-scene rows). Row names
  (`meme_1st_1p_a01/a02/a567`, `bms_ung_1th`, `pury2st`,
  `st_s_pulreton_3th_2010a01`, `m_pulreton_3th_end01`, `korhappy_a01`) read
  like a mix of tutorial/story cutscene names and real zone names.
- **Startup load (High confidence, directly read)**: a single giant
  "load every data table" function, `sub_79D620` (starts at decompiled line
  827188), contains the literal call
  `sub_4DA510((_DWORD **)Dst, "Data\\movie_scene.tbl", 0x14u);` (dump line
  830523) immediately followed by construction
  (`sub_51ED90`, VA `0x51ED90`) and loading (`sub_51EE30`, VA `0x51EE30`) of
  a `CN3TableBase<__TABLE_MOVIE_SCENE>` instance, stored into the global
  **`dword_1114480`** (line 828545: `dword_1114480 = (int)v70;`).
  `CN3TableBase<__TABLE_MOVIE_SCENE>` is a real, surviving C++ template
  symbol (`void *CN3TableBase<__TABLE_MOVIE_SCENE>::\`vftable' = &sub_528AA0;`,
  dump line 42851; `sub_528AA0` = its destructor at VA `0x528AA0`).
  `sub_51EE30` is the generic table-file loader shared by every
  `CN3TableBase<...>` instantiation (confirmed via its internal error string
  `"N3TableBase - Can't open file(read) File Handle error (%s)"` and a
  second XignCode-anti-cheat-flavored error string a few lines later), so
  this is the real production loader, not a separate verify-only duplicate.
- **Row selection (High confidence)**: `sub_7879F0(unsigned int row_id)`
  (VA `0x7879F0`) is `SelectMovieScene(row_id)`. It stores `row_id` into
  `*(dword_118A004 + 4)`, looks the row up in the `dword_1114480` table via
  `sub_4DE300`, and copies the row's `SignedShort` fields at row offset
  `+64/+66/+68` and `UnsignedByte` field at `+70` — these line up exactly
  with the table's own columns 10/11/12/13 from the `tbl_dump` output above
  — into the "current movie/camera" object `dword_118A004` at offsets
  `+60/+64/+68/+72`.
- **Who calls `SelectMovieScene` (High confidence)**: exactly two call
  sites, both inside **one** function, `sub_7DC620` (VA `0x7DC620`, starts
  at decompiled line 868206). That function is a small byte-code
  interpreter (cursor-driven `switch` over a byte read from a buffer+length
  pair passed in as `a2`/`a3`), i.e. a server-sent "cutscene script".
  `sub_7DC620` itself has exactly **one** caller (dump line 947612,
  `sub_7DC620((_DWORD *)v357, (int)a3, a4);`), inside the client's main S2C
  packet dispatcher — the same dispatcher described in
  `KNIGHTONLINE_2625_PROTOCOL_AUDIT.md`. This requires an active server
  connection and therefore **cannot be the pre-login trigger**.
- **The camera/scene player this drives (`CCurrentMovieSceneData`, Medium-High
  confidence)**: real class, vtable at VA `0x100DB90`
  (`CCurrentMovieSceneData::\`vftable' = &sub_50D3F0`). The object pointed
  to by `dword_118A004` is driven once per frame by a small dispatcher
  (inside a much larger per-frame function, around dump line 952107) that
  switches on `sub_790140(...)` into `sub_7E29E0`/`sub_7E25F0`/`sub_7E2150`
  (cases 0/1/2). `sub_7E25F0` (VA `0x7E25F0`) loads a **zone camera** file
  by numeric id: `sub_4DE9A0(FileName, 0x103u, "Zones\\Camera\\Zone_%d.n3camera", v3 / 10); CreateFileA(FileName, ...)`
  — note this is a *camera* file, not the row's own `.n3scene`/`.n3scenex`
  path; this pass did not trace far enough to find the exact call site that
  opens the row's own `.n3scene`/`.n3scenex` files (see Open items). The
  same function also sends a C2S packet (opcode `0x55` = `WizSelectMsg` in
  this project's `opcode.rs`) traced to `"CurrentMovieSceneData.cpp", 118`
  when a camera segment finishes — i.e. the client tells the server when
  one of these in-world camera scripts completes.
- **`CN3Scene::Load` (High confidence, located but not yet proven called
  from the movie-scene pipeline)**: `CN3Scene` vtable at VA `0x1007AB8`
  (`sub_6A6110`/dtor, 7 further virtuals). Slot 3, `sub_6A54E0` (VA
  `0x6A54E0`, `char __thiscall CN3Scene::Load(this, HANDLE hFile)`), reads
  raw fields via `ReadFile` directly from an **already-open file HANDLE**
  (confirming `.n3scenex` is the binary/compiled companion actually parsed
  at runtime, while `.n3scene` is presumably the human-authored source the
  offline tool compiles from) and builds one ~0x404-byte node record per
  scene-graph entry — consistent with the Camera/Light/Shape/Character
  placeholder nodes the user saw in N3Viewer. A sibling/derived class
  sitting immediately after `CN3Scene`'s vtable in `.rdata` has its own,
  larger `Load(HANDLE)` override (`sub_6A64B0`, VA `0x6A64B0`). **This pass
  could not find any caller of either function** (virtual dispatch only;
  no `.n3scene`/`.n3scenex` string literals exist anywhere in the 58 MB
  dump, since the paths only ever come from the table at runtime) — so
  `CN3Scene::Load` is a real, located function, but its exact trigger path
  from the movie-scene table is still unconfirmed.

## Confirmed: the real pre-login intro video (`CGameProcMovie`, DirectShow `.wmv`)

- The client has a family of top-level "game procedure" states, all
  deriving from a common `CGameProcedure` base (vtable VA `0x1018854`, 11
  virtual slots; slots 2/3/4/5/6/10 are identical function addresses across
  every subclass — confirmed by dumping each subclass's vtable with
  `pefile` — i.e. genuinely inherited/unoverridden base methods; slots
  0/1/7/8/9 differ per class). Relevant subclasses and their vtable VAs,
  all confirmed the same way:
  `CGameProcLogo` (`0x1019024`), `CGameProcIntro` (`0x1018A58`),
  `CGameProcMovie` (`0x101AD54`), `CGameProcIntroChrSelect` (`0x1018FA0`),
  `CGameProcLanguageSelect` (`0x1018FE4`), `CGameProcNationSelect`
  (`0x101ADA0`), `CGameProcMalaysiaIntro` (`0x101A95C`),
  `CGameProcCharacterCreate` (`0x100EC20`), `CGameProcDuelLobby`
  (`0x100EC68`), `CGameProcNotice` (`0x101ADD0`), `CGameProcOption`
  (`0x101AE00`), `CGameProcMain` (`0x101A92C`).
- All of these are pre-allocated **once**, as process-lifetime singletons,
  inside one giant application-init function (decompiled lines
  ~841500–841650), each stored in its own global, e.g.
  `dword_1114644` = the `CGameProcIntro` singleton, `dword_11145F4` =
  the `CGameProcMovie` singleton, `dword_1114648` = the `CGameProcLogo`
  singleton, `dword_1114640` = the `CGameProcIntroChrSelect` (character
  select) singleton.
- **State-switch helper, `sub_7B5360`** (VA `0x7B5360`,
  `SwitchActiveState(CGameProcedure *next)`): swaps the single global
  "currently active" pointer `dword_1114650` (saving the previous one into
  `dword_111464C`). This pass found **exactly three** call sites for this
  helper, switching to `dword_1114640` (CharSelect), `dword_1114644`
  (Intro/login), and `dword_1114648` (Logo) respectively — i.e. direct,
  confirmed evidence of a **Logo → Intro → CharSelect** top-level flow
  (Medium-High confidence; other states are entered via other, inlined
  `dword_1114650 = X` assignments this pass did not individually trace, so
  this is the confirmed *skeleton*, not a complete state diagram).
- **The video itself — High confidence, fully traced**: `CGameProcMovie`'s
  per-frame method, slot 8 of its vtable, is `sub_866630` (VA `0x866630`).
  On its first call it does:
  ```c
  ShowWindow(hwnd, 1); SetForegroundWindow(hwnd);
  GetCurrentDirectoryA(Buffer, 0x104);
  lstrcat(Buffer, "\\KnightMovie\\nuslamF.wmv");
  sub_8661A0(this, Buffer);   // opens + starts the DirectShow graph
  ```
  and then, every frame after that, polls for playback completion (treats
  event code `1` from the media-event interface as "done") **or** a
  separate flag `*((_DWORD *)dword_111465C + 158)` (not independently
  confirmed, but shaped like a "user pressed a skip key/clicked" flag) —
  either condition calls the finish/hand-off function `sub_865B40`.
  `sub_8661A0` (VA `0x8661A0`) is textbook DirectShow bootstrap, confirmed
  by its own literal error strings: `CoCreateInstance(&rclsid, ...)` (the
  surrounding trace string literally says
  `"...CoCreateInstance(CLSID_FilterGraph, NULL, CLSCTX_INPROC_SERVER, IID_IGraphBuilder, (void **)&pGB)"`),
  then `IGraphBuilder::RenderFile(path)`, then four `QueryInterface` calls
  traced as `IID_IMediaControl`, `IID_IMediaEventEx`, `IID_IMediaSeeking`,
  `IID_IVideoWindow`, `IID_IBasicVideo` in turn.
  `"\\KnightMovie\\nuslamF.wmv"` is the **only** `.wmv` path string used by
  this call site — no table, no per-region branch at this specific site (two
  other `.wmv` paths, `knight_Battle_Indun.wmv`/`knight_Battle_Indun_H.wmv`,
  exist elsewhere in the binary for an unrelated in-dungeon intro-video
  feature, confirming `KnightMovie\` + DirectShow `.wmv` is this client's
  general-purpose "play a real video file" mechanism, reused by at least
  two different features).
- **End-of-intro hand-off — High confidence**: `sub_865B40` (VA `0x865B40`,
  called from all three of `sub_866630`'s exit conditions) releases the
  DirectShow COM objects (direct vtable calls at offsets `+36`/`+76`/`+116`/
  `+8` on the stored interface pointers), hides and invalidates the movie
  window, writes the registry DWORD `ShowIntroMovie = 1` (see below), and
  performs the same `dword_111464C = dword_1114650; dword_1114650 = <prev>;`
  state-swap pattern used by `sub_7B5360` — i.e. an explicit,
  directly-read, code-driven "intro finished → switch to the next game
  state" call, not a bare timer. Given the confirmed Logo→Intro→CharSelect
  skeleton above and that `CGameProcMovie` is a distinct state from
  `CGameProcIntro`, the next state landed on here is almost certainly the
  `CGameProcIntro` (login/server-select) singleton, though this pass did not
  capture the exact pointer value at that specific call to prove it
  (Medium confidence on this specific last link).

## The `ShowIntroMovie`/`ShowIntroText` registry values

- `sub_7B0BD0` (VA `0x7B0BD0`) and `sub_7B0FD0` (VA `0x7B0FD0`) are, by
  direct code reading, plain `RegQueryValueExA`/`RegSetValueExA` wrappers
  against `HKEY_LOCAL_MACHINE\Software\KnightOnlineUS\ServerSelect`
  (the key path is built by `sub_7AF940`, VA `0x7AF940`, which literally
  contains the string `"Software\\KnightOnlineUS\\ServerSelect"`).
- Three call sites: `sub_7B0FD0("SelectServer", ...)`,
  `sub_7B0BD0("SelectServer", ...)` (an actual persisted setting, last
  server picked — unrelated to the intro), `sub_7B0BD0("ShowIntroText", ...)`
  (read once), `sub_7B0FD0("ShowIntroText", ...)` (write once), and
  `sub_7B0FD0("ShowIntroMovie", ...)` (write once, inside `sub_865B40`
  above) — **`ShowIntroMovie` is never read anywhere in this binary**.
- **Conclusion (Medium-High confidence)**: as far as `KnightOnline.exe`
  itself is concerned, the `KnightMovie\nuslamF.wmv` splash plays
  **unconditionally on every launch** that reaches the `CGameProcMovie`
  state — the per-run "already started" guard at the top of `sub_866630`
  (object field `this+352`) only prevents re-starting it multiple times
  *within* the same run, it is not persisted. The `ShowIntroMovie=1`
  registry write therefore looks like either (a) a flag meant to be read by
  a *different* process (a launcher/patcher, not in scope for this pass —
  see `docs/reverse-engineering/2625/client_src/` only contains the game
  EXE) to decide whether to re-show something on a later run, or (b)
  vestigial/telemetry from a feature that used to gate this and no longer
  does in this client build. Not resolved this pass.

## Feasibility assessment for a future patch

**Do not target `CN3Scene`/`movie_scene.tbl` for this** — that system is,
per the evidence above, the in-world server-scripted camera-cutscene
system, not the pre-login screen. The two realistic options both target
`CGameProcMovie`:

### Option A — "skip the intro entirely" (**Low risk, well understood**)

Two independent ways to do this, in increasing order of invasiveness:
1. **No code patch at all**: replace/delete `KnightMovie\nuslamF.wmv` (or
   substitute a near-instant near-silent clip). `IGraphBuilder::RenderFile`
   failing or completing near-instantly both already flow through the same
   confirmed `sub_865B40` hand-off path, so this should degrade gracefully
   — **not independently tested this pass**, flagged as an assumption, but
   a very low-risk one given `sub_8661A0`'s own code already branches to
   `sub_865B40` on failure (dump line 964338:
   `if (sub_8661A0(a1, path) < 0) sub_865B40(a1, &savedregs);`).
2. **Tiny code patch**: at `sub_866630` (VA `0x866630`), short-circuit the
   `if (!already_started) { ...; sub_8661A0(...); ... }` block to call
   `sub_865B40` immediately instead of starting playback — a single-branch
   patch comparable in size to the smallest entries in this project's
   existing `NTT2625-d3d9proxy/client_patches.cpp` table.

### Option B — "play a custom video instead" (**Low-to-Medium risk, high feasibility**)

The client already has a complete, real, working DirectShow playback path
(`sub_8661A0`) used for exactly this purpose elsewhere in the same binary
(the dungeon-intro `.wmv`s). This means a custom-video patch does **not**
need to introduce any new video API, codec, or engine integration — it only
needs to change **what file gets opened**:
- Simplest form: patch the literal string `"\\KnightMovie\\nuslamF.wmv"`
  embedded in `sub_866630` (or intercept/redirect the `lstrcat`/`lstrcpy`
  building `Buffer` just before the `sub_8661A0` call) to point at a
  different filename in the same folder. Author the replacement as a real
  `.wmv`/ASF file to guarantee no missing-codec failures on end-user
  machines (DirectShow's `RenderFile` auto-picks decoder/renderer filters
  from what's registered on the host OS; WMV/ASF ships with Windows Media
  components already present, which is almost certainly why the original
  developers picked that container).
- A more flexible variant (configurable path/filename, or multiple videos)
  would need a slightly larger detour patch around the `lstrcat` call, but
  still does not need to touch `sub_8661A0`'s DirectShow logic itself,
  which can be left completely untouched.
- Risk is "Low-to-Medium" rather than "Low" only because: (a) this pass did
  not independently verify the `*((_DWORD*)dword_111465C + 158)` skip-flag
  semantics (if that is a hotkey wired to assume a specific video length or
  aspect ratio somewhere else, a very differently-shaped replacement video
  could interact with it in an untested way), and (b) `IVideoWindow`/
  `IBasicVideo` sizing calls inside `sub_8661A0` (not fully read this pass
  past the `QueryInterface` chain) may hardcode an aspect ratio or window
  size assumption that a differently-proportioned replacement video should
  still match for best results, even though it's unlikely to hard-fail.

## Confidence summary

| Claim | Confidence | Basis |
|---|---|---|
| `movie_scene.tbl` loads into a real `CN3TableBase<__TABLE_MOVIE_SCENE>` singleton at startup | **High** | Directly read `sub_79D620` call site + surviving template vtable symbol |
| `SelectMovieScene`/`sub_7879F0` row-select is only reachable from a server-packet-driven script interpreter (`sub_7DC620`) | **High** | Directly read both functions and their single call sites each |
| → therefore the `movie_scene.tbl`/`CN3Scene` pipeline is an in-world, server-scripted camera-cutscene system, not the pre-login trigger | **Medium-High** | Strong structural/behavioral evidence (requires a live connection); not confirmed by watching real traffic |
| `CN3Scene::Load(HANDLE)` = `sub_6A54E0`, reads an already-open file handle, builds per-node records | **High** | Directly read function body |
| Exact caller of `CN3Scene::Load` (i.e. what actually opens `.n3scene`/`.n3scenex` and invokes it) | **Not found** | Virtual dispatch only, no caller located this pass |
| `CGameProcMovie::Run` (`sub_866630`) hardcodes `KnightMovie\nuslamF.wmv` and plays it via DirectShow (`sub_8661A0`) | **High** | Directly read, corroborated by literal COM-interface error-trace strings |
| `sub_865B40` is the real "intro finished → next state" hand-off, with both natural-end and skip-flag paths | **High** (hand-off mechanism) / **Medium** (skip-flag's exact trigger) | Directly read; skip-flag setter not traced |
| The next state landed on after the intro video is `CGameProcIntro` (login screen) specifically | **Medium** | Inferred from the confirmed Logo→Intro→CharSelect skeleton, not captured at this exact call |
| Top-level flow is Logo → Intro(login) → CharSelect | **Medium-High** | Only 3 call sites total for the one confirmed state-switch helper `sub_7B5360` |
| `ShowIntroMovie` registry value is never read by this EXE (plays every launch regardless) | **Medium-High** | Exhaustive grep for all uses of the value name in this binary; absence is the evidence |
| Patch option A (skip) is low-risk | **High** | Existing failure path already reuses the same hand-off function |
| Patch option B (custom video) is low-to-medium risk, high feasibility | **Medium-High** | Real existing DirectShow path confirmed; a couple of downstream details (`IVideoWindow`/`IBasicVideo` sizing, the skip-flag) not fully read |

## Open items / what a follow-up pass or live-debugger session should do first

1. **Breakpoint `CreateFileA`/`sub_8661A0` at real client launch** (the
   single most valuable next step): confirms, with a live process, whether
   `nuslamF.wmv` is really the file opened for the *first-launch pre-login*
   intro specifically (as opposed to some other trigger this pass didn't
   find for region-specific builds — recall `nuslamF` sits in `.text`
   immediately next to `CGameProcMalaysiaIntro`/`CUIMalaysiaIntro`/
   `CUINationIntro` symbols, and this binary clearly supports multiple
   national/regional SKUs at runtime; this pass found no second `.wmv`
   string for this specific call site, but did not rule out a
   region-specific *different* trigger function entirely).
2. **Read `sub_8661A0` past the five `QueryInterface` calls** (this pass
   stopped there) — it almost certainly continues into
   `IVideoWindow::put_WindowStyle` / `SetWindowPosition` /
   `IMediaControl::Run` calls (partially glimpsed in `sub_866630`'s error
   strings), which would confirm or rule out the "custom video" patch's
   one remaining concrete risk (hardcoded sizing assumptions).
3. **Determine what sets `*((_DWORD*)dword_111465C + 158)`** (the
   suspected skip flag) — likely a keyboard/mouse input flag checked
   elsewhere in the main input-handling code; not traced this pass.
4. **Determine who reads `ShowIntroMovie`** (if anyone) — would need the
   Launcher.exe/patcher binary (referenced in this project's
   `launcher-patch-system` memory note) rather than `KnightOnline.exe`,
   since this pass found no read of that value anywhere in this EXE.
5. **Trace `CN3Scene::Load`'s real caller** for completeness/to fully close
   out the `movie_scene.tbl` side investigation, even though it is now
   believed unrelated to the pre-login screen — needed only if a future
   task wants to also touch the in-world cutscene system.
6. None of the above blocks Option A or Option B above from being
   implemented with the addresses already confirmed in this document; they
   would mainly increase confidence and reduce the residual risk noted for
   Option B.
