-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 26052;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910051000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, -1, 4404, NPC, 18, 104);
	else 
		SelectMsg(UID, 4, 25, 4405, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, -1, 4406, NPC, 4176, 102, 4177, 103);
end

if (EVENT == 104) then
	ShowMap(UID, 429);
end

if (EVENT == 102) then
	Check = isRoomForItem(UID, 910052000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
	else
		CycleSpawn(UID);;
		RunQuestExchange(UID,472);
		SaveEvent(UID, 4223); 
	end   
end

if (EVENT == 103) then
	CycleSpawn(UID);;
	RobItem(UID, 910051000, 1);
	SelectMsg(UID, 2, -1, 4407, NPC, 10, -1);
end