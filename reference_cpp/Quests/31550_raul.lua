-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31550;

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

if (EVENT == 1001)then
	SelectMsg(UID, 2, 621, 21372, NPC, 10, 1002);
end

if (EVENT == 1002)then
	SelectMsg(UID, 2, 621, 21373, NPC, 3000, 1003,3005,-1);
	SaveEvent(UID, 12300);
end

if (EVENT == 1003)then
	SelectMsg(UID, 4, 621, 21231, NPC, 22, 1004,23,-1);
	SaveEvent(UID, 12302);
end

if (EVENT == 1004)then
	SelectMsg(UID, 2, 621, 21375, NPC, 10,-1);
	SaveEvent(UID, 12301);
	SaveEvent(UID, 12312);
	SaveEvent(UID, 12372);
end

if (EVENT == 1101) then
	SelectMsg(UID, 4, 622, 21232, NPC, 22, 1102, 27, -1);
end

if (EVENT == 1102) then
	QuestStatusCheck = GetQuestStatus(UID, 622)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12312);
	end
end

if (EVENT == 1106) then
	QuestStatusCheck = GetQuestStatus(UID, 622)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 622, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 622, 21232, NPC, 18, 1107);
		else
			SaveEvent(UID, 12314);
		end
	end
end

if (EVENT == 1105) then
	QuestStatusCheck = GetQuestStatus(UID, 622)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 622, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 622, 21232, NPC, 18, 1107);
		else
			SelectMsg(UID, 4, 622, 21232, NPC, 22, 1108, 23, -1);
		end
	end
end

if (EVENT == 1107) then
	ShowMap(UID, 822);
end

if (EVENT == 1108) then
	QuestStatusCheck = GetQuestStatus(UID, 622)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 622, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 622, 21232, NPC, 18, 1107);
		else
			SelectMsg(UID, 2, 622, 21403, NPC, 10, -1);
			RunQuestExchange(UID,13111);
			SaveEvent(UID, 12313);
			SaveEvent(UID, 12324);
		end
	end
end

if (EVENT == 1201) then
	SelectMsg(UID, 4, 624, 21234, NPC, 22, 1202, 27, -1);
end

if (EVENT == 1202) then
	QuestStatusCheck = GetQuestStatus(UID, 624)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12324);
	end
end

if (EVENT == 1206) then
	QuestStatusCheck = GetQuestStatus(UID, 624)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 624, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 624, 21234, NPC, 18, 1207);
		else
			SaveEvent(UID, 12326);
		end
	end
end

if (EVENT == 1205) then
	QuestStatusCheck = GetQuestStatus(UID, 624)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 624, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 624, 21234, NPC, 18, 1207);
		else
			SelectMsg(UID, 4, 624, 21234, NPC, 22, 1208, 23, -1);
		end
	end
end

if (EVENT == 1207) then
	ShowMap(UID, 824);
end

if (EVENT == 1208) then
	QuestStatusCheck = GetQuestStatus(UID, 624)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 624, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 624, 21234, NPC, 18, 1207);
		else
			RunQuestExchange(UID,13112);
			SaveEvent(UID, 12325);
			SaveEvent(UID, 12336);
		end
	end
end

if (EVENT == 1301) then
	SelectMsg(UID, 4, 626, 21236, NPC, 22, 1302, 27, -1);
end

if (EVENT == 1302) then
	QuestStatusCheck = GetQuestStatus(UID, 626)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12336);
	end
end

if (EVENT == 1306) then
	QuestStatusCheck = GetQuestStatus(UID, 626)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 626, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 626, 21236, NPC, 18, 1307);
		else
			SaveEvent(UID, 12338);
		end
	end
end

if (EVENT == 1305) then
	QuestStatusCheck = GetQuestStatus(UID, 626)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 626, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 626, 21236, NPC, 18, 1307);
		else
			SelectMsg(UID, 4, 626, 21236, NPC, 22, 1308, 23, -1);
		end
	end
end

if (EVENT == 1307) then
	ShowMap(UID, 826);
end

if (EVENT == 1308) then
	QuestStatusCheck = GetQuestStatus(UID, 626)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 626, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 626, 21236, NPC, 18, 1307);
		else
			RunQuestExchange(UID,13113);
			SaveEvent(UID, 12337);
			SaveEvent(UID, 12348);
		end
	end
end

if (EVENT == 1401) then
	SelectMsg(UID, 4, 628, 21238, NPC, 22, 1402, 27, -1);
end

if (EVENT == 1402) then
	QuestStatusCheck = GetQuestStatus(UID, 628)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12348);
	end
end

if (EVENT == 1406) then
	QuestStatusCheck = GetQuestStatus(UID, 628)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 628, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 628, 21238, NPC, 18, 1407);
		else
			SaveEvent(UID, 12350);
		end
	end
end

if (EVENT == 1405) then
	QuestStatusCheck = GetQuestStatus(UID, 628)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 628, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 628, 21238, NPC, 18, 1407);
		else
			SelectMsg(UID, 4, 628, 21238, NPC, 22, 1408, 23, -1);
		end
	end
end

if (EVENT == 1407) then
	ShowMap(UID, 825);
end

if (EVENT == 1408) then
	QuestStatusCheck = GetQuestStatus(UID, 628)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 628, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 628, 21238, NPC, 18, 1407);
		else
			RunQuestExchange(UID,13114);
			SaveEvent(UID, 12349);
			SaveEvent(UID, 12360);
		end
	end
end