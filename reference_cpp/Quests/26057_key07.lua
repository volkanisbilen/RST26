-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 26057;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910056000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, -1, 4424, NPC, 18, 104);
	else
		SelectMsg(UID, 4, 30, 4425, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 104) then
	ShowMap(UID, 431);
end

if (EVENT == 101) then
	SelectMsg(UID, 2, -1, 4426, NPC, 4186, 102, 4187, 103);
end

if (EVENT == 103) then
	Check = isRoomForItem(UID, 910057000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
	else
		CycleSpawn(UID);;
		RunQuestExchange(UID,477);
		SaveEvent(UID, 4228); 
	end   
end

if (EVENT == 102) then
	CycleSpawn(UID);;
	RobItem(UID, 910056000, 1);
	SelectMsg(UID, 2, -1, 4427, NPC, 10, -1);
end