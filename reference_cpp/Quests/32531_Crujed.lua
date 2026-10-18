-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32531;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 8986, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 8986, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 9402) then
	SelectMsg(UID, 4, 318, 8693, NPC, 22, 9403, 23, -1);
end

if (EVENT == 9403) then
	QuestStatus = GetQuestStatus(UID, 318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9436);
	end
end

if (EVENT == 9405) then
	QuestStatus = GetQuestStatus(UID, 318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount1 = CountMonsterQuestSub(UID, 318, 1);
	MonsterCount2 = CountMonsterQuestSub(UID, 318, 2);
	MonsterCount2 = CountMonsterQuestSub(UID, 318, 3);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 9408);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 1000);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 1001);
		else
			SaveEvent(UID, 9438);
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
	MonsterCount2 = CountMonsterQuestSub(UID, 318, 3);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 9408);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 1000);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 1001);
		else
			SelectMsg(UID, 4, savenum, 8693, NPC, 41, 9409, 23, -1);
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
	MonsterCount2 = CountMonsterQuestSub(UID, 318, 3);
		if (MonsterCount1 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 9408);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 1000);
		elseif (MonsterCount2 < 1) then
			SelectMsg(UID, 2, savenum, 8693, NPC, 18, 1001);
		else
			RunQuestExchange(UID, 1101);
			SaveEvent(UID, 9437);
		end
	end
end

if (EVENT == 1000) then 
	SaveEvent(UID, 11780);
end

if (EVENT == 1002) then
	SelectMsg(UID, 4, 576, 20722, NPC, 22, 1003, 23, -1);
end

if (EVENT == 1003) then
	SaveEvent(UID, 11782);
	SaveEvent(UID, 11784);
end

if (EVENT == 1005) then
	SaveEvent(UID, 11783);
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 577, 20724, NPC, 22, 1103, 23, -1);
end

if (EVENT == 1103) then 
	QuestStatus = GetQuestStatus(UID, 577)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11794);
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 577)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 577, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, -1, 20724, NPC, 18, 1106);
		else
			SaveEvent(UID, 11796);
		end
	end
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 577)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 577, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, -1, 20724, NPC, 18, 1106);
		else
			SelectMsg(UID, 4, 577, 20724, NPC, 41, 1107, 27, -1);
		end
	end
end

if (EVENT == 1106) then
	ShowMap(UID, 755);
end

if (EVENT == 1107) then
	QuestStatus = GetQuestStatus(UID, 577)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 577, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, -1, 20724, NPC, 18, 1106);
		else
			RunQuestExchange(UID,3067);
			SaveEvent(UID, 11795);
		end
	end
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 578, 20726, NPC, 22, 1203, 23, -1);
end

if (EVENT == 1203) then
	QuestStatus = GetQuestStatus(UID, 578)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
			SaveEvent(UID, 11806);
	end
end

if (EVENT == 1204) then
	QuestStatus = GetQuestStatus(UID, 578)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
	MonsterCount = CountMonsterQuestSub(UID, 578, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, -1, 20726, NPC, 18, 1206);
		else		
			SaveEvent(UID, 11808);
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
			SelectMsg(UID, 2, -1, 20726, NPC, 18, 1206);
		else
			SelectMsg(UID, 4, 578, 20726, NPC, 41, 1207, 27, -1);
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
			SelectMsg(UID, 2, -1, 20726, NPC, 18, 1206);
		else
			RunQuestExchange(UID,3068);
			SaveEvent(UID, 11807);
		end
	end
end

if (EVENT == 1302) then
	SelectMsg(UID, 4, 579, 20728, NPC, 22, 1303, 23, -1);
end

if (EVENT == 1303) then
	QuestStatus = GetQuestStatus(UID, 579)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else  
			SaveEvent(UID, 11818);
	end
end

if (EVENT == 1304) then
	QuestStatus = GetQuestStatus(UID, 579)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 579, 1);
		if (MonsterCount < 50) then
			SelectMsg(UID, 2, -1, 20728, NPC, 18, 1306);
		else		
			SaveEvent(UID, 11820);
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
			SelectMsg(UID, 2, -1, 20728, NPC, 18, 1306);
		else
			SelectMsg(UID, 4, 579, 20728, NPC, 41, 1307, 27, -1);
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
			SelectMsg(UID, 2, -1, 20728, NPC, 18, 1306);
		else
			RunQuestExchange(UID,3069);
			SaveEvent(UID, 11819);
		end
	end
end

if (EVENT == 111) then
	SelectMsg(UID, 4, 274, 8986, NPC, 22, 112, 23, -1);
end

if (EVENT == 112) then
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else 
			SaveEvent(UID, 1687);
	end
end

if (EVENT == 113) then 
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, -1, 8986, NPC, 18, 115);
		else
			SaveEvent(UID, 1689);
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
			SelectMsg(UID, 2, -1, 8986, NPC, 18, 115);
		else
			SelectMsg(UID, 4, 274, 8986, NPC, 22, 117, 23, -1);
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
			SelectMsg(UID, 2, -1, 8986, NPC, 18, 115);
		else
			RunQuestExchange(UID,3);
			SaveEvent(UID, 1688);
		end
	end
end