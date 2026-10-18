-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25165;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43929, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43929, NPC)
	else
		EVENT = QuestNum
	end
end

if(EVENT == 1112) then 
	SelectMsg(UID, 4, 1325, 43929, NPC, 22, 1113, 23, -1);
end

if(EVENT == 1113) then
	QuestStatus = GetQuestStatus(UID, 1325)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3660);
	end
end

if(EVENT == 1117) then
	QuestStatus = GetQuestStatus(UID, 1325)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	EAR = HowmuchItem(UID, 900655000)
		if( EAR < 1) then
			SelectMsg(UID, 2, 1325, 43929, NPC, 18, 1116);
		else
			SaveEvent(UID, 3662);
		end
	end
end

if(EVENT == 1115) then
	QuestStatus = GetQuestStatus(UID, 1325)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	EAR = HowmuchItem(UID, 900655000)
		if( EAR < 1) then
			SelectMsg(UID, 2, 1325, 43929, NPC, 18, 1116);
		else
			SelectMsg(UID, 4, 1325, 43929, NPC, 10, 1118, 27, -1);
		end
	end
end

if(EVENT == 1116) then
	ShowMap(UID, 1336);
end

if(EVENT == 1118) then
	QuestStatus = GetQuestStatus(UID, 1325)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	EAR = HowmuchItem(UID, 900655000)
		if( EAR < 1) then
			SelectMsg(UID, 2, 1325, 43929, NPC, 18, 1116);
		else
			SaveEvent(UID, 3661);
			RunQuestExchange(UID,6119);
		end
	end
end