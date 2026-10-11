local Ret = 0;
local NPC = 31718;

-- [Lunar Order Priest] Blanc
-- Full Moon Rift Jewel shop + Draki Supply Box opening.
-- 914069000 Full Moon Rift Jewel, 810678000 Old Draki Supply Box, 810596000 Superior Draki Supply Box,
-- 930661000 Grace of the Moon God, 810942000 Lunar Tattoo (7 days)

local JEWEL = 914069000;

-- ROOT
if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 45188, NPC, 40753, 200, 44411, 2500, 27, 3001);
end

-- Jewel shop
if (EVENT == 200) then
	SelectMsg(UID, 3, -1, 44689, NPC, 45150, 201, 45151, 202, 40756, 203, 40757, 204, 27, 3001);
end

if (EVENT == 201) then
	if (HowmuchItem(UID, JEWEL) < 250) then
		SelectMsg(UID, 2, -1, 44689, NPC, 10, 3001);
	elseif (CheckGiveSlot(UID, 1) == false) then
		SelectMsg(UID, 2, -1, 44689, NPC, 10, 3001);
	else
		RobItem(UID, JEWEL, 250);
		GiveItem(UID, 810678000, 1);
	end
end

if (EVENT == 202) then
	if (HowmuchItem(UID, JEWEL) < 350) then
		SelectMsg(UID, 2, -1, 44689, NPC, 10, 3001);
	elseif (CheckGiveSlot(UID, 1) == false) then
		SelectMsg(UID, 2, -1, 44689, NPC, 10, 3001);
	else
		RobItem(UID, JEWEL, 350);
		GiveItem(UID, 810596000, 1);
	end
end

if (EVENT == 203) then
	if (HowmuchItem(UID, JEWEL) < 100) then
		SelectMsg(UID, 2, -1, 44689, NPC, 10, 3001);
	elseif (CheckGiveSlot(UID, 1) == false) then
		SelectMsg(UID, 2, -1, 44689, NPC, 10, 3001);
	else
		RobItem(UID, JEWEL, 100);
		GiveItem(UID, 930661000, 1);
	end
end

if (EVENT == 204) then
	if (HowmuchItem(UID, JEWEL) < 200) then
		SelectMsg(UID, 2, -1, 44689, NPC, 10, 3001);
	elseif (CheckGiveSlot(UID, 1) == false) then
		SelectMsg(UID, 2, -1, 44689, NPC, 10, 3001);
	else
		RobItem(UID, JEWEL, 200);
		GiveItem(UID, 810942000, 1);
	end
end

-- Open Draki Supply Boxes (same exchange tables as Operator Moira)
if EVENT == 2500 then
	DRAKI = HowmuchItem(UID, 810596000);
	OLDDRAKI = HowmuchItem(UID, 810678000);
	if (DRAKI > 0 and OLDDRAKI > 0) then
		SelectMsg(UID, 3, -1, 4035, NPC, 40501, 2501, 40502, 2503);
	elseif (DRAKI > 0) then
		EVENT = 2501
	elseif (OLDDRAKI > 0) then
		EVENT = 2503
	else
		SelectMsg(UID, 2, -1, 44410, NPC, 10, -1);
	end
end

if EVENT == 2501 then
	if (HowmuchItem(UID, 810596000) > 0) then
		SelectMsg(UID, 4, 1553, 44412, NPC, 4006, 2502, 27, -1);
	else
		SelectMsg(UID, 2, -1, 44410, NPC, 10, -1);
	end
end

if EVENT == 2502 then
	if (CheckGiveSlot(UID, 1) == false) then
	elseif (HowmuchItem(UID, 810596000) < 1) then
		SelectMsg(UID, 2, -1, 44410, NPC, 10, -1);
	else
		Roll = RollDice(UID, 25)
		RunQuestExchange(UID, Roll + 6501);
	end
end

if EVENT == 2503 then
	if (HowmuchItem(UID, 810678000) > 0) then
		SelectMsg(UID, 4, 6531, 44412, NPC, 4006, 2504, 27, -1);
	else
		SelectMsg(UID, 2, -1, 44410, NPC, 10, -1);
	end
end

if EVENT == 2504 then
	if (CheckGiveSlot(UID, 1) == false) then
	elseif (HowmuchItem(UID, 810678000) < 1) then
		SelectMsg(UID, 2, -1, 44410, NPC, 10, -1);
	else
		Roll = RollDice(UID, 37)
		RunQuestExchange(UID, Roll + 6531);
	end
end

-- Close dialog
if (EVENT == 3001) then
	Ret = 1;
end
