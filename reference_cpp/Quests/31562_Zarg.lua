-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31562;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 21310, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 21310, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1001) then
	SelectMsg(UID, 2, 663, 21882, NPC, 10, 1002);
end

if (EVENT == 1002) then
SaveEvent(UID, 12751);
	SelectMsg(UID, 2, 663, 21883, NPC, 3000, 1003,3005,-1);
end

if (EVENT == 1003) then
	SelectMsg(UID, 4, 663, 21883, NPC, 3000, 1004,3005,-1);
	SaveEvent(UID, 12753);
end

if (EVENT == 1004) then
	SaveEvent(UID, 12752);
	SaveEvent(UID, 12763);
end

if (EVENT == 1101) then
	SelectMsg(UID, 4, 664, 21308, NPC, 22, 1102, 27, -1);
end

if (EVENT == 1102) then
	QuestStatus = GetQuestStatus(UID, 664)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
		SaveEvent(UID, 12763);
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 664)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 370004000);   
	ITEM1_COUNT2 = HowmuchItem(UID, 168210003); 
	ITEM1_COUNT3 = HowmuchItem(UID, 391010000); 
		if (ITEM1_COUNT1 < 10 and ITEM1_COUNT2 < 1 and ITEM1_COUNT3 < 5000) then
			SelectMsg(UID, 2, 664, 21308, NPC, 18,-1);
		else
			SaveEvent(UID, 12765);
		end
	end
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 664)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 370004000);   
	ITEM1_COUNT2 = HowmuchItem(UID, 168210003); 
	ITEM1_COUNT3 = HowmuchItem(UID, 391010000); 
		if (ITEM1_COUNT1 < 10 and ITEM1_COUNT2 < 1 and ITEM1_COUNT3 < 5000) then
			SelectMsg(UID, 2, 664, 21308, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 664, 21308, NPC, 22, 1107,27, -1);
		end
	end
end	

if (EVENT == 1107)then
	QuestStatus = GetQuestStatus(UID, 664)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 370004000);   
	ITEM1_COUNT2 = HowmuchItem(UID, 168210003); 
	ITEM1_COUNT3 = HowmuchItem(UID, 391010000); 
		if (ITEM1_COUNT1 < 10 and ITEM1_COUNT2 < 1 and ITEM1_COUNT3 < 5000) then
			SelectMsg(UID, 2, 664, 21308, NPC, 18,-1);
		else
			SelectMsg(UID, 2, 664, 21897, NPC, 10,-1);
			RunQuestExchange(UID,3149);
			SaveEvent(UID,12764);
			SaveEvent(UID,12781);
		end
	end
end


if (EVENT == 1201) then
	SelectMsg(UID, 4, 665, 21310, NPC, 22, 1202, 27, -1);
end


if (EVENT == 1202) then
	QuestStatus = GetQuestStatus(UID, 665)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12775);
	end
end

if (EVENT == 1206) then
	QuestStatus = GetQuestStatus(UID, 665)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 665, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 665, 21310, NPC, 18, 1207);
		else
			SaveEvent(UID, 12777);
		end
	end
end
	
if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 665)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 665, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 665, 21310, NPC, 18, 1207);
		else
			SelectMsg(UID, 4, 665, 21310, NPC, 22, 1208, 23, -1);
		end
	end
end

if (EVENT == 1207) then
	ShowMap(UID, 545);
end

if (EVENT == 1208)then
	QuestStatus = GetQuestStatus(UID, 665)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 665, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 665, 21310, NPC, 18, 1207);
		else
			RunQuestExchange(UID,13150);
			SaveEvent(UID,12776);
			SaveEvent(UID,12793);
		end
	end
end

if (EVENT == 1301) then
	SelectMsg(UID, 4, 667, 21312, NPC, 22, 1302, 27, -1);
end

if (EVENT == 1302) then
	QuestStatus = GetQuestStatus(UID, 667)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12787);
	end
end

if (EVENT == 1306) then
	QuestStatus = GetQuestStatus(UID, 667)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 667, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 667, 21312, NPC, 18, 1307);
		else
			SaveEvent(UID, 12789);
		end
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 667)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 667, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 667, 21312, NPC, 18, 1307);
		else
			SelectMsg(UID, 4, 667, 21312, NPC, 22, 1308, 23, -1);
		end
	end
end

if (EVENT == 1307) then
	ShowMap(UID, 344);
end

if (EVENT == 1308)then
	QuestStatus = GetQuestStatus(UID, 667)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 667, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 667, 21312, NPC, 18, 1307);
		else
			SelectMsg(UID, 2, 667, 21925, NPC, 10, -1);
			RunQuestExchange(UID,13151);
			SaveEvent(UID,12788);
			SaveEvent(UID,12799);
		end
	end
end

if (EVENT == 1401) then
	SelectMsg(UID, 4, 669, 21314, NPC, 22, 1402, 27, -1);
end

if (EVENT == 1402) then
	QuestStatus = GetQuestStatus(UID, 669)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12799);
	end
end

if (EVENT == 1406) then
	QuestStatus = GetQuestStatus(UID, 669)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 669, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 669, 21314, NPC, 18, 1407);
		else
			SaveEvent(UID, 12801);
		end
	end
end
	
if (EVENT == 1405) then
	QuestStatus = GetQuestStatus(UID, 669)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 669, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 669, 21314, NPC, 18, 1407);
		else
			SelectMsg(UID, 4, 669, 21314, NPC, 22, 1408, 23, -1);
		end
	end
end

if (EVENT == 1407) then
	ShowMap(UID, 827);
end

if (EVENT == 1408)then
	QuestStatus = GetQuestStatus(UID, 669)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 669, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 669, 21314, NPC, 18, 1407);
		else
			SelectMsg(UID, 2, 669, 21939, NPC, 10, -1);
			RunQuestExchange(UID,13152);
			SaveEvent(UID,12800);
			SaveEvent(UID,12811);
		end
	end
end

if (EVENT == 1501) then
	SelectMsg(UID, 4, 671, 21316, NPC, 22, 1502, 27, -1);
end

if (EVENT == 1502) then
	QuestStatus = GetQuestStatus(UID, 671)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12811);
	end
end

if (EVENT == 1506) then
	QuestStatus = GetQuestStatus(UID, 671)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 671, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 671, 21316, NPC, 18, 1507);
		else
			SaveEvent(UID, 12813);
		end
	end
end
	
if (EVENT == 1505) then
	QuestStatus = GetQuestStatus(UID, 671)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 671, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 671, 21316, NPC, 18, 1507);
		else
			SelectMsg(UID, 4, 671, 21316, NPC, 22, 1508, 23, -1);
		end
	end
end

if (EVENT == 1507) then
	ShowMap(UID, 819);
end

if (EVENT == 1508)then
	QuestStatus = GetQuestStatus(UID, 671)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 671, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 671, 21316, NPC, 18, 1507);
		else
			RunQuestExchange(UID,13153);
			SaveEvent(UID,12812);
			SaveEvent(UID,12823);
		end
	end
end

if (EVENT == 1601) then
	SelectMsg(UID, 4, 673, 21318, NPC, 22, 1602, 27, -1);
end

if (EVENT == 1602) then
	QuestStatus = GetQuestStatus(UID, 673)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12823);
	end
end

if (EVENT == 1606) then
	QuestStatus = GetQuestStatus(UID, 673)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 673, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 673, 21318, NPC, 18, 1607);
		else
			SaveEvent(UID, 12825);
		end
	end
end
	
if (EVENT == 1605) then
	QuestStatus = GetQuestStatus(UID, 673)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 673, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 673, 21318, NPC, 18, 1607);
		else
			SelectMsg(UID, 4, 673, 21318, NPC, 22, 1608, 23, -1);
		end
	end
end

if (EVENT == 1607) then
	ShowMap(UID, 681);
end

if (EVENT == 1608)then
	QuestStatus = GetQuestStatus(UID, 673)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 673, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, 673, 21318, NPC, 18, 1607);
		else
			SelectMsg(UID, 2, 673, 21965, NPC, 10, -1);
			SaveEvent(UID,12824);
			RunQuestExchange(UID,13154);
		end
	end
end