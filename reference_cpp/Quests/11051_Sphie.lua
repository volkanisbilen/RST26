-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 11051;

if (EVENT == 215) then
	ITEM = HowmuchItem(UID, 910044000); 
		if (ITEM < 1 or ITEM == 0) then
			SelectMsg(UID, 2, 179, 677, NPC, 18, 191);
		else
			SelectMsg(UID, 4, 179, 678, NPC, 22, 218, 23, -1);
	end
end

if (EVENT == 191) then
	ShowMap(UID, 39);
end

if (EVENT == 218) then
	QuestStatusCheck = GetQuestStatus(UID, 179) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	Check = isRoomForItem(UID, 910041000);
	ITEM = HowmuchItem(UID, 910044000); 
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		elseif (ITEM < 1 or ITEM == 0) then
			SelectMsg(UID, 2, 179, 677, NPC, 18, 191);
		else
			SelectMsg(UID, 2, 179, 676, NPC, 10, -1);
			RunQuestExchange(UID,89);
			SaveEvent(UID, 446);
		end
	end
end