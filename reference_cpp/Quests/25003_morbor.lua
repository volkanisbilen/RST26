-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25003;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43796, NPC, 3001, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43796, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1122) then
	SelectMsg(UID, 4, 1308, 43803, NPC, 22, 1124, 23, -1);
end

if (EVENT == 1124) then
	QuestStatus = GetQuestStatus(UID, 1308)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3558);
	end
end

if (EVENT == 1127) then
	QuestStatus = GetQuestStatus(UID, 1308)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 1308, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 1308, 2);
		if (MonsterCount01 < 30) then
			SelectMsg(UID, 2, 1308, 43803, NPC, 4440, 1128);
		elseif (MonsterCount02 < 30) then
			SelectMsg(UID, 2, 1308, 43803, NPC, 4440, 1129);
		else
			SaveEvent(UID, 3560);
		end
	end
end

if (EVENT == 1125) then
	QuestStatus = GetQuestStatus(UID, 1308)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 1308, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 1308, 2);
		if (MonsterCount01 < 30) then
			SelectMsg(UID, 2, 1308, 43803, NPC, 4440, 1128);
		elseif (MonsterCount02 < 30) then
			SelectMsg(UID, 2, 1308, 43803, NPC, 4440, 1129);
		else
			SelectMsg(UID, 4, 1308, 43803, NPC, 10, 1126, 27, -1);
		end
	end
end

if (EVENT == 1128) then
	ShowMap(UID, 1323);
end

if (EVENT == 1129) then
	ShowMap(UID, 1322);
end

if (EVENT == 1126) then
	QuestStatus = GetQuestStatus(UID, 1308)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount01 = CountMonsterQuestSub(UID, 1308, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 1308, 2);
		if (MonsterCount01 < 30) then
			SelectMsg(UID, 2, 1308, 43803, NPC, 4440, 1128);
		elseif (MonsterCount02 < 30) then
			SelectMsg(UID, 2, 1308, 43803, NPC, 4440, 1129);
		else
			SaveEvent(UID, 3559);
			RunQuestExchange(UID,6101);
		end
	end
end

if (EVENT == 1132) then
	SelectMsg(UID, 4, 1309, 43806, NPC, 22, 1133, 23, -1);
end

if (EVENT == 1133) then
	QuestStatus = GetQuestStatus(UID, 1309)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3564);
	end
end

if (EVENT == 1137) then
	QuestStatus = GetQuestStatus(UID, 1309)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SAVAGEMEAT = HowmuchItem(UID, 900651000);
		if (SAVAGEMEAT < 20) then
			SelectMsg(UID, 2, 1309, 43806, NPC, 19, 1138);
		else
			SaveEvent(UID, 3566);
		end
	end
end

if (EVENT == 1135) then
	QuestStatus = GetQuestStatus(UID, 1309)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SAVAGEMEAT = HowmuchItem(UID, 900651000);
		if (SAVAGEMEAT < 20) then
			SelectMsg(UID, 2, 1309, 43806, NPC, 19, 1138);
		else
			SelectMsg(UID, 4, 1309, 43806, NPC, 22, 1136, 23, -1);
		end
	end
end

if (EVENT == 1138) then
	ShowMap(UID, 1322);
end

if (EVENT == 1136) then
	QuestStatus = GetQuestStatus(UID, 1309)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	SAVAGEMEAT = HowmuchItem(UID, 900651000);
		if (SAVAGEMEAT < 20) then
			SelectMsg(UID, 2, 1309, 43806, NPC, 19, 1138);
		else
			RunQuestExchange(UID,6102);
			SaveEvent(UID, 3565);
		end
	end
end

if (EVENT == 1112) then
	SelectMsg(UID, 4, 1307, 43800, NPC, 22, 1113, 23, -1);
end

if (EVENT == 1113) then
	QuestStatus = GetQuestStatus(UID, 1307)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3552);
	end
end

if (EVENT == 1117) then
	QuestStatus = GetQuestStatus(UID, 1307)
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3554);
	end
end

if (EVENT == 1115) then
	SelectMsg(UID, 4, 1307, 43800, NPC, 22, 1116, 23, -1);
end

if (EVENT == 1116) then
RunQuestExchange(UID,6100);
	SaveEvent(UID, 3553);
end