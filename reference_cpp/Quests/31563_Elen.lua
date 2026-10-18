-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31563;

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
SelectMsg(UID, 4, 651, 21283, NPC, 22, 1002, 23, -1);
end

if (EVENT == 1002) then
	QuestStatus = GetQuestStatus(UID, 651)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12618);
	end
end

if (EVENT == 1006) then
	QuestStatus = GetQuestStatus(UID, 651)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900191000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 651, 21283, NPC, 18,-1);
		else
			SaveEvent(UID, 12620);
		end
	end
end

if (EVENT == 1005) then
	QuestStatus = GetQuestStatus(UID, 651)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900191000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 651, 21283, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 651, 21283, NPC, 22, 1007, 27, -1);
		end
	end
end	

if (EVENT == 1007)then
	QuestStatus = GetQuestStatus(UID, 651)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900191000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 651, 21283, NPC, 18,-1);
		else
			RunQuestExchange(UID,3136);
			SaveEvent(UID,12619);
			SaveEvent(UID,12630);
			SelectMsg(UID, 2, 651, 21736, NPC, 10,-1);
		end
	end
end

if (EVENT == 1101) then
SelectMsg(UID, 2, 652, 21285, NPC, 10,1103);
SaveEvent(UID,12630);
end

if (EVENT == 1103) then
SelectMsg(UID, 4, 652, 21285, NPC, 22, 1104, 23, -1);
SaveEvent(UID,12632);
end

if (EVENT == 1104) then
SelectMsg(UID, 2, 652, 21746, NPC, 10,-1);
SaveEvent(UID,12631);
SaveEvent(UID,12642);
end


if (EVENT == 1201) then
SelectMsg(UID, 2, 660, 21301, NPC, 10,1203);
SaveEvent(UID,12726);
end


if (EVENT == 1203) then
SelectMsg(UID, 4, 660, 21301, NPC, 22, 1204, 23, -1);
SaveEvent(UID,12728);
end

if (EVENT == 1204) then
	QuestStatus = GetQuestStatus(UID, 660)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SelectMsg(UID, 2, 660, 21844, NPC, 10,-1);
			SaveEvent(UID,12727);
			SaveEvent(UID,12733);
			RunQuestExchange(UID,3145);
	end
end

if (EVENT == 1301) then
SelectMsg(UID, 4, 662, 21304, NPC, 22, 1302, 23, -1);
end

if (EVENT == 1302) then
	QuestStatus = GetQuestStatus(UID, 662)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12745);
	end
end

if (EVENT == 1306) then
	QuestStatus = GetQuestStatus(UID, 662)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900168000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 662, 662, NPC, 18,-1);
		else
			SaveEvent(UID, 12747);
		end
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 662)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900168000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 662, 662, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 662, 662, NPC, 22, 1307, 27, -1);
		end
	end
end	

if (EVENT == 1307) then
	QuestStatus = GetQuestStatus(UID, 662)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900168000);   
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 662, 662, NPC, 18,-1);
		else
			SelectMsg(UID, 2, 662, 21862, NPC, 10,-1);
			SaveEvent(UID,12746);
			RunQuestExchange(UID,3147);
		end
	end
end