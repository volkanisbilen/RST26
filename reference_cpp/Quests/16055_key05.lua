-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 16055;

local savenum = 28;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910054000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, savenum, 4316, NPC, 18, 104);
	else
		SelectMsg(UID, 4, savenum, 4317, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 104) then
	ShowMap(UID, 433);
end

if (EVENT == 101) then
	SelectMsg(UID, 2, savenum, 4318, NPC, 4182, 102, 4183, 103);
end

if (EVENT == 102) then
	Check = isRoomForItem(UID, 910055000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
	else
		CycleSpawn(UID);
RunQuestExchange(UID,475)
		SaveEvent(UID, 4240); 
	end   
end

if (EVENT == 103) then
	CycleSpawn(UID);
	RobItem(UID, 910054000, 1);
	SelectMsg(UID, 2, savenum, 4319, NPC, 10, -1);
end