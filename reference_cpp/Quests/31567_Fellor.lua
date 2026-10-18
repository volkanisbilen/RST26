-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31567;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 23029, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 23029, NPC)
	else
		EVENT = QuestNum
	end
	end
	
if (EVENT == 6101)then
SelectMsg(UID, 2, 771, 22259, NPC, 10,6102);
end

if (EVENT == 6102)then
SaveEvent(UID, 13621);
end

if (EVENT == 6103)then
SelectMsg(UID, 4, 771, 22259, NPC, 3000,6104,3005,-1);
SaveEvent(UID, 13623);
end

if (EVENT == 6104)then
SaveEvent(UID, 13622);
SaveEvent(UID, 13633);
end

if (EVENT == 6201)then
	SelectMsg(UID, 4, 772, 22261, NPC, 3000,6202,4005,-1);
end

if (EVENT == 6202)then
	QuestStatus = GetQuestStatus(UID, 772)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13633);
	end
end

if (EVENT == 6206)then
	QuestStatus = GetQuestStatus(UID, 772)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900291000);  
    ITEM2_COUNT = HowmuchItem(UID, 900292000); 
    ITEM3_COUNT = HowmuchItem(UID, 900293000); 	
		if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, -1, 22261, NPC, 10, -1);
		else
			SaveEvent(UID, 13635);
		end
	end
end

if (EVENT == 6205)then
	QuestStatus = GetQuestStatus(UID, 772)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900291000);  
    ITEM2_COUNT = HowmuchItem(UID, 900292000); 
    ITEM3_COUNT = HowmuchItem(UID, 900293000); 	
		if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, -1, 22261, NPC, 10, -1);
		else
			SelectMsg(UID, 4, 772, 22261, NPC,3000,6207,3005,-1);
		end
	end
end

if (EVENT == 6207) then
	QuestStatus = GetQuestStatus(UID, 772)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900291000);  
    ITEM2_COUNT = HowmuchItem(UID, 900292000); 
    ITEM3_COUNT = HowmuchItem(UID, 900293000); 	
		if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, -1, 22261, NPC, 10, -1);
		else
			SaveEvent(UID, 13634);
			SaveEvent(UID, 13645);
			RunQuestExchange(UID,3221);
			SelectMsg(UID, 2, -1, 22157, NPC, 10, -1);
		end
	end
end

if (EVENT == 6601)then
	SelectMsg(UID, 4, 777, 22269, NPC, 3000,6602,4005,-1);
end

if (EVENT == 6602)then
	QuestStatus = GetQuestStatus(UID, 777)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13681);
	end
end

if (EVENT == 6606)then
	QuestStatus = GetQuestStatus(UID, 777)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900296000);  
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, -1, 22269, NPC, 10, -1);
		else
			SaveEvent(UID, 13683);
		end
	end
end

if (EVENT == 6605)then
	QuestStatus = GetQuestStatus(UID, 777)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900296000);  
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, -1, 22269, NPC, 10, -1);
		else
			SelectMsg(UID, 4, 777, 22269, NPC,3000,6607,3005,-1);
		end
	end
end

if (EVENT == 6607) then
	QuestStatus = GetQuestStatus(UID, 777)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900296000);  
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, -1, 22269, NPC, 10, -1);
		else
			SaveEvent(UID, 13682);
			RunQuestExchange(UID,3225);
		end
	end
end