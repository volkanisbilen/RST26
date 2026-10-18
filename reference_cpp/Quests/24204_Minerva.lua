-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24204;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 3826, NPC, 10, 193);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 3826, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 224) then
	SelectMsg(UID, 4, 310, 3038, NPC, 22, 225, 23, -1);
end

if (EVENT == 225) then
	--GiveItem(UID,900017000,7);
	SaveEvent(UID, 3093);
end

if EVENT == 228 then
   SaveEvent(UID, 3095);
   SelectMsg(UID, 1, 310, 3045, NPC, 32, -1);
end

if (EVENT == 229) then
	ITEM_COUNT1 = HowmuchItem(UID, 379044000);
	ITEM_COUNT2 = HowmuchItem(UID, 389076000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 99 and ITEM_COUNT2 > 29 and ITEM_COUNT3 > 9999999) then
		SelectMsg(UID, 4, 310, 3043, NPC, 41, 230, 27, -1);
	else
		SelectMsg(UID, 2, 310, 3038, NPC, 18, 227);
	end
end

if (EVENT == 227) then
	ShowMap(UID, 226);
end

if (EVENT == 230) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID,307);
			SaveEvent(UID, 3094);	 
end
end	

if (EVENT == 303) then
	SelectMsg(UID, 2, 327, 3048, NPC, 10, 304);
end

if (EVENT == 304) then
	SelectMsg(UID, 4, 327, 3049, NPC, 22, 305, 23, -1);
end

if (EVENT == 305) then
	SaveEvent(UID, 3113);
end

if (EVENT == 306) then
	SaveEvent(UID, 3115);
	SelectMsg(UID, 2, 327, 3054, NPC, 3009, -1);
end

if (EVENT == 308) then
	ITEM_COUNTA = HowmuchItem(UID, 379044000);
	ITEM_COUNTB = HowmuchItem(UID, 379112000);
	ITEM_COUNTC = HowmuchItem(UID, 379202000);
		if (ITEM_COUNTA < 30) then
			SelectMsg(UID, 2, 327, 3052, NPC, 18, 309);
		elseif (ITEM_COUNTB < 2) then
			SelectMsg(UID, 2, 327, 3052, NPC, 18, 311);
		elseif (ITEM_COUNTC < 50) then
			SelectMsg(UID, 2, 327, 3052, NPC, 18, 312);
		else
			SelectMsg(UID, 4, 327, 3055, NPC, 41, 310, 27, -1);
	end
end

if (EVENT == 309) then
	ShowMap(UID, 312);
end

if (EVENT == 311) then
	ShowMap(UID, 317);
end

if (EVENT == 312) then
	ShowMap(UID, 19);
end

if (EVENT == 310) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID, 308);
			SaveEvent(UID, 3116);	 
end
end

if (EVENT == 403) then
	SelectMsg(UID, 2, 328, 3056, NPC, 10, 404);
end

if (EVENT == 404) then
	SelectMsg(UID, 4, 328, 3057, NPC, 22, 405, 23, -1);
end

if (EVENT == 405) then
	SaveEvent(UID, 3133);
end

if (EVENT == 406) then
	SaveEvent(UID, 3135);
	SelectMsg(UID, 2, 328, 3062, NPC, 3006, -1);
end

if (EVENT == 408) then
	ITEM_COUNTA = HowmuchItem(UID, 379046000);
	ITEM_COUNTB = HowmuchItem(UID, 389075000);
	ITEM_COUNTC = HowmuchItem(UID, 379067000);
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 328, 3060, NPC, 18, 409);
		elseif (ITEM_COUNTB < 30) then
			SelectMsg(UID, 2, 328, 3060, NPC, 18, 411);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 328, 3060, NPC, 18, 412);
		else
		SelectMsg(UID, 4, 328, 3063, NPC, 41, 410, 27, -1);
	end
end

if (EVENT == 409) then
	ShowMap(UID, 311);
end

if (EVENT == 411) then
	ShowMap(UID, 309);
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
			RunQuestExchange(UID, 309);
			SaveEvent(UID, 3136);	 
end
end

if (EVENT == 623) then
	SelectMsg(UID, 2, 337, 3246, NPC, 10, 624);
end

if (EVENT == 624) then
	SelectMsg(UID, 4, 337, 3247, NPC, 22, 625, 23, -1);
end

if (EVENT == 625) then
	SaveEvent(UID, 3473);
	SelectMsg(UID, 2, 337, 3248, NPC, 10, -1);
end

if (EVENT == 626) then
	SaveEvent(UID, 3475);
	SelectMsg(UID, 2, 337, 3252, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 628) then
	ITEM_COUNT1 = HowmuchItem(UID, 379249000);
	ITEM_COUNT2 = HowmuchItem(UID, 379250000);
	ITEM_COUNT3 = HowmuchItem(UID, 379066000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 337, 3253, NPC, 41, 630, 27, -1);
	else
		if (ITEM_COUNT1 < 1 or ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 54, 3250, NPC, 18, 629);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 54, 3250, NPC, 18, 632);
		end
	end
end

if (EVENT == 629) then
	ShowMap(UID, 303);
end

if (EVENT == 632) then
	ShowMap(UID, 336);
end

if (EVENT == 630) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID,332);
			SaveEvent(UID, 3474);
			--Skill Açma Komutu-- 
end
end	
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------


if (EVENT == 197) then
	SelectMsg(UID, 4, 456, 3009, NPC, 3018, 198, 3019, -1);
end

if (EVENT == 198) then
	QuestStatusCheck = GetQuestStatus(UID, 456) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 3502);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 3507);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 3512);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 3517);
		end
	end
end

if (EVENT == 201) then
	QuestStatusCheck = GetQuestStatus(UID, 456) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 3504);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 3509);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 3514);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 3519);
		end
	end
end

if EVENT == 210 then
   ITEM_COUNT = HowmuchItem(UID, 900033000);
   if ITEM_COUNT > 0 then
		SelectMsg(UID, 4, 456, 3022, NPC, 10, 214, 27, -1);   
   else
		SelectMsg(UID, 2, 456, 3019, NPC, 18, 213);
   end
end

if (EVENT == 213) then
	ShowMap(UID, 308);
end

if (EVENT == 214) then
	QuestStatusCheck = GetQuestStatus(UID, 456) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
   ITEM_COUNT = HowmuchItem(UID, 900033000);
		if (ITEM_COUNT == 0) then
		SelectMsg(UID, 2, 456, 3019, NPC, 18, 213);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,210);
			SaveEvent(UID, 3503);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,211);
			SaveEvent(UID, 3508);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,212);
			SaveEvent(UID, 3513);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,213);
			SaveEvent(UID, 3518);
			end
		end
	end
end

if (EVENT == 532) then   
	SelectMsg(UID, 4, 273, 4120, NPC, 22, 533, 23, -1);
end

if (EVENT == 533) then
	SaveEvent(UID, 4118);
end

if (EVENT == 535) then
	SaveEvent(UID, 4120);
	SelectMsg(UID, 2, 273, 4124, NPC, 4080, -1);
end

if (EVENT == 536) then
	ITEM_COUNTA = HowmuchItem(UID, 379047000);
	ITEM_COUNTB = HowmuchItem(UID, 379236000);
	ITEM_COUNTC = HowmuchItem(UID, 379067000);
	if (ITEM_COUNTA > 0 and ITEM_COUNTB > 0 and ITEM_COUNTC > 0) then 
		SelectMsg(UID, 4, 273, 4125, NPC, 4062, 537, 4063, -1);
	else
		if (ITEM_COUNTA < 1) then
			SelectMsg(UID, 2, 273, 4121, NPC, 18, 538);
		elseif (ITEM_COUNTB < 1) then
			SelectMsg(UID, 2, 273, 4121, NPC, 18, 539);
		elseif (ITEM_COUNTC < 1) then
			SelectMsg(UID, 2, 273, 4121, NPC, 18, 540);
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
	ShowMap(UID, 710);
end

if (EVENT == 537) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID,464);
			SaveEvent(UID, 4119);
			PromoteUser(UID);	 
end
end

if (EVENT == 723) then
	SelectMsg(UID, 2, 349, 5191, NPC, 10, 724);
end

if (EVENT == 724) then
	SelectMsg(UID, 4, 349, 5192, NPC, 22, 725, 23, -1);
end

if (EVENT == 725) then
	SaveEvent(UID, 5204);
	SelectMsg(UID, 2, 349, 5193, NPC, 10, -1);
end

if (EVENT == 726) then
	SaveEvent(UID, 5206);
	SelectMsg(UID, 2, 349, 5197, NPC, 32, -1);
end


-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 728) then
	ITEM_COUNT1 = HowmuchItem(UID, 379250000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 9999999) then
	SelectMsg(UID, 2, 349, 5195, NPC, 18, 730);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5195, NPC, 18, 729);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, savenum, 5195, NPC, 18, 732);
		elseif (ITEM_COUNT3 < 10000000) then
			SelectMsg(UID, 2, savenum, 5195, NPC, 18, 733);
		end
	end
end

if (EVENT == 729) then
	ShowMap(UID, 303);
end

if (EVENT == 732) then
	ShowMap(UID, 19);
end

if (EVENT == 733) then
	ShowMap(UID, 336);
end

if (EVENT == 730) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID, 528);
			SaveEvent(UID, 5205);
			--Skill Açma Komutu-- 
end
end	
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------



if (EVENT == 1302) then
	SelectMsg(UID, 4, 531, 20245, NPC, 4552, 1304);
end

if (EVENT == 1304) then
	SaveEvent(UID, 11218);
end

if (EVENT == 1303) then
	SelectMsg(UID, 2, 531, 20245, NPC, 4552, 1305);
end

if (EVENT == 1305) then
	SaveEvent(UID, 11220);
	SaveEvent(UID, 11219);
end

if (EVENT == 1402) then
	SelectMsg(UID, 4, 532, 20032, NPC, 22, 1403,23,-1);
end

if (EVENT == 1403) then
	QuestStatus = GetQuestStatus(UID, 532)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11230);
	end
end

if (EVENT == 1408) then
	QuestStatus = GetQuestStatus(UID, 532)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11232);
	end
end

if (EVENT == 1405) then
	QuestStatus = GetQuestStatus(UID, 532)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389083000);   
	ITEM2_COUNT = HowmuchItem(UID, 379006000);  
	ITEM3_COUNT = HowmuchItem(UID, 379062000);  
		if (ITEM1_COUNT < 1 and ITEM2_COUNT < 3 and ITEM3_COUNT < 100) then
			SelectMsg(UID, 2, 532, 20032, NPC, 18,1406);
		else
			SelectMsg(UID, 4, 532, 20032, NPC, 22, 1407, 27, -1); 
		end
	end
end

if (EVENT == 1406) then
	ShowMap(UID, 414);
end

if (EVENT == 1407) then
	QuestStatus = GetQuestStatus(UID, 532)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389083000);   
	ITEM2_COUNT = HowmuchItem(UID, 379006000);  
	ITEM3_COUNT = HowmuchItem(UID, 379062000);  
		if (ITEM1_COUNT < 1 and ITEM2_COUNT < 3 and ITEM3_COUNT < 100) then
			SelectMsg(UID, 2, 532, 20032, NPC, 18,1406);
		else
			RunQuestExchange(UID,3019);
			SaveEvent(UID, 11231);
			SaveEvent(UID, 11236);
		end
	end
end

if (EVENT == 823) then
	SelectMsg(UID, 2, 357, 5202, NPC, 10, 824);
end

if (EVENT == 824) then
	SelectMsg(UID, 4, 357, 5203, NPC, 22, 825, 23, -1);
end

if (EVENT == 825) then
	SaveEvent(UID, 5216);
	SelectMsg(UID, 2, 357, 5204, NPC, 10, 193);
end

if (EVENT == 826) then
	SaveEvent(UID, 5218);
	SelectMsg(UID, 2, 357, 5208, NPC, 32, 193);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 828) then
	ITEM_COUNT1 = HowmuchItem(UID, 379249000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 9999999) then
	SelectMsg(UID, 2, 357, 5206, NPC, 10, 830);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5206, NPC, 10, 829);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, savenum, 5206, NPC, 10, 832);
		elseif (ITEM_COUNT3 < 10000000) then
			SelectMsg(UID, 2, savenum, 5206, NPC, 10, 833);
		end
	end
end

if (EVENT == 829) then
	ShowMap(UID, 303);
end

if (EVENT == 832) then
	ShowMap(UID, 19);
end

if (EVENT == 833) then
	ShowMap(UID, 336);
end

if (EVENT == 830) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID, 529);
			SaveEvent(UID, 5217);
			--Skill Açma Komutu-- 
end
end	
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------

if (EVENT == 923) then
	SelectMsg(UID, 2, 362, 5213, NPC, 10, 924);
end

if (EVENT == 924) then
	SelectMsg(UID, 4, 362, 5214, NPC, 22, 925, 23, -1);
end

if (EVENT == 925) then
	SaveEvent(UID, 5228);
	SelectMsg(UID, 2, 362, 5215, NPC, 10, -1);
end

if (EVENT == 926) then
	SaveEvent(UID, 5230);
	SelectMsg(UID, 2, 362, 5219, NPC, 32, -1);
end

-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 928) then
	ITEM_COUNT1 = HowmuchItem(UID, 379250000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 1 and ITEM_COUNT3 > 9999999) then
	SelectMsg(UID, 4, 362, 5220, NPC, 41, 930, 27, -1);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5217, NPC, 18, 929);
		elseif (ITEM_COUNT2 < 2) then
			SelectMsg(UID, 2, savenum, 5217, NPC, 18, 932);
		elseif (ITEM_COUNT3 < 10000000) then
			SelectMsg(UID, 2, savenum, 5217, NPC, 18, 933);
		end
	end
end

if (EVENT == 932) then
	ShowMap(UID, 19);
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
			--Skill Açma Komutu--
			RunQuestExchange(UID, 530);
			SaveEvent(UID, 5229);	 
end
end

-------------------------------------------------------------------------------
-------------------------------------------------------------------------------


if (EVENT == 1023) then
	SelectMsg(UID, 2, 363, 5224, NPC, 10, 1024);
end

if (EVENT == 1024) then
	SelectMsg(UID, 4, 363, 5225, NPC, 22, 1025, 23, -1);
end

if (EVENT == 1025) then
	SaveEvent(UID, 5240);
	SelectMsg(UID, 2, 363, 5226, NPC, 10, -1);
end

if (EVENT == 1026) then
	SaveEvent(UID, 5242);
	SelectMsg(UID, 2, 363, 5230, NPC, 32, -1);
end


-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 1028) then
	ITEM_COUNT1 = HowmuchItem(UID, 379249000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 1 and ITEM_COUNT3 > 9999999) then
	SelectMsg(UID, 4, 363, 5231, NPC, 41, 1030, 27, -1);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5228, NPC, 18, 1029);
		elseif (ITEM_COUNT2 < 2) then
			SelectMsg(UID, 2, savenum, 5228, NPC, 18, 1032);
		elseif (ITEM_COUNT3 < 10000000) then
			SelectMsg(UID, 2, savenum, 5228, NPC, 18, 1033);
		end
	end
end

if (EVENT == 1029) then
	ShowMap(UID, 303);
end

if (EVENT == 1032) then
	ShowMap(UID, 19);
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
			RunQuestExchange(UID, 531);
			SaveEvent(UID, 5241);
			--Skill Açma Komutu-- 
end
end	
------------------------------------------------------------------------------
-------------------------------------------------------------------------------


if (EVENT == 1123) then
	SelectMsg(UID, 2, 364, 5235, NPC, 10, 1124);
end

if (EVENT == 1124) then
	SelectMsg(UID, 4, 364, 5236, NPC, 22, 1125, 23, -1);
end

if (EVENT == 1125) then
	SaveEvent(UID, 5252);
	SelectMsg(UID, 2, 364, 5237, NPC, 10, 193);
end

if (EVENT == 1126) then
	SaveEvent(UID, 5254);
	SelectMsg(UID, 2, 364, 5241, NPC, 32, 193);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 1128) then
	ITEM_COUNT1 = HowmuchItem(UID, 379250000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 2 and ITEM_COUNT3 > 9999999) then
	SelectMsg(UID, 4, 364, 5242, NPC, 41, 1130, 27, -1);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5239, NPC, 18, 1129);
		elseif (ITEM_COUNT2 < 3) then
			SelectMsg(UID, 2, savenum, 5239, NPC, 18, 1132);
		elseif (ITEM_COUNT3 < 10000000) then
			SelectMsg(UID, 2, savenum, 5239, NPC, 18, 1133);
		end
	end
end

if (EVENT == 1129) then
	ShowMap(UID, 303);
end

if (EVENT == 1132) then
	ShowMap(UID, 19);
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
			RunQuestExchange(UID, 532);
			SaveEvent(UID, 5253); 
			--Skill Açma Komutu-- 
end
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------


if (EVENT == 1223) then
	SelectMsg(UID, 2, 368, 5246, NPC, 10, 1224);
end

if (EVENT == 1224) then
	SelectMsg(UID, 4, 368, 5247, NPC, 22, 1225, 23, -1);
end

if (EVENT == 1225) then
	SaveEvent(UID, 5264);
	SelectMsg(UID, 2, 368, 5248, NPC, 10, -1);
end

if (EVENT == 1226) then
	SaveEvent(UID, 5266);
	SelectMsg(UID, 2, 368, 5252, NPC, 32, -1);
end
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------
if (EVENT == 1228) then
	ITEM_COUNT1 = HowmuchItem(UID, 379249000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 2 and ITEM_COUNT3 > 9999999) then
		SelectMsg(UID, 4, 368, 5253, NPC, 41, 1230, 27, -1);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, savenum, 5251, NPC, 18, 1229);
		elseif (ITEM_COUNT2 < 3) then
			SelectMsg(UID, 2, savenum, 5251, NPC, 18, 1232);
		elseif (ITEM_COUNT3 < 10000000) then
			SelectMsg(UID, 2, savenum, 5251, NPC, 18, 1233);
		end
	end
end

if (EVENT == 1229) then
	ShowMap(UID, 303);
end

if (EVENT == 1232) then
	ShowMap(UID, 19);
end

if (EVENT == 1233) then
	ShowMap(UID, 336);
end

if (EVENT == 1230) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID, 533);
			SaveEvent(UID, 5265);
			--Skill Açma Komutu-- 
end
end	
-------------------------------------------------------------------------------
-------------------------------------------------------------------------------