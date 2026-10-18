local NPC = 24414;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 3003, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 3200, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 195) then
	SelectMsg(UID, 2, 107, 3201, NPC, 28, 196);
end

if (EVENT == 196) then
	ShowMap(UID, 334);
	SaveEvent(UID, 3053);
end

if (EVENT == 197) then
	SelectMsg(UID, 2, 107, 3201, NPC, 28, 196);
end

if (EVENT == 200) then 
	SelectMsg(UID, 2, 107, 3201, NPC, 10, 201);
end

if (EVENT == 201) then
	SelectMsg(UID, 4, 107, 3202, NPC, 22, 202, 23, -1);
end

if (EVENT == 202) then
	QuestStatus = GetQuestStatus(UID, 107)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3054);
	end
end

if (EVENT == 205) then
	QuestStatus = GetQuestStatus(UID, 107)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3056);
			NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 2, 107, 3005, NPC, 3009, -1);
		else
			SelectMsg(UID, 2, 107, 3006, NPC, 3009, -1);
		end
	end
end

if (EVENT == 210) then
	QuestStatus = GetQuestStatus(UID, 107)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 810418000);
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, 107, 3203, NPC, 18, 213);
		else
			SelectMsg(UID, 5, 107, 3204, NPC, 41, 214, 27, -1);
		end
	end
end

if (EVENT == 213) then
	ShowMap(UID, 22);
end

if (EVENT == 214) then
	QuestStatus = GetQuestStatus(UID, 107)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 810418000);
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, 107, 3203, NPC, 18, 213);
		else
			RunQuestExchange(UID,328,STEP,1);
			SaveEvent(UID, 3055); 
		end
	end
end

if (EVENT == 250) then
	SelectMsg(UID, 2, 106, 476, NPC, 24, 251);
end

if (EVENT == 251) then
	ShowMap(UID, 334);
	SaveEvent(UID, 7012);
end

if (EVENT == 252) then
	SelectMsg(UID, 2, 106, 476, NPC, 24, 251);
end

if (EVENT == 300) then
	SelectMsg(UID, 2, 106, 3205, NPC, 3006, 308, 13, -1);
end

if (EVENT == 308) then
	SelectMsg(UID, 2, 106, 3206, NPC, 3000, 301);
end

if (EVENT == 301) then
	SelectMsg(UID, 4, 106, 3207, NPC, 22, 302, 23, -1);
end

if (EVENT == 302) then
	QuestStatus = GetQuestStatus(UID, 106)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7013);
	end
end

if (EVENT == 303) then
	QuestStatus = GetQuestStatus(UID, 106)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7015);
			NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 2, 106, 3851, NPC, 3003, -1);
		elseif (NATION == 2) then
			SelectMsg(UID, 2, 106, 3852, NPC, 3003, -1);
		end
	end
end

if (EVENT == 305) then
	QuestStatus = GetQuestStatus(UID, 106)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 106, 1);
		if (MonsterCount < 5) then
			SelectMsg(UID, 2, 106, 3210, NPC, 18, 306);
		else
			SelectMsg(UID, 5, 106, 3211, NPC, 41, 307, 27, -1);
		end
	end
end

if (EVENT == 306) then
	ShowMap(UID, 576);
end

if (EVENT == 307) then
	QuestStatus = GetQuestStatus(UID, 106)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 106, 1);
		if (MonsterCount < 5) then
			SelectMsg(UID, 2, 106, 3210, NPC, 18, 306);
		else
			RunQuestExchange(UID,305,STEP,1);
			SaveEvent(UID, 7014); 
		end
	end
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=106 status=2 n_index=7014
if (EVENT == 165) then
	QuestStatusCheck = GetQuestStatus(UID, 106)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		SelectMsg(UID, 5, 107, 3204, NPC, 41, 214, 27, -1);
	end
end

