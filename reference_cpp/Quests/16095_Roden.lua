-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 16095;

local savenum = -1;

if (EVENT == 100) then
	SelectMsg(UID, 2, savenum, 4441, NPC, 4188, 101, 4005, -1);
end

if (EVENT == 101) then
	ITEM_COUNTA = HowmuchItem(UID, 389190000);
	if (ITEM_COUNTA > 0) then
		RobItem(UID, 389190000, 1);
		ZoneChangeParty(UID, 31, 940, 186)
	else
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, savenum, 4437, NPC, 18, 102);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, savenum, 4438, NPC, 18, 103);
		elseif (ITEM_COUNTC < 1) then 
			SelectMsg(UID, 2, savenum, 4439, NPC, 18, 104);
		elseif (ITEM_COUNTD < 1) then
			SelectMsg(UID, 2, savenum, 4440, NPC, 18, 105);
		end
	end
end

if (EVENT == 102) then
	ShowMap(UID, 440);
end

if (EVENT == 103) then
	ShowMap(UID, 445);
end

if (EVENT == 104) then
	ShowMap(UID, 446);
end

if (EVENT == 105) then
	ShowMap(UID, 447);
end