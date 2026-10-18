-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================

local Ret = 0;
local NPC = 24201;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 3824, NPC, 3001, 193);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 3824, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 193) then
	Ret = 1;
end

local savenum = 273;

if (EVENT == 530) then
	Class = CheckClass (UID);
	if (Class == 5) then
		SaveEvent(UID, 4081);
		SelectMsg(UID, 2, savenum, 4081, NPC, 4080, 538);
	else
		Ret = 1;
	end
end

if (EVENT == 532) then -- 60 Level Job Change 
	Level = CheckLevel(UID)
	if (Level > 59) then
		Class = CheckClass (UID);
		if (Class == 5 or Class == 7 or Class == 9 or Class == 11) then
			SelectMsg(UID, 4, savenum, 4084, NPC, 22, 533, 23, 534);
		else
			SaveEvent(UID, 4083);
			SelectMsg(UID, 2, savenum, 4083, NPC, 10, 193);
		end
	else 
		SelectMsg(UID, 2, savenum, 4082, NPC, 10, 193);
	end
end

if (EVENT == 533) then
	SaveEvent(UID, 4082);
end

if (EVENT == 534) then
	SaveEvent(UID, 4085); 
end

if (EVENT == 535) then
	SaveEvent(UID, 4084);
	SelectMsg(UID, 2, savenum, 4090, NPC, 4080, 193);
end

if (EVENT == 536) then
	ITEM_COUNTA = HowmuchItem(UID, 810095000);
	ITEM_COUNTB = HowmuchItem(UID, 810090000);
	ITEM_COUNTC = HowmuchItem(UID, 810094000);
	if (ITEM_COUNTA > 0 and ITEM_COUNTB > 0 and ITEM_COUNTC > 0) then
		SelectMsg(UID, 4, savenum, 4091, NPC, 4062, 537, 4063, 193);
	else
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, savenum, 4085, NPC, 18, 538);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, savenum, 4085, NPC, 18, 539);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, savenum, 4085, NPC, 18, 540);
		end
	end
end

if (EVENT == 538) then
	ShowMap(UID, 188);
end

if (EVENT == 539) then
	ShowMap(UID, 184);
end

if (EVENT == 540) then
	ShowMap(UID, 186);
end

if (EVENT == 537) then
	SaveEvent(UID, 4083);
	RobItem(UID, 810095000, 1)
	RobItem(UID, 810090000, 1)
	RobItem(UID, 810094000, 1)
	PromoteUser(UID)
	SelectMsg(UID, 2, savenum, 4093, NPC, 4064, 193);
end

if (EVENT == 220) then
	SelectMsg(UID, 2, 304, 3028, NPC, 3006, 221);
end

if (EVENT == 221) then
	ShowMap(UID, 305);
	SaveEvent(UID, 3152);
end

if (EVENT == 222) then
	SelectMsg(UID, 2, 304, 3028, NPC, 3006, 223);
end

if (EVENT == 223) then -- 61 Level Sream Scroll
	SelectMsg(UID, 2, 304, 3064, NPC, 3010, 224);
end

if (EVENT == 224) then
	SelectMsg(UID, 4, 304, 3065, NPC, 22, 225, 23, 193);
end
if (EVENT == 225) then
	SaveEvent(UID, 3153);
end

if (EVENT == 226) then
	SaveEvent(UID, 3155);
	SelectMsg(UID, 2, 304, 3070, NPC, 32, 193);
end

if (EVENT == 227) then
	ITEM_COUNT = HowmuchItem(UID, 900017000);
	if (ITEM_COUNT > 6) then
		SelectMsg(UID, 4, 304, 3071, NPC, 41, 230, 27, 193);
	else
		SelectMsg(UID, 2, 304, 3068, NPC, 18, 229);
	end
end

if (EVENT == 229) then
	ShowMap(UID, 726);
end

if (EVENT == 230) then
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
       SelectMsg(UID,2,-1,8898,NPC,10)
         else
RunQuestExchange(UID,310)
	SaveEvent(UID, 3156);	 
end
end

if (EVENT == 300) then
	SelectMsg(UID, 2, 329, 3001, NPC, 28, 301);
end

if (EVENT == 301) then
	ShowMap(UID, 305);
	SaveEvent(UID, 3172);
end

if (EVENT == 302) then
	SelectMsg(UID, 2, 329, 3002, NPC, 28, 301);
end

if (EVENT == 303) then -- 69 Level Stamp of Strength
	SelectMsg(UID, 2, 329, 3072, NPC, 3002, 304);
end

if (EVENT == 304) then
	SelectMsg(UID, 4, 329, 3073, NPC, 22, 305, 23, 193);
end

if (EVENT == 305) then
	SaveEvent(UID, 3173);
end

if (EVENT == 306) then
	SaveEvent(UID, 3175);
	SelectMsg(UID, 2, 329, 3078, NPC, 21, 193);
end

if (EVENT == 308) then
	ITEM_COUNT1 = HowmuchItem(UID, 379042000);
	ITEM_COUNT2 = HowmuchItem(UID, 379040000);
	ITEM_COUNT3 = HowmuchItem(UID, 379236000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 1) then
		SelectMsg(UID, 4, 329, 3079, NPC, 41, 310, 27, 193);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 329, 3076, NPC, 18, 309);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 329, 3076, NPC, 18, 311);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 329, 3076, NPC, 18, 312);
		end
	end
end

if (EVENT == 309) then
	ShowMap(UID, 319);
end

if (EVENT == 311) then
	ShowMap(UID, 28);
end

if (EVENT == 312) then
	ShowMap(UID, 19);
end

if (EVENT == 310) then
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
       SelectMsg(UID,2,-1,8898,NPC,10)
         else
RunQuestExchange(UID,311)
	SaveEvent(UID, 3176);
end
end

if (EVENT == 400) then
	SelectMsg(UID, 2, 330, 3001, NPC, 28, 401);
end

if (EVENT == 401) then
	ShowMap(UID, 305);
	SaveEvent(UID, 3192);
end

if (EVENT == 402) then
	SelectMsg(UID, 2, 330, 3002, NPC, 28, 401);
end

if (EVENT == 403) then -- 69 Level Nostrum of Constitution
	SelectMsg(UID, 2, 330, 3080, NPC, 3000, 404);
end

if (EVENT == 404) then
	SelectMsg(UID, 4, 330, 3081, NPC, 22, 405, 23, 407);
end

if (EVENT == 405) then
	SaveEvent(UID, 3193);
end

if (EVENT == 406) then
	SaveEvent(UID, 3195);
	SelectMsg(UID, 2, 330, 3086, NPC, 21, 193);
end

if (EVENT == 407) then
	SaveEvent(UID, 3196);
end

if (EVENT == 408) then
	ITEM_COUNT1 = HowmuchItem(UID, 320410011);
	ITEM_COUNT2 = HowmuchItem(UID, 320410012);
	ITEM_COUNT3 = HowmuchItem(UID, 379067000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 330, 3087, NPC, 41, 410, 27, 193);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 330, 3084, NPC, 18, 409);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 330, 3084, NPC, 18, 411);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 330, 3084, NPC, 18, 412);
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
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
       SelectMsg(UID,2,-1,8898,NPC,10)
         else
RunQuestExchange(UID,312)
	SaveEvent(UID, 3196);   
end
end

if (EVENT == 620) then
	SelectMsg(UID, 2, 334, 3215, NPC, 3006, 621);
end

if (EVENT == 621) then
	ShowMap(UID, 305);
	SaveEvent(UID, 3412);
end

if (EVENT == 622) then
	SelectMsg(UID, 2, 334, 3214, NPC, 3006, 621);
end


if (EVENT == 623) then
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, 334, 3216, NPC, 10, 624);
	else
		SelectMsg(UID, 2, 334, 4710, NPC, 10, 193);
	end
end

if (EVENT == 624) then
	SelectMsg(UID, 4, 334, 3217, NPC, 22, 625, 23, 631);
end

if (EVENT == 625) then
	SaveEvent(UID, 3413);
	SelectMsg(UID, 2, 334, 3218, NPC, 10, 193);
end

if (EVENT == 631) then
	SelectMsg(UID, 2, 334, 3219, NPC, 10, 193);
end

if (EVENT == 626) then
	SaveEvent(UID, 3415);
	SelectMsg(UID, 2, 334, 3222, NPC, 32, 193);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 628) then   -- WARRİOR 70 SKILL AÇMA - 5 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 4 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 334, 3223, NPC, 41, 630, 27, 193);
	else
		if (ITEM_COUNT1 < 5) then
			SelectMsg(UID, 2, 334, 3220, NPC, 18, 629);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 334, 3220, NPC, 18, 632);
		end
	end
end

if (EVENT == 629) then
	ShowMap(UID, 305);
end

if (EVENT == 632) then
	ShowMap(UID, 336);
end

if (EVENT == 630) then
RunQuestExchange(UID,329)
	SaveEvent(UID, 3414);  
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

local savenum = 359;

if (EVENT == 720) then
	SelectMsg(UID, 2, savenum, 5100, NPC, 3006, 721);
end

if (EVENT == 721) then
	ShowMap(UID, 305);
	SaveEvent(UID, 5107);
end

if (EVENT == 722) then
	SelectMsg(UID, 2, savenum, 5101, NPC, 3006, 721);
end

if (EVENT == 723) then 
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, savenum, 5103, NPC, 10, 724);
	else
		SelectMsg(UID, 2, savenum, 5102, NPC, 10, 193);
	end
end

if (EVENT == 724) then
	SelectMsg(UID, 4, savenum, 5104, NPC, 22, 725, 23, 731);
end

if (EVENT == 725) then
	SaveEvent(UID, 5108);
	SelectMsg(UID, 2, savenum, 5105, NPC, 10, 193);
end

if (EVENT == 731) then
	SelectMsg(UID, 2, savenum, 5106, NPC, 10, 193);
end

if (EVENT == 726) then
	SaveEvent(UID, 5110);
	SelectMsg(UID, 2, savenum, 5109, NPC, 32, 193);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 728) then   -- WARRİOR 75 SKILL AÇMA - 10 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 9 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, savenum, 5110, NPC, 41, 730, 27, 193);
	else
		if (ITEM_COUNT1 < 10) then
			SelectMsg(UID, 2, savenum, 5107, NPC, 18, 727);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, savenum, 5107, NPC, 18, 732);
		end
	end
end

if (EVENT == 727) then
	ShowMap(UID, 305);
end

if (EVENT == 729) then
	ShowMap(UID, 19);
end

if (EVENT == 732) then
	ShowMap(UID, 336);
end

if (EVENT == 730) then
RunQuestExchange(UID,520)
	SaveEvent(UID, 5109);   
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
local savenum = 365;

if (EVENT == 820) then
	SelectMsg(UID, 2, savenum, 5111, NPC, 3006, 821);
end

if (EVENT == 821) then
	ShowMap(UID, 305);
	SaveEvent(UID, 5119);
end

if (EVENT == 822) then
	SelectMsg(UID, 2, savenum, 5112, NPC, 3006, 821);
end

if (EVENT == 823) then 
	Class = CheckClass (UID);
	if (Class == 6 or Class == 8 or Class == 10 or Class == 12) then
		SelectMsg(UID, 2, savenum, 5114, NPC, 10, 824);
	else
		SelectMsg(UID, 2, savenum, 5113, NPC, 10, 193);
	end
end

if (EVENT == 824) then
	SelectMsg(UID, 4, savenum, 5115, NPC, 22, 825, 23, 831);
end

if (EVENT == 825) then
	SaveEvent(UID, 5120);
	SelectMsg(UID, 2, savenum, 5116, NPC, 10, 193);
end

if (EVENT == 831) then
	SelectMsg(UID, 2, savenum, 5117, NPC, 10, 193);
end

if (EVENT == 826) then
	SaveEvent(UID, 5122);
	SelectMsg(UID, 2, savenum, 5120, NPC, 32, 193);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 828) then   -- WARRİOR 80 SKILL AÇMA - 15 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 14 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, savenum, 5121, NPC, 41, 830, 27, 193);
	else
		if (ITEM_COUNT1 < 15) then
			SelectMsg(UID, 2, savenum, 5118, NPC, 18, 829);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, savenum, 5118, NPC, 18, 833);
		end
	end
end

if (EVENT == 829) then
	ShowMap(UID, 305);
end

if (EVENT == 832) then
	ShowMap(UID, 19);
end

if (EVENT == 833) then
	ShowMap(UID, 336);
end

if (EVENT == 830) then
RunQuestExchange(UID,521)
	SaveEvent(UID, 5121);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
