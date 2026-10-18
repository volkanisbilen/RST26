-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14413;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 331, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 331, NPC)
	else 
		EVENT = QuestNum
	end
end


if (EVENT == 195) then
	SaveEvent(UID, 114);
end

if (EVENT == 200) then
	SelectMsg(UID, 4, 201, 1247, NPC, 22, 201, 23, 202);
end

if (EVENT == 201) then
	SaveEvent(UID, 115);
	ShowMap(UID, 14);
end

if (EVENT == 202) then
	SaveEvent(UID, 118);
end

if (EVENT == 205) then
	SelectMsg(UID, 2, 201, 1247, NPC, 10, -1);
	SaveEvent(UID, 117);
end

if (EVENT == 210) then
	ItemA = HowmuchItem(UID, 379204000);
	if (ItemA < 1) then
		SelectMsg(UID, 2, 201, 1248, NPC, 18, 212);
	else
		SelectMsg(UID, 4, 201, 1247, NPC, 41, 211, 27, -1);
	end
end

if (EVENT == 212) then
	ShowMap(UID, 14);
end

if (EVENT == 211) then
	QuestStatusCheck = GetQuestStatus(UID, 201) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ItemA = HowmuchItem(UID, 379204000);
	if (ItemA < 1) then
		SelectMsg(UID, 2, 201, 1248, NPC, 18, 212);
	else
RunQuestExchange(UID,31)
	SaveEvent(UID, 116);
end
end
end

if (EVENT == 9530) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9702);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9707);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9712);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9717);
	end
end

if (EVENT == 9532) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 215, 8770, NPC, 22, 9533, 23, 9534);
	else
		SelectMsg(UID, 2, 215, 8770, NPC, 10, -1);
	end
end

if (EVENT == 9533) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9703);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9708);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9713);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9718);
	end
end

if (EVENT == 9534) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9706);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9711);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9716);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9721);
	end
end

if (EVENT == 9540) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9705);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9710);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9715);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9720);
	end
end

if (EVENT == 9536) then
	MonsterCount = CountMonsterQuestSub(UID, 215, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 215, 8770, NPC, 18, 9537);
	else
		SelectMsg(UID, 5, 215, 8770, NPC, 41, 9538, 27, -1);
	end
end

if (EVENT == 9537) then
	ShowMap(UID, 14);
end

if (EVENT == 9538) then
	QuestStatusCheck = GetQuestStatus(UID, 215) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 215, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 215, 8770, NPC, 18, 9537);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,1150,STEP,1);
		SaveEvent(UID, 9704);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,1151,STEP,1);
		SaveEvent(UID, 9709);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,1152,STEP,1);
		SaveEvent(UID, 9714);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,1153,STEP,1);
		SaveEvent(UID, 9719);
	end
end
end
end