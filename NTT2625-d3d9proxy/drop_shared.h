#pragma once
// shared by drop_panel.cpp (server reply) and drop_button.cpp (compact list next to the target bar)
#include <windows.h>
#include <string>
#include <vector>

struct DropMiniEntry { UINT32 item; UINT16 pct; UINT32 icon; std::string name; };   // pct in 1/10000

void Drop_RequestMini(UINT32 npcId);                                                            // drop_panel.cpp: target's drops -> DropMini_Show
void ItemTip_Show(UINT32 item, RECT anchor);                                         // item_tooltip.cpp
void ItemTip_Hide();
// drop block list (drop_panel.cpp): at most 10 items per character that do not drop for the player
bool DropBlock_Has(UINT32 item);
int  DropBlock_Count();
int  DropBlock_Toggle(const DropMiniEntry& e);                                       // 1 blocked, 0 allowed again, -1 list full
void DropMini_Show(const std::string& name, unsigned level, const std::vector<DropMiniEntry>& drops);   // drop_button.cpp
