-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25000;

if (EVENT == 200) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43647, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43647, NPC)
	else
		EVENT = QuestNum
	end
end

if(EVENT == 102) then 
	SelectMsg(UID, 4, 1200, 8244, NPC, 22, 106, 23, -1);
end

if(EVENT == 106) then
	QuestStatus = GetQuestStatus(UID, 1200)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7310);
	end
end

if(EVENT == 107) then
	QuestStatus = GetQuestStatus(UID, 1200)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CountA = HowmuchItem(UID, 810418000)
		if( CountA < 2) then
			SelectMsg(UID, 2, 1200, 8267, NPC, 18, 108);
		else
			SaveEvent(UID, 7312);
		end
	end
end

if(EVENT == 105) then
	QuestStatus = GetQuestStatus(UID, 1200)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CountA = HowmuchItem(UID, 810418000)
		if( CountA < 2) then
			SelectMsg(UID, 2, 1200, 8267, NPC, 18, 108);
		else
			SelectMsg(UID, 4, 1200, 8268, NPC, 41, 110, 27, -1);
		end
	end
end

if(EVENT == 108) then
	ShowMap(UID, 1);
end

if(EVENT == 110) then
	QuestStatus = GetQuestStatus(UID, 1200)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CountA = HowmuchItem(UID, 810418000)
		if( CountA < 2) then
			SelectMsg(UID, 2, 1200, 8267, NPC, 18, 108);
		else
			RunQuestExchange(UID,6000);
			SaveEvent(UID, 7311);
		end
	end
end

if(EVENT == 112) then
	SelectMsg(UID, 4, 1201, 8244, NPC, 22, 114, 23, -1);
end

if(EVENT == 114) then
	QuestStatus = GetQuestStatus(UID, 1201)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7317);
	end
end

if(EVENT == 117) then
	QuestStatus = GetQuestStatus(UID, 1201)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CountA = HowmuchItem(UID, 810418000)
		if( CountA < 2) then
			SelectMsg(UID, 2, 1201, 8267, NPC, 18, 118);
		else
			SaveEvent(UID, 7319);
		end
	end
end

if(EVENT == 115) then
	QuestStatus = GetQuestStatus(UID, 1201)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CountA = HowmuchItem(UID, 810418000)
		if( CountA < 2) then
			SelectMsg(UID, 2, 1201, 8267, NPC, 18, 118);
		else
			SelectMsg(UID, 4, 1201, 8268, NPC, 41, 120, 27, -1);
		end
	end
end

if (EVENT == 118 ) then
	ShowMap(UID, 2)
end

if(EVENT == 120) then
	QuestStatus = GetQuestStatus(UID, 1201)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	CountA = HowmuchItem(UID, 810418000)
		if( CountA < 2) then
			SelectMsg(UID, 2, 1201, 8267, NPC, 18, 118);
		else
			RunQuestExchange(UID, 6001);
			SaveEvent(UID, 7318);
		end
	end
end

if(EVENT == 122) then
	SelectMsg(UID, 4, 1202, 8244, NPC, 22, 124, 23, -1);
end

if(EVENT == 124) then
	QuestStatus = GetQuestStatus(UID, 1202)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 7324);
	end
end

if(EVENT == 128) then
	QuestStatus = GetQuestStatus(UID, 1202)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount  = CountMonsterQuestSub(UID, 1202, 1);
		if (MonsterCount < 5) then
			SelectMsg(UID, 2, 1202, 8267, NPC, 18, 129);
		else
			SaveEvent(UID, 7326);
		end
	end
end

if(EVENT == 125) then
	MonsterCount  = CountMonsterQuestSub(UID, 1202, 1);
		if (MonsterCount < 5) then
			SelectMsg(UID, 2, 1202, 8267, NPC, 18, 129);
		else
			SelectMsg(UID, 4, 1202, 8268, NPC, 41, 130, 27, -1);
	end
end

if(EVENT == 129) then
	ShowMap(UID, 2)
end

if(EVENT == 130) then
	QuestStatus = GetQuestStatus(UID, 1202)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			RunQuestExchange(UID, 6002);
			SaveEvent(UID, 7325);
	end
end

if(EVENT == 132) then
SelectMsg(UID, 2, 1203, 43627, NPC, 40147, 133);
end

if(EVENT == 133) then
	SelectMsg(UID, 4, 1203, 43627, NPC, 22, 134, 23, -1);
end

if(EVENT == 134) then	
SaveEvent(UID, 7330);
SaveEvent(UID, 7335);
SaveEvent(UID, 7340);
end

if(EVENT == 137) then
SaveEvent(UID, 7332)
end

if (EVENT == 135) then
	ITEM1_COUNT = HowmuchItem(UID, 900600000);
    ITEM2_COUNT = HowmuchItem(UID, 900601000);
    ITEM3_COUNT = HowmuchItem(UID, 900603000);
    ITEM4_COUNT = HowmuchItem(UID, 900604000);
	if (ITEM1_COUNT > 0 and ITEM2_COUNT > 0 and ITEM3_COUNT > 0 and ITEM4_COUNT > 0) then
	SelectMsg(UID, 4, 1203, 43627, NPC, 41, 141, 27, -1);
	else
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 1203, 43627, NPC, 18, 136);
		elseif (ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 1203, 43627, NPC, 18, 138);
		elseif (ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 1203, 43627, NPC, 18, 139);
		elseif (ITEM4_COUNT < 1) then
			SelectMsg(UID, 2, 1203, 43627, NPC, 18, 140);
		end
	end
end

if (EVENT == 136) then
	ShowMap(UID, 4);
end
if (EVENT == 138) then
	ShowMap(UID, 5);
end
if (EVENT == 139) then
	ShowMap(UID, 2);
end
if (EVENT == 140) then
	ShowMap(UID, 3);
end

if(EVENT == 141) then
	ITEM1_COUNT = HowmuchItem(UID, 900600000);
    ITEM2_COUNT = HowmuchItem(UID, 900601000);
    ITEM3_COUNT = HowmuchItem(UID, 900603000);
    ITEM4_COUNT = HowmuchItem(UID, 900604000);
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 1203, 43627, NPC, 18, 136);
		elseif (ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 1203, 43627, NPC, 18, 138);
		elseif (ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 1203, 43627, NPC, 18, 139);
		elseif (ITEM4_COUNT < 1) then
			SelectMsg(UID, 2, 1203, 43627, NPC, 18, 140);
		else
			RunQuestExchange(UID, 6003);
			SaveEvent(UID, 7331);
	end
end

if(EVENT == 142) then
	SelectMsg(UID, 2, 1209, 43647, NPC, 40150, 143);
end

if(EVENT == 143) then
	SelectMsg(UID, 2, 1209, 43648, NPC, 22, 144,23,-1);
end

if(EVENT == 144) then
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
			GiveItem(UID, 900599000, 1);
		    SaveEvent(UID, 7364);
		 	SaveEvent(UID, 7368);
			SelectMsg(UID, 2, 1209, 43675, NPC, 10, 113);
	end
end