-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25017;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43796, NPC, 3001, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43803, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 122) then
	SelectMsg(UID, 4, 1238, 43803, NPC, 22, 124, 23, -1);
end

if (EVENT == 124) then
	QuestStatus = GetQuestStatus(UID, 1238)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7536);
	end
end

if (EVENT == 127) then
	QuestStatus = GetQuestStatus(UID, 1238)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 1238, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 1238, 2);
		if (MonsterCount01 < 30) then
			SelectMsg(UID, 2, 1238, 43803, NPC, 4440, 128);
		elseif (MonsterCount02 < 30) then
			SelectMsg(UID, 2, 1238, 43803, NPC, 4440, 129);
		else
			SaveEvent(UID, 7538);
		end
	end
end

if (EVENT == 125) then
	QuestStatus = GetQuestStatus(UID, 1238)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 1238, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 1238, 2);
		if (MonsterCount01 < 30) then
			SelectMsg(UID, 2, 1238, 43803, NPC, 4440, 128);
		elseif (MonsterCount02 < 30) then
			SelectMsg(UID, 2, 1238, 43803, NPC, 4440, 129);
		else
			SelectMsg(UID, 4, 1238, 43803, NPC, 10, 126, 27, -1);
		end
	end
end

if (EVENT == 128) then
	ShowMap(UID, 1318);
end

if (EVENT == 129) then
	ShowMap(UID, 1317);
end

if (EVENT == 126) then
	QuestStatus = GetQuestStatus(UID, 1238)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 1238, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 1238, 2);
		if (MonsterCount01 < 30) then
			SelectMsg(UID, 2, 1238, 43803, NPC, 4440, 128);
		elseif (MonsterCount02 < 30) then
			SelectMsg(UID, 2, 1238, 43803, NPC, 4440, 129);
		else
			SaveEvent(UID, 7537);
			RunQuestExchange(UID,6034);
		end
	end
end

if (EVENT == 132) then
	SelectMsg(UID, 4, 1239, 43806, NPC, 22, 133, 23, -1);
end

if (EVENT == 133) then
	QuestStatus = GetQuestStatus(UID, 1239)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7542);
	end
end

if (EVENT == 137) then
	QuestStatus = GetQuestStatus(UID, 1239)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7544);
	end
end

if (EVENT == 135) then
	QuestStatus = GetQuestStatus(UID, 1239)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SAVAGEMEAT = HowmuchItem(UID, 900651000);
		if (SAVAGEMEAT < 20) then
			SelectMsg(UID, 2, 1239, 43806, NPC, 19, 138);
		else
			SelectMsg(UID, 4, 1239, 43806, NPC, 22, 136, 23, -1);
		end
	end
end

if (EVENT == 138) then
	ShowMap(UID, 1317);
end

if (EVENT == 136) then
	QuestStatus = GetQuestStatus(UID, 1239)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SAVAGEMEAT = HowmuchItem(UID, 900651000);
		if (SAVAGEMEAT < 20) then
			SelectMsg(UID, 2, 1239, 43806, NPC, 19, 138);
		else
			RunQuestExchange(UID,6035);
			SaveEvent(UID, 7543);
		end
	end
end

if (EVENT == 112) then
	SelectMsg(UID, 4, 1237, 43800, NPC, 22, 113, 23, 0);
end

if (EVENT == 113) then
	QuestStatus = GetQuestStatus(UID, 1237)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7530);
	end
end

if (EVENT == 117) then
	QuestStatus = GetQuestStatus(UID, 1237)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7532);
	end
end

if (EVENT == 115) then
SelectMsg(UID, 4, 1237, 43800, NPC, 22, 116, 23, -1);
end

if (EVENT == 116) then
RunQuestExchange(UID,6033);
	SaveEvent(UID, 7531);
end