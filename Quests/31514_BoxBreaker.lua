-- [Box Breaker] Moradon (proto 31514)
-- Breaks the Chaotic Generator "boxes" (item.effect2 = 251) without the generator window:
-- same items, same reward tables (item_exchange random_flag 1..3), same weights, no gold.
-- Server side: crates/ko-game/src/world/box_breaker.rs (bindings GetBreakableBoxes, BoxPick,
-- BoxBreak, BoxBreakAll, SendChatLine). Client texts: client_patch/box_breaker_npc (Quest_Menu_us
-- / Quest_Talk_us, text ids 62101.. and 62201..). The ids in BOX_TEXT must stay in sync with that
-- patch (the patch builder reads the table below).
--
-- Menu events:
--   100          NPC click -> box list, page 0
--   200..299     box list, page EVENT-200
--   >= 100000000 a box was picked (EVENT = the box item id)
--   410/420/430  break 1 / 10 / all of the picked kind
--   500          "break everything" confirmation, 510 = confirmed

local NPC = 31514;
local PAGE_SIZE = 8;        -- box buttons per page (+ break-all, prev, next, close = 12 buttons)

-- Quest_Menu_us button texts
local M_CLOSE      = 27;    -- existing client text "Close"
local M_BREAK1     = 62101;
local M_BREAK10    = 62102;
local M_BREAKTYPE  = 62103;
local M_BREAKALL   = 62104;
local M_PREV       = 62105;
local M_NEXT       = 62106;
local M_LIST       = 62107;
local M_OTHER      = 62108;
local M_YES_ALL    = 62109;
local M_NO_ALL     = 62110;

-- Quest_Talk_us header texts
local T_LIST       = 62101;
local T_NONE       = 62102;
local T_PICK       = 62103;
local T_CONFIRM    = 62104;
local T_DONE       = 62105;
local T_CAPPED     = 62106;
local T_FULL       = 62107;
local T_HEAVY      = 62108;
local T_FAIL       = 62109;

-- item id -> Quest_Menu_us text id with the item's name (one per breakable box type)
local BOX_TEXT = {
	[379154000] = 62201, -- Red Treasure Chest
	[379155000] = 62202, -- Green Treasure Chest
	[379156000] = 62203, -- Blue Treasure Chest
	[389160000] = 62204, -- Fragment of Arrogance
	[389161000] = 62205, -- Fragment of Gluttony
	[389162000] = 62206, -- Fragment of Rage
	[389163000] = 62207, -- Fragment of Sloth
	[389164000] = 62208, -- Fragment of Lechery
	[389165000] = 62209, -- Fragment of Jealousy
	[389166000] = 62210, -- Fragment of Avarice
	[389196000] = 62211, -- Silvery Gem
	[389197000] = 62212, -- Red Gem
	[389198000] = 62213, -- Yellow Gem
	[389199000] = 62214, -- Blue Gem
	[389201000] = 62215, -- Green Gem
	[389205000] = 62216, -- Black Gem
	[389556000] = 62217, -- 70 Quest - Dark Fragment Chest
	[507184000] = 62218, -- Holy Armor - Legend Dark Star Wapon Box
	[507194000] = 62219, -- Krowaz Armor - Weapon Box
	[507204000] = 62220, -- Legend Dragon Box
	[507284000] = 62221, -- Black - Red Chitin - Weapon Box
	[508194000] = 62222, -- Accessory Box
	[509194000] = 62223, -- Exp Item - Chitin Shell Box
	[810341000] = 62224, -- Dragon Box
	[810596000] = 62225, -- Superior Draki Supply Box
	[810678000] = 62226, -- Old Draki Supply Box
	[810943000] = 62227, -- Dark Fragment of Pride
	[810944000] = 62228, -- Dark Fragment of Gluttony
	[810948000] = 62229, -- Dark Fragment of Wrath
	[810949000] = 62230, -- Dark Fragment of Sloth
	[810950000] = 62231, -- Dark Fragment of Lust
	[810951000] = 62232, -- Dark Fragment of Envy
	[810952000] = 62233, -- Dark Fragment of Greed
	[811053000] = 62234, -- Akara Accessory Box
	[811137000] = 62235, -- Fortified Sterling Silver Gemstone
	[999915000] = 62236, -- Black Lion Weapon Box
};

-- Chat helper: one line in the player's chat window (ASCII only).
local function say(text)
	SendChatLine(UID, text);
end

-- Lists "[n] Name xCount" entries of the page in the chat, four per line.
local function say_page(boxes, first, last, page, pages)
	local line = "Kutu Kirici / Box Breaker " .. (page + 1) .. "/" .. pages .. ": ";
	local n = 0;
	for i = first, last do
		local b = boxes[i];
		line = line .. b.name .. " x" .. b.count;
		n = n + 1;
		if i < last then
			if n % 4 == 0 then
				say(line);
				line = "  ";
			else
				line = line .. " | ";
			end
		end
	end
	say(line);
end

local function show_list(page)
	local boxes = GetBreakableBoxes(UID);
	local total = #boxes;
	if total == 0 then
		SelectMsg(UID, 2, -1, T_NONE, NPC, M_CLOSE, -1);
		return;
	end
	local pages = math.floor((total + PAGE_SIZE - 1) / PAGE_SIZE);
	if page < 0 then page = 0; end
	if page > pages - 1 then page = pages - 1; end
	local first = page * PAGE_SIZE + 1;
	local last = math.min(total, first + PAGE_SIZE - 1);
	say_page(boxes, first, last, page, pages);

	local args = { UID, 3, -1, T_LIST, NPC };
	for i = first, last do
		local b = boxes[i];
		table.insert(args, BOX_TEXT[b.id] or M_OTHER);
		table.insert(args, b.id);
	end
	table.insert(args, M_BREAKALL);
	table.insert(args, 500);
	if page > 0 then
		table.insert(args, M_PREV);
		table.insert(args, 200 + page - 1);
	end
	if page < pages - 1 then
		table.insert(args, M_NEXT);
		table.insert(args, 200 + page + 1);
	end
	table.insert(args, M_CLOSE);
	table.insert(args, -1);
	SelectMsg(unpack(args));
end

local function pick_box(item_id)
	local owned = BoxPick(UID, item_id);
	if owned < 1 then
		show_list(0);
		return;
	end
	local name = "?";
	local boxes = GetBreakableBoxes(UID);
	for i = 1, #boxes do
		if boxes[i].id == item_id then
			name = boxes[i].name;
		end
	end
	say("Secilen / Selected: " .. name .. " x" .. owned);

	local args = { UID, 3, -1, T_PICK, NPC, M_BREAK1, 410 };
	if owned >= 10 then
		table.insert(args, M_BREAK10);
		table.insert(args, 420);
	end
	if owned >= 2 then
		table.insert(args, M_BREAKTYPE);
		table.insert(args, 430);
	end
	table.insert(args, M_LIST);
	table.insert(args, 200);
	table.insert(args, M_CLOSE);
	table.insert(args, -1);
	SelectMsg(unpack(args));
end

-- Chat summary of a BoxBreak / BoxBreakAll result, then the matching dialog.
local function show_result(res)
	if res.broken > 0 then
		say("Kutu Kirici / Box Breaker: " .. res.broken .. " kirildi / broken. Oduller / Rewards:");
		local line = "  ";
		local n = 0;
		for i = 1, #res.rewards do
			local r = res.rewards[i];
			line = line .. r.name .. " x" .. r.count;
			n = n + 1;
			if i < #res.rewards then
				if n % 3 == 0 then
					say(line);
					line = "  ";
				else
					line = line .. " | ";
				end
			end
		end
		say(line);
	end

	local header = T_FAIL;
	if res.broken > 0 and res.stop == "done" then
		header = T_DONE;
	elseif res.stop == "capped" then
		header = T_CAPPED;
	elseif res.stop == "full" then
		header = T_FULL;
	elseif res.stop == "heavy" then
		header = T_HEAVY;
	elseif res.stop == "nobox" and res.broken > 0 then
		header = T_DONE;
	end
	SelectMsg(UID, 3, -1, header, NPC, M_LIST, 200, M_CLOSE, -1);
end

-- ROOT / paging
if (EVENT == 100) then
	show_list(0);
elseif (EVENT >= 200 and EVENT < 300) then
	show_list(EVENT - 200);

-- a box kind was picked: the event is the item id
elseif (EVENT >= 100000000) then
	pick_box(EVENT);

-- break the picked kind
elseif (EVENT == 410) then
	show_result(BoxBreak(UID, 1));
elseif (EVENT == 420) then
	show_result(BoxBreak(UID, 10));
elseif (EVENT == 430) then
	show_result(BoxBreak(UID, 0));

-- break everything (confirmation first)
elseif (EVENT == 500) then
	SelectMsg(UID, 3, -1, T_CONFIRM, NPC, M_YES_ALL, 510, M_NO_ALL, 200);
elseif (EVENT == 510) then
	show_result(BoxBreakAll(UID));
end
