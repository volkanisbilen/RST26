CLAN PREMIUM MENU LABEL PATCH

Copy Quest_Menu_us.tbl into the client's Data directory (back up the old file first).
It adds text 60010 "Clan Premium", which Kaira's premium menu now uses instead of
"Special Defense Scroll" (text 45241 stays unchanged for the other NPCs).
Rebuild with:
  cargo run -p ko-tbl-import --bin patch_menu_text -- <client>/Data/Quest_Menu_us.tbl Quest_Menu_us.tbl 60010 "Clan Premium"
