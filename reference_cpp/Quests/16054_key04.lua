-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 16054;

local savenum = 27;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910053000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, savenum, 4312, NPC, 18, 104);
	else
		SelectMsg(UID, 4, savenum, 4313, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 104) then
	ShowMap(UID, 435);
end

if (EVENT == 101) then
	SelectMsg(UID, 2, savenum, 4314, NPC, 4180, 102, 4181, 103);
end

if (EVENT == 103) then
	Check = isRoomForItem(UID, 910054000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
	else
		CycleSpawn(UID);
RunQuestExchange(UID,474)
		SaveEvent(UID, 4239); 
	end   
end

if (EVENT == 102) then
	CycleSpawn(UID);
	RobItem(UID, 910053000, 1);
	SelectMsg(UID, 2, savenum, 4315, NPC, 10, -1);
end