-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 15276;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 44293, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 44293, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 300) then
SelectMsg(UID, 4,1517, 44273, NPC, 3000, 301,3005,-1);
end


if (EVENT == 301) then
    SaveEvent(UID, 9801);
end

if (EVENT == 303) then
    SaveEvent(UID, 9803);
end

if (EVENT == 304) then
	MonsterCount = CountMonsterQuestSub(UID, 1517, 1);
	if (MonsterCount < 50) then
		SelectMsg(UID, 2, 1517, 44273, NPC, 18, 305);
	else
		SelectMsg(UID, 4, 1517, 44273, NPC, 22, 306, 23,-1);
	end
end

if (EVENT == 305) then
	ShowMap(UID, 174);
end

if (EVENT == 306) then
	QuestStatusCheck = GetQuestStatus(UID, 1517) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 1517, 1);
	if (MonsterCount < 50) then
		SelectMsg(UID, 2, 1517, 44273, NPC, 18, 305);
	else
RunQuestExchange(UID,16231) 
	SaveEvent(UID, 9802);
end
end
end

if (EVENT == 310) then
	CheckParty = isInParty(UID)
		if CheckParty == false then
	SelectMsg(UID, 2, -1, 44272, NPC, 10, -1)
	else
	SelectMsg(UID, 4,1518, 44273, NPC, 3000, 313,3005,-1);
	end
end

if (EVENT == 313) then
    SaveEvent(UID, 9806);
end

if (EVENT == 311) then
    SaveEvent(UID, 9808);
end

if (EVENT == 312) then
	MonsterCount = CountMonsterQuestSub(UID, 1518, 1);
	if (MonsterCount < 50) then
		SelectMsg(UID, 2, 1518, 44273, NPC, 18, 314);
	else
		SelectMsg(UID, 4, 1518, 44273, NPC, 22, 315, 23,-1);
	end
end

if (EVENT == 314) then
	ShowMap(UID, 1184);
end

if (EVENT == 315) then
	QuestStatusCheck = GetQuestStatus(UID, 1518) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 1518, 1);
	if (MonsterCount < 50) then
		SelectMsg(UID, 2, 1518, 44273, NPC, 18, 314);
	else
RunQuestExchange(UID,16232) 
	SaveEvent(UID, 9807);
end
end
end

if (EVENT == 320) then
	CheckParty = isInParty(UID)
		if CheckParty == false then
	SelectMsg(UID, 2, -1, 44272, NPC, 10, -1)
	else
	SelectMsg(UID, 4,1519, 44273, NPC, 3000, 323,3005,-1);
	end
end	

if (EVENT == 323) then
    SaveEvent(UID, 9811);
end

if (EVENT == 321) then
    SaveEvent(UID, 9813);
end

if (EVENT == 322) then
	MonsterCount = CountMonsterQuestSub(UID, 1519, 1);
	if (MonsterCount < 50) then
		SelectMsg(UID, 2, 1519, 44273, NPC, 18, 324);
	else
		SelectMsg(UID, 4, 1519, 44273, NPC, 22, 325, 23,-1);
	end
end

if (EVENT == 324) then
	ShowMap(UID, 1495);
end

if (EVENT == 325) then
	QuestStatusCheck = GetQuestStatus(UID, 1519) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 1519, 1);
	if (MonsterCount < 50) then
		SelectMsg(UID, 2, 1519, 44273, NPC, 18, 324);
	else
	RunQuestExchange(UID,16233) 
	SaveEvent(UID, 9812);
end
end
end

if (EVENT == 330) then
	CheckParty = isInParty(UID)
		if CheckParty == false then
	SelectMsg(UID, 2, -1, 44272, NPC, 10, -1)
	else
	SelectMsg(UID, 4,1520, 44273, NPC, 3000, 333,3005,-1);
	end
end

if (EVENT == 333) then
    SaveEvent(UID, 9816);
end

if (EVENT == 331) then
    SaveEvent(UID, 9818);
end

if (EVENT == 332) then
	MonsterCount = CountMonsterQuestSub(UID, 1520, 1);
	if (MonsterCount < 50) then
		SelectMsg(UID, 2, 1520, 44273, NPC, 18, 334);
	else
		SelectMsg(UID, 4, 1520, 44273, NPC, 22, 335, 23,-1);
	end
end

if (EVENT == 334) then
	ShowMap(UID, 1183);
end

if (EVENT == 335) then
	QuestStatusCheck = GetQuestStatus(UID, 1520) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 1520, 1);
	if (MonsterCount < 50) then
		SelectMsg(UID, 2, 1520, 44273, NPC, 18, 334);
	else
	RunQuestExchange(UID,16234);
	SaveEvent(UID, 9817);
end
end
end

if (EVENT == 340) then
	CheckParty = isInParty(UID)
		if CheckParty == false then
	SelectMsg(UID, 2, -1, 44272, NPC, 10, -1)
	else
	SelectMsg(UID, 4,1522, 44282, NPC, 3000, 343,3005,-1);
	end
end

if (EVENT == 343) then
    SaveEvent(UID, 9826);
end

if (EVENT == 341) then
    SaveEvent(UID, 9828);
end

if (EVENT == 342) then
	MonsterCount01 = CountMonsterQuestSub(UID, 1522, 1); --Quest Uruk Hai
	MonsterCount02 = CountMonsterQuestSub(UID, 1522, 2); --Quest Stone Golem
	MonsterCount03 = CountMonsterQuestSub(UID, 1522, 3); --Quest Troll Berserker
	MonsterCount04 = CountMonsterQuestSub(UID, 1522, 4); --Quest Apostles  
	if (MonsterCount01 > 0 and MonsterCount02 > 0 and MonsterCount03 > 0 and MonsterCount04 > 0) then 
		SelectMsg(UID, 4, 1522, 44282, NPC, 3000, 348,27, 193);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 1522, 4626, NPC, 18, 344);
		elseif ( MonsterCount02 < 1) then
			SelectMsg(UID, 2, 1522, 4627, NPC, 18, 345);
		elseif ( MonsterCount03 < 1) then
			SelectMsg(UID, 2, 1522, 4628, NPC, 18, 346);
		elseif ( MonsterCount04 < 1) then
			SelectMsg(UID, 2, 1522, 4629, NPC, 18, 347);
		end
	end
end

if (EVENT == 344) then
	ShowMap(UID, 474);
end

if (EVENT == 345) then
	ShowMap(UID, 475);
end

if (EVENT == 346) then
	ShowMap(UID, 476);
end

if (EVENT == 347) then
	ShowMap(UID, 477);
end

if (EVENT == 348) then
	QuestStatusCheck = GetQuestStatus(UID, 1522) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount01 = CountMonsterQuestSub(UID, 1522, 1); --Quest Uruk Hai
	MonsterCount02 = CountMonsterQuestSub(UID, 1522, 2); --Quest Stone Golem
	MonsterCount03 = CountMonsterQuestSub(UID, 1522, 3); --Quest Troll Berserker
	MonsterCount04 = CountMonsterQuestSub(UID, 1522, 4); --Quest Apostles  
			if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 1522, 4626, NPC, 18, 344);
		elseif ( MonsterCount02 < 1) then
			SelectMsg(UID, 2, 1522, 4627, NPC, 18, 345);
		elseif ( MonsterCount03 < 1) then
			SelectMsg(UID, 2, 1522, 4628, NPC, 18, 346);
		elseif ( MonsterCount04 < 1) then
			SelectMsg(UID, 2, 1522, 4629, NPC, 18, 347);
			else
	RunQuestExchange(UID,16235) 
	SaveEvent(UID, 9827);
		end
	end
end

if (EVENT == 400) then
SelectMsg(UID, 2,-1, 44287, NPC, 40391, 401,40392,402);
end

if (EVENT == 401) then
	DARKMYTHIL = HowmuchItem(UID, 998080000);
	if (DARKMYTHIL < 5) then
SelectMsg(UID, 2,-1, 44289, NPC,65,-1);
else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	SelectMsg(UID, 3,-1, 44288, NPC,40393,-1,40394,-1,40395,-1);
	elseif (Class == 2 or Class == 7 or Class == 8) then
	SelectMsg(UID, 3,-1, 44288, NPC,40396,-1,40397,-1);
	elseif (Class == 3 or Class == 9 or Class == 10) then
	SelectMsg(UID, 3,-1, 44288, NPC,40398,-1,40399,-1);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	SelectMsg(UID, 3,-1, 44288, NPC,40400,-1,40395,-1);
		end
	end
end

if (EVENT == 402) then
	DARKMYTHIL = HowmuchItem(UID, 998080000);
	if (DARKMYTHIL < 3) then
	SelectMsg(UID, 2,-1, 44292, NPC,65,-1);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	SelectMsg(UID, 3,-1, 44291, NPC,40405,-1,40406,-1,40407,-1,40408,-1,40409,-1);
	elseif (Class == 2 or Class == 7 or Class == 8) then
	SelectMsg(UID, 3,-1, 44291, NPC,40410,-1,40411,-1,40412,-1,40413,-1,40414,-1);
	elseif (Class == 3 or Class == 9 or Class == 10) then
	SelectMsg(UID, 3,-1, 44291, NPC,40415,-1,40416,-1,40417,-1,40418,-1,40419,-1);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	SelectMsg(UID, 3,-1, 44291, NPC,40420,-1,40421,-1,40422,-1,40423,-1,40424,-1);
		end
	end
end
