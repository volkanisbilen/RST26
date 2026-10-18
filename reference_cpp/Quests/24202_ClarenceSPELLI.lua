-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24202;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 3825, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 3825,NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 703) then
	SelectMsg(UID, 2, 243, 3131, NPC, 10, 704);
end

if (EVENT == 704) then
	SelectMsg(UID, 4, 243, 3132, NPC, 22, 705, 23, -1);
end

if (EVENT == 705) then
	SaveEvent(UID, 3293);
end

if (EVENT == 706) then
	SaveEvent(UID, 3295);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 243, 3136, NPC, 3015, -1);
	else
		SelectMsg(UID, 2, 243, 3006, NPC, 3015, -1);
	end
end

if (EVENT == 708) then
	ITEM_COUNT = HowmuchItem(UID, 910082000);
	if (ITEM_COUNT == 0) then
		SelectMsg(UID, 2, 243, 3135, NPC, 18, 709);
	else
		SelectMsg(UID, 4, 243, 3137, NPC, 41, 710, 27, -1);
	end
end

if (EVENT == 709) then
	ShowMap(UID, 315);
end

if (EVENT == 710) then
	ITEM_COUNT = HowmuchItem(UID, 910082000);
		if (ITEM_COUNT == 0) then
			SelectMsg(UID, 2, 243, 3135, NPC, 18, 709);
		else
			RunCountExchange(UID,318);
			SaveEvent(UID, 3296); 
	end
end

local savenum = 273;

if (EVENT == 532) then
	Level = CheckLevel(UID)
	if (Level > 59) then 
		Class = CheckClass (UID);
		if (Class == 7) then
			SelectMsg(UID, 4, 273, 4098, NPC, 22, 533, 23, -1);
		else
			SelectMsg(UID, 2, 273, 4097, NPC, 10, -1);
		end
	else
		SelectMsg(UID, 2, 273, 4096, NPC, 10, -1);
	end
end

if (EVENT == 533) then
	SaveEvent(UID, 4094);
end

if (EVENT == 535) then
	SaveEvent(UID, 4096);
	SelectMsg(UID, 2, 273, 4102, NPC, 4080, -1);
end

if (EVENT == 536) then
	ITEM_COUNTA = HowmuchItem(UID, 810095000);
	ITEM_COUNTB = HowmuchItem(UID, 810092000);
	ITEM_COUNTC = HowmuchItem(UID, 810093000);
	if (ITEM_COUNTA > 0 and ITEM_COUNTB > 0 and ITEM_COUNTC > 0) then
		SelectMsg(UID, 4, 273, 4103, NPC, 4062, 537, 4063, -1);
	else
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 538);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 539);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 540);
		end
	end
end

if (EVENT == 538) then
	ShowMap(UID, 188);
end

if (EVENT == 539) then
	ShowMap(UID, 712);
end

if (EVENT == 540) then
	ShowMap(UID, 710);
end

if (EVENT == 537) then
	ITEM_COUNTA = HowmuchItem(UID, 810095000);
	ITEM_COUNTB = HowmuchItem(UID, 810092000);
	ITEM_COUNTC = HowmuchItem(UID, 810093000);
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 538);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 539);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 540);
		else
			RunQuestExchange(UID,462);
			PromoteUser(UID);
			SaveEvent(UID, 4095);
			SelectMsg(UID, 2, 273, 4093, NPC, 4064, -1);
	end
end

if (EVENT == 303) then 
	SelectMsg(UID, 2, 331, 3096, NPC, 3011, 304);
end

if (EVENT == 304) then
	SelectMsg(UID, 4, 331, 3097, NPC, 22, 305, 23, -1);
end

if (EVENT == 305) then
	SaveEvent(UID, 3233);
end

if (EVENT == 306) then
	SaveEvent(UID, 3235);
	SelectMsg(UID, 2, 331, 3102, NPC, 21, -1);
end

if (EVENT == 308) then
	ITEM_COUNTA = HowmuchItem(UID, 910042000);
	ITEM_COUNTB = HowmuchItem(UID, 379040000);
	ITEM_COUNTC = HowmuchItem(UID, 379236000);
	if (ITEM_COUNTA > 4 and ITEM_COUNTB > 0 and ITEM_COUNTC > 1) then
		SelectMsg(UID, 4, 331, 3103, NPC, 41, 310, 27, -1);
	else
		if (ITEM_COUNTA < 5) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 309);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 311);
		elseif (ITEM_COUNTC < 2) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 312);
		end
	end
end

if (EVENT == 309) then
	ShowMap(UID, 324);
end

if (EVENT == 311) then
	ShowMap(UID, 28);
end

if (EVENT == 312) then
	ShowMap(UID, 18);
end

if (EVENT == 310) then
	ITEM_COUNTA = HowmuchItem(UID, 910042000);
	ITEM_COUNTB = HowmuchItem(UID, 379040000);
	ITEM_COUNTC = HowmuchItem(UID, 379236000);
		if (ITEM_COUNTA < 5) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 309);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 311);
		elseif (ITEM_COUNTC < 2) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 312);
		else
			RunQuestExchange(UID,314);
			SaveEvent(UID, 3236);
	end
end

if (EVENT == 403) then
	SelectMsg(UID, 2, 332, 3104, NPC, 10, 404);
end

if (EVENT == 404) then
	SelectMsg(UID, 4, 332, 3105, NPC, 22, 405, 23, -1);
end

if (EVENT == 405) then
	SaveEvent(UID, 3253);
end

if (EVENT == 406) then
	SaveEvent(UID, 3255);
	SelectMsg(UID, 2, 332, 3110, NPC, 21, -1);
end

if (EVENT == 408) then
	ITEM_COUNTA = HowmuchItem(UID, 320410011);
	ITEM_COUNTB = HowmuchItem(UID, 320410012);
	ITEM_COUNTC = HowmuchItem(UID, 379067000);
	if (ITEM_COUNTA > 0 and ITEM_COUNTB > 0 and ITEM_COUNTC > 0) then
		SelectMsg(UID, 4, 332, 3111, NPC, 41, 410, 27, -1);
	else
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 332, 3108, NPC, 18, 409);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 332, 3108, NPC, 18, 411);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 332, 3108, NPC, 18, 412);
		end
	end
end

if (EVENT == 409) then
	ShowMap(UID, 309);
end

if (EVENT == 411) then
	ShowMap(UID, 310);
end

if (EVENT == 412) then
	ShowMap(UID, 30);
end

if (EVENT == 410) then
	ITEM_COUNTA = HowmuchItem(UID, 320410011);
	ITEM_COUNTB = HowmuchItem(UID, 320410012);
	ITEM_COUNTC = HowmuchItem(UID, 379067000);
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 332, 3108, NPC, 18, 409);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 332, 3108, NPC, 18, 411);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 332, 3108, NPC, 18, 412);
		else
			RunQuestExchange(UID,315);
			SaveEvent(UID, 3256);
	end
end

if (EVENT == 823) then
	SelectMsg(UID, 2, 335, 3226, NPC, 10, 824);
end

if (EVENT == 824) then
	SelectMsg(UID, 4, 335, 3227, NPC, 22, 825, 23, -1);
end

if (EVENT == 825) then
	SaveEvent(UID, 3433);
	SelectMsg(UID, 2, 335, 3228, NPC, 10, -1);
end

if (EVENT == 826) then
	SaveEvent(UID, 3435);
	SelectMsg(UID, 2, 335, 3232, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 828) then  -- ROGUE 70 SKILL AÇMA - 5 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 4 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 335, 3233, NPC, 41, 830, 27, -1);
	else
		if (ITEM_COUNT1 < 5) then
			SelectMsg(UID, 2, 335, 3230, NPC, 18, 829);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 335, 3230, NPC, 18, 831);
		end
	end
end

if (EVENT == 829) then
	ShowMap(UID, 307);
end

if (EVENT == 831) then
	ShowMap(UID, 336);
end

if (EVENT == 830) then
			RunQuestExchange(UID, 330);
			SaveEvent(UID, 3434);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 923) then
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, 347, 5125, NPC, 10, 924);
	else
		SelectMsg(UID, 2, 347, 5124, NPC, 10, -1);
	end
end

if (EVENT == 924) then
	SelectMsg(UID, 4, 347, 5126, NPC, 22, 925, 23, -1);
end

if (EVENT == 925) then
	SaveEvent(UID, 5132);
	SelectMsg(UID, 2, 347, 5127, NPC, 10, -1);
end

if (EVENT == 926) then
	SaveEvent(UID, 5134);
	SelectMsg(UID, 2, 347, 5131, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 928) then    -- ROGUE 72 SKILL AÇMA - 7 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 6 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 347, 5132, NPC, 41, 930, 27, -1);
	else
		if (ITEM_COUNT1 < 7) then
			SelectMsg(UID, 2, 347, 5129, NPC, 18, 929);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 347, 5129, NPC, 18, 933);
		end
	end
end

if (EVENT == 929) then
	ShowMap(UID, 307);
end

if (EVENT == 932) then
	ShowMap(UID, 19);
end

if (EVENT == 933) then
	ShowMap(UID, 336);
end

if (EVENT == 930) then
			RunQuestExchange(UID, 522);
			SaveEvent(UID, 5133);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 1023) then
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, 360, 5136, NPC, 10, 1024);
	else
		SelectMsg(UID, 2, 360, 5135, NPC, 10, -1);
	end
end

if (EVENT == 1024) then
	SelectMsg(UID, 4, 360, 5137, NPC, 22, 1025, 23, -1);
end

if (EVENT == 1025) then
	SaveEvent(UID, 5144);
	SelectMsg(UID, 2, 360, 5138, NPC, 10, -1);
end

if (EVENT == 1026) then
	SaveEvent(UID, 5146);
	SelectMsg(UID, 2, 360, 5142, NPC, 32, -1);
end

-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 1028) then   -- ROGUE 75 SKILL AÇMA - 10 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 9 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 360, 5143, NPC, 41, 1030, 27, -1);
	else
		if (ITEM_COUNT1 < 10) then
		SelectMsg(UID, 2, 360, 5140, NPC, 18, 1029);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 360, 5140, NPC, 18, 1033);
		end
	end
end

if (EVENT == 1029) then
	ShowMap(UID, 307);
end

if (EVENT == 1032) then
	ShowMap(UID, 19);
end

if (EVENT == 1033) then
	ShowMap(UID, 336);
end

if (EVENT == 1030) then
			RunQuestExchange(UID, 523);
			SaveEvent(UID, 5145); 
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 1123) then 
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, 366, 5147, NPC, 10, 1124);
	else
		SelectMsg(UID, 2, 366, 5146, NPC, 10, -1);
	end
end

if (EVENT == 1124) then
	SelectMsg(UID, 4, 366, 5148, NPC, 22, 1125, 23, -1);
end

if (EVENT == 1125) then
	SaveEvent(UID, 5156);
	SelectMsg(UID, 2, 366, 5149, NPC, 10, -1);
end

if (EVENT == 1126) then
	SaveEvent(UID, 5158);
	SelectMsg(UID, 2, 366, 5153, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 1128) then -- ROGUE 80 SKILL AÇMA - 15 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 14 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 366, 5154, NPC, 41, 1130, 27, -1);
	else
		if (ITEM_COUNT1 < 15) then
			SelectMsg(UID, 2, 366, 5151, NPC, 18, 1129);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 366, 5151, NPC, 18, 1133);
		end
	end
end

if (EVENT == 1129) then
	ShowMap(UID, 307);
end

if (EVENT == 1132) then
	ShowMap(UID, 19);
end

if (EVENT == 1133) then
	ShowMap(UID, 336);
end

if (EVENT == 1130) then
			RunQuestExchange(UID, 524);
			SaveEvent(UID, 5157);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 2002) then
	SelectMsg(UID, 4, 492, 9223, NPC, 22, 2003, 23, -1);
end

if (EVENT == 2003) then
	QuestStatusCheck = GetQuestStatus(UID, 492) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 2411);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 2416);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 2421);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 2426);
		end
	end
end

if (EVENT == 2005) then
	QuestStatusCheck = GetQuestStatus(UID, 492) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 2413);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 2418);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 2423);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 2428);
		end
	end
end

if (EVENT == 2007) then
	QuestStatusCheck = GetQuestStatus(UID, 492) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 492, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 492, 9223, NPC, 18, 2006);
		else
			SelectMsg(UID, 5, 492, 9223, NPC, 41, 2008,23, 163);
		end
	end
end



if (EVENT == 2008) then
	QuestStatusCheck = GetQuestStatus(UID, 492) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 492, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 492, 9223, NPC, 18, 2006);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,214,STEP,1);
			SaveEvent(UID, 2412);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,215,STEP,1);
			SaveEvent(UID, 2417);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,216,STEP,1);
			SaveEvent(UID, 2422);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,217,STEP,1);
			SaveEvent(UID, 2427);
			end
		end
	end
end

if (EVENT == 222) then 
	SelectMsg(UID, 2, 305, 3112, NPC, 3013, 232);
end

if (EVENT == 232) then
	SelectMsg(UID, 2, 305, 3113, NPC, 3003, 233);
end

if (EVENT == 233) then
	SelectMsg(UID, 4, 305, 3089, NPC, 22, 224, 23, -1);
end

if (EVENT == 224) then
	SaveEvent(UID, 3213);
	--GiveItem(UID,900017000,7);
end

if (EVENT == 226) then
	SaveEvent(UID, 3215);
end

if (EVENT == 228) then
	MAGICSCROLL = HowmuchItem(UID, 900017000);
	if (MAGICSCROLL > 6) then
		SelectMsg(UID, 4, 305, 3095, NPC, 41, 230, 27, -1);
	else
		SelectMsg(UID, 2, 305, 3098, NPC, 18, 229);
	end
end

if (EVENT == 229) then
	ShowMap(UID, 726);
end

if (EVENT == 230) then
	MAGICSCROLL = HowmuchItem(UID, 900017000);
		if (MAGICSCROLL < 7) then
			SelectMsg(UID, 2, 305, 3098, NPC, 18, 229);
		else
			RunQuestExchange(UID,313);
			SaveEvent(UID, 3216);
	end
end

if (EVENT == 3001) then
	SelectMsg(UID, 4, 517, 20002, NPC, 3018, 3002, 3019, -1);
end

if (EVENT == 3002) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 11035);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 11040);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 11045);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 11050);
	end
end

if (EVENT == 3004) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 11037);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 11042);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 11047);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 11052);
	end
end

if(EVENT == 3005) then
	AGED = HowmuchItem(UID, 508102000)	
	if( AGED < 5) then
		SelectMsg(UID, 2, 517, 20002, NPC, 18, 3006);
	else
		SelectMsg(UID, 5, 517, 20002, NPC, 20, 3008,25,-1);
	end
end

if (EVENT == 3006 ) then
	ShowMap(UID, 547)
end

if (EVENT == 3008) then
	QuestStatusCheck = GetQuestStatus(UID, 517) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	AGED = HowmuchItem(UID, 508102000)	
		if( AGED < 5) then
			SelectMsg(UID, 2, 517, 20002, NPC, 18, 3006);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID, 3001,STEP,1);
			SaveEvent(UID, 11036);
			SaveEvent(UID, 11062);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID, 3002,STEP,1);
			SaveEvent(UID, 11041);
			SaveEvent(UID, 11062);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID, 3003,STEP,1);
			SaveEvent(UID, 11046);
			SaveEvent(UID, 11062);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID, 3004,STEP,1);
			SaveEvent(UID, 11051);
			SaveEvent(UID, 11062);
			end
		end
	end
end

if (EVENT == 3101) then
	SelectMsg(UID, 4, 518, 20004, NPC,3018, 3102, 3019, -1);
end

if (EVENT == 3102) then
SaveEvent(UID, 11062);
end

if (EVENT == 3104) then
SaveEvent(UID, 11064);
end

if(EVENT == 3105) then
	ITEMA = HowmuchItem(UID, 508103000)	
	if(ITEMA < 5) then
		SelectMsg(UID, 2, 518, 20004, NPC, 18, 3106);
	else
		SelectMsg(UID, 5, 518, 20004, NPC, 22, 3108,23,-1);
	end
end

if (EVENT == 3106 ) then
	ShowMap(UID, 623)
end

if (EVENT == 3108 ) then
	QuestStatusCheck = GetQuestStatus(UID, 518) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 508103000)	
		if(ITEMA < 5) then
			SelectMsg(UID, 2, 518, 20004, NPC, 18, 3106);
		else
			RunQuestExchange(UID,3005,STEP,1);
			SaveEvent(UID, 11063);
			SaveEvent(UID, 11074);
		end
	end
end

if (EVENT == 3201) then
	SelectMsg(UID, 4, 786, 22998, NPC, 22, 3202, 27, -1);
end

if (EVENT == 3202) then
	SaveEvent(UID, 13783);
end

if (EVENT == 3206) then
	SaveEvent(UID, 13785);
end

if(EVENT == 3205) then
	ITEMA = HowmuchItem(UID, 900323000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 786, 22998, NPC, 18,-1);
	else
		SelectMsg(UID, 4, 786, 22998, NPC, 22, 3207,23,-1);
	end
end

if (EVENT == 3207 ) then
	QuestStatusCheck = GetQuestStatus(UID, 786) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	ITEMA = HowmuchItem(UID, 900323000)	
		if(ITEMA < 1) then
			SelectMsg(UID, 2, 786, 22998, NPC, 18,-1);
		else
			SelectMsg(UID, 2, 786, 23149, NPC, 10,3208,4005,-1);
			RunQuestExchange(UID,3234);
			SaveEvent(UID, 13784);
			SaveEvent(UID, 13795);
		end
	end
end

if (EVENT == 3208 ) then
MonsterStoneQuestJoin(UID,787);
end

if (EVENT == 3301) then
	SelectMsg(UID, 4, 787, 23000, NPC, 22, 3302, 27, -1);
end

if (EVENT == 3302) then
	SaveEvent(UID, 13795);
end

if (EVENT == 3306) then
	SaveEvent(UID, 13797);
end

if (EVENT == 3305)then
	ITEM1_COUNT = HowmuchItem(UID, 900326000);  
    ITEM2_COUNT = HowmuchItem(UID, 900325000); 	
		if (ITEM1_COUNT < 4 and ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 787, 23149, NPC, 10,3208,4005,-1);
		else
			SelectMsg(UID, 4, 787, 23000, NPC, 22, 3308, 27, -1);
	end
end

if (EVENT == 3308)then
	ITEM1_COUNT = HowmuchItem(UID, 900326000);  
    ITEM2_COUNT = HowmuchItem(UID, 900325000); 	
		if (ITEM1_COUNT < 4 and ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 787, 23149, NPC, 10,3208,4005,-1);
		else
			SelectMsg(UID, 2, -1, 23146, NPC, 10,-1);
			RunQuestExchange(UID,3235);
			SaveEvent(UID,13796);
			SaveEvent(UID,13807);
	end
end