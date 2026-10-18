-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 26051;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910050000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, -1, 4400, NPC, 18, 104);
	else
		SelectMsg(UID, 4, 24, 4401, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, -1, 4402, NPC, 4174, 102, 4175, 103);
end

if (EVENT == 104) then
	ShowMap(UID, 45);
end

if (EVENT == 102) then
	Check = isRoomForItem(UID, 910051000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
	else
		CycleSpawn(UID);;
		RunQuestExchange(UID,471);
		SaveEvent(UID, 4222);
	end
end

if (EVENT == 103) then
	CycleSpawn(UID);;
	SelectMsg(UID, 2, -1, 4403, NPC, 10, -1);
end