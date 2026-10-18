-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32532;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 8996, NPC, 10, 101);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 8996, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 9402) then
	SelectMsg(UID, 4, 318, 8692, NPC, 22, 9403, 23, -1);
end

if (EVENT == 9403) then
	QuestStatus = GetQuestStatus(UID, 318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9442);
	end
end

if (EVENT == 9404) then
	QuestStatus = GetQuestStatus(UID, 318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 318, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 318, 2);
	MonsterCount3 = CountMonsterQuestSub(UID, 318, 3);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 9408);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 1000);
		elseif (MonsterCount3 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 1001);
		else
			SaveEvent(UID, 9445);
		end
	end
end

if (EVENT == 9407) then
	QuestStatus = GetQuestStatus(UID, 318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 318, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 318, 2);
	MonsterCount3 = CountMonsterQuestSub(UID, 318, 3);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 9408);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 1000);
		elseif (MonsterCount3 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 1001);
		else
			SelectMsg(UID, 4, 318, 8692, NPC, 41, 9409, 23, -1);
		end
	end
end

if (EVENT == 9408) then
	ShowMap(UID, 614);
end

if (EVENT == 1000) then
	ShowMap(UID, 613);
end

if (EVENT == 1001) then
	ShowMap(UID, 612);
end

if (EVENT == 9409) then
	QuestStatus = GetQuestStatus(UID, 318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 318, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 318, 2);
	MonsterCount3 = CountMonsterQuestSub(UID, 318, 3);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 9408);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 1000);
		elseif (MonsterCount3 < 1) then
			SelectMsg(UID, 2, -1, 8692, NPC, 18, 1001);
		else
			RunQuestExchange(UID, 1101);
			SaveEvent(UID, 9443);
		end
	end
end

if (EVENT == 1002) then
	SelectMsg(UID, 4, 576, 20723, NPC, 22, 1003, 23, -1);
end

if (EVENT == 1003) then
	SaveEvent(UID, 11788);
	SaveEvent(UID, 11790);
end

if (EVENT == 1005) then
	SaveEvent(UID, 11789);
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 577, 20725, NPC, 22, 1103, 23, -1);
end

if (EVENT == 1103) then 
	QuestStatus = GetQuestStatus(UID, 577)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11800);
		end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 577)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 577, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, -1, 20725, NPC, 18, 1108);
		else
			SaveEvent(UID, 11802);
		end
	end
end

if (EVENT == 1108) then
	ShowMap(UID, 755);
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 577)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 577, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, -1, 20725, NPC, 18, 1108);
		else
			SelectMsg(UID, 4, 577, 20725, NPC, 41, 1107, 27, -1);
		end
	end
end

if (EVENT == 1107) then
	QuestStatus = GetQuestStatus(UID, 577)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 577, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, -1, 20725, NPC, 18, 1108);
		else
			RunQuestExchange(UID,3067);
			SaveEvent(UID, 11801);
		end
	end
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 578, 20727, NPC, 22, 1203, 23, -1);
end

if (EVENT == 1203) then
	QuestStatus = GetQuestStatus(UID, 578)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
			SaveEvent(UID, 11812);
	end
end

if (EVENT == 1204) then 
	QuestStatus = GetQuestStatus(UID, 578)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
	MonsterCount = CountMonsterQuestSub(UID, 578, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, -1, 20727, NPC, 18, 1206);
		else
			SaveEvent(UID, 11814);
		end
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 578)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
	MonsterCount = CountMonsterQuestSub(UID, 578, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, -1, 20727, NPC, 18, 1206);
		else
			SelectMsg(UID, 4, 578, 20727, NPC, 41, 1207, 27, -1);
		end
	end
end

if (EVENT == 1206) then
	ShowMap(UID, 756);
end

if (EVENT == 1207) then
	QuestStatus = GetQuestStatus(UID, 578)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
	MonsterCount = CountMonsterQuestSub(UID, 578, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, -1, 20727, NPC, 18, 1206);
		else
			RunQuestExchange(UID,3068);
			SaveEvent(UID, 11813);
		end
	end
end

if (EVENT == 1302) then
	SelectMsg(UID, 4, 579, 20729, NPC, 22, 1303, 23, -1);
end

if (EVENT == 1303) then
	QuestStatus = GetQuestStatus(UID, 579)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else  
	SaveEvent(UID, 11824);
	end
end

if (EVENT == 1304) then
	QuestStatus = GetQuestStatus(UID, 579)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
	MonsterCount = CountMonsterQuestSub(UID, 579, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, -1, 20729, NPC, 18, 1306);
		else		
			SaveEvent(UID, 11826);
		end
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 579)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
	MonsterCount = CountMonsterQuestSub(UID, 579, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, -1, 20729, NPC, 18, 1306);
		else
			SelectMsg(UID, 4, 579, 20729, NPC, 41, 1307, 27, -1);
		end
	end
end

if (EVENT == 1306) then
	ShowMap(UID, 757);
end

if (EVENT == 1307) then
	QuestStatus = GetQuestStatus(UID, 579)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
	MonsterCount = CountMonsterQuestSub(UID, 579, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, -1, 20729, NPC, 18, 1306);
		else
			RunQuestExchange(UID,3069);
			SaveEvent(UID, 11825);
		end
	end
end

if (EVENT == 111) then
	SelectMsg(UID, 4, 274, 8996, NPC, 22, 112, 23, -1);
end

if (EVENT == 112) then
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else  
			SaveEvent(UID, 1693);
	end
end

if (EVENT == 113) then 
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, -1, 8996, NPC, 18, 115);
		else
			SaveEvent(UID, 1695);
		end
	end
end

if (EVENT == 116) then
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, -1, 8996, NPC, 18, 115);
		else
			SelectMsg(UID, 4, 274, 8996, NPC, 22, 117, 23, -1);
		end
	end
end

if (EVENT == 115) then
	ShowMap(UID, 757);
end

if (EVENT == 117) then
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, -1, 8996, NPC, 18, 115);
		else
			RunQuestExchange(UID,18);
			SaveEvent(UID, 1694);
		end
	end
end