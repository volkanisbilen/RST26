-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25006;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4515, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 4516, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 112) then
	SelectMsg(UID, 4, 1285, 44135, NPC, 22, 113, 23, -1);
end

if (EVENT == 113) then
	QuestStatus = GetQuestStatus(UID, 1285)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7856);
	end
end

if (EVENT == 114) then
	QuestStatus = GetQuestStatus(UID, 1285)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7858);
	end
end

if (EVENT == 115) then
	QuestStatus = GetQuestStatus(UID, 1285)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1285, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1285, 44135, NPC, 18, 116);
		else
			SelectMsg(UID, 4, 1285, 44135, NPC, 41, 117, 27, -1);
		end
	end
end

if (EVENT == 116) then
	ShowMap(UID, 1245);
end

if (EVENT == 117) then
	QuestStatus = GetQuestStatus(UID, 1285)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1285, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1285, 44135, NPC, 18, 116);
		else
			RunQuestExchange(UID,6081);
			SaveEvent(UID, 7857);
		end
	end
end

if (EVENT == 122) then
	SelectMsg(UID, 4, 1286, 44136, NPC, 22, 123, 23, -1);
end

if (EVENT == 123) then
	QuestStatus = GetQuestStatus(UID, 1286)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7862);
	end
end

if (EVENT == 124) then
	QuestStatus = GetQuestStatus(UID, 1286)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1286, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 1286, 44136, NPC, 18, 126);
		else
			SaveEvent(UID, 7864);
		end
	end
end

if (EVENT == 125) then
	QuestStatus = GetQuestStatus(UID, 1286)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1286, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 1286, 44136, NPC, 18, 126);
		else
			SelectMsg(UID, 4, 1286, 44136, NPC, 41, 127, 27, -1);
		end
	end
end

if (EVENT == 126) then
	ShowMap(UID, 488);
end

if (EVENT == 127) then
	QuestStatus = GetQuestStatus(UID, 1286)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1286, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 1286, 44136, NPC, 18, 126);
		else
			RunQuestExchange(UID,6082);
			SaveEvent(UID, 7863);   
		end
	end
end

if (EVENT == 132) then
	SelectMsg(UID, 4, 1287, 44137, NPC, 22, 133, 23, -1);
end

if (EVENT == 133) then
	QuestStatus = GetQuestStatus(UID, 1287)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7868);
	end
end

if (EVENT == 137) then
	QuestStatus = GetQuestStatus(UID, 1287)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CONTTA = HowmuchItem(UID, 900687000);
		if (CONTTA < 10) then
			SelectMsg(UID, 2, 1287, 44137, NPC, 19, 134);
		else
			SaveEvent(UID, 7870);
		end
	end
end

if (EVENT == 134) then
	ShowMap(UID, 488);
end

if (EVENT == 135) then
	QuestStatus = GetQuestStatus(UID, 1287)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CONTTA = HowmuchItem(UID, 900687000);
		if (CONTTA < 10) then
			SelectMsg(UID, 2, 1287, 44137, NPC, 19, 134);
		else
			SelectMsg(UID, 4, 1287, 44137, NPC, 22, 136, 23, -1);
		end
	end
end

if (EVENT == 136) then
	QuestStatus = GetQuestStatus(UID, 1287)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CONTTA = HowmuchItem(UID, 900687000);
		if (CONTTA < 10) then
			SelectMsg(UID, 2, 1287, 44137, NPC, 19, 134);
		else
			RunQuestExchange(UID,6083);
			SaveEvent(UID, 7869);
		end
	end
end

if (EVENT == 142) then
	SelectMsg(UID, 4, 1288, 44138, NPC, 22, 143, 23, 293);
end

if (EVENT == 143) then
	SaveEvent(UID, 7874);
	SaveEvent(UID, 7876);
end

if (EVENT == 145) then
	SelectMsg(UID, 4, 1288, 44138, NPC, 22, 146, 23, 158);
end

if (EVENT == 146) then
	RunQuestExchange(UID,6084);
	SaveEvent(UID, 7875);   
end