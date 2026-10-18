-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24203;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 335, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 336,NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 200) then
	SelectMsg(UID, 2, 217, 1014, NPC, 10, 201);
end

if (EVENT == 201) then
	SelectMsg(UID, 4, 217, 1015, NPC, 22, 202, 23, -1);
end

if (EVENT == 202) then
	QuestStatusCheck = GetQuestStatus(UID, 217) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		SaveEvent(UID, 136);
	end
end

if (EVENT == 205) then
	QuestStatusCheck = GetQuestStatus(UID, 217) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		SaveEvent(UID, 138);
		SelectMsg(UID, 2, 217, 1016, NPC, 21, -1);
	end
end

if (EVENT == 210) then
	QuestStatusCheck = GetQuestStatus(UID, 217) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910090000);
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, 217, 1017, NPC, 18, 213);
		else
			SelectMsg(UID, 4, 217, 1018, NPC, 41, 214, 27, -1);
		end
	end
end

if (EVENT == 213) then
	ShowMap(UID, 16);
end

if (EVENT == 214) then
	QuestStatusCheck = GetQuestStatus(UID, 217) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910090000);
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, 217, 1017, NPC, 18, 213);
		else
			RunQuestExchange(UID,34);       
			SaveEvent(UID, 137);
		end
	end
end


if (EVENT == 336) then
	SelectMsg(UID, 2, 308, 375, NPC, 10, 337);
end

if (EVENT == 337) then
	SelectMsg(UID, 4, 308, 376, NPC, 22, 338, 23, -1);
end

if (EVENT == 338) then
	SaveEvent(UID, 556);
	--GiveItem(UID,900017000,7);
end

if (EVENT == 339) then
	SaveEvent(UID, 558);
	SelectMsg(UID, 2, 308, 378, NPC, 21, -1);
end

if (EVENT == 341) then
	ITEM_COUNT1 = HowmuchItem(UID, 900017000);
	if (ITEM_COUNT1 < 7) then
		SelectMsg(UID, 2, 308, 379, NPC, 18, 342);
	else
		SelectMsg(UID, 4, 308, 380, NPC, 41, 346, 27, -1);
	end
end

if (EVENT == 342) then
	ShowMap(UID, 726);
end

if (EVENT == 346) then
	ITEM_COUNT1 = HowmuchItem(UID, 900017000);
		if (ITEM_COUNT1 < 7) then
			SelectMsg(UID, 2, 308, 379, NPC, 18, 342);
		else
			RunQuestExchange(UID,38);
			SaveEvent(UID, 559);
	end
end

if (EVENT == 551) then
	SelectMsg(UID, 4, 273, 4108, NPC, 22, 552, 23, -1);
end

if (EVENT == 552) then
	SaveEvent(UID, 4106);
end

if (EVENT == 535) then
	SelectMsg(UID, 2, 273, 4110, NPC, 10, -1);
	SaveEvent(UID, 4108);
end

if (EVENT == 536) then
	ITEM01 = HowmuchItem(UID, 810095000);
	ITEM02 = HowmuchItem(UID, 810091000);
	ITEM03 = HowmuchItem(UID, 810092000);
	if (ITEM01 > 0 and ITEM02 > 0 and ITEM03 > 0) then
		SelectMsg(UID, 4, 273, 4111, NPC, 4006, 537, 4005, -1);
	else
		if (ITEM01 < 1) then
			SelectMsg(UID, 2, 273, 4109, NPC, 18, 538);
		elseif (ITEM02 < 1) then
			SelectMsg(UID, 2, 273, 4109, NPC, 18, 539);
		elseif (ITEM03 < 1) then
			SelectMsg(UID, 2, 273, 4109, NPC, 18, 540);
		end
	end
end

if (EVENT == 538) then
	ShowMap(UID, 681);
end

if (EVENT == 539) then
	ShowMap(UID, 714);
end

if (EVENT == 540) then
	ShowMap(UID, 712);
end

if (EVENT == 537) then
	ITEM01 = HowmuchItem(UID, 810095000);
	ITEM02 = HowmuchItem(UID, 810091000);
	ITEM03 = HowmuchItem(UID, 810092000);
		if (ITEM01 < 1) then
			SelectMsg(UID, 2, 273, 4109, NPC, 18, 538);
		elseif (ITEM02 < 1) then
			SelectMsg(UID, 2, 273, 4109, NPC, 18, 539);
		elseif (ITEM03 < 1) then
			SelectMsg(UID, 2, 273, 4109, NPC, 18, 540);
		else
			RunQuestExchange(UID,463);
			PromoteUser(UID);
			SaveEvent(UID, 4107);
	end
end


if (EVENT == 349) then
	SelectMsg(UID, 2, 323, 386, NPC, 10, 350);
end

if (EVENT == 350) then
	SelectMsg(UID, 4, 323, 387, NPC, 22, 351, 23, -1);
end

if (EVENT == 351) then
	SaveEvent(UID, 562);
end

if (EVENT == 352) then
	SaveEvent(UID, 564);
	SelectMsg(UID, 2, 323, 389, NPC, 21, -1);
end

if (EVENT == 354) then
	ITEM_COUNT1 = HowmuchItem(UID, 379046000);
	ITEM_COUNT2 = HowmuchItem(UID, 379041000);
	ITEM_COUNT3 = HowmuchItem(UID, 379236000);
	if (ITEM_COUNT1 < 1) then
		SelectMsg(UID, 2, 323, 390, NPC, 18, 355);
	elseif (ITEM_COUNT2 < 1) then
		SelectMsg(UID, 2, 323, 390, NPC, 18, 356);
	elseif (ITEM_COUNT3 < 2) then
		SelectMsg(UID, 2, 323, 390, NPC, 18, 357);
	elseif (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 1) then
		SelectMsg(UID, 4, 323, 393, NPC, 41, 358, 27, -1);
	end
end

if (EVENT == 355) then
	ShowMap(UID, 24);
end

if (EVENT == 356) then
	ShowMap(UID, 317);
end

if (EVENT == 357) then
	ShowMap(UID, 19);
end

if (EVENT == 358) then
	ITEM_COUNT1 = HowmuchItem(UID, 379046000);
	ITEM_COUNT2 = HowmuchItem(UID, 379041000);
	ITEM_COUNT3 = HowmuchItem(UID, 379236000);
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 323, 390, NPC, 18, 355);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 323, 390, NPC, 18, 356);
		elseif (ITEM_COUNT3 < 2) then
			SelectMsg(UID, 2, 323, 390, NPC, 18, 357);
		else
			RunQuestExchange(UID,39);
			SaveEvent(UID, 565);
	end
end

if (EVENT == 361) then
	SelectMsg(UID, 2, 325, 501, NPC, 10, 362);
end

if (EVENT == 362) then
	SelectMsg(UID, 4, 325, 502, NPC, 22, 363, 23, -1);
end

if (EVENT == 363) then
	SaveEvent(UID, 568);
end

if (EVENT == 364) then
	SaveEvent(UID, 570);
	SelectMsg(UID, 2, 325, 503, NPC, 32, -1);
end

if (EVENT == 366) then
	ITEM_COUNT1 = HowmuchItem(UID, 320410012);
	ITEM_COUNT2 = HowmuchItem(UID, 320410013);
	ITEM_COUNT3 = HowmuchItem(UID, 379067000);
	if (ITEM_COUNT1 < 1) then
		SelectMsg(UID, 2, 325, 505, NPC, 18, 367);
	elseif (ITEM_COUNT2 < 1) then
		SelectMsg(UID, 2, 325, 506, NPC, 18, 368);
	elseif (ITEM_COUNT3 < 1) then
		SelectMsg(UID, 2, 325, 507, NPC, 18, 369);
	elseif (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 0) then 
		SelectMsg(UID, 4, 325, 508, NPC, 41, 370, 27, -1);
	end
end

if (EVENT == 367) then
	ShowMap(UID, 310);
end

if (EVENT == 368) then
	ShowMap(UID, 311);
end

if (EVENT == 369) then
	ShowMap(UID, 30);
end

if (EVENT == 370) then
	ITEM_COUNT1 = HowmuchItem(UID, 320410012);
	ITEM_COUNT2 = HowmuchItem(UID, 320410013);
	ITEM_COUNT3 = HowmuchItem(UID, 379067000);
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 325, 505, NPC, 18, 367);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 325, 506, NPC, 18, 368);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 325, 507, NPC, 18, 369);
		else
			RunQuestExchange(UID,40);
			SaveEvent(UID, 571);
	end
end


if (EVENT == 373) then
	SelectMsg(UID, 2, 235, 511, NPC, 10, 374);
end

if (EVENT == 374) then
	SelectMsg(UID, 4, 235, 512, NPC, 22, 375, 23, -1);
end

if (EVENT == 375) then
	SaveEvent(UID, 574);
end

if (EVENT == 376) then
	SaveEvent(UID, 576);
	SelectMsg(UID, 2, 235, 514, NPC, 21, -1);
end

if (EVENT == 378) then
	ITEM_COUNT1 = HowmuchItem(UID, 389074000);
	ITEM_COUNT2 = HowmuchItem(UID, 389075000);
	ITEM_COUNT3 = HowmuchItem(UID, 389076000);
	if (ITEM_COUNT1 < 3) then
		SelectMsg(UID, 2, 235, 515, NPC, 18, 379);
	elseif (ITEM_COUNT2 < 3) then
		SelectMsg(UID, 2, 235, 516, NPC, 18, 379);
	elseif (ITEM_COUNT3 < 3) then
		SelectMsg(UID, 2, 235, 517, NPC, 18, 379);
	elseif (ITEM_COUNT1 > 2 and ITEM_COUNT2 > 2 and ITEM_COUNT3 > 2) then
		SelectMsg(UID, 4, 235, 518, NPC, 41, 382, 27, -1);
	end   
end

if (EVENT == 379) then
	ShowMap(UID, 23);
end

if (EVENT == 382) then
	ITEM_COUNT1 = HowmuchItem(UID, 389074000);
	ITEM_COUNT2 = HowmuchItem(UID, 389075000);
	ITEM_COUNT3 = HowmuchItem(UID, 389076000);
		if (ITEM_COUNT1 < 3) then
			SelectMsg(UID, 2, 235, 515, NPC, 18, 379);
		elseif (ITEM_COUNT2 < 3) then
			SelectMsg(UID, 2, 235, 516, NPC, 18, 379);
		elseif (ITEM_COUNT3 < 3) then
			SelectMsg(UID, 2, 235, 517, NPC, 18, 379);
		else
			RunQuestExchange(UID,41);
			SaveEvent(UID, 577);
	end
end

if (EVENT == 623) then
	SelectMsg(UID, 2, 336, 3236, NPC, 10, 624);
end

if (EVENT == 624) then
	SelectMsg(UID, 4, 336, 3237, NPC, 22, 625, 23, -1);
end

if (EVENT == 625) then
	SaveEvent(UID, 3453);
	SelectMsg(UID, 2, 336, 3238, NPC, 10, -1);
end

if (EVENT == 626) then
	SaveEvent(UID, 3455);
	SelectMsg(UID, 2, 336, 3242, NPC, 32, -1);
end

-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 628) then -- MAGE 70 SKILL AÇMA - 5 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 4 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 336, 3243, NPC, 41, 630, 27, -1);
	else
		if (ITEM_COUNT1 < 5) then
			SelectMsg(UID, 2, 336, 3240, NPC, 18, 629);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 336, 3240, NPC, 18, 632);
		end
	end
end

if (EVENT == 629) then
	ShowMap(UID, 19);
end

if (EVENT == 632) then
	ShowMap(UID, 336);
end

if (EVENT == 629) then
	ShowMap(UID, 304);
end

if (EVENT == 630) then
			RunQuestExchange(UID,331);
			SaveEvent(UID, 3454);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 923) then
	SelectMsg(UID, 2, 348, 5158, NPC, 10, 924);
end

if (EVENT == 924) then
	SelectMsg(UID, 4, 348, 5159, NPC, 22, 925, 23, -1);
end

if (EVENT == 925) then
	SaveEvent(UID, 5168);
	SelectMsg(UID, 2, 348, 5160, NPC, 10, -1);
end

if (EVENT == 926) then
	SaveEvent(UID, 5170);
	SelectMsg(UID, 2, 348, 5164, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 928) then  -- MAGE 72 SKILL AÇMA - 7 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 6 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 348, 5165, NPC, 41, 930, 27, -1);
	else
		if (ITEM_COUNT1 < 7) then
			SelectMsg(UID, 2, 348, 5162, NPC, 18, 929);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 348, 5162, NPC, 18, 932);
		end
	end
end

if (EVENT == 929) then
	ShowMap(UID, 19);
end

if (EVENT == 932) then
	ShowMap(UID, 336);
end

if (EVENT == 930) then
			RunQuestExchange(UID,525);
			SaveEvent(UID, 5169);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 1023) then
	SelectMsg(UID, 2, 361, 5169, NPC, 10, 1024);
end

if (EVENT == 1024) then
	SelectMsg(UID, 4, 361, 5170, NPC, 22, 1025, 23, -1);
end

if (EVENT == 1025) then
	SaveEvent(UID, 5174);
	SelectMsg(UID, 2, 361, 5171, NPC, 10, -1);
end

if (EVENT == 1026) then
	SaveEvent(UID, 5176);
	SelectMsg(UID, 2, 361, 5175, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 1028) then  -- MAGE 75 SKILL AÇMA - 10 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 9 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 361, 5176, NPC, 41, 1030, 27, -1);
	else
		if (ITEM_COUNT1 < 10) then
			SelectMsg(UID, 2, 361, 5173, NPC, 18, 1029);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 361, 5173, NPC, 18, 1032);
		end
	end
end

if (EVENT == 1029) then
	ShowMap(UID, 19);
end

if (EVENT == 1032) then
	ShowMap(UID, 336);
end

if (EVENT == 1030) then
			RunQuestExchange(UID,526);
			SaveEvent(UID, 5181);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 1123) then
	SelectMsg(UID, 2, 367, 5180, NPC, 10, 1124);
end

if (EVENT == 1124) then
	SelectMsg(UID, 4, 367, 5181, NPC, 22, 1125, 23, -1);
end

if (EVENT == 1125) then
	SaveEvent(UID, 5192);
	SelectMsg(UID, 2, 367, 5182, NPC, 10, -1);
end

if (EVENT == 1126) then
	SaveEvent(UID, 5194);
	SelectMsg(UID, 2, 367, 5186, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 1128) then    -- MAGE 80 SKILL AÇMA - 10 SPELL İSTİYOR
	ITEM_COUNT1 = HowmuchItem(UID, 810369000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 14 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 367, 5187, NPC, 41, 1130, 27, -1);
	else
		if (ITEM_COUNT1 < 15) then
			SelectMsg(UID, 2, 367, 5184, NPC, 18, 1129);
		elseif (ITEM_COUNT3 < 0) then
			SelectMsg(UID, 2, 367, 5184, NPC, 18, 1132);
		end
	end
end

if (EVENT == 1129) then
	ShowMap(UID, 19);
end

if (EVENT == 1132) then
	ShowMap(UID, 336);
end

if (EVENT == 1130) then
			RunQuestExchange(UID,527);
			SaveEvent(UID, 5193);
			--Skill Açma Komutu--
	end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------


if (EVENT == 1302) then
	SelectMsg(UID, 4, 528, 20025, NPC, 22, 1303, 27, -1);
end

if (EVENT == 1303) then
	QuestStatusCheck = GetQuestStatus(UID, 528) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
			SaveEvent(UID, 11176);
	end
end

if (EVENT == 1308) then
	QuestStatusCheck = GetQuestStatus(UID, 528) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 508104000);   
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, 528, 20025, NPC, 18,1306);
		else
			SaveEvent(UID, 11178);
		end
	end
end

if (EVENT == 1305) then
	QuestStatusCheck = GetQuestStatus(UID, 528) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 508104000);   
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, 528, 20025, NPC, 18,1306);
		else
			SelectMsg(UID, 5, 528, 20025, NPC, 22, 1307,27, -1); 
		end
	end
end

if (EVENT == 1306) then
	ShowMap(UID, 17);
end

if (EVENT == 1307)then
	QuestStatusCheck = GetQuestStatus(UID, 528) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 508104000);   
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, 528, 20025, NPC, 18,1306);
		else
			RunQuestExchange(UID,3015,STEP,1);
			SaveEvent(UID,11177);
			SaveEvent(UID,11188);
		end
	end
end