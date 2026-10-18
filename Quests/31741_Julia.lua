local Ret = 0;
local NPC = 31741;

-- [Trader] Julia
-- Auto-generated from sniffer capture (dialog_builder v4)
-- 5 menus, 8 mapped, 0 inferred, 18 unknown

-- ROOT: header=44704 flag=2
if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 44704, NPC, 40776, 101, 40777, 101, 40800, 102, 40831, 3001 --[[ TODO: unknown ]], 45330, 104);
end

-- header=44707 flag=2
if (EVENT == 101) then
	SelectMsg(UID, 2, -1, 44707, NPC, 27, 3001);
end

-- header=44704 flag=2
if (EVENT == 102) then
	SelectMsg(UID, 2, -1, 44704, NPC, 40851, 3001 --[[ TODO: unknown ]], 40802, 3001 --[[ TODO: unknown ]], 40853, 3001 --[[ TODO: unknown ]], 40804, 3001 --[[ TODO: unknown ]], 40854, 3001 --[[ TODO: unknown ]], 40805, 3001 --[[ TODO: unknown ]], 45337, 103, 40857, 3001 --[[ TODO: unknown ]], 40806, 3001 --[[ TODO: unknown ]]);
end

-- header=44705 flag=2
if (EVENT == 103) then
	SelectMsg(UID, 2, -1, 44705, NPC, 27, 3001);
end

-- header=45422 flag=2
if (EVENT == 104) then
	SelectMsg(UID, 2, -1, 45422, NPC, 45054, 201, 40852, 202, 45331, 203, 40906, 204, 45332, 205, 45357, 206, 40855, 207, 45055, 208, 45359, 209, 45333, 210);
end

local function ExchangeManesOrb(required, reward, reward_count, expiry_days)
	if (HowmuchItem(UID, 978026000) < required) then
		SelectMsg(UID, 2, -1, 10596, NPC, 18, 5000);
		return;
	end
	if (CheckGiveSlot(UID, 1) == false) then
		return;
	end
	local delivered = false;
	if (expiry_days > 0) then
		delivered = GiveItem(UID, reward, reward_count, expiry_days);
	else
		delivered = GiveItem(UID, reward, reward_count);
	end
	if (delivered == true) then
		RobItem(UID, 978026000, required);
	end
end

-- The capture routes the generic "where can I find it?" button here. Julia
-- has no Quest_Helper map target for Manes' Orb, so close with the same
-- explanatory text instead of dispatching a nonexistent quest map entry.
if (EVENT == 5000) then
	SelectMsg(UID, 2, -1, 10596, NPC, 27, -1);
end

if (EVENT == 201) then ExchangeManesOrb(7, 931731000, 1, 3); end

if (EVENT == 202) then
	if (GetNation(UID) == 1) then
		ExchangeManesOrb(15, 800123000, 1, 0);
	else
		ExchangeManesOrb(15, 800122000, 1, 0);
	end
end

if (EVENT == 203) then
	if (GetNation(UID) == 1) then
		ExchangeManesOrb(15, 518012000, 1, 0);
	else
		ExchangeManesOrb(15, 518011000, 1, 0);
	end
end

if (EVENT == 204) then ExchangeManesOrb(20, 931695000, 1, 7); end
if (EVENT == 205) then ExchangeManesOrb(45, 814038000, 1, 1); end
if (EVENT == 206) then ExchangeManesOrb(50, 890226896, 1, 0); end
if (EVENT == 207) then ExchangeManesOrb(100, 508070000, 1, 7); end
if (EVENT == 208) then ExchangeManesOrb(150, 379099000, 10, 0); end

if (EVENT == 209) then
	local result = ExchangeItemForAchievement(UID, 978026000, 300, 471);
	if (result == 2) then
		SelectMsg(UID, 2, -1, 44759, NPC, 27, -1);
	elseif (result == 3) then
		SelectMsg(UID, 2, -1, 10596, NPC, 18, 5000);
	end
end
if (EVENT == 210) then ExchangeManesOrb(450, 931751000, 1, 15); end

-- Close dialog
if (EVENT == 3001) then
	Ret = 1;
end
