-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24410;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 680, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 681, NPC)
	else 
		EVENT = QuestNum
	end
end

if (EVENT == 200) then
	SelectMsg(UID, 4, 202, 1280, NPC, 22, 202, 23, -1);
end

if (EVENT == 202) then
	QuestStatus = GetQuestStatus(UID, 202)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 605);
	end
end

if (EVENT == 205) then
	QuestStatus = GetQuestStatus(UID, 202)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 379204000);   
		if (ITEM_COUNT < 2) then
			SelectMsg(UID, 2, 202, 1280, NPC, 18, 213);
		else
			SaveEvent(UID, 607);
	NATION = CheckNation(UID);
		if (NATION == 1) then
			SelectMsg(UID, 2, 202, 1280, NPC, 32, -1);
		else
			SelectMsg(UID, 2, 202, 1280, NPC, 21, -1);
			end
		end
	end
end

if (EVENT == 210) then
	QuestStatus = GetQuestStatus(UID, 202)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 379204000);   
		if (ITEM_COUNT < 2) then
			SelectMsg(UID, 2, 202, 1280, NPC, 18, 213);
		else
			SelectMsg(UID, 4, 202, 1280, NPC, 41, 214, 27, -1); 
		end
	end
end

if (EVENT == 213) then
	ShowMap(UID, 344);
end

if (EVENT == 214) then
	QuestStatus = GetQuestStatus(UID, 202)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Check = isRoomForItem(UID, 389620000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			RunQuestExchange(UID,90);
			SaveEvent(UID, 606);
		end
	end
end

if (EVENT == 1001) then
SelectMsg(UID, 4, 519, 20143, NPC, 22, 1002, 23, -1); 
end

if (EVENT == 1002) then
	QuestStatus = GetQuestStatus(UID, 519)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11074);
	end
end

if (EVENT == 1006) then
	QuestStatus = GetQuestStatus(UID, 519)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910209000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 519, 20143, NPC, 18, -1);
		else
			SaveEvent(UID, 11076);
		end
	end
end

if (EVENT == 1003) then
	QuestStatus = GetQuestStatus(UID, 519)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910209000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 519, 20143, NPC, 18, -1);
		else
			SelectMsg(UID, 4, 519, 20143, NPC, 22, 1005, 27, -1); 
		end
	end
end

if (EVENT == 1005) then
	QuestStatus = GetQuestStatus(UID, 519)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910209000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 519, 20143, NPC, 18, -1);
		else
			RunQuestExchange(UID,3006);
			SaveEvent(UID, 11075);
			SaveEvent(UID, 11080);
		end
	end
end