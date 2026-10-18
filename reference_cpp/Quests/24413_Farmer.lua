-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24413;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 330, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 330, NPC)
	else 
		EVENT = QuestNum
	end
end

if (EVENT == 200) then
	SelectMsg(UID, 4, 200, 1247, NPC, 22, 201, 23, -1);
end

if (EVENT == 201) then
	QuestStatus = GetQuestStatus(UID, 200)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 122);
			ShowMap(UID, 344);
	end
end

if (EVENT == 205) then
	QuestStatus = GetQuestStatus(UID, 200)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SelectMsg(UID, 2, 200, 1247, NPC, 10, -1);
			SaveEvent(UID, 124);
	end
end

if (EVENT == 210) then
	QuestStatus = GetQuestStatus(UID, 200)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ItemA = HowmuchItem(UID, 379204000);
		if (ItemA < 1) then
			SelectMsg(UID, 2, 200, 1248, NPC, 18, 212);
		else
			SelectMsg(UID, 4, 200, 1247, NPC, 41, 211, 27, -1);
		end
	end
end

if (EVENT == 212) then
	ShowMap(UID, 344);
end

if (EVENT == 211) then
	QuestStatus = GetQuestStatus(UID, 200)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ItemA = HowmuchItem(UID, 379204000);
		if (ItemA < 1) then
			SelectMsg(UID, 2, 200, 1248, NPC, 18, 212);
		else
			RunQuestExchange(UID,32);
			SaveEvent(UID, 123);
		end
	end
end

if (EVENT == 9532) then
	SelectMsg(UID, 4, 214, 8770, NPC, 22, 9533, 23, -1);
end

if (EVENT == 9533) then
	QuestStatus = GetQuestStatus(UID, 214)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9682);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9687);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9692);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9697);
		end
	end
end

if (EVENT == 9540) then
	QuestStatus = GetQuestStatus(UID, 214)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 214, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 214, 8770, NPC, 18, 9537);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 9684);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 9689);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 9694);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 9699);
			end
		end
	end
end

if (EVENT == 9536) then
	QuestStatus = GetQuestStatus(UID, 214)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 214, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 214, 8770, NPC, 18, 9537);
		else
			SelectMsg(UID, 5, 214, 8770, NPC, 41, 9538, 27, -1);
		end
	end
end

if (EVENT == 9537) then
	ShowMap(UID, 344);
end

if (EVENT == 9538) then
	QuestStatus = GetQuestStatus(UID, 214)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 214, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 214, 8770, NPC, 18, 9537);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,1146,STEP,1);
			SaveEvent(UID, 9683);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,1147,STEP,1);
			SaveEvent(UID, 9688);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,1148,STEP,1);
			SaveEvent(UID, 9693);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,1149,STEP,1);
			SaveEvent(UID, 9698);
			end
		end
	end
end