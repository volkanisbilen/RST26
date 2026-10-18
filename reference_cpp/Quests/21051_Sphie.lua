-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 21051;

if (EVENT == 215) then
	ITEM_COUNT1 = HowmuchItem(UID, 910044000); 
	if (ITEM_COUNT1 < 1) then
		SelectMsg(UID, 2, 178, 1183, NPC, 18, 191);
	else
		SelectMsg(UID, 4, 178, 1184, NPC, 22, 218, 23, -1);
	end
end

if (EVENT == 191) then
	ShowMap(UID, 45);
end

if (EVENT == 218) then
	QuestStatusCheck = GetQuestStatus(UID, 178) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	ITEM_COUNT1 = HowmuchItem(UID, 910044000); 
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 178, 1183, NPC, 18, 191);
		else
	Check = isRoomForItem(UID, 910041000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			RunQuestExchange(UID,89);
			SaveEvent(UID, 474);
			SelectMsg(UID, 2, 178, 1185, NPC, 10, -1);
			end
		end
	end
end