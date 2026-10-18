-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local Ret = 0;
local NPC = 14202;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 3825, NPC, 10, 193);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 3825, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 193) then
	Ret = 1;
end

if (EVENT == 600) then -- 45 Level Asga Furit
	SelectMsg(UID, 2, 244, 3001, NPC, 28, 601);
end

if (EVENT == 601) then
	ShowMap(UID, 306);
	SaveEvent(UID, 3282);
end

if (EVENT == 602) then
	SelectMsg(UID, 2, 244, 3002, NPC, 28, 601);
end

if (EVENT == 603) then
	SelectMsg(UID, 2, 244, 3131, NPC, 10, 604);
end

if (EVENT == 604) then
	SelectMsg(UID, 4, 244, 3132, NPC, 22, 605, 23, 193);
end

if (EVENT == 605) then
	SaveEvent(UID, 3283);
end

if (EVENT == 606) then
	SaveEvent(UID, 3285);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 243, 3136, NPC, 3015, 193);
	else
		SelectMsg(UID, 2, 243, 3006, NPC, 3015, 193);
	end
end

if (EVENT == 608) then
	ITEM_COUNT = HowmuchItem(UID, 910082000);
	if (ITEM_COUNT == 0) then
		SelectMsg(UID, 2, 244, 3135, NPC, 18, 609);
	else
		SelectMsg(UID, 4, 244, 3137, NPC, 41, 610, 27, 193);
	end
end

if (EVENT == 609) then
	ShowMap(UID, 314);
end

if (EVENT == 610) then
	ITEM_COUNT = HowmuchItem(UID, 910082000);
	if (ITEM_COUNT == 0) then
		SelectMsg(UID, 2, 244, 3135, NPC, 18, 609);
	else
--RobItem(UID, 910082000, ITEM_COUNT);
--COIN = ITEM_COUNT * 100000;
--GoldGain(UID, COIN);
   RunCountExchange(UID,317)
   SaveEvent(UID, 3286); 
end
end
--------------------------------
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
--------------------------------
if (EVENT == 533) then
	SaveEvent(UID, 4100);
end
--------------------------------
if (EVENT == 534) then
	SaveEvent(UID, 4103);
end
--------------------------------
if (EVENT == 535) then
	SaveEvent(UID, 4102);
	SelectMsg(UID, 2, 273, 4102, NPC, 4080, 193);
end
--------------------------------
if (EVENT == 536) then
	ITEM_COUNTA = HowmuchItem(UID, 379041000);
	ITEM_COUNTB = HowmuchItem(UID, 379040000);
	ITEM_COUNTC = HowmuchItem(UID, 379042000);
	ITEM_COUNTD = HowmuchItem(UID, 379236000);
	if (ITEM_COUNTA > 0 and ITEM_COUNTB > 0 and ITEM_COUNTC > 0 and ITEM_COUNTD > 0) then
		SelectMsg(UID, 4, 273, 4103, NPC, 4062, 537, 4063, 193);
	else
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 538);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 539);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 273, 4099, NPC, 18, 540);
		elseif (ITEM_COUNTD < 1) then
			SelectMsg(UID, 2, savenum, 4099, NPC, 18, 540);
		end
	end
end
--------------------------------
if (EVENT == 538) then
	ShowMap(UID, 189);
end
--------------------------------
if (EVENT == 539) then
	ShowMap(UID, 713);
end
--------------------------------
if (EVENT == 540) then
	ShowMap(UID, 711);
end
--------------------------------
if (EVENT == 537) then
	RobItem(UID, 379041000, 1)
	RobItem(UID, 379040000, 1)
	RobItem(UID, 379042000, 1)
	RobItem(UID, 379236000, 1)
	PromoteUser(UID)
	SaveEvent(UID, 4101);
	SelectMsg(UID, 2, savenum, 4093, NPC, 4064, 193);
end


if (EVENT == 220) then
	SaveEvent(UID, 3202);
end
--------------------------------
if (EVENT == 222) then 
	SelectMsg(UID, 2, 305, 3112, NPC, 3013, 232);
end
--------------------------------
if (EVENT == 232) then
	SelectMsg(UID, 2, 305, 3113, NPC, 3003, 233);
end
--------------------------------
if (EVENT == 233) then
SelectMsg(UID, 4, 305, 3089, NPC, 22, 224, 23, 193);
end
--------------------------------
if (EVENT == 224) then
	SaveEvent(UID, 3203);
end
--------------------------------
if (EVENT == 225) then
	SaveEvent(UID, 3206);
end
--------------------------------
if (EVENT == 226) then
	SaveEvent(UID, 3205);
end
--------------------------------
if (EVENT == 228) then --Magic Shield 
	ITEM_COUNTA = HowmuchItem(UID, 330310014);
	ITEM_COUNTB = HowmuchItem(UID, 389075000);
	ITEM_COUNTC = HowmuchItem(UID, 900000000);
	if (ITEM_COUNTA > 0 and ITEM_COUNTB > 29 and ITEM_COUNTC > 5000000) then
		SelectMsg(UID, 4, 305, 3095, NPC, 41, 230, 27, 193);
	else
		SelectMsg(UID, 2, 305, 3098, NPC, 18, 229);
	end
end
--------------------------------
if (EVENT == 229) then
	ShowMap(UID, 726);
end
--------------------------------
if (EVENT == 230) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
    RunQuestExchange(UID,313)
	SaveEvent(UID, 3206);
end
end
--------------------------------
if (EVENT == 303) then 
	SelectMsg(UID, 2, 331, 3096, NPC, 3011, 304);
end
--------------------------------
if (EVENT == 304) then
	SelectMsg(UID, 4, 331, 3097, NPC, 22, 305, 23, 193);
end
--------------------------------
if (EVENT == 305) then
	SaveEvent(UID, 3223);
end
--------------------------------
if (EVENT == 306) then
	SaveEvent(UID, 3225);
	SelectMsg(UID, 2, 331, 3102, NPC, 21, 193);
end
--------------------------------
if (EVENT == 308) then
	ITEM_COUNTA = HowmuchItem(UID, 379113000);
	ITEM_COUNTB = HowmuchItem(UID, 379201000);
	ITEM_COUNTC = HowmuchItem(UID, 379014000);
	if (ITEM_COUNTA > 4 and ITEM_COUNTB > 49 and ITEM_COUNTC > 29) then
		SelectMsg(UID, 4, 331, 3103, NPC, 41, 310, 27, 193);
	else
		if (ITEM_COUNTA < 5) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 309);
		elseif (ITEM_COUNTB < 50) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 311);
		elseif (ITEM_COUNTC < 30) then
			SelectMsg(UID, 2, 331, 3100, NPC, 18, 312);
		end
	end
end
--------------------------------
if (EVENT == 309) then
	ShowMap(UID, 324);
end
--------------------------------
if (EVENT == 311) then
	ShowMap(UID, 28);
end
--------------------------------
if (EVENT == 312) then
	ShowMap(UID, 18);
end
--------------------------------
if (EVENT == 310) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
    RunQuestExchange(UID,314)
	SaveEvent(UID, 3226);
end
end
--------------------------------
if (EVENT == 400) then
	SelectMsg(UID, 2, 332, 3001, NPC, 28, 401);
end

if (EVENT == 401) then
	ShowMap(UID, 306);
	SaveEvent(UID, 3242);
end

if (EVENT == 402) then
	SelectMsg(UID, 2, 332, 3002, NPC, 28, 401);
end

if (EVENT == 403) then 
	SelectMsg(UID, 2, 332, 3104, NPC, 10, 404);
end

if (EVENT == 404) then
	SelectMsg(UID, 4, 332, 3105, NPC, 22, 405, 23, 193);
end

if (EVENT == 405) then
	SaveEvent(UID, 3243);
end

if (EVENT == 406) then
	SaveEvent(UID, 3245);
	SelectMsg(UID, 2, 332, 3110, NPC, 21, 193);
end

if (EVENT == 408) then
	ITEM_COUNTA = HowmuchItem(UID, 379045000);
	ITEM_COUNTB = HowmuchItem(UID, 379042000);
	ITEM_COUNTC = HowmuchItem(UID, 379067000);
	if (ITEM_COUNTA > 19 and ITEM_COUNTB > 0 and ITEM_COUNTC > 0) then
		SelectMsg(UID, 4, 332, 3111, NPC, 41, 410, 27, 193);
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
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
    RunQuestExchange(UID,315)
	SaveEvent(UID, 3246);
end
end
-----------------------------------------
if (EVENT == 820) then
	SelectMsg(UID, 2, 335, 3225, NPC, 3006, 821);
end

if (EVENT == 821) then
	ShowMap(UID, 306);
	SaveEvent(UID, 3422);
end

if (EVENT == 822) then
	SelectMsg(UID, 2, 335, 3224, NPC, 3006, 821);
end

if (EVENT == 823) then
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, 335, 3226, NPC, 10, 824);
	else
		SelectMsg(UID, 2, 335, 4711, NPC, 10, 193);
	end
end

if (EVENT == 824) then
	SelectMsg(UID, 4, 335, 3227, NPC, 22, 825, 23, 831);
end

if (EVENT == 825) then
	SaveEvent(UID, 3423);
	SelectMsg(UID, 2, 335, 3228, NPC, 10, 193);
end

if (EVENT == 831) then
	SelectMsg(UID, 2, 335, 3229, NPC, 10, 193);
end

if (EVENT == 826) then
	SaveEvent(UID, 3425);
	SelectMsg(UID, 2, 335, 3232, NPC, 32, 193);
end

if (EVENT == 828) then
	ITEM_COUNT1 = HowmuchItem(UID, 379245000);
	ITEM_COUNT2 = HowmuchItem(UID, 379246000);
	ITEM_COUNT3 = HowmuchItem(UID, 379064000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 335, 3233, NPC, 41, 830, 27, 193);
	else
		if (ITEM_COUNT1 < 1 or ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 335, 3230, NPC, 18, 829);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 335, 3230, NPC, 18, 831);
		end
	end
end

if (EVENT == 829) then
	ShowMap(UID, 306);
end

if (EVENT == 831) then
	ShowMap(UID, 336);
end

if (EVENT == 830) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
RunQuestExchange(UID, 330);
	SaveEvent(UID, 3424);
end
end

local savenum = 347;

if (EVENT == 920) then
	SelectMsg(UID, 2, savenum, 5122, NPC, 3006, 921);
end

if (EVENT == 921) then
	ShowMap(UID, 306);
	SaveEvent(UID, 5125);
end

if (EVENT == 922) then
	SelectMsg(UID, 2, savenum, 5123, NPC, 3006, 921);
end

if (EVENT == 923) then -- 72 Level Skill
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, savenum, 5125, NPC, 10, 924);
	else
		SelectMsg(UID, 2, savenum, 5124, NPC, 10, 193);
	end
end

if (EVENT == 924) then
	SelectMsg(UID, 4, savenum, 5126, NPC, 22, 925, 23, 931);
end

if (EVENT == 925) then
	SaveEvent(UID, 5126);
	SelectMsg(UID, 2, savenum, 5127, NPC, 10, 193);
end

if (EVENT == 931) then
	SelectMsg(UID, 2, savenum, 5128, NPC, 10, 193);
end

if (EVENT == 926) then
	SaveEvent(UID, 5128);
	SelectMsg(UID, 2, savenum, 5131, NPC, 32, 193);
end

if (EVENT == 928) then
	ITEM_COUNT1 = HowmuchItem(UID, 379040000); --- Tail of Shaula
	ITEM_COUNT2 = HowmuchItem(UID, 379041000); --- Tail of Lesath
	ITEM_COUNT3 = HowmuchItem(UID, 379042000); --- Fang of Bakirra
	ITEM_COUNT4 = HowmuchItem(UID, 379236000); --- Magic Jewel Powder 3x
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 0 and ITEM_COUNT4 > 2) then
		SelectMsg(UID, 4, savenum, 5132, NPC, 41, 930, 27, 193);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5129, NPC, 18, 929);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, savenum, 5129, NPC, 18, 932);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, savenum, 5129, NPC, 18, 933);
		elseif (ITEM_COUNT4 < 3) then
			SelectMsg(UID, 2, savenum, 5129, NPC, 18, 933);
		end
	end
end

if (EVENT == 929) then
	ShowMap(UID, 306);
end

if (EVENT == 932) then
	ShowMap(UID, 18);
end

if (EVENT == 933) then
	ShowMap(UID, 336);
end

if (EVENT == 930) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
RunQuestExchange(UID, 522);
	SaveEvent(UID, 5127);
end
end

local savenum = 360;

if (EVENT == 1020) then
	SelectMsg(UID, 2, savenum, 5133, NPC, 3006, 1021);
end

if (EVENT == 1021) then
	ShowMap(UID, 306);
	SaveEvent(UID, 5137);
end

if (EVENT == 1022) then
	SelectMsg(UID, 2, savenum, 5134, NPC, 3006, 1021);
end

if (EVENT == 1023) then -- 75 Level Skill
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, savenum, 5136, NPC, 10, 1024);
	else
		SelectMsg(UID, 2, savenum, 5135, NPC, 10, 193);
	end
end

if (EVENT == 1024) then
	SelectMsg(UID, 4, savenum, 5137, NPC, 22, 1025, 23, 1031);
end

if (EVENT == 1025) then
	SaveEvent(UID, 5138);
	SelectMsg(UID, 2, savenum, 5138, NPC, 10, 193);
end

if (EVENT == 1031) then
	SelectMsg(UID, 2, savenum, 5139, NPC, 10, 193);
end

if (EVENT == 1026) then
	SaveEvent(UID, 5140);
	SelectMsg(UID, 2, savenum, 5142, NPC, 32, 193);
end

if (EVENT == 1028) then
	ITEM_COUNT1 = HowmuchItem(UID, 379246000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 2 and ITEM_COUNT3 > 9999999) then
		SelectMsg(UID, 4, savenum, 5143, NPC, 41, 1030, 27, 193);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5140, NPC, 18, 1029);
		elseif (ITEM_COUNT2 < 3) then
			SelectMsg(UID, 2, savenum, 5140, NPC, 18, 1032);
		elseif (ITEM_COUNT3 < 10000000) then
			SelectMsg(UID, 2, savenum, 5140, NPC, 18, 1033);
		end
	end
end

if (EVENT == 1029) then
	ShowMap(UID, 306);
end

if (EVENT == 1032) then
	ShowMap(UID, 18);
end

if (EVENT == 1033) then
	ShowMap(UID, 336);
end

if (EVENT == 1030) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
RunQuestExchange(UID, 523);
	SaveEvent(UID, 5139);   
end
end

local savenum = 366;

if (EVENT == 1120) then
	SelectMsg(UID, 2, savenum, 5144, NPC, 3006, 1121);
end

if (EVENT == 1121) then
	ShowMap(UID, 306);
	SaveEvent(UID, 5149);
end

if (EVENT == 1122) then
	SelectMsg(UID, 2, savenum, 5145, NPC, 3006, 1121);
end

if (EVENT == 1123) then -- 80 Level Skill
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, savenum, 5147, NPC, 10, 1124);
	else
		SelectMsg(UID, 2, savenum, 5146, NPC, 10, 193);
	end
end

if (EVENT == 1124) then
	SelectMsg(UID, 4, savenum, 5148, NPC, 22, 1125, 23, 1131);
end

if (EVENT == 1125) then
	SaveEvent(UID, 5150);
	SelectMsg(UID, 2, savenum, 5149, NPC, 10, 193);
end

if (EVENT == 1131) then
	SelectMsg(UID, 2, savenum, 5150, NPC, 10, 193);
end

if (EVENT == 1126) then
	SaveEvent(UID, 5152);
	SelectMsg(UID, 2, savenum, 5153, NPC, 32, 193);
end

if (EVENT == 1128) then
	ITEM_COUNT1 = HowmuchItem(UID, 379245000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 2 and ITEM_COUNT3 > 9999999) then
		SelectMsg(UID, 4, savenum, 5154, NPC, 41, 1130, 27, 193);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5151, NPC, 18, 1129);
		elseif (ITEM_COUNT2 < 3) then
			SelectMsg(UID, 2, savenum, 5151, NPC, 18, 1132);
		elseif (ITEM_COUNT3 < 10000000) then
			SelectMsg(UID, 2, savenum, 5151, NPC, 18, 1133);
		end
	end
end

if (EVENT == 1129) then
	ShowMap(UID, 306);
end

if (EVENT == 1132) then
	ShowMap(UID, 18);
end

if (EVENT == 1133) then
	ShowMap(UID, 336);
end

if (EVENT == 1130) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
RunQuestExchange(UID, 524);
	SaveEvent(UID, 5151); 
end
end



local savenum = 493;

if (EVENT == 2000) then -- 55 Cardinal
	SelectMsg(UID, 2, savenum, 9223, NPC, 10, 2001);
end

if (EVENT == 2001) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
		SaveEvent(UID, 2431);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 2436);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 2441);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 2446);
	end
end

if (EVENT == 2002) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 9223, NPC, 22, 2003, 23, 2004);
	else
		SelectMsg(UID, 2, savenum, 9223, NPC, 10, 163);
	end
end

if (EVENT == 2003) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
		SaveEvent(UID, 2432);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 2437);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 2442);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 2447);
	end
end

if (EVENT == 2004) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
		SaveEvent(UID, 2435);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 2440);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 2445);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 2450);
	end
end

if (EVENT == 2005) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
		SaveEvent(UID, 2434);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 2439);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 2444);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 2449);
	end
end

--if (EVENT == 2006) then
	--ShowMap(UID, 306);
--end

if (EVENT == 2007) then
	MonsterCount = CountMonsterQuestSub(UID, 493, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, savenum, 9223, NPC, 18, 2006);
	else
		SelectMsg(UID, 5, savenum, 9223, NPC, 41, 2008,41, 2008,41, 2008,41, 2008,23, 163);
	end
end



if (EVENT == 2008) then
SLOTKONTROL = CheckGiveSlot(UID, 3)
     if SLOTKONTROL == false then
       SelectMsg(UID,2,-1,8898,NPC,10)
         else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
RunQuestExchange(UID,218,STEP)
		SaveEvent(UID, 2433);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,219,STEP)
		SaveEvent(UID, 2438);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,220,STEP)
		SaveEvent(UID, 2443);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,221,STEP)
		SaveEvent(UID, 2448);
	end
end
end

if (EVENT == 3001) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 517, 20002, NPC, 3018, 3002, 3019, -1);
	else
		SelectMsg(UID, 2, 517, 20002, NPC, 4242, -1);
	end
end

if (EVENT == 3002) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
		SaveEvent(UID, 11014);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 11019);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 11024);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 11029);
	end
end

if (EVENT == 3004) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
		SaveEvent(UID, 11016);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 11021);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 11026);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 11031);
	end
end

if(EVENT == 3005) then
	AGED = HowmuchItem(UID, 508102000)	
	if( AGED < 5) then
		SelectMsg(UID, 2, 517, 20002, NPC, 18, 3006);
	else
		SelectMsg(UID, 5, 517, 20002, NPC, 20, 3008, 20, 3008, 20, 3008, 20, 3008, 20, 3008, 20, 3008,25,-1);
	end
end

if (EVENT == 3006 ) then
	ShowMap(UID, 546)
end

if (EVENT == 3008) then
SLOTKONTROL = CheckGiveSlot(UID, 5)
     if SLOTKONTROL == false then
       SelectMsg(UID,2,-1,8898,NPC,10)
         else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6) then
	RunQuestExchange(UID, 3001);
		SaveEvent(UID, 11015);
		SaveEvent(UID, 11056);
	elseif (Class == 2 or Class == 7 or Class == 8) then
	RunQuestExchange(UID, 3002);
		SaveEvent(UID, 11020);
		SaveEvent(UID, 11056);
	elseif (Class == 3 or Class == 9 or Class == 10) then
	RunQuestExchange(UID, 3003);
		SaveEvent(UID, 11025);
		SaveEvent(UID, 11056);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	RunQuestExchange(UID, 3004);
	    SaveEvent(UID, 11030);
		SaveEvent(UID, 11056);
	end
end
end

if (EVENT == 3101) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 518, 20004, NPC,3018, 3102, 3019, -1);
	else
		SelectMsg(UID, 2, 518, 20004, NPC, 4242, -1);
	end
end

if (EVENT == 3102) then
SaveEvent(UID, 11056);
end

if (EVENT == 3104) then
SaveEvent(UID, 11058);
end

if(EVENT == 3105) then
	ITEMA = HowmuchItem(UID, 508103000)	
	if(ITEMA < 5) then
		SelectMsg(UID, 2, 518, 20004, NPC, 18, 3106);
	else
		SelectMsg(UID, 5, 518, 20004, NPC, 22, 3108,22, 3108,22, 3108, 22, 3108, 22, 3108,23,-1);
	end
end

if (EVENT == 3106 ) then
	ShowMap(UID, 624)
end

if (EVENT == 3108 ) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
RunQuestExchange(UID,3005)
SaveEvent(UID, 11057);
SaveEvent(UID, 11068);
end
end

if (EVENT == 3201) then
	SelectMsg(UID, 4, 786, 22998, NPC, 22, 3202, 27, -1);
end

if (EVENT == 3202) then
	SaveEvent(UID, 13789);
end

if (EVENT == 3206) then
	SaveEvent(UID, 13791);
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
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
RunQuestExchange(UID,3234)
SaveEvent(UID, 13790);
SaveEvent(UID, 13801);
end
end

if (EVENT == 3208 ) then
ZoneChange(UID, 83, 204, 200)
end

if (EVENT == 3301) then
	SelectMsg(UID, 4, 787, 23000, NPC, 22, 3302, 27, -1);
end

if (EVENT == 3302) then
	SaveEvent(UID, 13801);
end

if (EVENT == 3306) then
	SaveEvent(UID, 13803);
end

if (EVENT == 3305)then
	QuestStatus = GetQuestStatus(UID, 787)	
	if(QuestStatus == 1) then
	SelectMsg(UID, 2, 787, 23149, NPC, 10,3208,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900326000);  
    ITEM2_COUNT = HowmuchItem(UID, 900325000); 	
	if (ITEM1_COUNT < 4 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 787, 23149, NPC, 10,3208,4005,-1);
	else
	SelectMsg(UID, 4, 787, 23000, NPC, 22, 3308, 27, -1);
end
end
end

if (EVENT == 3308)then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
RunQuestExchange(UID,3235)
	SaveEvent(UID,13802)
	SaveEvent(UID,13813)
	end
	end