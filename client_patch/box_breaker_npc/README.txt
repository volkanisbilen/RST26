BOX BREAKER NPC CLIENT PATCH (Moradon, proto 31514)

The Box Breaker NPC (Quests/31514_BoxBreaker.lua) is a normal Lua SelectMsg dialog, so its
button / header texts come from the client tables. Copy into the client Data directory (back up
the old files first):

  Quest_Menu_us.tbl   buttons: ids 62101-62110 (fixed UI) + 62201-62236 (one per box name)
  Quest_Talk_us.tbl   dialog headers: ids 62101-62109

Optional (cosmetic):
  optional_npc_name\NPC_us.tbl, NPC_tk.tbl, NPC.tbl
      the NPC already renders without any NPC table change (proto 31514 is in the client as
      "[Monster Strategist]", model 30900, unused). These three files only rename it to
      "[Box Breaker]". NOTE: they were built from the same Data directory as the two files
      above; do not mix them with the older client_patch\funclass_npc NPC tables.

Built from "D:\ai\JstKO Client\Data" (Quest_Menu_us.tbl of Oct 13, Quest_Talk_us.tbl of Oct 9,
NPC tables of Oct 11). If another change to one of these tables lands first, rebuild from the
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
