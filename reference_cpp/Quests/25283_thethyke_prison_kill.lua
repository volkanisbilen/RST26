-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25283;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 44152, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 44152, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 112) then
	SelectMsg(UID, 4, 1607, 44152, NPC, 22, 113, 23, -1);
end

if (EVENT == 113) then
	QuestStatus = GetQuestStatus(UID, 1607)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 20096);
	end
end

if (EVENT == 120) then
	QuestStatus = GetQuestStatus(UID, 1607)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1607, 1);
		if (MonsterCount < 0) then
			SelectMsg(UID, 2, 1607 ,44152, NPC, 18, 117);
		else
			SaveEvent(UID, 20098);
		end
	end
end

if (EVENT == 116) then
	QuestStatus = GetQuestStatus(UID, 1607)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1607, 1);
		if (MonsterCount < 0) then
			SelectMsg(UID, 2, 1607 ,44152, NPC, 18, 117);
		else
			SelectMsg(UID, 4, 1607, 44152, NPC, 4172, 118, 23, -1);
		end
	end
end

if (EVENT == 117) then
	ShowMap(UID, 488);
end

if (EVENT == 118) then
	QuestStatus = GetQuestStatus(UID, 1607)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1607, 1);
		if (MonsterCount < 0) then
			SelectMsg(UID, 2, 1607 ,44152, NPC, 18, 117);
		else
			RunQuestExchange(UID,6250);
			SaveEvent(UID, 20097);
		end
	end
end

if (EVENT == 132) then
	SelectMsg(UID, 4, 1608, 44152, NPC, 22, 133, 23, -1);
end

if (EVENT == 133) then
	QuestStatus = GetQuestStatus(UID, 1608)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 20102);
	end
end

if (EVENT == 140) then
	QuestStatus = GetQuestStatus(UID, 1608)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1 = HowmuchItem(UID, 0);
	ITEM2 = HowmuchItem(UID, 0);
	ITEM3 = HowmuchItem(UID, 0);
	ITEM4 = HowmuchItem(UID, 0);
		if (ITEM1 < 0 and ITEM2 < 0 and ITEM3 < 0 and ITEM4 < 0) then
			SelectMsg(UID, 2, 1608 ,44152, NPC, 18, 137);
		else
			SaveEvent(UID, 20104);
		end
	end
end

if (EVENT == 136) then
	QuestStatus = GetQuestStatus(UID, 1608)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1 = HowmuchItem(UID, 0);
	ITEM2 = HowmuchItem(UID, 0);
	ITEM3 = HowmuchItem(UID, 0);
	ITEM4 = HowmuchItem(UID, 0);
		if (ITEM1 < 0 and ITEM2 < 0 and ITEM3 < 0 and ITEM4 < 0) then
			SelectMsg(UID, 2, 1608 ,44152, NPC, 18, 137);
		else
			SelectMsg(UID, 4, 1608, 44152, NPC, 4172, 138, 23, -1);
		end
	end
end

if (EVENT == 137) then
	ShowMap(UID, 488);
end

if (EVENT == 138) then
	QuestStatus = GetQuestStatus(UID, 1608)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1 = HowmuchItem(UID, 0);
	ITEM2 = HowmuchItem(UID, 0);
	ITEM3 = HowmuchItem(UID, 0);
	ITEM4 = HowmuchItem(UID, 0);
		if (ITEM1 < 0 and ITEM2 < 0 and ITEM3 < 0 and ITEM4 < 0) then
			SelectMsg(UID, 2, 1608 ,44152, NPC, 18, 137);
		else
			RunQuestExchange(UID,6251);
			SaveEvent(UID, 20103);
		end
	end
end

if (EVENT == 152) then
	SelectMsg(UID, 4, 1609, 44152, NPC, 22, 153, 23, -1);
end

if (EVENT == 153) then
	QuestStatus = GetQuestStatus(UID, 1609)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 20108);
	end
end

if (EVENT == 160) then
	QuestStatus = GetQuestStatus(UID, 1609)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1609, 1);
		if (MonsterCount < 0) then
			SelectMsg(UID, 2, 1609 ,44152, NPC, 18, 157);
		else
			SaveEvent(UID, 20110);
		end
	end
end

if (EVENT == 156) then
	QuestStatus = GetQuestStatus(UID, 1609)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1609, 1);
		if (MonsterCount < 0) then
			SelectMsg(UID, 2, 1609 ,44152, NPC, 18, 157);
		else
			SelectMsg(UID, 4, 1609, 44152, NPC, 4172, 158, 23, -1);
		end
	end
end

if (EVENT == 157) then
	ShowMap(UID, 488);
end

if (EVENT == 158) then
	QuestStatus = GetQuestStatus(UID, 1609)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1609, 1);
		if (MonsterCount < 0) then
			SelectMsg(UID, 2, 1609 ,44152, NPC, 18, 157);
		else
			RunQuestExchange(UID,6252);
			SaveEvent(UID, 20109);
		end
	end
end


if (EVENT == 172) then
	SelectMsg(UID, 4, 1610, 44152, NPC, 22, 173, 23, -1);
end

if (EVENT == 173) then
	QuestStatus = GetQuestStatus(UID, 1610)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 20114);
	end
end

if (EVENT == 180) then
	QuestStatus = GetQuestStatus(UID, 1610)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 1610, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 1610, 2);
		if (MonsterCount1 < 0 and MonsterCount2 < 0) then
			SelectMsg(UID, 2, 1610 ,44152, NPC, 18, 177);
		else
			SaveEvent(UID, 20116);
		end
	end
end

if (EVENT == 176) then
	QuestStatus = GetQuestStatus(UID, 1610)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 1610, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 1610, 2);
		if (MonsterCount1 < 0 and MonsterCount2 < 0) then
			SelectMsg(UID, 2, 1610 ,44152, NPC, 18, 177);
		else
			SelectMsg(UID, 4, 1610, 44152, NPC, 4172, 178, 23, -1);
		end
	end
end

if (EVENT == 177) then
	ShowMap(UID, 488);
end

if (EVENT == 178) then
	QuestStatus = GetQuestStatus(UID, 1610)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 1610, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 1610, 2);
		if (MonsterCount1 < 0 and MonsterCount2 < 0) then
			SelectMsg(UID, 2, 1610 ,44152, NPC, 18, 177);
		else
			RunQuestExchange(UID,6253);
			SaveEvent(UID, 20115);
		end
	end
end


if (EVENT == 192) then
	SelectMsg(UID, 4, 1611, 44152, NPC, 22, 193, 23, -1);
end

if (EVENT == 193) then
	QuestStatus = GetQuestStatus(UID, 1611)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 20120);
	end
end

if (EVENT == 200) then
	QuestStatus = GetQuestStatus(UID, 1611)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 1611, 1);
		if (MonsterCount1 < 0) then
			SelectMsg(UID, 2, 1611 ,44152, NPC, 18, 197);
		else	
			SaveEvent(UID, 20122);
		end
	end
end

if (EVENT == 196) then
	QuestStatus = GetQuestStatus(UID, 1611)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 1611, 1);
		if (MonsterCount1 < 0) then
			SelectMsg(UID, 2, 1611 ,44152, NPC, 18, 197);
		else
			SelectMsg(UID, 4, 1611, 44152, NPC, 4172, 198, 23, -1);
		end
	end
end

if (EVENT == 197) then
	ShowMap(UID, 488);
end

if (EVENT == 198) then
	QuestStatus = GetQuestStatus(UID, 1611)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 1611, 1);
		if (MonsterCount1 < 0) then
			SelectMsg(UID, 2, 1611 ,44152, NPC, 18, 197);
		else
			RunQuestExchange(UID,6254);
			SaveEvent(UID, 20121);
		end
	end
end