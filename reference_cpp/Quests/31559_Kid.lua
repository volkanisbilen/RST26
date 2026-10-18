-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31559;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then 
		SelectMsg(UID, 2, -1, 4703, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then 
		NpcMsg(UID, 8060, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1001) then
SelectMsg(UID, 4, 644, 21269, NPC, 22, 1002, 23, -1);
end

if (EVENT == 1002) then
	QuestStatus = GetQuestStatus(UID, 644)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12534);
	end
end

if (EVENT == 1006) then
	QuestStatus = GetQuestStatus(UID, 644)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900169000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 644, 21269, NPC, 18,-1);
		else
			SaveEvent(UID, 12536);
		end
	end
end

if (EVENT == 1005) then
	QuestStatus = GetQuestStatus(UID, 644)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900169000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 644, 21269, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 644, 21269, NPC, 22, 1007, 27, -1);
		end
	end
end	

if (EVENT == 1007)then
	QuestStatus = GetQuestStatus(UID, 644)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900169000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 644, 21269, NPC, 18,-1);
		else
			RunQuestExchange(UID,3129);
			SaveEvent(UID,12535);
			SaveEvent(UID,12546);
		end
	end
end


if (EVENT == 1101) then
SelectMsg(UID, 4, 646, 21273, NPC, 22, 1102, 23, -1);
end

if (EVENT == 1102) then
	QuestStatus = GetQuestStatus(UID, 646)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12558);
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 646)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900195000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 646, 21273, NPC, 18,-1);
		else
			SaveEvent(UID, 12560);
		end
	end
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 646)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900195000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 646, 21273, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 646, 21273, NPC, 22, 1107, 27, -1);
		end
	end
end	

if (EVENT == 1107)then
	QuestStatus = GetQuestStatus(UID, 646)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900195000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 646, 21273, NPC, 18,-1);
		else
			RunQuestExchange(UID,3131);
			SaveEvent(UID,12559);
			SaveEvent(UID,12570);
		end
	end
end

if (EVENT == 1201) then
SelectMsg(UID, 4, 648, 21277, NPC, 22, 1202, 23, -1);
end

if (EVENT == 1202) then
	QuestStatus = GetQuestStatus(UID, 648)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12582);
	end
end

if (EVENT == 1206) then
	QuestStatus = GetQuestStatus(UID, 648)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900193000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 648, 21277, NPC, 18,1204);
		else
			SaveEvent(UID, 12584);
		end
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 648)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900193000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 648, 21277, NPC, 18,1204);
		else
			SelectMsg(UID, 4, 648, 21277, NPC, 22, 1207, 27, -1);
		end
	end
end	

if (EVENT == 1204) then
	ShowMap(UID, 828);
end

if (EVENT == 1207)then
	QuestStatus = GetQuestStatus(UID, 648)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900193000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 648, 21277, NPC, 18,1204);
		else
			RunQuestExchange(UID,3133);
			SaveEvent(UID,12583);
			SaveEvent(UID,12594);
			SelectMsg(UID, 2, 648, 21696, NPC, 10,-1);
		end
	end
end

if (EVENT == 1301) then
SelectMsg(UID, 4, 650, 21281, NPC, 22, 1302, 23, -1);
end

if (EVENT == 1302) then
	QuestStatus = GetQuestStatus(UID, 650)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
		SaveEvent(UID, 12606);
	end
end

if (EVENT == 1306) then
	QuestStatus = GetQuestStatus(UID, 650)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900199000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 650, 21277, NPC, 18,-1);
		else
			SaveEvent(UID, 12608);
		end
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 650)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900199000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 650, 21277, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 650, 21277, NPC, 22, 1307, 27, -1);
		end
	end
end	

if (EVENT == 1307)then
	QuestStatus = GetQuestStatus(UID, 650)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900199000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 650, 21277, NPC, 18,-1);
		else
			RunQuestExchange(UID,3135);
			SaveEvent(UID,12607);
			SaveEvent(UID,12618);
		end
	end
end