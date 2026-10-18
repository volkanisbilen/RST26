-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25166;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43812, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43812, NPC)
	else
		EVENT = QuestNum
	end
end

if(EVENT == 1112) then 
	SelectMsg(UID, 4, 1310, 43812, NPC, 22, 1113, 23, -1);
end

if(EVENT == 1113) then
	QuestStatus = GetQuestStatus(UID, 1310)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3570);
	end
end

if(EVENT == 1117) then
	QuestStatus = GetQuestStatus(UID, 1310)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	WOLFDOG = HowmuchItem(UID, 900652000)
		if( WOLFDOG < 20) then
			SelectMsg(UID, 2, 1310, 43812, NPC, 18, 1116);
		else
			SaveEvent(UID, 3572);
		end
	end
end

if(EVENT == 1115) then
	QuestStatus = GetQuestStatus(UID, 1310)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	WOLFDOG = HowmuchItem(UID, 900652000)
		if( WOLFDOG < 20) then
			SelectMsg(UID, 2, 1310, 43812, NPC, 18, 1116);
		else
			SelectMsg(UID, 4, 1310, 43812, NPC, 10, 1118, 27, -1);
		end
	end
end

if(EVENT == 1116) then
	ShowMap(UID, 1323);
end

if(EVENT == 1118) then
	QuestStatus = GetQuestStatus(UID, 1310)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	WOLFDOG = HowmuchItem(UID, 900652000)
		if( WOLFDOG < 20) then
			SelectMsg(UID, 2, 1310, 43812, NPC, 18, 1116);
		else
			SaveEvent(UID, 3571);
			RunQuestExchange(UID,6103);
		end
	end
end


if(EVENT == 1122) then 
	SelectMsg(UID, 4, 1311, 43815, NPC, 22, 1123, 23, -1);
end

if(EVENT == 1123) then
	QuestStatus = GetQuestStatus(UID, 1311)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3576);
			GiveItem(UID, 900670000,1);
	end
end

if(EVENT == 1127) then
	QuestStatus = GetQuestStatus(UID, 1311)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SADI = HowmuchItem(UID, 900659000)
		if( SADI < 1) then
			SelectMsg(UID, 2, 1311, 43815, NPC, 18, 1126);
		else
			SaveEvent(UID, 3578);
		end
	end
end

if(EVENT == 1125) then
	QuestStatus = GetQuestStatus(UID, 1311)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SADI = HowmuchItem(UID, 900659000)
		if( SADI < 1) then
			SelectMsg(UID, 2, 1311, 43815, NPC, 18, 1126);
		else
			SelectMsg(UID, 4, 1311, 43815, NPC, 10, 1128, 27, -1);
		end
	end
end

if(EVENT == 1126) then
	ShowMap(UID, 1291);
end

if(EVENT == 1128) then
	QuestStatus = GetQuestStatus(UID, 1311)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SADI = HowmuchItem(UID, 900659000)
		if( SADI < 1) then
			SelectMsg(UID, 2, 1311, 43815, NPC, 18, 1126);
		else
			SaveEvent(UID, 3577);
			RunQuestExchange(UID,6104);
		end
	end
end