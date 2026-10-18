-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14204;

if EVENT == 190 then
	QuestNum = SearchQuest(UID, NPC);
	if QuestNum == 0 then
	   SelectMsg(UID, 2, -1, 3826, NPC, 10, -1);
	elseif QuestNum > 1 and  QuestNum < 100 then
       NpcMsg(UID, 3826, NPC)
    else
       EVENT = QuestNum
	end
end

if EVENT == 224 then
   SelectMsg(UID, 4, 310, 3038, NPC, 22, 225, 23, -1);
end

if EVENT == 225 then
	--GiveItem(UID,900017000,7);
	SaveEvent(UID, 3083);
end

if EVENT == 228 then
   SaveEvent(UID, 3085);
   SelectMsg(UID, 1, 310, 3045, NPC, 32, -1);
end

if (EVENT == 229) then
	ITEM_COUNT1 = HowmuchItem(UID, 379044000);
	ITEM_COUNT2 = HowmuchItem(UID, 389076000);
	ITEM_COUNT3 = HowmuchItem(UID, 900000000);
	if (ITEM_COUNT1 > 99 and ITEM_COUNT2 > 29 and ITEM_COUNT3 > 9999999) then
		SelectMsg(UID, 4, 310, 3071, NPC, 41, 230, 27, -1);
	else
		SelectMsg(UID, 2, 310, 3068, NPC, 18, 227);
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
			SaveEvent(UID, 3086);	 
end
end

if EVENT == 303 then
   SelectMsg(UID, 2, 327, 3048, NPC, 10, 304);
end

if EVENT == 304 then
   SelectMsg(UID, 4, 327, 3049, NPC, 22, 305, 23, -1);
end

if EVENT == 305 then
   SaveEvent(UID, 3103);
end

if EVENT == 306 then
   SaveEvent(UID, 3105);
end

if (EVENT == 308) then
	ITEM_COUNT1 = HowmuchItem(UID, 379044000);
	ITEM_COUNT2 = HowmuchItem(UID, 379112000);
	ITEM_COUNT3 = HowmuchItem(UID, 379202000);
	if (ITEM_COUNT1 > 29 and ITEM_COUNT2 > 1 and ITEM_COUNT3 > 49) then
		SelectMsg(UID, 4, 327, 3055, NPC, 10, 310, 27, -1);
	else
		if (ITEM_COUNT1 < 30) then
			SelectMsg(UID, 2, 327, 3052, NPC, 18, 309);
		elseif (ITEM_COUNT2 < 2) then
			SelectMsg(UID, 2, 327, 3053, NPC, 18, 311);
		elseif (ITEM_COUNT3 < 50) then
			SelectMsg(UID, 2, 327, 3054, NPC, 18, 312);
		end
	end
end

if EVENT == 309 then
   ShowMap(UID, 7);
end

if EVENT == 311 then
   ShowMap(UID, 8);
end

if EVENT == 312 then
   ShowMap(UID, 9);
end

if EVENT == 310 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID, 308);
			SaveEvent(UID, 3106);	 
end
end

if EVENT == 403 then
   SelectMsg(UID, 2, 328, 3056, NPC, 10, 404);
end

if EVENT == 404 then
   SelectMsg(UID, 4, 328, 3057, NPC, 22, 405, 23, -1);
end

if EVENT == 405 then
   SaveEvent(UID, 3123);
end

if EVENT == 406 then
   SaveEvent(UID, 3125);
end

if (EVENT == 408) then
	ITEM_COUNT1 = HowmuchItem(UID, 379046000);
	ITEM_COUNT2 = HowmuchItem(UID, 389075000);
	ITEM_COUNT3 = HowmuchItem(UID, 379067000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 29 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 328, 5121, NPC, 41, 410, 27, -1);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 328, 3063, NPC, 18, -1);
		elseif (ITEM_COUNT2 < 30) then
			SelectMsg(UID, 2, 328, 3060, NPC, 18, -1);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 328, 3060, NPC, 18, -1);
		end
	end
end


if EVENT == 409 then
   ShowMap(UID, 7);
end

if EVENT == 410 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID, 309);
			SaveEvent(UID, 3126);	 
end
end

if EVENT == 532 then   
    SelectMsg(UID, 4, 273, 4120, NPC, 22, 533, 23, -1);
end

if EVENT == 533 then
   SaveEvent(UID, 4124);
end

if EVENT == 535 then
   SaveEvent(UID, 4126);
end

if (EVENT == 536) then
	ITEM_COUNT1 = HowmuchItem(UID, 379047000);
	ITEM_COUNT2 = HowmuchItem(UID, 379236000);
	ITEM_COUNT3 = HowmuchItem(UID, 379067000);
	if (ITEM_COUNT1 > 0 and ITEM_COUNT2 > 0 and ITEM_COUNT3 > 0) then
		SelectMsg(UID, 4, 273, 4125, NPC, 41, 537, 27, -1);
	else
		if (ITEM_COUNT1 < 1) then
			SelectMsg(UID, 2, 273, 4121, NPC, 18, 538);
		elseif (ITEM_COUNT2 < 1) then
			SelectMsg(UID, 2, 273, 4122, NPC, 18, 539);
		elseif (ITEM_COUNT3 < 1) then
			SelectMsg(UID, 2, 273, 4123, NPC, 18, 540);
		end
	end
end

if (EVENT == 538) then
	ShowMap(UID, 302);
end

if (EVENT == 539) then
	ShowMap(UID, 302);
end

if (EVENT == 540) then
	ShowMap(UID, 302);
end

if EVENT == 537 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID,464);
			SaveEvent(UID, 4125);
			PromoteUser(UID);	 
end
end

if EVENT == 623 then
   SelectMsg(UID, 2, 337, 3246, NPC, 10, 624);
end


if EVENT == 624 then
   SelectMsg(UID, 4, 337, 3247, NPC, 22, 625, 23, -1);
end

if EVENT == 625 then
   SaveEvent(UID, 3463);
   SelectMsg(UID, 2, 337, 3248, NPC, 10, -1);
end

if EVENT == 626 then
   SaveEvent(UID, 3465);
   SelectMsg(UID, 1, 337, 3252, NPC, 32, -1);
end


if EVENT == 628 then
   ITEM_COUNT1 = HowmuchItem(UID, 379249000);
   ITEM_COUNT2 = HowmuchItem(UID, 379250000);
   ITEM_COUNT3 = HowmuchItem(UID, 379066000);
   if ITEM_COUNT1  > 0 and ITEM_COUNT2  > 0 and ITEM_COUNT3  > 0 then
      SelectMsg(UID, 4, 337, 3253, NPC, 41, 630, 27, -1);
   else
      if ITEM_COUNT1 < 1 then
        SelectMsg(UID, 2, 337, 3250, NPC, 10, -1);
      elseif ITEM_COUNT2 < 1 then
         SelectMsg(UID, 2, 337, 3250, NPC, 10, -1);
      elseif ITEM_COUNT3 < 1 then
         SelectMsg(UID, 2, 337, 3250, NPC, 10, -1);
      end
   end
end

if EVENT == 629 then
   ShowMap(UID, 304);
end

if EVENT == 630 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID,332);
			SaveEvent(UID, 3464);	 
end
end

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
		SaveEvent(UID, 3523);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 3528);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 3533);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 3538);
		end
	end
end

if (EVENT == 201) then
	QuestStatusCheck = GetQuestStatus(UID, 456) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
   ITEM_COUNT = HowmuchItem(UID, 900033000);
		if  ITEM_COUNT < 1 then
			SelectMsg(UID, 2, 456, 3019, NPC, 18, 213);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			SaveEvent(UID, 3525);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			SaveEvent(UID, 3530);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			SaveEvent(UID, 3535);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			SaveEvent(UID, 3540);
			end
		end
	end
end

if EVENT == 210 then
	QuestStatusCheck = GetQuestStatus(UID, 456) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
   ITEM_COUNT = HowmuchItem(UID, 900033000);
		if  ITEM_COUNT < 1 then
			SelectMsg(UID, 2, 456, 3019, NPC, 18, 213);
		else
			SelectMsg(UID, 4, 456, 3022, NPC, 10, 214, 27, -1);   
		end
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
		if  ITEM_COUNT == 0 then
			SelectMsg(UID, 2, 456, 3019, NPC, 18, 213);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID,210);
			SaveEvent(UID, 3524);
		elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID,211);
			SaveEvent(UID, 3529);
		elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID,212);
			SaveEvent(UID, 3534);
		elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID,213);
			SaveEvent(UID, 3539);
			end
		end
	end
end

if EVENT == 723 then
    SelectMsg(UID, 2, 349, 5191, NPC, 10, 724);
end

if EVENT == 724 then
   SelectMsg(UID, 4, 349, 5192, NPC, 22, 725, 23, -1);
end

if EVENT == 725 then
   SaveEvent(UID, 5198);
   SelectMsg(UID, 2, 349, 5193, NPC, 10, -1);
end

if EVENT == 726 then
   SaveEvent(UID, 5200);
   SelectMsg(UID, 1, 349, 5197, NPC, 32, -1);
end

if EVENT == 728 then
   ITEM_COUNT1 = HowmuchItem(UID, 379250000);
   ITEM_COUNT2 = HowmuchItem(UID, 379236000);
   ITEM_COUNT3 = HowmuchItem(UID, 900000000);
   if ITEM_COUNT1  > 0 and ITEM_COUNT2  > 0 and ITEM_COUNT3 > 9999999 then
      SelectMsg(UID, 4, 349, 5198, NPC, 41, 730, 27, -1);
   else
      if ITEM_COUNT1 < 1 then
        SelectMsg(UID, 2, 349, 5195, NPC, 10, -1);
      elseif ITEM_COUNT2 < 1 then
         SelectMsg(UID, 2, 349, 5195, NPC, 10, -1);
      elseif ITEM_COUNT3 < 10000000 then
         SelectMsg(UID, 2, 349, 5195, NPC, 10, -1);
      else
         SelectMsg(UID, 2, 349, 5195, NPC, 10, -1);
      end
   end
end

if EVENT == 730 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
		RunQuestExchange(UID, 528);
		SaveEvent(UID, 5199); 	 
end
end

if EVENT == 823 then
    SelectMsg(UID, 2, 357, 5202, NPC, 10, 824);
end

if EVENT == 824 then
   SelectMsg(UID, 4, 357, 5203, NPC, 22, 825, 23, -1);
end

if EVENT == 825 then
   SaveEvent(UID, 5210);
   SelectMsg(UID, 2, 357, 5204, NPC, 10, -1);
end

if EVENT == 826 then
   SaveEvent(UID, 5212);
     SelectMsg(UID, 1, 357, 5208, NPC, 32, -1);
end

if EVENT == 828 then
   ITEM_COUNT1 = HowmuchItem(UID, 379249000);
   ITEM_COUNT2 = HowmuchItem(UID, 379236000);
   ITEM_COUNT3 = HowmuchItem(UID, 900000000);
   if ITEM_COUNT1  > 0 and ITEM_COUNT2  > 0 and ITEM_COUNT3 > 9999999 then
      SelectMsg(UID, 4, 357, 5209, NPC, 41, 830, 27, -1);
   else
      if ITEM_COUNT1 < 1 then
        SelectMsg(UID, 2, 357, 5206, NPC, 10, -1);
      elseif ITEM_COUNT2 < 1 then
         SelectMsg(UID, 2, 357, 5206, NPC, 10, -1);
      elseif ITEM_COUNT3 < 10000000 then
         SelectMsg(UID, 2, 357, 5206, NPC, 10, -1);
      else
         SelectMsg(UID, 2, 357, 5206, NPC, 10, -1);
      end
   end
end

if EVENT == 830 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID, 529);
			SaveEvent(UID, 5211);	 
end
end

if EVENT == 923 then
    SelectMsg(UID, 2, 362, 5213, NPC, 10, 924);
end

if EVENT == 924 then
   SelectMsg(UID, 4, 362, 5214, NPC, 22, 925, 23, -1);
end

if EVENT == 925 then
   SaveEvent(UID, 5222);
   SelectMsg(UID, 2, 362, 5215, NPC, 10, -1);
end

if EVENT == 926 then
   SaveEvent(UID, 5224);
   SelectMsg(UID, 1, 362, 5219, NPC, 32, -1);
end

if EVENT == 928 then
   ITEM_COUNT1 = HowmuchItem(UID, 379250000);
   ITEM_COUNT2 = HowmuchItem(UID, 379236000);
   ITEM_COUNT3 = HowmuchItem(UID, 900000000);
   if ITEM_COUNT1  > 0 and ITEM_COUNT2  > 1 and ITEM_COUNT3 > 9999999 then
      SelectMsg(UID, 4, 362, 5220, NPC, 41, 930, 27, -1);
   else
      if ITEM_COUNT1 < 1 then
        SelectMsg(UID, 2, 362, 5217, NPC, 10, -1);
      elseif ITEM_COUNT2 < 2 then
         SelectMsg(UID, 2, 362, 5217, NPC, 10, -1);
      elseif ITEM_COUNT3 < 10000000 then
         SelectMsg(UID, 2, 362, 5217, NPC, 10, -1);
      else
         SelectMsg(UID, 2, 362, 5217, NPC, 10, -1);
      end
   end
end

if EVENT == 930 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
		RunQuestExchange(UID, 530);
        SaveEvent(UID, 5223);	 
end
end

if EVENT == 1023 then
    SelectMsg(UID, 2, 363, 5224, NPC, 10, 1024);
end

if EVENT == 1024 then
   SelectMsg(UID, 4, 363, 5225, NPC, 22, 1025, 23, -1);
end

if EVENT == 1025 then
   SaveEvent(UID, 5234);
   SelectMsg(UID, 2, 363, 5226, NPC, 10, -1);
end

if EVENT == 1026 then
   SaveEvent(UID, 5236);
   SelectMsg(UID, 1, 363, 5230, NPC, 32, -1);
end

if EVENT == 1028 then
   ITEM_COUNT1 = HowmuchItem(UID, 379249000);
   ITEM_COUNT2 = HowmuchItem(UID, 379236000);
   ITEM_COUNT3 = HowmuchItem(UID, 900000000);
   if ITEM_COUNT1  > 0 and ITEM_COUNT2  > 1 and ITEM_COUNT3 > 9999999 then
      SelectMsg(UID, 4, 363, 5231, NPC, 41, 1030, 27, -1);
   else
      if ITEM_COUNT1 < 1 then
        SelectMsg(UID, 2, 363, 5228, NPC, 10, -1);
      elseif ITEM_COUNT2 < 2 then
         SelectMsg(UID, 2, 363, 5228, NPC, 10, -1);
      elseif ITEM_COUNT3 < 10000000 then
         SelectMsg(UID, 2, 363, 5228, NPC, 10, -1);
      else
         SelectMsg(UID, 2, 363, 5228, NPC, 10, -1);
      end
   end
end

if EVENT == 1030 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
		RunQuestExchange(UID, 531);
        SaveEvent(UID, 5235); 
end
end

if EVENT == 1123 then
    SelectMsg(UID, 2, 364, 5235, NPC, 10, 1124);
end

if EVENT == 1124 then
   SelectMsg(UID, 4, 364, 5236, NPC, 22, 1125, 23, -1);
end

if EVENT == 1125 then
   SaveEvent(UID, 5246);
   SelectMsg(UID, 2, 364, 5237, NPC, 10, -1);
end

if EVENT == 1126 then
   SaveEvent(UID, 5248);
   SelectMsg(UID, 1, 364, 5241, NPC, 32, -1);
end

if EVENT == 1128 then
   ITEM_COUNT1 = HowmuchItem(UID, 379250000);
   ITEM_COUNT2 = HowmuchItem(UID, 379236000);
   ITEM_COUNT3 = HowmuchItem(UID, 900000000);
   if ITEM_COUNT1  > 0 and ITEM_COUNT2  > 2 and ITEM_COUNT3 > 9999999 then
      SelectMsg(UID, 4, 364, 5242, NPC, 41, 1130, 27, -1);
   else
      if ITEM_COUNT1 < 1 then
        SelectMsg(UID, 2, 364, 5239, NPC, 10, -1);
      elseif ITEM_COUNT2 < 3 then
         SelectMsg(UID, 2, 364, 5239, NPC, 10, -1);
      elseif ITEM_COUNT3 < 10000000 then
         SelectMsg(UID, 2, 364, 5239, NPC, 10, -1);
      else
         SelectMsg(UID, 2, 364, 5239, NPC, 10, -1);
      end
   end
end

if EVENT == 1130 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
		RunQuestExchange(UID, 532);
        SaveEvent(UID, 5247);	 
end
end

local savenum = 368;

if EVENT == 1223 then
    SelectMsg(UID, 2, 368, 5246, NPC, 10, 1224);
end

if EVENT == 1224 then
   SelectMsg(UID, 4, 368, 5247, NPC, 22, 1225, 23, -1);
end

if EVENT == 1225 then
   SaveEvent(UID, 5258);
   SelectMsg(UID, 2, 368, 5248, NPC, 10, -1);
end

if EVENT == 1226 then
   SaveEvent(UID, 5260);
      SelectMsg(UID, 1, 368, 5252, NPC, 32, -1);
end

if EVENT == 1228 then
   ITEM_COUNT1 = HowmuchItem(UID, 379249000);
   ITEM_COUNT2 = HowmuchItem(UID, 379236000);
   ITEM_COUNT3 = HowmuchItem(UID, 900000000);
   if ITEM_COUNT1  > 0 and ITEM_COUNT2  > 2 and ITEM_COUNT3 > 9999999 then
      SelectMsg(UID, 4, 368, 4704, NPC, 41, 1230, 27, -1);
   else
      if ITEM_COUNT1 < 1 then
        SelectMsg(UID, 2, 368, 5251, NPC, 10, -1);
      elseif ITEM_COUNT2 < 3 then
         SelectMsg(UID, 2, 368, 5251, NPC, 10, -1);
      elseif ITEM_COUNT3 < 10000000 then
         SelectMsg(UID, 2, 368, 5251, NPC, 10, -1);
      else
         SelectMsg(UID, 2, 368, 5251, NPC, 10, -1);
      end
   end
end

if EVENT == 1230 then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
		RunQuestExchange(UID, 533);
        SaveEvent(UID, 5259);
end
end

if (EVENT == 1302) then
	SelectMsg(UID, 4, 531, 20245, NPC, 4552, 1304);
end

if (EVENT == 1304) then
	SaveEvent(UID, 11212);
end

if (EVENT == 1303) then
	SelectMsg(UID, 2, 531, 20245, NPC, 4552, 1305);
end

if (EVENT == 1305) then
	SaveEvent(UID, 11214);
	SaveEvent(UID, 11213);
end

if (EVENT == 1402) then
	SelectMsg(UID, 4, 532, 20032, NPC, 22, 1403,23,-1);
end

if (EVENT == 1403) then
	QuestStatusCheck = GetQuestStatus(UID, 532) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
			SaveEvent(UID, 11224);
	end
end

if (EVENT == 1408) then
	QuestStatusCheck = GetQuestStatus(UID, 532) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
			SaveEvent(UID, 11226);
	end
end

if (EVENT == 1405) then
	QuestStatusCheck = GetQuestStatus(UID, 532) 
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
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
	ShowMap(UID, 415);
end

if (EVENT == 1407) then
AGIRLIKKONTROL = CheckWeight(UID,379243000,1)
SLOTKONTROL = CheckGiveSlot(UID, 1)
     if SLOTKONTROL == false then
	 elseif AGIRLIKKONTROL == false then	
     else
			RunQuestExchange(UID,3019);
			SaveEvent(UID, 11225);
			SaveEvent(UID, 11242);	 
end
end