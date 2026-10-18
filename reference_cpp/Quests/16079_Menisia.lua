-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 16079;

if (EVENT == 190) then
	QuestStatusCheck = GetQuestStatus(UID, 697)	
	ITEM1_COUNT = HowmuchItem(UID, 900216000);  
	if(QuestStatusCheck == 1 and ITEM1_COUNT < 1) then
		EVENT = 2000
	else	
	NpcMsg(UID, 193,NPC)
end
end

local savenum = 61;

if (EVENT == 105) then -- 2 Level Silk Bundle
	SaveEvent(UID, 6);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, savenum, 105, NPC, 28, 107);
	else
		SelectMsg(UID, 2, savenum, 111, NPC, 28, 107);
	end
end

if (EVENT == 110) then
	SelectMsg(UID, 2, savenum, 150, NPC, 29, 111);
end

if (EVENT == 111) then
	SelectMsg(UID, 4, savenum, 156, NPC, 22, 112, 23, -1);
end

if (EVENT == 112) then
	SaveEvent(UID, 7);
end

if (EVENT == 120) then
	SaveEvent(UID, 9);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, savenum, 131, NPC, 14, -1);
	else
		SelectMsg(UID, 2, savenum, 132, NPC, 14, -1);
	end
end

if (EVENT == 280) then 
	ItemA = HowmuchItem(UID, 810418000);  
	if (ItemA < 2) then 
		SelectMsg(UID, 2, savenum, 157, NPC, 18, 282);
	else
		SelectMsg(UID, 4, savenum, 158, NPC, 4006, 281, 27, -1);
	end
end

if (EVENT == 282) then
	ShowMap(UID, 1);
end

if (EVENT == 281) then
	QuestStatusCheck = GetQuestStatus(UID, 61) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	RunQuestExchange(UID,1);
	SaveEvent(UID, 8)
end
end

if (EVENT == 195) then -- 4 Level Teeth of Bandicoot
	SelectMsg(UID, 2, 63, 195, NPC, 28, 196);
end

if (EVENT == 196) then
	ShowMap(UID, 5);
	SaveEvent(UID, 56);
end

if (EVENT == 197) then
	SelectMsg(UID, 2, 63, 197, NPC, 29, 196);
end


if (EVENT == 200) then
	SelectMsg(UID, 2, 63, 200, NPC, 29, 201);
end

if (EVENT == 201) then
	SelectMsg(UID, 4, 63, 201, NPC, 22, 202, 23, -1);
end

if (EVENT == 202) then
	SaveEvent(UID, 57);
end

if (EVENT == 205) then
	SaveEvent(UID, 59);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 63, 206, NPC, 32, 189);
	else
		SelectMsg(UID, 2, 63, 207, NPC, 4080, 189);
	end
end

if (EVENT == 189) then
	ShowMap(UID, 5);
end

if (EVENT == 210) then
	ITEM_COUNT = HowmuchItem(UID, 810418000);
	if (ITEM_COUNT < 2) then
		SelectMsg(UID, 2, 63, 211, NPC, 18, 213);
	else
		SelectMsg(UID, 4, 63, 212, NPC, 4006, 214, 27, -1);
	end
end

if (EVENT == 213) then
	ShowMap(UID, 7);
end

if (EVENT == 214) then
	QuestStatusCheck = GetQuestStatus(UID, 63) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
		else
	RunQuestExchange(UID,6)
	SaveEvent(UID, 58)	 
end
end

if (EVENT == 300) then -- 6 Level Kekoon Gallbladder
	SelectMsg(UID, 2, 66, 292, NPC, 28, 301);
end

if (EVENT == 301) then
	ShowMap(UID, 5);
	SaveEvent(UID, 93);
end

if (EVENT == 302) then
	SelectMsg(UID, 2, 66, 293, NPC, 29, 301);
end

if (EVENT == 303) then
	SelectMsg(UID, 2, 66, 294, NPC, 29, 304);
end

if (EVENT == 304) then
	SelectMsg(UID, 4, 66, 295, NPC, 22, 305, 23, -1);
end

if (EVENT == 305) then
	SaveEvent(UID, 94);
end

if (EVENT == 306) then
	SaveEvent(UID, 96);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 66, 299, NPC, 32, 307);
	else
		SelectMsg(UID, 2, 66, 300, NPC, 4080, 307);
	end
end

if (EVENT == 307) then
	ShowMap(UID, 5);
end

if (EVENT == 308) then
	ITEM_COUNT = HowmuchItem(UID, 810418000);
	if (ITEM_COUNT < 2) then
		SelectMsg(UID, 2, 66, 298, NPC, 18, 309);
	else
		SelectMsg(UID, 5, 66, 301, NPC, 4006, 310, 27, -1);
	end
end

if (EVENT == 309) then
   ShowMap(UID, 11);
end

if (EVENT == 310) then
	QuestStatusCheck = GetQuestStatus(UID, 66)
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then	
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
    RunQuestExchange(UID,28,STEP,1);
	SaveEvent(UID, 95);
end
end

if (EVENT == 311) then -- 12 Level Gabolt Scales
	SelectMsg(UID, 2, 82, 302, NPC, 28, 312);
end

if (EVENT == 312) then
	ShowMap(UID, 5);
	SaveEvent(UID, 100);
end

if (EVENT == 313) then
	SelectMsg(UID, 2, 82, 303, NPC, 29, 312);
end

if (EVENT == 314) then
	SelectMsg(UID, 2, 82, 304, NPC, 29, 315);
end   

if (EVENT == 315) then
	SelectMsg(UID, 4, 82, 305, NPC, 22, 316, 23, -1);
end   

if (EVENT == 316) then
	SaveEvent(UID, 101);
end

if (EVENT == 317) then
	SaveEvent(UID, 103);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 82,311, NPC, 32, 318);
	else
		SelectMsg(UID, 2, 82, 312, NPC, 4080, 318);
	end
end

if (EVENT == 318) then
	ShowMap(UID, 5);
end

if (EVENT == 319) then
	ITEM_COUNT = HowmuchItem(UID, 810418000);
	if (ITEM_COUNT < 2) then
		SelectMsg(UID, 2, 82, 310, NPC, 18, 320);
	else
		SelectMsg(UID, 5, 82, 314, NPC, 4006, 321, 27, -1);
	end
end

if (EVENT == 320) then
	ShowMap(UID, 12);
end

if (EVENT == 321) then
	QuestStatusCheck = GetQuestStatus(UID, 82) 
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
          RunQuestExchange(UID,29,STEP,1);
		  SaveEvent(UID, 102);   
end
end

if (EVENT == 350) then -- 17 Level
	SelectMsg(UID, 2, 91, 316, NPC, 28, 351);
end   

if (EVENT == 351) then
	ShowMap(UID, 5);
	SaveEvent(UID, 740);
end

if (EVENT == 352) then
	SelectMsg(UID, 2, 91, 318, NPC, 29, 353);
end   

if (EVENT == 353) then
	SelectMsg(UID, 4, 91, 319, NPC, 22, 354, 23, -1);
end   

if (EVENT == 354) then
	SaveEvent(UID, 741);
end

if (EVENT == 357) then
	SaveEvent(UID, 743);
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, 91, 320, NPC, 32, 359);
	else
		SelectMsg(UID, 2, 91, 321, NPC, 4080, 359);
	end
end

if (EVENT == 359) then
	ShowMap(UID, 5);
end

if (EVENT == 358) then
	ITEM_COUNT = HowmuchItem(UID, 810418000);
	if (ITEM_COUNT < 5) then
		SelectMsg(UID, 2, 91, 320, NPC, 4080, 355);
	else
		SelectMsg(UID, 4, 91, 321, NPC, 4080, 360, 27, -1);
	end
end

if (EVENT == 355) then
	ShowMap(UID, 629);
end

if (EVENT == 360) then
	QuestStatusCheck = GetQuestStatus(UID, 91) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
      RunQuestExchange(UID,131)
	  SaveEvent(UID, 742);
end
end

local savenum = 510;

if (EVENT == 1000) then -- 20 Level Voucher of Chaos 1
	SelectMsg(UID, 2, savenum, 9350, NPC, 28, 1001);
end

if (EVENT == 1001) then
	SaveEvent(UID, 1861);
end

if (EVENT == 1002) then
	SelectMsg(UID, 2, savenum, 9350, NPC, 29, 1003);
end

if (EVENT == 1003) then
	SelectMsg(UID, 4, savenum, 9350, NPC, 22, 1004, 23, -1);
end

if (EVENT == 1004) then
	SaveEvent(UID, 1862);
end

if (EVENT == 1006) then
	SaveEvent(UID, 1864);
	SelectMsg(UID, 2, savenum, 9350, NPC, 10, -1);
end

if (EVENT == 1005) then
	ITEM_COUNT = HowmuchItem(UID, 900106000);
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, savenum, 9350, NPC, 18, 1007);
	else
		SelectMsg(UID, 4, savenum, 9350, NPC, 41, 1008, 27, -1);
	end
end

if (EVENT == 1007) then
	ShowMap(UID, 338);
end

if (EVENT == 1008) then
	QuestStatusCheck = GetQuestStatus(UID, 510) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
    RunQuestExchange(UID,2443);
	SaveEvent(UID, 1863);
end
end

local savenum = 511;

if (EVENT == 1100) then -- 40 Level Voucher of Chaos 2
	SelectMsg(UID, 2, savenum, 9351, NPC, 28, 1101);
end

if (EVENT == 1101) then
	SaveEvent(UID, 1867);
end

if (EVENT == 1102) then
	SelectMsg(UID, 2, savenum, 9351, NPC, 29, 1103);
end

if (EVENT == 1103) then
	SelectMsg(UID, 4, savenum, 9351, NPC, 22, 1104, 23, -1);
end

if (EVENT == 1104) then
	SaveEvent(UID, 1868);
end

if (EVENT == 1106) then
	SaveEvent(UID, 1870);
	SelectMsg(UID, 2, savenum, 9351, NPC, 10, -1);
end

if (EVENT == 1105) then
	ITEM_COUNT = HowmuchItem(UID, 900106000);
	if (ITEM_COUNT < 2) then
		SelectMsg(UID, 2, savenum, 9351, NPC, 18, 1107);
	else
		SelectMsg(UID, 4, savenum, 9351, NPC, 41, 1108, 27, -1);
	end
end

if (EVENT == 1107) then
	ShowMap(UID, 338);
end

if (EVENT == 1108) then
	QuestStatusCheck = GetQuestStatus(UID, 511) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
    RunQuestExchange(UID,2444);
	SaveEvent(UID, 1869);
end
end

local savenum = 512;

if (EVENT == 1200) then -- 60 Level Voucher of Chaos 3
	SelectMsg(UID, 2, savenum, 9352, NPC, 28, 1201);
end

if (EVENT == 1201) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1873);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1878);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1883);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1888);
	end
end

if (EVENT == 1202) then
	SelectMsg(UID, 2, savenum, 9352, NPC, 29, 1203);
end

if (EVENT == 1203) then
	SelectMsg(UID, 4, savenum, 9352, NPC, 22, 1204, 23, -1);
end

if (EVENT == 1204) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1874);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1879);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1884);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1889);
	end
end

if (EVENT == 1206) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1876);
		SelectMsg(UID, 2, savenum, 9352, NPC, 10, -1);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1881);
		SelectMsg(UID, 2, savenum, 9352, NPC, 10, -1);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1886);
		SelectMsg(UID, 2, savenum, 9352, NPC, 10, -1);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1891);
		SelectMsg(UID, 2, savenum, 9352, NPC, 10, -1);
	end
end

if (EVENT == 1205) then
	ITEM_COUNT = HowmuchItem(UID, 900106000);
	if (ITEM_COUNT < 10) then
		SelectMsg(UID, 2, savenum, 9352, NPC, 18, 1207);
	else
		SelectMsg(UID, 5, savenum, 9352, NPC, 41, 1208,27,-1);
	end
end

if (EVENT == 1207) then
	ShowMap(UID, 338);
end


if (EVENT == 1208) then
	QuestStatusCheck = GetQuestStatus(UID, 512) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	Class = CheckClass(UID);
    if Class == 1 or Class == 5 or Class == 6 then    
        RunQuestExchange(UID, 2445,STEP,1);
        SaveEvent(UID, 1875);
    elseif Class == 2 or Class == 7 or Class == 8 then   
        RunQuestExchange(UID, 2446,STEP,1);
        SaveEvent(UID, 1880);
    elseif Class == 3 or Class == 9 or Class == 10 then      
        RunQuestExchange(UID, 2447,STEP,1);
        SaveEvent(UID, 1885);
    elseif Class == 4 or Class == 11 or Class == 12 then     
        RunQuestExchange(UID, 2448,STEP,1);
		SaveEvent(UID, 1890);
	  end
	end
	end

local savenum = 513;

if (EVENT == 1300) then -- 70 Level Voucher of Chaos 4
	SelectMsg(UID, 2, savenum, 9353, NPC, 28, 1301);
end

if (EVENT == 1301) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1894);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1899);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1904);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1909);
	end
end

if (EVENT == 1302) then
	SelectMsg(UID, 2, savenum, 9353, NPC, 29, 1303);
end

if (EVENT == 1303) then
	SelectMsg(UID, 4, savenum, 9353, NPC, 22, 1304, 23, -1);
end

if (EVENT == 1304) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1895);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1900);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1905);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1910);
	end
end

if (EVENT == 1306) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1897);
		SelectMsg(UID, 2, savenum, 9353, NPC, 10, -1);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1902);
		SelectMsg(UID, 2, savenum, 9353, NPC, 10, -1);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1907);
		SelectMsg(UID, 2, savenum, 9353, NPC, 10, -1);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1912);
		SelectMsg(UID, 2, savenum, 9353, NPC, 10, -1);
	end
end

if (EVENT == 1305) then
	ITEM_COUNT = HowmuchItem(UID, 900106000);
	if (ITEM_COUNT < 20) then
		SelectMsg(UID, 2, savenum, 9353, NPC, 18, 1307);
	else
		SelectMsg(UID, 5, savenum, 9353, NPC, 41, 1308, 27, -1);
	end
end

if (EVENT == 1307) then
	ShowMap(UID, 338);
end

if (EVENT == 1308) then
	QuestStatusCheck = GetQuestStatus(UID, 513)
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then	
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
			RunQuestExchange(UID, 2449,STEP,1);
			SaveEvent(UID, 1896);
	elseif (Class == 2 or Class == 7 or Class == 8) then
			RunQuestExchange(UID, 2450,STEP,1);
			SaveEvent(UID, 1901);
	elseif (Class == 3 or Class == 9 or Class == 10) then
			RunQuestExchange(UID, 2451,STEP,1);
			SaveEvent(UID, 1906);
	elseif (Class == 4 or Class == 11 or Class == 12) then
			RunQuestExchange(UID, 2452,STEP,1);
			SaveEvent(UID, 1911);
	end
end
end


if (EVENT == 1401)then
	SelectMsg(UID, 4, 619, 21228, NPC, 22, 1402,23,-1);
end

if (EVENT == 1402) then
    SaveEvent(UID, 12287);
end

if (EVENT == 1406) then
    SaveEvent(UID, 12289);
end

if (EVENT == 1405) then
	ITEM1_COUNT = HowmuchItem(UID, 389770000);   
	if (ITEM1_COUNT < 5) then
		SelectMsg(UID, 2, 619, 21228, NPC, 18,1404);
	else
		SelectMsg(UID, 4, 619, 21228, NPC, 22, 1407, 27, -1); 
	end
end


if (EVENT == 1407)then
	QuestStatusCheck = GetQuestStatus(UID, 619) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	RunQuestExchange(UID,3108);
	SaveEvent(UID,12288);
end
end


if (EVENT == 1501)then
	SelectMsg(UID, 4, 620, 21229, NPC, 22, 1502,23,-1);
end


if (EVENT == 1502) then
    SaveEvent(UID, 12294);
end

if (EVENT == 1506) then
    SaveEvent(UID, 12296);
end


if (EVENT == 1505) then
	ITEM1_COUNT = HowmuchItem(UID, 389770000);   
	if (ITEM1_COUNT < 10) then
		SelectMsg(UID, 2, 620, 21229, NPC, 18,1504);
	else
		SelectMsg(UID, 4, 620, 21229, NPC, 22, 1507, 27, -1); 
	end
end

if (EVENT == 1507)then
	QuestStatusCheck = GetQuestStatus(UID, 620) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	RunQuestExchange(UID,3109);
	SaveEvent(UID,12295)
end
end

if (EVENT == 501)then
SlotCheck = CheckGiveSlot(UID, 3)
     if SlotCheck == false then
       
         else
SelectMsg(UID, 2, 1205, 43663, NPC, 8144,-1);
GiveItem(UID, 900601000);
SaveEvent(UID, 7341);
end
end

if (EVENT == 2000)then
SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
         else
SelectMsg(UID, 2, -1, 19999, NPC, 22,-1);
GiveItem(UID, 900216000,1);
end
end