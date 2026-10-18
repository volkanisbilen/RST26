-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25020;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 44135, NPC, 10, 193);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 44135, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1112) then
	SelectMsg(UID, 4, 1347, 44135, NPC, 22, 1113, 23, -1);
end

if (EVENT == 1113) then
	QuestStatus = GetQuestStatus(UID, 1347)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3792);
	end
end

if (EVENT == 1114) then
	QuestStatus = GetQuestStatus(UID, 1347)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1347, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1347, 44135, NPC, 18, 1116);
		else
			SaveEvent(UID, 3794);
		end
	end
end

if (EVENT == 1115) then
	QuestStatus = GetQuestStatus(UID, 1347)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1347, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1347, 44135, NPC, 18, 1116);
		else
			SelectMsg(UID, 4, 1347, 44135, NPC, 41, 1117, 27, -1);
		end
	end
end

if (EVENT == 1116) then
	ShowMap(UID, 1245);
end

if (EVENT == 1117) then
	QuestStatus = GetQuestStatus(UID, 1347)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1347, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1347, 44135, NPC, 18, 1116);
		else
			RunQuestExchange(UID,6141);
			SaveEvent(UID, 3793);   
		end
	end
end

if (EVENT == 1122) then
	SelectMsg(UID, 4, 1348, 44136, NPC, 22, 1123, 23, -1);
end

if (EVENT == 1123) then
	QuestStatus = GetQuestStatus(UID, 1348)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3798);
	end
end

if (EVENT == 1124) then
	QuestStatus = GetQuestStatus(UID, 1348)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1348, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 1348, 44136, NPC, 18, 1126);
		else
			SaveEvent(UID, 3800);
		end
	end
end

if (EVENT == 1125) then
	QuestStatus = GetQuestStatus(UID, 1348)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1348, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 1348, 44136, NPC, 18, 1126);
		else
			SelectMsg(UID, 4, 1348, 44136, NPC, 41, 1127, 27, 193);
		end
	end
end

if (EVENT == 1126) then
	ShowMap(UID, 488);
end

if (EVENT == 1127) then
	QuestStatus = GetQuestStatus(UID, 1348)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1348, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 1348, 44136, NPC, 18, 1126);
		else
			RunQuestExchange(UID,6142);
			SaveEvent(UID, 3799);
		end
	end
end

if (EVENT == 1132) then
	SelectMsg(UID, 4, 1349, 44137, NPC, 22, 1133, 23, -1);
end

if (EVENT == 1133) then
	QuestStatus = GetQuestStatus(UID, 1349)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3804);
	end
end

if (EVENT == 1137) then
	QuestStatus = GetQuestStatus(UID, 1349)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CONTTA = HowmuchItem(UID, 900687000);
		if (CONTTA < 10) then
			SelectMsg(UID, 2, 1349, 44137, NPC, 19, 1134);
		else
			SaveEvent(UID, 3806);
		end
	end
end

if (EVENT == 1134) then
	ShowMap(UID, 488);
end

if (EVENT == 1135) then
	QuestStatus = GetQuestStatus(UID, 1349)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CONTTA = HowmuchItem(UID, 900687000);
		if (CONTTA < 10) then
			SelectMsg(UID, 2, 1349, 44137, NPC, 19, 1134);
		else
			SelectMsg(UID, 4, 1349, 44137, NPC, 22, 1136, 23, -1);
		end
	end
end

if (EVENT == 1136) then
	QuestStatus = GetQuestStatus(UID, 1349)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CONTTA = HowmuchItem(UID, 900687000);
		if (CONTTA < 10) then
			SelectMsg(UID, 2, 1349, 44137, NPC, 19, 1134);
		else
			RunQuestExchange(UID,6143);
			SaveEvent(UID, 3805);
		end
	end
end

if (EVENT == 1142) then
	SelectMsg(UID, 4, 1350, 44138, NPC, 22, 1143, 23, -1);
end

if (EVENT == 1143) then
	SaveEvent(UID, 3810);
	SaveEvent(UID, 3812);
end

if (EVENT == 1145) then
	SelectMsg(UID, 4, 1350, 44138, NPC, 22, 1146, 23, -1);
end

if (EVENT == 1146) then
	RunQuestExchange(UID,6144);
	SaveEvent(UID, 3811);   
end