BOX BREAKER NPC CLIENT PATCH (Moradon, proto 31514)

The Box Breaker NPC (Quests/31514_BoxBreaker.lua) is a normal Lua SelectMsg dialog, so its
button / header texts come from the client tables. Copy into the client Data directory (back up
the old files first):

  Quest_Menu_us.tbl   buttons: ids 62101-62110 (fixed UI) + 62201-62286 (one per box name, 86 boxes; duplicate names are numbered 'Accessory Box (1)', '(2)', ...)
  Quest_Talk_us.tbl   dialog headers: ids 62101-62109

Optional (cosmetic):
  optional_npc_name\NPC_us.tbl, NPC_tk.tbl, NPC.tbl
      the NPC already renders without any NPC table change (proto 31514 is in the client as
      "[Monster Strategist]", model 30900, unused). These three files only rename it to
      "[Box Breaker]". NOTE: they were built from the same Data directory as the two files
      above; do not mix them with the older client_patch\funclass_npc NPC tables.

IMPORTANT: the Quest_Menu_us.tbl / Quest_Talk_us.tbl that were in "D:\ai\JstKO Client\Data" before this
rebuild had been written by the old patch tool, which re-encoded every string as EUC-KR (non-Korean
text became "&#NNNN;" entities, +15 % size). The tool now edits the tables byte-preserving
(crates/ko-tbl-import/src/raw_tbl.rs) and checks on every load that the untouched table re-encodes
to the exact source bytes. These files were therefore built from the clean originals
Data\Quest_Menu_us.pre_patch2.bak / Quest_Talk_us.pre_patch2.bak (= the Data files minus our rows);
replace the installed ones. Rebuilt after the box rewards migration
(20261018320000) made 50 more boxes breakable (86 box buttons in total, ids 62201-62286; free id
block 62201-62299, button text limit 100 chars). The NPC tables in optional_npc_name were rebuilt
in the same run. If another change to one of these tables lands first, rebuild from the
newer file instead of copying these:

  cargo run -p ko-tbl-import --bin patch_box_breaker -- "<client>\Data" <out dir> [--lang tr] [--rename-npc]

  --lang tr       plain-ASCII Turkish texts instead of English
  --rename-npc    also writes the three NPC tables

Existing rows with the same ids are replaced, new rows are appended, nothing else changes.

IMPORTANT (client integrity): Data\*.tbl files are hash-checked by the DLL. After copying them run
NTT2625-d3d9proxy\gen_manifest.py, rebuild/redistribute the DLL and run package_patch.py (+ upload),
otherwise players with the new files are kicked by their own DLL.

Adding a new box later: add a line `[<item id>] = <next free 622xx>, -- <item name>` to the
BOX_TEXT table of Quests/31514_BoxBreaker.lua and rebuild this patch (boxes missing from the table
still work, they just show the generic "Other box (name in chat)" button).
