-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14201;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 3824, NPC, 3001, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 3824, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 532) then 
	Level = CheckLevel(UID)
	if (Level > 59) then
		Class = CheckClass (UID);
		if (Class == 5 or Class == 7 or Class == 9 or Class == 11) then
			SelectMsg(UID, 4, 273, 4084, NPC, 22, 533, 23, -1);
		else
			SaveEvent(UID, 4089);
			SelectMsg(UID, 2, 273, 4083, NPC, 10, -1);
		end
	else 
		SelectMsg(UID, 2, 273, 4082, NPC, 10, -1);
	end
end

if (EVENT == 533) then
	SaveEvent(UID, 4088);
end

if (EVENT == 535) then
	SaveEvent(UID, 4090);
	SelectMsg(UID, 2, 273, 4090, NPC, 4080, -1);
end

if (EVENT == 536) then
	ITEM_COUNTA = HowmuchItem(UID, 810095000);
	ITEM_COUNTB = HowmuchItem(UID, 810090000);
	ITEM_COUNTC = HowmuchItem(UID, 810094000);
	if (ITEM_COUNTA > 0 and ITEM_COUNTB > 0 and ITEM_COUNTC > 0) then
		SelectMsg(UID, 4, 273, 4091, NPC, 4062, 537, 4063, -1);
	else
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 273, 4085, NPC, 18, 538);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 273, 4085, NPC, 18, 539);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 273, 4085, NPC, 18, 540);
		end
	end
end

if (EVENT == 538) then
	ShowMap(UID, 189);
end

if (EVENT == 539) then
	ShowMap(UID, 185);
end

if (EVENT == 540) then
	ShowMap(UID, 187);
end

if (EVENT == 537) then
	ITEM_COUNTA = HowmuchItem(UID, 810095000);
	ITEM_COUNTB = HowmuchItem(UID, 810090000);
	ITEM_COUNTC = HowmuchItem(UID, 810094000);
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 273, 4085, NPC, 18, 538);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 273, 4085, NPC, 18, 539);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 273, 4085, NPC, 18, 540);
		else
			RunQuestExchange(UID,461);
			SaveEvent(UID, 4089);
			PromoteUser(UID);
			SelectMsg(UID, 2, 273, 4093, NPC, 4064, -1);
	end
end

if (EVENT == 222) then
	SelectMsg(UID, 2, 304, 3028, NPC, 3006, 223);
end

if (EVENT == 223) then
	SelectMsg(UID, 2, 304, 3064, NPC, 3010, 224);
end

if (EVENT == 224) then
	SelectMsg(UID, 4, 304, 3065, NPC, 22, 225, 23, -1);
end
if (EVENT == 225) then
	SaveEvent(UID, 3143);
	--GiveItem(UID,900017000,7);
end

if (EVENT == 226) then
	SaveEvent(UID, 3145);
	SelectMsg(UID, 2, 304, 3070, NPC, 32, -1);
end

if (EVENT == 227) then
	ITEM_COUNT = HowmuchItem(UID, 900017000);
	if (ITEM_COUNT > 6) then
		SelectMsg(UID, 4, 304, 3071, NPC, 41, 230, 27, -1);
	else
		SelectMsg(UID, 2, 304, 3068, NPC, 18, 229);
	end
end

if (EVENT == 229) then
	ShowMap(UID, 726);
end

if (EVENT == 230) then
	ITEM_COUNT = HowmuchItem(UID, 900017000);
		if (ITEM_COUNT < 7) then
			SelectMsg(UID, 2, 304, 3068, NPC, 18, 229);
		else
			RunQuestExchange(UID,310);
			SaveEvent(UID, 3146);
	end
end

if (EVENT == 303) then
	SelectMsg(UID, 2, 329, 3072, NPC, 3002, 304);
end

if (EVENT == 304) then
	SelectMsg(UID, 4, 329, 3073, NPC, 22, 305, 23, -1);
end

if (EVENT == 305) then
	SaveEvent(UID, 3163);
end

if (EVENT == 306) then
	SaveEvent(UID, 3165);
	SelectMsg(UID, 2, 329, 3078, NPC, 21, -1);
end

if (EVENT == 308) then
	ITEM_COUNT1 = HowmuchItem(UID, 379042000);
	ITEM_COUNT2 = HowmuchItem(UID, 379040000);
	ITEM_COUNT3 = HowmuchItem(UID, 379236000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 1) then
		SelectMsg(UID, 4, 329, 3079, NPC, 41, 310, 27, -1);
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
	ShowMap(UID, 18);
end

if (EVENT == 310) then
	ITEM_COUNT1 = HowmuchItem(UID, 379042000);
	ITEM_COUNT2 = HowmuchItem(UID, 379040000);
	ITEM_COUNT3 = HowmuchItem(UID, 379236000);
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 329, 3076, NPC, 18, 309);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 329, 3076, NPC, 18, 311);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 329, 3076, NPC, 18, 312);
			else
			RunQuestExchange(UID,311);
			SaveEvent(UID, 3166);
	end
end

if (EVENT == 403) then
	SelectMsg(UID, 2, 330, 3080, NPC, 3000, 404);
end

if (EVENT == 404) then
	SelectMsg(UID, 4, 330, 3081, NPC, 22, 405, 23, -1);
end

if (EVENT == 405) then
	SaveEvent(UID, 3183);
end

if (EVENT == 406) then
	SaveEvent(UID, 3185);
	SelectMsg(UID, 2, 330, 3086, NPC, 21, -1);
end

if (EVENT == 408) then
	ITEM_COUNT1 = HowmuchItem(UID, 320410011);
	ITEM_COUNT2 = HowmuchItem(UID, 320410012);
	ITEM_COUNT3 = HowmuchItem(UID, 379067000);
		if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 0) then
			SelectMsg(UID, 4, 330, 3087, NPC, 41, 410, 27, -1);
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
	ITEM_COUNT1 = HowmuchItem(UID, 320410011);
	ITEM_COUNT2 = HowmuchItem(UID, 320410012);
	ITEM_COUNT3 = HowmuchItem(UID, 379067000);
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 330, 3084, NPC, 18, 409);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 330, 3084, NPC, 18, 411);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 330, 3084, NPC, 18, 412);
		else
			RunQuestExchange(UID,312);
			SaveEvent(UID, 3186);
	end
end

if (EVENT == 623) then
	SelectMsg(UID, 2, 334, 3216, NPC, 10, 624);
end

if (EVENT == 624) then
	SelectMsg(UID, 4, 334, 3217, NPC, 22, 625, 23, -1);
end

if (EVENT == 625) then
	SaveEvent(UID, 3403);
	SelectMsg(UID, 2, 334, 3218, NPC, 10, -1);
end

if (EVENT == 626) then
	SaveEvent(UID, 3405);
	SelectMsg(UID, 2, 334, 3222, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 628) then  -- WARRİOR 70 SKILL AÇMA - 5 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 334, 3223, NPC, 41, 630, 27, -1);
	else
		if (ITEM_COUNT1 < 5) then
			SelectMsg(UID, 2, 334, 3220, NPC, 18, 629);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 334, 3220, NPC, 18, 632);
		end
	end
end

if (EVENT == 629) then
	ShowMap(UID, 304);
end

if (EVENT == 632) then
	ShowMap(UID, 336);
end

if (EVENT == 630) then
			RunQuestExchange(UID,329);
			SaveEvent(UID, 3404);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 723) then
	SelectMsg(UID, 2, 359, 5103, NPC, 10, 724);
end

if (EVENT == 724) then
	SelectMsg(UID, 4, 359, 5104, NPC, 22, 725, 23, -1);
end

if (EVENT == 725) then
	SaveEvent(UID, 5102);
	SelectMsg(UID, 2, 359, 5105, NPC, 10, -1);
end

if (EVENT == 726) then
	SaveEvent(UID, 5104);
	SelectMsg(UID, 2, 359, 5109, NPC, 32, -1);
end

-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 728) then  -- WARRİOR 75 SKILL AÇMA - 10 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 359, 5110, NPC, 41, 730, 27, -1);
	else
		if (ITEM_COUNT1 < 10) then
		SelectMsg(UID, 2, 359, 5107, NPC, 18, 727);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 359, 5107, NPC, 18, 732);
		end
	end
end

if (EVENT == 727) then
	ShowMap(UID, 304);
end

if (EVENT == 729) then
	ShowMap(UID, 18);
end

if (EVENT == 732) then
	ShowMap(UID, 336);
end

if (EVENT == 730) then
			RunQuestExchange(UID,520);
			SaveEvent(UID, 5103);
			--Skill Açma Komutu--
	end
	
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 823) then 
	SelectMsg(UID, 2, 365, 5114, NPC, 10, 824);
end

if (EVENT == 824) then
	SelectMsg(UID, 4, 365, 5115, NPC, 22, 825, 23, -1);
end

if (EVENT == 825) then
	SaveEvent(UID, 5114);
	SelectMsg(UID, 2, 365, 5116, NPC, 10, -1);
end

if (EVENT == 826) then
	SaveEvent(UID, 5116);
	SelectMsg(UID, 2, 365, 5120, NPC, 32, -1);
end

-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 828) then  -- WARRİOR 80SKILL AÇMA - 15 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 365, 5121, NPC, 41, 830, 27, -1);
	else
		if (ITEM_COUNT1 < 15) then
			SelectMsg(UID, 2, 365, 5118, NPC, 18, 830);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 365, 5118, NPC, 18, 833);
		end
	end
end

if (EVENT == 829) then
	ShowMap(UID, 304);
end

if (EVENT == 832) then
	ShowMap(UID, 18);
end

if (EVENT == 833) then
	ShowMap(UID, 336);
end

if (EVENT == 830) then
			RunQuestExchange(UID,521);
			SaveEvent(UID, 5115);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------