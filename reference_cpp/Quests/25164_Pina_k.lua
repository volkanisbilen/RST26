-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25164;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43928, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43928, NPC)
	else
		EVENT = QuestNum
	end
end

if(EVENT == 1112) then 
	SelectMsg(UID, 4, 1324, 43928, NPC, 22, 1113, 23, -1);
end

if(EVENT == 1113) then
	QuestStatus = GetQuestStatus(UID, 1324)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3654);
	end
end

if(EVENT == 1117) then
	QuestStatus = GetQuestStatus(UID, 1324)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BRACE1 = HowmuchItem(UID, 900654000)
		if( BRACE1 < 1) then
			SelectMsg(UID, 2, 1324, 43928, NPC, 18, 1116);
		else
			SaveEvent(UID, 3656);
		end
	end
end

if(EVENT == 1115) then
	QuestStatus = GetQuestStatus(UID, 1324)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BRACE1 = HowmuchItem(UID, 900654000)
		if( BRACE1 < 1) then
			SelectMsg(UID, 2, 1324, 43928, NPC, 18, 1116);
		else
			SelectMsg(UID, 4, 1324, 43928, NPC, 10, 1118, 27, -1);
		end
	end
end

if(EVENT == 1116) then
	ShowMap(UID, 1336);
end

if(EVENT == 1118) then
	QuestStatus = GetQuestStatus(UID, 1324)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BRACE1 = HowmuchItem(UID, 900654000)
		if( BRACE1 < 1) then
			SelectMsg(UID, 2, 1324, 43928, NPC, 18, 1116);
		else
			SaveEvent(UID, 3655);
			RunQuestExchange(UID,6118);
		end
	end
end

if(EVENT == 1152) then 
	SelectMsg(UID, 4, 1351, 44139, NPC, 22, 1156, 23, -1);
end

if(EVENT == 1156) then
SaveEvent(UID, 3816);
SaveEvent(UID, 3818);
SaveEvent(UID, 3817);
RunQuestExchange(UID,6145);
end

if(EVENT == 1182) then 
	SelectMsg(UID, 4, 1352, 44140, NPC, 22, 1183, 23, -1);
end

if(EVENT == 1183) then
	QuestStatus = GetQuestStatus(UID, 1352)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3822);
	end
end

if(EVENT == 1187) then
	QuestStatus = GetQuestStatus(UID, 1352)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1352, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 1352, 44140, NPC, 18, 1186);
		else
			SaveEvent(UID, 3824);
		end
	end
end

if (EVENT == 1185) then
	QuestStatus = GetQuestStatus(UID, 1352)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1352, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 1352, 44140, NPC, 18, 1186);
		else
			SelectMsg(UID, 4, 1352, 44140, NPC, 22, 1188, 23, -1);
		end
	end
end

if(EVENT == 1186) then
	ShowMap(UID, 489);
end

if(EVENT == 1188) then
	QuestStatus = GetQuestStatus(UID, 1352)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1352, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 1352, 44140, NPC, 18, 1186);
		else
			SaveEvent(UID, 3823);
			RunQuestExchange(UID,6146);
		end
	end
end

if(EVENT == 1192) then 
	SelectMsg(UID, 4, 1353, 44141, NPC, 22, 1193, 23, -1);
end

if(EVENT == 1193) then
	QuestStatus = GetQuestStatus(UID, 1353)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3828);
	end
end

if(EVENT == 1197) then
	QuestStatus = GetQuestStatus(UID, 1353)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	GHOST = HowmuchItem(UID, 900638000)
		if( GHOST < 1) then
			SelectMsg(UID, 2, 1353, 44141, NPC, 18, 1196);
		else
			SaveEvent(UID, 3830);
		end
	end
end

if(EVENT == 1195) then
	QuestStatus = GetQuestStatus(UID, 1353)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	GHOST = HowmuchItem(UID, 900638000)
		if( GHOST < 1) then
			SelectMsg(UID, 2, 1353, 44141, NPC, 18, 1196);
		else
			SelectMsg(UID, 4, 1353, 44141, NPC, 10, 1198, 27, -1);
		end
	end
end

if(EVENT == 1196) then
	ShowMap(UID, 1332);
end

if(EVENT == 1198) then
	QuestStatus = GetQuestStatus(UID, 1353)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	GHOST = HowmuchItem(UID, 900638000)
		if( GHOST < 1) then
			SelectMsg(UID, 2, 1353, 44141, NPC, 18, 1196);
		else
			SaveEvent(UID, 3829);
			RunQuestExchange(UID,6147);
		end
	end
end