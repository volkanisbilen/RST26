-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14407;

if (EVENT == 100) then
	QuestStatusCheck = GetQuestStatus(UID, 789)	
	if(QuestStatusCheck == 3) then
		SelectMsg(UID, 2, 788, 23165, NPC, 10, 1908,4005,-1);
		else
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then 
		SelectMsg(UID, 2, -1, 4273, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then 
		NpcMsg(UID, 1, NPC)
	else
		EVENT = QuestNum
	end
end
end

if(EVENT == 1501) then
	SelectMsg(UID, 4, 778, 22983, NPC, 3000, 1502,3005,-1);
	
end

if(EVENT == 1502) then
	SaveEvent(UID, 13693);

end

if(EVENT == 1503) then
	SelectMsg(UID, 4, 778, 22983, NPC, 10, 1504,4005,-1);
	
end

if(EVENT == 1504) then
	SaveEvent(UID, 13695);
	SaveEvent(UID, 13694);
	SaveEvent(UID, 13705);

end

if(EVENT == 1601) then
	SelectMsg(UID, 4, 779, 23044, NPC, 3000, 1602,3005,-1);
end

if(EVENT == 1602) then
	SaveEvent(UID, 13705);
end

if(EVENT == 1606) then
	SaveEvent(UID, 13707);
end

if(EVENT == 1605 ) then
	COUNTA = HowmuchItem(UID, 508214000)	
	if(COUNTA > 3) then
		SelectMsg(UID, 4, 779, 23044, NPC, 41, 1603, 27, -1);
	else
		SelectMsg(UID, 2, 779, 23044, NPC, 10, -1);
	end
end

if(EVENT == 1603 ) then
	QuestStatusCheck = GetQuestStatus(UID, 779) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	COUNTA = HowmuchItem(UID, 508214000)	
		if(COUNTA > 3) then
			RunQuestExchange(UID, 3227)
			SaveEvent(UID, 13706);
			SaveEvent(UID, 13717);	
		else		
			SelectMsg(UID, 2, 779, 23044, NPC, 10, -1);
		end
	end
end


if (EVENT == 1301) then
	SelectMsg(UID, 4, 630, 21241, NPC, 22, 1302, 27, -1);
end

if (EVENT == 1302) then
	SaveEvent(UID, 12366);
end

if (EVENT == 1306) then
	SaveEvent(UID, 12368);
end

if(EVENT == 1305 ) then
	COUNTA = HowmuchItem(UID, 900197000)	
	if(COUNTA > 0) then
		SelectMsg(UID, 4, 630, 21241, NPC, 41, 1307, 27, -1);
	else
		SelectMsg(UID, 2, 630, 21241, NPC, 10, -1);
	end
end

if(EVENT == 1307 ) then
	QuestStatusCheck = GetQuestStatus(UID, 630) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	COUNTA = HowmuchItem(UID, 900197000)	
	if(COUNTA > 0) then
		RunQuestExchange(UID, 3115)
		SaveEvent(UID, 12367)
	else
		SelectMsg(UID, 2, 630, 21241, NPC, 10, -1);		
end
end
end

if (EVENT == 1401) then
	SelectMsg(UID, 4, 641, 21263, NPC, 22, 1402, 27, -1);
end

if (EVENT == 1402) then
	SaveEvent(UID, 12498);
end

if (EVENT == 1406) then
	SaveEvent(UID, 12500);
end

if(EVENT == 1405 ) then
	COUNTA = HowmuchItem(UID, 900192000)	
	if(COUNTA > 0) then
		SelectMsg(UID, 4, 641, 21263, NPC, 41, 1407, 27, -1);
	else
		SelectMsg(UID, 2, 641, 21263, NPC, 10, -1);
	end
end

if(EVENT == 1407 ) then
	QuestStatusCheck = GetQuestStatus(UID, 641) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	COUNTA = HowmuchItem(UID, 900192000)	
	if(COUNTA > 0) then
		RunQuestExchange(UID, 3126)
		SaveEvent(UID, 12499);
	else
		SelectMsg(UID, 2, 641, 21263, NPC, 10, -1);		
end
end
end

if (EVENT == 1701) then
	SelectMsg(UID, 4, 782, 22990, NPC, 22, 1702, 27, -1);
end

if (EVENT == 1702) then
	SaveEvent(UID, 13741);
end

if (EVENT == 1706) then
	SaveEvent(UID, 13743);
end

if(EVENT == 1705) then
	ITEMA = HowmuchItem(UID, 508215000)	
	if(ITEMA < 4) then
		SelectMsg(UID, 2, 782, 22990, NPC, 18, 1707);
	else
		SelectMsg(UID, 4, 782, 22990, NPC, 22, 1708,23,-1);
	end
end

if (EVENT == 1707 ) then
	ShowMap(UID, 58)
end

if(EVENT == 1708 ) then
	QuestStatusCheck = GetQuestStatus(UID, 782) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 508215000)	
	if(ITEMA < 4) then
		SelectMsg(UID, 2, 782, 22990, NPC, 18, 1707);
	else
		RunQuestExchange(UID, 3230)
		SaveEvent(UID, 13742);
		SaveEvent(UID, 13753);
end
end
end

if (EVENT == 1801) then
    SelectMsg(UID, 2, 785, 22996, NPC, 22, 1802);
end

if (EVENT == 1802) then
SaveEvent(UID, 13777);
end

if (EVENT == 1803) then
	SelectMsg(UID, 4, 785, 22996, NPC, 22, 1804, 27, -1);
	SaveEvent(UID, 13779);
end

if (EVENT == 1804) then
	QuestStatusCheck = GetQuestStatus(UID, 785) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	SelectMsg(UID, 2, 785, 23127, NPC, 10, -1);
	SaveEvent(UID, 13778);
	SaveEvent(UID, 13789);
	RunQuestExchange(UID, 3233)
end
end

if (EVENT == 1901) then
	SelectMsg(UID, 4, 788, 23002, NPC, 22, 1902, 27, -1);
end

if (EVENT == 1902) then
	SaveEvent(UID, 13813);
end

if (EVENT == 1906) then
	SaveEvent(UID, 13815);
end

if(EVENT == 1905) then
	ITEMA = HowmuchItem(UID, 900326000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 788, 23002, NPC, 18, -1);
	else
		SelectMsg(UID, 4, 788, 23002, NPC, 22, 1907,23,-1);
	end
end

if (EVENT == 1907) then
	QuestStatusCheck = GetQuestStatus(UID, 788) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 900326000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 788, 23002, NPC, 18, -1);
	else
	SelectMsg(UID, 2, 788, 23165, NPC, 10, 1908,4005,-1);
	SaveEvent(UID, 13814);
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
	SaveEvent(UID, 13851);
end

if (EVENT == 2006) then
	SaveEvent(UID, 13853);
end

if(EVENT == 2005) then
	ITEMA = HowmuchItem(UID, 900330000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 792, 23002, NPC, 18, -1);
	else
		SelectMsg(UID, 4, 792, 23002, NPC, 22, 2007,23,-1);
	end
end

if (EVENT == 2007) then
	QuestStatusCheck = GetQuestStatus(UID, 792) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 900330000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 792, 23002, NPC, 18, -1);
	else
	SelectMsg(UID, 2, 792, 23213, NPC, 10, -1);
	SaveEvent(UID, 13852);
	SaveEvent(UID, 13863);
	RunQuestExchange(UID, 3240);
	GiveItem(UID, 900335523);
end
end
end


if (EVENT == 2101) then
	SelectMsg(UID, 4, 794, 23011, NPC, 22, 2102, 27, -1);
end

if (EVENT == 2102) then
	SaveEvent(UID, 13863);
end

if (EVENT == 2106) then
	SaveEvent(UID, 13865);
end


if (EVENT == 2105) then
	MonsterCount = CountMonsterQuestSub(UID, 794, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 794, 23011, NPC, 18, 2107);
	else
		SelectMsg(UID, 4, 794, 23011, NPC, 41, 2108, 27, -1);
	end
end

if (EVENT == 2107 ) then
	ShowMap(UID, 14)
end

if (EVENT == 2108) then
	QuestStatusCheck = GetQuestStatus(UID, 794) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 794, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 794, 23011, NPC, 18, 2107);
	else
	SelectMsg(UID, 2, 794, 23226, NPC, 10, -1);
	SaveEvent(UID, 13864);
	SaveEvent(UID, 13875);
	RunQuestExchange(UID, 3241);
	GiveItem(UID, 900336524);
	RobItem(UID, 900335523);
end
end
end

if (EVENT == 2201) then
	SelectMsg(UID, 4, 796, 23013, NPC, 22, 2202, 27, -1);
end

if (EVENT == 2202) then
	SaveEvent(UID, 13875);
end

if (EVENT == 2206) then
	SaveEvent(UID, 13877);
end

if (EVENT == 2205) then
	MonsterCount = CountMonsterQuestSub(UID, 796, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 796, 23013, NPC, 18, 2207);
	else
		SelectMsg(UID, 4, 796, 23013, NPC, 41, 2208, 27, -1);
	end
end

if (EVENT == 2207 ) then
	ShowMap(UID, 112)
end

if (EVENT == 2208) then
	MonsterCount = CountMonsterQuestSub(UID, 796, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 796, 23013, NPC, 18, 2207);
	else
	SelectMsg(UID, 2, 796, 23013, NPC, 10, -1);
	SaveEvent(UID, 13876);
	SaveEvent(UID, 13887);
	RunQuestExchange(UID, 3242);
	RobItem(UID, 900336524);
end
end

if (EVENT == 2301) then
	SelectMsg(UID, 4, 799, 23017, NPC, 22, 2302, 27, -1);
end

if (EVENT == 2302) then
	SaveEvent(UID, 13899);
end

if (EVENT == 2306) then
	SaveEvent(UID, 13901);
end

if (EVENT == 2305) then
	MonsterCount = CountMonsterQuestSub(UID, 799, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 799, 23017, NPC, 18, 2307);
	else
		SelectMsg(UID, 4, 799, 23017, NPC, 41, 2308, 27, -1);
	end
end

if (EVENT == 2307 ) then
	ShowMap(UID, 36)
end

if (EVENT == 2308) then
	QuestStatusCheck = GetQuestStatus(UID, 799) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 799, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 799, 23017, NPC, 18, 2307);
	else
	ShowMap(UID, 1343);
	SaveEvent(UID, 13900);
	SaveEvent(UID, 13911);
	RunQuestExchange(UID, 3244);
	RobItem(UID, 900337525);
end
end
end


if (EVENT == 2401)then
	SelectMsg(UID, 2, 800, 23018, NPC, 3000, 2403);
	SaveEvent(UID, 13911);
end


if (EVENT == 2403)then
	SelectMsg(UID, 4, 800, 23018, NPC, 3000, 2404,3005,-1);
	SaveEvent(UID, 13913);
end

if (EVENT == 2404)then
	SelectMsg(UID, 2, 800, 23018, NPC, 10, -1);
	SaveEvent(UID, 13912);
	SaveEvent(UID, 13923);
end

if (EVENT == 2501) then
	SelectMsg(UID, 4, 803, 23025, NPC, 22, 2502, 27, -1);
end

if (EVENT == 2502) then
	SaveEvent(UID, 13947);
end

if (EVENT == 2506) then
	SaveEvent(UID, 13949);
end

if(EVENT == 2505) then
	ITEMA = HowmuchItem(UID, 900333000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 803, 23025, NPC, 18,-1);
	else
		SelectMsg(UID, 4, 803, 23025, NPC, 22, 2507,23,-1);
	end
end

if (EVENT == 2507 ) then
	QuestStatusCheck = GetQuestStatus(UID, 803) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 900333000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 803, 23025, NPC, 18,-1);
	else
SelectMsg(UID, 2, -1, 23303, NPC, 10, -1);
RunQuestExchange(UID,3248)
SaveEvent(UID, 13948);
SaveEvent(UID, 13959);
end
end
end

if (EVENT == 2601)then
	SelectMsg(UID, 2, 804, 3249, NPC, 3000, 2603);
	SaveEvent(UID, 13959);
end

if (EVENT == 2603)then
	SelectMsg(UID, 4, 804, 3249, NPC, 3000, 2604,3005,-1);
	SaveEvent(UID, 13961);
end

if (EVENT == 2604)then
	QuestStatusCheck = GetQuestStatus(UID, 804) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	RunQuestExchange(UID,3249)
	SaveEvent(UID, 13960);
	SaveEvent(UID, 13971);
end
end