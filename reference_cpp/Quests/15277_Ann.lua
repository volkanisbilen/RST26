-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 15277;

if (EVENT == 100) then -- normali 100
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 44375, NPC, 10, 0);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 44375, NPC)
	else
		EVENT = QuestNum
	end
end


if (EVENT == 5000) then
	SelectMsg(UID, 4, 1536, 44375, NPC, 22, 5001, 23, -1);
	SaveEvent(UID, 10277);
	SaveEvent(UID, 10279);
end

if (EVENT == 5001) then
	QuestStatusCheck = GetQuestStatus(UID, 1536) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		RunQuestExchange(UID,6237);
		SaveEvent(UID, 10278);
	end
end

if (EVENT == 300) then
	SelectMsg(UID, 4, 1564, 44448, NPC, 22, 301, 23, -1);
end

if (EVENT == 301) then
	SaveEvent(UID, 10405);
end

if (EVENT == 303) then
	SaveEvent(UID, 10407);
end

if (EVENT == 304) then
	MonsterCount = CountMonsterQuestSub(UID, 1564, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 1564, 44448, NPC, 18, 305);
	else
		SelectMsg(UID, 4, 1564, 44448, NPC, 4172, 306, 4173, -1);
	end
end

if (EVENT == 305) then
	ShowMap(UID, 1498);
end

if (EVENT == 306) then
	QuestStatusCheck = GetQuestStatus(UID, 1564) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		MonsterCount = CountMonsterQuestSub(UID, 1564, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1564, 44448, NPC, 18, 305);
		else
			RunQuestExchange(UID,6701)
			SaveEvent(UID, 10406);
		end
	end
end

if (EVENT == 310) then
	SelectMsg(UID, 4, 1566, 44448, NPC, 22, 313, 23, -1);
end

if (EVENT == 313) then
	SaveEvent(UID, 10415);
end

if (EVENT == 311) then
	SaveEvent(UID, 10417);
end

if (EVENT == 312) then
	MonsterCount = CountMonsterQuestSub(UID, 1566, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 1566, 44448, NPC, 18, 314);
	else
		SelectMsg(UID, 4, 1566, 44448, NPC, 4172, 315, 4173, -1);
	end
end

if (EVENT == 314) then
	ShowMap(UID, 1500);
end

if (EVENT == 315) then
	QuestStatusCheck = GetQuestStatus(UID, 1566) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		MonsterCount = CountMonsterQuestSub(UID, 1566, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1566, 44448, NPC, 18, 314);
		else
			RunQuestExchange(UID,6702)
			SaveEvent(UID, 10416);
		end
	end
end

if (EVENT == 320) then
	SelectMsg(UID, 4, 1547, 44377, NPC, 22, 323, 23, -1);
end

if (EVENT == 323) then
	SaveEvent(UID, 10501);
end

if (EVENT == 321) then
	SaveEvent(UID, 10503);
end

if (EVENT == 322) then
	MonsterCount = CountMonsterQuestSub(UID, 1547, 1);
	if (MonsterCount < 3) then
		SelectMsg(UID, 2, 1547, 44377, NPC, 18, 324);
	else
		SelectMsg(UID, 4, 1547, 44377, NPC, 4172, 325, 4173, -1);
	end
end

if (EVENT == 324) then
	ShowMap(UID, 1502);
end

if (EVENT == 325) then
	QuestStatusCheck = GetQuestStatus(UID, 1547) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		MonsterCount = CountMonsterQuestSub(UID, 1547, 1);
		if (MonsterCount < 3) then
			SelectMsg(UID, 2, 1547, 44377, NPC, 18, 324);
		else
			RunQuestExchange(UID,6247)
			SaveEvent(UID, 10502);
		end
	end
end

if (EVENT == 330) then
	SelectMsg(UID, 4, 1549, 44379, NPC, 22, 333, 23, -1);
end

if (EVENT == 333) then
	SaveEvent(UID, 10511);
end

if (EVENT == 331) then
	SaveEvent(UID, 10513);
end

if (EVENT == 332) then
	MonsterCount = CountMonsterQuestSub(UID, 1549, 1);
	if (MonsterCount < 3) then
		SelectMsg(UID, 2, 1549, 44379, NPC, 18, 334);
	else
		SelectMsg(UID, 4, 1549, 44379, NPC, 4172, 335, 4173, -1);
	end
end

if (EVENT == 334) then
	ShowMap(UID, 1504);
end

if (EVENT == 335) then
	QuestStatusCheck = GetQuestStatus(UID, 1549) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		MonsterCount = CountMonsterQuestSub(UID, 1549, 1);
		if (MonsterCount < 3) then
			SelectMsg(UID, 2, 1549, 44379, NPC, 18, 334);
		else
			RunQuestExchange(UID,6248)
			SaveEvent(UID, 10512);
		end
	end
end

if (EVENT == 600) then
	SelectMsg(UID, 4, 1551, 44381, NPC, 22, 601, 23, -1);
end

if (EVENT == 601) then
	SaveEvent(UID, 10521);
end

if (EVENT == 603) then
	SaveEvent(UID, 10523);
end

if (EVENT == 604) then
	MonsterCount = CountMonsterQuestSub(UID, 1551, 1);
	if (MonsterCount < 3) then
		SelectMsg(UID, 2, 1551, 44381, NPC, 18, 605);
	else
		SelectMsg(UID, 4, 1551, 44381, NPC, 4172, 606, 4173, -1);
	end
end

if (EVENT == 605) then
	ShowMap(UID, 1510);
end

if (EVENT == 606) then
	QuestStatusCheck = GetQuestStatus(UID, 1551) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		MonsterCount = CountMonsterQuestSub(UID, 1551, 1);
		if (MonsterCount < 3) then
			SelectMsg(UID, 2, 1551, 44381, NPC, 18, 605);
		else
			RunQuestExchange(UID,6249)
			SaveEvent(UID, 10522);
		end
	end
end

if (EVENT == 700) then
	SelectMsg(UID, 2, 1563, 44381, NPC, 56, -1);
end

if (EVENT == 800) then
	ITEM = HowmuchItem(UID, 998009000);
	if (ITEM < 30) then
		SelectMsg(UID, 2, 1568, 44381, NPC, 56, -1);
	else
		RobItem(UID, 998009000, 30);
		GiveItem(UID, 810596000, 1);
	end
end

if (EVENT == 400) then
	SelectMsg(UID, 2, 1569, 44381, NPC, 56, -1);
end
