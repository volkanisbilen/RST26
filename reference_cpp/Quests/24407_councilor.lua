-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC= 24407;

if (EVENT == 100) then
	QuestStatusCheck = GetQuestStatus(UID, 789)	
	if(QuestStatusCheck == 3) then
		SelectMsg(UID, 2, 788, 23165, NPC, 10, 1908,4005,-1);
		else
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then 
		SelectMsg(UID, 2, -1, 4178, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then 
		NpcMsg(UID, 4174, NPC)
	else
		EVENT = QuestNum
	end
end
end

if(EVENT == 1501) then
	SelectMsg(UID, 4, 778, 23043, NPC, 3000, 1502,3005,-1);
end

if(EVENT == 1502) then
	SaveEvent(UID, 13687);
end

if(EVENT == 1503) then
	SelectMsg(UID, 4, 778, 23043, NPC, 10, 1504,4005,-1);
end

if(EVENT == 1504) then
	SaveEvent(UID, 13689);
	SaveEvent(UID, 13688);
	SaveEvent(UID, 13699);
end

if(EVENT == 1601) then
	SelectMsg(UID, 4, 779, 23044, NPC, 3000, 1602,3005,-1);
end

if(EVENT == 1602) then
	QuestStatus = GetQuestStatus(UID, 779)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13699);
	end
end

if(EVENT == 1606) then
	QuestStatus = GetQuestStatus(UID, 779)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 508214000)	
		if(COUNTA < 4) then
			SelectMsg(UID, 2, 779, 23044, NPC, 18, 1604);
		else
			SaveEvent(UID, 13701);
		end
	end
end

if(EVENT == 1605 ) then
	QuestStatus = GetQuestStatus(UID, 779)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 508214000)	
		if(COUNTA < 4) then
			SelectMsg(UID, 2, 779, 23044, NPC, 18, 1604);
		else
			SelectMsg(UID, 4, 779, 23044, NPC, 41, 1603, 27, -1);
		end
	end
end

if (EVENT == 1604) then
	ShowMap(UID, 514);
end

if(EVENT == 1603 ) then
	QuestStatus = GetQuestStatus(UID, 779)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 508214000)	
		if(COUNTA < 4) then
			SelectMsg(UID, 2, 779, 23044, NPC, 18, 1604);
		else
			RunQuestExchange(UID, 3227);
			SaveEvent(UID, 13700);
			SaveEvent(UID, 13711);
		end
	end
end

if (EVENT == 1301) then
	SelectMsg(UID, 4, 630, 21240, NPC, 22, 1302, 27, -1);
end

if (EVENT == 1302) then
	QuestStatus = GetQuestStatus(UID, 630)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12360);
	end
end

if (EVENT == 1306) then
	QuestStatus = GetQuestStatus(UID, 630)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 900197000)	
		if(COUNTA < 1) then
			SelectMsg(UID, 2, 630, 21240, NPC, 10, -1);
		else
			SaveEvent(UID, 12362);
		end
	end
end

if(EVENT == 1305 ) then
	QuestStatus = GetQuestStatus(UID, 630)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 900197000)	
		if(COUNTA < 1) then
			SelectMsg(UID, 2, 630, 21240, NPC, 10, -1);
		else
			SelectMsg(UID, 4, 630, 21240, NPC, 41, 1307, 27, -1);
		end
	end
end

if(EVENT == 1307 ) then
	QuestStatus = GetQuestStatus(UID, 630)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 900197000)	
		if(COUNTA < 1) then
			SelectMsg(UID, 2, 630, 21240, NPC, 10, -1);
		else
			RunQuestExchange(UID, 3115);
			SaveEvent(UID, 12361);
		end
	end
end

if (EVENT == 1401) then
	SelectMsg(UID, 4, 641, 21262, NPC, 22, 1402, 27, -1);
end

if (EVENT == 1402) then
	QuestStatus = GetQuestStatus(UID, 641)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12492);
	end
end

if (EVENT == 1406) then
	QuestStatus = GetQuestStatus(UID, 641)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 900192000)	
		if(COUNTA < 1) then
			SelectMsg(UID, 2, 641, 21262, NPC, 10, -1);
		else
			SaveEvent(UID, 12494);
		end
	end
end

if(EVENT == 1405 ) then
	QuestStatus = GetQuestStatus(UID, 641)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 900192000)	
		if(COUNTA < 1) then
			SelectMsg(UID, 2, 641, 21262, NPC, 10, -1);
		else
			SelectMsg(UID, 4, 641, 21262, NPC, 41, 1407, 27, -1);
		end
	end
end

if(EVENT == 1407 ) then
	QuestStatus = GetQuestStatus(UID, 641)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 900192000)	
		if(COUNTA < 1) then
			SelectMsg(UID, 2, 641, 21262, NPC, 10, -1);
		else
			RunQuestExchange(UID, 3126);
			SaveEvent(UID, 12493);
		end
	end
end

if (EVENT == 1701) then
	SelectMsg(UID, 4, 782, 22990, NPC, 22, 1702, 27, -1);
end

if (EVENT == 1702) then
	QuestStatus = GetQuestStatus(UID, 782)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13735);
	end
end

if (EVENT == 1706) then
	QuestStatus = GetQuestStatus(UID, 782)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 508215000)	
		if(ITEMA < 4) then
			SelectMsg(UID, 2, 782, 22990, NPC, 18, 1707);
		else
			SaveEvent(UID, 13737);
		end
	end
end

if(EVENT == 1705) then
	QuestStatus = GetQuestStatus(UID, 782)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 508215000)	
		if(ITEMA < 4) then
			SelectMsg(UID, 2, 782, 22990, NPC, 18, 1707);
		else
			SelectMsg(UID, 4, 782, 22990, NPC, 22, 1708,23,-1);
		end
	end
end

if (EVENT == 1707 ) then
	ShowMap(UID, 58)
end

if(EVENT == 1708 ) then
	QuestStatus = GetQuestStatus(UID, 782)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 508215000)	
		if(ITEMA < 4) then
			SelectMsg(UID, 2, 782, 22990, NPC, 18, 1707);
		else
			RunQuestExchange(UID, 3230);
			SaveEvent(UID, 13736);
			SaveEvent(UID, 13747);
		end
	end
end

if (EVENT == 1801) then
    SelectMsg(UID, 2, 785, 22996, NPC, 22, 1802);
end

if (EVENT == 1802) then
	QuestStatus = GetQuestStatus(UID, 785)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13771);
	end
end

if (EVENT == 1803) then
	SelectMsg(UID, 4, 785, 22996, NPC, 22, 1804, 27, -1);
	SaveEvent(UID, 13773);
end

if (EVENT == 1804) then
	QuestStatus = GetQuestStatus(UID, 785)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SelectMsg(UID, 2, 785, 23127, NPC, 10, -1);
			SaveEvent(UID, 13772);
			SaveEvent(UID, 13783);
			RunQuestExchange(UID, 3233);
	end
end

if (EVENT == 1901) then
	SelectMsg(UID, 4, 788, 23002, NPC, 22, 1902, 27, -1);
end

if (EVENT == 1902) then
	QuestStatus = GetQuestStatus(UID, 788)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13807);
	end
end

if (EVENT == 1906) then
	QuestStatus = GetQuestStatus(UID, 788)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900326000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 788, 23002, NPC, 18, -1);
		else
			SaveEvent(UID, 13809);
		end
	end
end

if(EVENT == 1905) then
	QuestStatus = GetQuestStatus(UID, 788)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900326000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 788, 23002, NPC, 18, -1);
		else
			SelectMsg(UID, 4, 788, 23002, NPC, 22, 1907,23,-1);
		end
	end
end

if (EVENT == 1907) then
	QuestStatus = GetQuestStatus(UID, 788)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900326000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 788, 23002, NPC, 18, -1);
		else
			SelectMsg(UID, 2, 788, 23165, NPC, 10, 1908,4005,-1);
			SaveEvent(UID, 13808);
			SaveEvent(UID, 13820);
			RunQuestExchange(UID, 3236);
		end
	end
end

if (EVENT == 1908)then
MonsterStoneQuestJoin(UID,790);
end

if (EVENT == 2001) then
	SelectMsg(UID, 4, 792, 23008, NPC, 22, 2002, 27, -1);
end

if (EVENT == 2002) then
	QuestStatus = GetQuestStatus(UID, 792)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13845);
	end
end

if (EVENT == 2006) then
	QuestStatus = GetQuestStatus(UID, 792)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900330000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 792, 23002, NPC, 18, -1);
		else
			SaveEvent(UID, 13847);
		end
	end
end

if(EVENT == 2005) then
	QuestStatus = GetQuestStatus(UID, 792)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900330000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 792, 23002, NPC, 18, -1);
		else
			SelectMsg(UID, 4, 792, 23002, NPC, 22, 2007,23,-1);
		end
	end
end

if (EVENT == 2007) then
	QuestStatus = GetQuestStatus(UID, 792)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900330000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 792, 23002, NPC, 18, -1);
		else
			SelectMsg(UID, 2, 792, 23213, NPC, 10, -1);
			SaveEvent(UID, 13846);
			SaveEvent(UID, 13857);
			RunQuestExchange(UID, 3240);
			GiveItem(UID, 900335523);
		end
	end
end

if (EVENT == 2101) then
	SelectMsg(UID, 4, 793, 23010, NPC, 22, 2102, 27, -1);
end

if (EVENT == 2102) then
	QuestStatus = GetQuestStatus(UID, 793)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13857);
	end
end

if (EVENT == 2106) then
	QuestStatus = GetQuestStatus(UID, 793)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 793, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 793, 23010, NPC, 18, 2107);
		else
			SaveEvent(UID, 13859);
		end
	end
end

if (EVENT == 2105) then
	QuestStatus = GetQuestStatus(UID, 793)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 793, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 793, 23010, NPC, 18, 2107);
		else
			SelectMsg(UID, 4, 793, 23010, NPC, 41, 2108, 27, -1);
		end
	end
end

if (EVENT == 2107 ) then
	ShowMap(UID, 344)
end

if (EVENT == 2108) then
	QuestStatus = GetQuestStatus(UID, 793)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 793, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 793, 23010, NPC, 18, 2107);
		else
			SelectMsg(UID, 2, 793, 23226, NPC, 10, -1);
			SaveEvent(UID, 13858);
			SaveEvent(UID, 13869);
			RunQuestExchange(UID, 3241);
			GiveItem(UID, 900336524);
			RobItem(UID, 900335523);
		end
	end
end

if (EVENT == 2201) then
	SelectMsg(UID, 4, 795, 23012, NPC, 22, 2202, 27, -1);
end

if (EVENT == 2202) then
	QuestStatus = GetQuestStatus(UID, 795)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13869);
	end
end

if (EVENT == 2206) then
	QuestStatus = GetQuestStatus(UID, 795)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13871);
	end
end

if (EVENT == 2205) then
	QuestStatus = GetQuestStatus(UID, 795)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 795, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 795, 23012, NPC, 18, 2207);
		else
			SelectMsg(UID, 4, 795, 23012, NPC, 41, 2208, 27, -1);
		end
	end
end

if (EVENT == 2207 ) then
	ShowMap(UID, 111)
end

if (EVENT == 2208) then
	QuestStatus = GetQuestStatus(UID, 795)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 795, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 795, 23012, NPC, 18, 2207);
		else
			SelectMsg(UID, 2, 795, 23012, NPC, 10, -1);
			SaveEvent(UID, 13870);
			SaveEvent(UID, 13881);
			RunQuestExchange(UID, 3242);
			RobItem(UID, 900336524);
		end
	end
end

if (EVENT == 2301) then
	SelectMsg(UID, 4, 798, 23016, NPC, 22, 2302, 27, -1);
end

if (EVENT == 2302) then
	QuestStatus = GetQuestStatus(UID, 798)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13893);
	end
end

if (EVENT == 2306) then
	QuestStatus = GetQuestStatus(UID, 798)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 798, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 798, 23016, NPC, 18, 2307);
		else
			SaveEvent(UID, 13895);
		end
	end
end

if (EVENT == 2305) then
	QuestStatus = GetQuestStatus(UID, 798)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 798, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 798, 23016, NPC, 18, 2307);
		else
			SelectMsg(UID, 4, 798, 23016, NPC, 41, 2308, 27, -1);
		end
	end
end

if (EVENT == 2307 ) then
	ShowMap(UID, 37)
end

if (EVENT == 2308) then
	QuestStatus = GetQuestStatus(UID, 798)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 798, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 798, 23016, NPC, 18, 2307);
		else
			ShowMap(UID, 1332);
			SaveEvent(UID, 13894);
			SaveEvent(UID, 13905);
			RunQuestExchange(UID, 3244);
			RobItem(UID, 900337525);
		end
	end
end

if (EVENT == 2401)then
	SelectMsg(UID, 2, 800, 23018, NPC, 3000, 2403);
	SaveEvent(UID, 13905);
end


if (EVENT == 2403)then
	SelectMsg(UID, 4, 800, 23018, NPC, 3000, 2404,3005,-1);
	SaveEvent(UID, 13907);
end

if (EVENT == 2404)then
	SelectMsg(UID, 2, 800, 23018, NPC, 10, -1);
	SaveEvent(UID, 13906);
	SaveEvent(UID, 13917);
end

if (EVENT == 2501) then
	SelectMsg(UID, 4, 803, 23024, NPC, 22, 2502, 27, -1);
end

if (EVENT == 2502) then
	QuestStatus = GetQuestStatus(UID, 803)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 13941);
	end
end

if (EVENT == 2506) then
	QuestStatus = GetQuestStatus(UID, 803)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900333000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 803, 23024, NPC, 18,-1);
		else
			SaveEvent(UID, 13943);
		end
	end
end

if(EVENT == 2505) then
	QuestStatus = GetQuestStatus(UID, 803)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900333000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 803, 23024, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 803, 23024, NPC, 22, 2507,23,-1);
		end
	end
end

if (EVENT == 2507 ) then
	QuestStatus = GetQuestStatus(UID, 803)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900333000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 803, 23024, NPC, 18,-1);
		else
			SelectMsg(UID, 2, -1, 23303, NPC, 10, -1);
			RunQuestExchange(UID,3248);
			SaveEvent(UID, 13942);
			SaveEvent(UID, 13953);
		end
	end
end

if (EVENT == 2601)then
	SelectMsg(UID, 2, 804, 3249, NPC, 3000, 2603);
	SaveEvent(UID, 13953);
end

if (EVENT == 2603)then
	SelectMsg(UID, 4, 804, 3249, NPC, 3000, 2604,3005,-1);
	SaveEvent(UID, 13955);
end

if (EVENT == 2604)then
	QuestStatus = GetQuestStatus(UID, 804)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			RunQuestExchange(UID,3249);
			SaveEvent(UID, 13954);
			SaveEvent(UID, 13965);
	end
end