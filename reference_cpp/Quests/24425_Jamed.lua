-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24425;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4578, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 4579, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 532) then
	SelectMsg(UID, 2, 246, 4582, NPC, 4228, 535, 4063, -1);
end

if (EVENT == 535) then
	SelectMsg(UID, 4, 246, 4646, NPC, 22, 533, 23, -1);
end

if (EVENT == 533) then
	QuestStatus = GetQuestStatus(UID, 246)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4307);
	end
end

if (EVENT == 180) then
	QuestStatus = GetQuestStatus(UID, 246)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 4309);
			SelectMsg(UID, 2, 246, 4585, NPC, 14, -1);
	end
end

if (EVENT == 536) then
	QuestStatus = GetQuestStatus(UID, 246)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 246, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 246, 4587, NPC, 18, 538);
		else
			SelectMsg(UID, 4, 246, 4588, NPC, 4172, 537, 4173, -1);
		end
	end
end

if (EVENT == 538) then
	ShowMap(UID, 489);
end

if (EVENT == 537) then
	QuestStatus = GetQuestStatus(UID, 246)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 246, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 246, 4587, NPC, 18, 538);
		else
			RunQuestExchange(UID,490);
			SaveEvent(UID, 4308);   
		end
	end
end

if (EVENT == 9362) then
	SelectMsg(UID, 2, 271, 8684, NPC, 4228, 9363, 4063, -1);
end

if (EVENT == 9363) then
	SelectMsg(UID, 4, 271, 8684, NPC, 22, 9364, 23, -1);
end

if (EVENT == 9364) then
	QuestStatus = GetQuestStatus(UID, 271)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9388);
	end
end

if (EVENT == 9365) then
	QuestStatus = GetQuestStatus(UID, 271)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 271, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 271, 2);
		if (MonsterCount1 < 1 and MonsterCount2 < 1) then
			SelectMsg(UID, 2, 271, 8684, NPC, 18, 9370);
		else
			SaveEvent(UID, 9390);
			SelectMsg(UID, 2, 271, 8684, NPC, 14, -1);
		end
	end
end

if (EVENT == 9367) then
	QuestStatus = GetQuestStatus(UID, 271)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 271, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 271, 2);
		if (MonsterCount1 < 1 and MonsterCount2 < 1) then
			SelectMsg(UID, 2, 271, 8684, NPC, 18, 9370);
		else
			SelectMsg(UID, 4, 271, 8684, NPC, 4172, 9369, 4173, -1);
		end
	end
end

if (EVENT == 9370) then
	ShowMap(UID, 489);
end

if (EVENT == 9369) then
	QuestStatus = GetQuestStatus(UID, 271)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 271, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 271, 2);
		if (MonsterCount1 < 1 and MonsterCount2 < 1) then
			SelectMsg(UID, 2, 271, 8684, NPC, 18, 9370);
		else
			RunQuestExchange(UID,1094);
			SaveEvent(UID, 9389);   
		end
	end
end

if (EVENT == 400) then
	SelectMsg(UID, 4, 440, 6109, NPC, 10, 401, 4005, -1);
end

if (EVENT == 401) then
	QuestStatus = GetQuestStatus(UID, 440)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SelectMsg(UID, 15, -1, -1, NPC);
			SaveEvent(UID, 7123);
			RunQuestExchange(UID,55);
	end
end

if (EVENT == 100) then
	SelectMsg(UID, 4, 189, 8878, NPC, 10, 101, 4005, -1);
end