-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24440;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 411, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 411, NPC);
	else
		EVENT = QuestNum
	end
end   

if (EVENT == 111) then
	SelectMsg(UID, 4, 167, 412, NPC, 22, 112, 23, -1);
end

if (EVENT == 112) then
	QuestStatus = GetQuestStatus(UID, 167)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 276);
	end
end

if (EVENT == 114) then
	QuestStatus = GetQuestStatus(UID, 167)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 167, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 167, 412, NPC, 21, 117);
		else
			SaveEvent(UID, 278);
		end
	end
end

if (EVENT == 115) then
	QuestStatus = GetQuestStatus(UID, 167)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 167, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 167, 412, NPC, 21, 117);
		else
			SelectMsg(UID, 4, 167, 412, NPC, 41, 118, 23, 117);
		end
	end
end

if (EVENT == 117) then
	ShowMap(UID, 585);
end

if (EVENT == 118) then
	QuestStatus = GetQuestStatus(UID, 167)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 167, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 167, 412, NPC, 21, 117);
		else
			RunQuestExchange(UID, 23);
			SaveEvent(UID, 277);
		end
	end
end

if (EVENT == 120) then 
	SelectMsg(UID, 4, 168, 425, NPC, 22, 121, 23, -1);
end

if (EVENT == 121) then
	QuestStatus = GetQuestStatus(UID, 168)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 289);
	end
end

if (EVENT == 123) then
	QuestStatus = GetQuestStatus(UID, 168)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 168, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 168, 425, NPC, 21, 126);
		else
			SaveEvent(UID, 291);
		end
	end
end

if (EVENT == 124) then
	QuestStatus = GetQuestStatus(UID, 168)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 168, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 168, 425, NPC, 21, 126);
		else
			SelectMsg(UID, 4, 168, 425, NPC, 41, 127, 23, -1);
		end
	end
end

if (EVENT == 126) then
	ShowMap(UID, 186);
end

if (EVENT == 127) then
	QuestStatus = GetQuestStatus(UID, 168)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 168, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 168, 425, NPC, 21, 126);
		else
			RunQuestExchange(UID, 24);
			SaveEvent(UID, 290);
		end
	end
end


if (EVENT == 130) then 
	SelectMsg(UID, 4, 169, 430, NPC, 22, 131, 23, -1);
end

if (EVENT == 131) then
	QuestStatus = GetQuestStatus(UID, 169)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 299);
	end
end

if (EVENT == 133) then
	QuestStatus = GetQuestStatus(UID, 169)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 169, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 169, 430, NPC, 21, 136);
		else
			SaveEvent(UID, 308);
		end
	end
end

if (EVENT == 134) then
	QuestStatus = GetQuestStatus(UID, 169)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 169, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 169, 430, NPC, 21, 136);
		else
			SelectMsg(UID, 4, 169, 430, NPC, 41, 137, 23, -1);
		end
	end
end

if (EVENT == 136) then
	ShowMap(UID, 587);
end

if (EVENT == 137) then
	QuestStatus = GetQuestStatus(UID, 169)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 169, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 169, 430, NPC, 21, 136);
		else
			RunQuestExchange(UID, 25);
			SaveEvent(UID, 307);
		end
	end
end

if (EVENT == 140) then
	SelectMsg(UID, 4, 170, 441, NPC, 22, 141, 23, -1);
end

if (EVENT == 141) then
	QuestStatus = GetQuestStatus(UID, 170)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 316);
	end
end

if (EVENT == 143) then
	QuestStatus = GetQuestStatus(UID, 170)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 170, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 170, 441, NPC, 21, 146);
		else
			SaveEvent(UID, 318);
		end
	end
end

if (EVENT == 144) then
	QuestStatus = GetQuestStatus(UID, 170)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 170, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 170, 441, NPC, 21, 146);
		else
			SelectMsg(UID, 4, 170, 441, NPC, 41, 147, 23, -1);
		end
	end
end

if (EVENT == 146) then
	ShowMap(UID, 17);
end

if (EVENT == 147) then
	QuestStatus = GetQuestStatus(UID, 170)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 170, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 170, 441, NPC, 21, 146);
		else
			RunQuestExchange(UID, 26);
			SaveEvent(UID, 317);
		end
	end
end

if (EVENT == 150) then
	SelectMsg(UID, 4, 171, 446, NPC, 22, 151, 23, -1);
end

if (EVENT == 151) then
	QuestStatus = GetQuestStatus(UID, 171)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 326);
	end
end

if (EVENT == 153) then
	QuestStatus = GetQuestStatus(UID, 171)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 171, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 171, 446, NPC, 21, 156);
		else
			SaveEvent(UID, 328);
		end
	end
end

if (EVENT == 154) then
	QuestStatus = GetQuestStatus(UID, 171)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 171, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 171, 446, NPC, 21, 156);
		else
			SelectMsg(UID, 4, 171, 446, NPC, 41, 157, 23, -1);
		end
	end
end

if (EVENT == 156) then
	ShowMap(UID, 518);
end

if (EVENT == 157) then
	QuestStatus = GetQuestStatus(UID, 171)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 171, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 171, 446, NPC, 21, 156);
		else
			RunQuestExchange(UID, 27);
			SaveEvent(UID, 327);
		end
	end
end

if (EVENT == 170) then
	SelectMsg(UID, 4, 173, 454, NPC, 22, 171, 23, -1);
end

if (EVENT == 171) then
	QuestStatus = GetQuestStatus(UID, 173)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 352);
	end
end

if (EVENT == 173) then
	QuestStatus = GetQuestStatus(UID, 173)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 173, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 173, 454, NPC, 21, 176);
		else
			SaveEvent(UID, 354);
		end
	end
end

if (EVENT == 174) then
	QuestStatus = GetQuestStatus(UID, 173)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 173, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 173, 454, NPC, 21, 176);
		else
			SelectMsg(UID, 4, 173, 454, NPC, 41, 177, 23, -1);
		end
	end
end

if (EVENT == 176) then
	ShowMap(UID, 555);
end

if (EVENT == 177) then
	QuestStatus = GetQuestStatus(UID, 173)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 173, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 173, 454, NPC, 21, 176);
		else
			RunQuestExchange(UID, 58);
			SaveEvent(UID, 353);
		end
	end
end

if (EVENT == 160) then
	SelectMsg(UID, 4, 172, 446, NPC, 22, 161, 23, -1);
end

if (EVENT == 161) then
	QuestStatus = GetQuestStatus(UID, 172)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 342);
	end
end

if (EVENT == 163) then
	QuestStatus = GetQuestStatus(UID, 172)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 172, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 172, 446, NPC, 21, 166);
		else
			SaveEvent(UID, 344);
		end
	end
end

if (EVENT == 164) then
	QuestStatus = GetQuestStatus(UID, 172)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 172, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 172, 446, NPC, 21, 166);
		else
			SelectMsg(UID, 4, 172, 446, NPC, 41, 167, 23, -1);
		end
	end
end

if (EVENT == 166) then
	ShowMap(UID, 553);
end

if (EVENT == 167) then
	QuestStatus = GetQuestStatus(UID, 172)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 172, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 172, 446, NPC, 21, 166);
		else
			RunQuestExchange(UID, 57);
			SaveEvent(UID, 343);
		end
	end
end

if (EVENT == 180) then
	SelectMsg(UID, 4, 174, 464, NPC, 22, 181, 23, -1);
end

if (EVENT == 181) then
	QuestStatus = GetQuestStatus(UID, 174)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 369);
	end
end

if (EVENT == 183) then
	QuestStatus = GetQuestStatus(UID, 174)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 174, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 174, 464, NPC, 21, 186);
		else
			SaveEvent(UID, 371);
		end
	end
end

if (EVENT == 184) then
	QuestStatus = GetQuestStatus(UID, 174)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 174, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 174, 464, NPC, 21, 186);
		else
			SelectMsg(UID, 4, 174, 464, NPC, 41, 187, 23, -1);
		end
	end
end

if (EVENT == 186) then
	ShowMap(UID, 549);
end

if (EVENT == 187) then
	QuestStatus = GetQuestStatus(UID, 174)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 174, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 174, 464, NPC, 21, 186);
		else
			RunQuestExchange(UID, 59);
			SaveEvent(UID, 370);
		end
	end
end