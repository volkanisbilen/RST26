-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 15895;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 723, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 723, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 110) then
	SaveEvent(UID, 20074);
end

if (EVENT == 112) then
	SelectMsg(UID, 4, 1604, 723, NPC, 22, 113, 23, -1);
end

if (EVENT == 113) then
	SaveEvent(UID, 20075);
end

if (EVENT == 120) then
	SaveEvent(UID, 20077);
end

if (EVENT == 116) then
	MonsterCount = CountMonsterQuestSub(UID, 1604, 1);
	if (MonsterCount < 0) then
		SelectMsg(UID, 2, 1604, 723, NPC, 18, 117);
	else
		SelectMsg(UID, 4, 1604, 723, NPC, 22, 118, 23, -1);
	end
end

if (EVENT == 117) then
	ShowMap(UID, 488);
end

if (EVENT == 118) then
	QuestStatusCheck = GetQuestStatus(UID, 1604) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 1604, 1);
	if (MonsterCount < 0) then
		SelectMsg(UID, 2, 1604, 723, NPC, 18, 117);
	else
    RunQuestExchange(UID,22048)
	SaveEvent(UID, 20076);
end
end
end

--2.Wing Görevi 1000 Kills

if (EVENT == 130) then
	SaveEvent(UID, 20080);
end

if (EVENT == 132) then
	SelectMsg(UID, 4, 1605, 723, NPC, 22, 133, 23, -1);
end

if (EVENT == 133) then
	SaveEvent(UID, 20081);
end

if (EVENT == 140) then
	SaveEvent(UID, 20083);
end

if (EVENT == 136) then
	MonsterCount = CountMonsterQuestSub(UID, 1605, 1);
	if (MonsterCount < 0) then
		SelectMsg(UID, 2, 1605, 723, NPC, 18, 137);
	else
		SelectMsg(UID, 4, 1605, 723, NPC, 22, 138, 23, -1);
	end
end

if (EVENT == 137) then
	ShowMap(UID, 488);
end

if (EVENT == 138) then
	QuestStatusCheck = GetQuestStatus(UID, 1605) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 1605, 1);
	if (MonsterCount < 0) then
		SelectMsg(UID, 2, 1605, 723, NPC, 18, 137);
	else
    RunQuestExchange(UID,22049)
	SaveEvent(UID, 20082);
end
end
end

--3.Wing ve Pathos Görevi 3000 Kills

if (EVENT == 150) then
	SaveEvent(UID, 20086);
end

if (EVENT == 152) then
	SelectMsg(UID, 4, 1606, 723, NPC, 22, 153, 23, -1);
end

if (EVENT == 153) then
	SaveEvent(UID, 20087);
end

if (EVENT == 160) then
	SaveEvent(UID, 20089);
end

if (EVENT == 156) then
	MonsterCount = CountMonsterQuestSub(UID, 1606, 1);
	if (MonsterCount < 0) then
		SelectMsg(UID, 2, 1606, 723, NPC, 18, 157);
	else
		SelectMsg(UID, 4, 1606, 723, NPC, 22, 158, 23, -1);
	end
end

if (EVENT == 157) then
	ShowMap(UID, 489);
end

if (EVENT == 158) then
	QuestStatusCheck = GetQuestStatus(UID, 1606) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 1606, 1);
	if (MonsterCount < 0) then
		SelectMsg(UID, 2, 1606, 723, NPC, 18, 157);
	else
    RunQuestExchange(UID,22050)
	SaveEvent(UID, 20088);
end
end
end
