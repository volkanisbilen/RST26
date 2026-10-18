-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14435;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4937, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 4937, NPC)
	else
		EVENT = QuestNum
	end
end

local savenum = 127;

if (EVENT == 232) then -- 35 Level
	SelectMsg(UID, 4, savenum, 4931, NPC, 22, 233, 23, -1);
end

if (EVENT == 233) then
	SaveEvent(UID, 10013);
end

if (EVENT == 235) then
	SaveEvent(UID, 10015);
end

if (EVENT == 236) then
	ANIMAL = HowmuchItem(UID, 379273000);
	if (ANIMAL < 3) then
		SelectMsg(UID, 2, savenum, 4991, NPC, 19, 238);
	else
		SelectMsg(UID, 4, savenum, 4998, NPC, 22, 237, 23, -1);
	end
end

if (EVENT == 238) then
	ShowMap(UID, 96);
end

if (EVENT == 237) then
	ANIMAL = HowmuchItem(UID, 379273000);
	if (ANIMAL < 3) then
		SelectMsg(UID, 2, savenum, 4991, NPC, 19, 238);
	else
RunQuestExchange(UID,541)
	SaveEvent(UID, 10014);
end
end

local savenum1 = 130;

if (EVENT == 242) then -- 36 Level
	SelectMsg(UID, 4, savenum1, 575, NPC, 22, 243, 23, -1);
end

if (EVENT == 243) then
	SaveEvent(UID, 10025);
end

if (EVENT == 245) then
	SaveEvent(UID, 10027);
end

if (EVENT == 246) then
	ROTTEN = HowmuchItem(UID, 379274000);
	if (ROTTEN < 3) then
		SelectMsg(UID, 2, savenum1, 788, NPC, 19, 248);
	else
		SelectMsg(UID, 4, savenum1, 6106, NPC, 22, 247, 23, -1);
	end
end

if (EVENT == 248) then
	ShowMap(UID, 98);
end

if (EVENT == 247) then
	QuestStatusCheck = GetQuestStatus(UID, 130) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ROTTEN = HowmuchItem(UID, 379274000);
	if (ROTTEN < 3) then
		SelectMsg(UID, 2, savenum1, 788, NPC, 19, 248);
	else
RunQuestExchange(UID,542);
	SaveEvent(UID, 10026);
end
end
end

local savenum2 = 133;

if (EVENT == 252) then -- 37 Level
	SelectMsg(UID, 4, savenum2, 576, NPC, 22, 253, 23, -1);
end

if (EVENT == 253) then
	SaveEvent(UID, 9867);
end

if (EVENT == 255) then
	SaveEvent(UID, 9869);
end

if (EVENT == 256) then
	FEATHER = HowmuchItem(UID, 379272000);
	if (FEATHER < 3) then
		SelectMsg(UID, 2, savenum2, 6187, NPC, 19, 258);
	else
		SelectMsg(UID, 4, savenum2, 6196, NPC, 22, 257, 23, -1);
	end
end

if (EVENT == 258) then
	ShowMap(UID, 100);
end

if (EVENT == 257) then
	QuestStatusCheck = GetQuestStatus(UID, 133) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	FEATHER = HowmuchItem(UID, 379272000);
	if (FEATHER < 3) then
		SelectMsg(UID, 2, savenum2, 6187, NPC, 19, 258);
	else
RunQuestExchange(UID,543)
	SaveEvent(UID, 9868);
end
end
end

local savenum3 = 136;

if (EVENT == 262) then -- 38 Level
	SelectMsg(UID, 4, savenum3, 586, NPC, 22, 263, 23, -1);
end

if (EVENT == 263) then
	SaveEvent(UID, 9879);
end

if (EVENT == 265) then
	SaveEvent(UID, 9881);
end

if (EVENT == 266) then
	SKULL = HowmuchItem(UID, 810418000);
	if (SKULL < 3) then
		SelectMsg(UID, 2, savenum3, 6127, NPC, 19, 268);
	else
		SelectMsg(UID, 4, savenum3, 6128, NPC, 22, 267, 23, -1);
	end
end

if (EVENT == 268) then
	ShowMap(UID, 102);
end

if (EVENT == 267) then
	QuestStatusCheck = GetQuestStatus(UID, 136) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	SKULL = HowmuchItem(UID, 810418000);
	if (SKULL < 3) then
		SelectMsg(UID, 2, savenum3, 6127, NPC, 19, 268);
	else
RunQuestExchange(UID,544)
	SaveEvent(UID, 9880);
end
end
end

local savenum4 = 139;

if (EVENT == 272) then -- 39 Level
	SelectMsg(UID, 4, savenum4, 6139, NPC, 22, 273, 23, -1);
end

if (EVENT == 273) then
	SaveEvent(UID, 9891);
end

if (EVENT == 275) then
	SaveEvent(UID, 9893);
end

if (EVENT == 276) then
	COARSE = HowmuchItem(UID, 379275000);
	if (COARSE < 3) then
		SelectMsg(UID, 2, savenum4, 6141, NPC, 19, 278);
	else
		SelectMsg(UID, 4, savenum4, 6142, NPC, 22, 277, 23, -1);
	end
end

if (EVENT == 278) then
	ShowMap(UID, 104);
end

if (EVENT == 277) then
	QuestStatusCheck = GetQuestStatus(UID, 139) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	COARSE = HowmuchItem(UID, 379275000);
	if (COARSE < 3) then
		SelectMsg(UID, 2, savenum4, 6141, NPC, 19, 278);
	else
RunQuestExchange(UID,545)
	SaveEvent(UID, 9892);
end
end
end

local savenum5 = 142;

if (EVENT == 282) then -- 40 Level
	SelectMsg(UID, 4, savenum5, 590, NPC, 22, 283, 23, -1);
end

if (EVENT == 283) then
	SaveEvent(UID, 9903);
end

if (EVENT == 285) then
	SaveEvent(UID, 9905);
end

if (EVENT == 286) then
	APPLE = HowmuchItem(UID, 810418000);
	if (APPLE < 10 ) then
		SelectMsg(UID, 2, savenum5, 6141, NPC, 19, 288);
	else
		SelectMsg(UID, 4, savenum5, 6142, NPC, 22, 287, 23, -1);
	end
end

if (EVENT == 288) then
	ShowMap(UID, 106);
end

if (EVENT == 287) then
	QuestStatusCheck = GetQuestStatus(UID, 142) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	APPLE = HowmuchItem(UID, 810418000);
	if (APPLE < 10 ) then
		SelectMsg(UID, 2, savenum5, 6141, NPC, 19, 288);
	else
RunQuestExchange(UID,546)
	SaveEvent(UID, 9904);
end
end
end

local savenum6 = 145;

if (EVENT == 292) then -- 41 Level
	SelectMsg(UID, 4, savenum6, 6163, NPC, 22, 293, 23, -1);
end

if (EVENT == 293) then
	SaveEvent(UID, 9915);
end

if (EVENT == 295) then
	SaveEvent(UID, 9917);
end

if (EVENT == 296) then
	ORK = HowmuchItem(UID, 379277000);
	if (ORK < 7) then
		SelectMsg(UID, 2, savenum6, 6163, NPC, 19, 298);
	else
		SelectMsg(UID, 4, savenum6, 6142, NPC, 22, 297, 23, -1);
	end
end

if (EVENT == 298) then
	ShowMap(UID, 108);
end

if (EVENT == 297) then
	QuestStatusCheck = GetQuestStatus(UID, 145) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ORK = HowmuchItem(UID, 379277000);
	if (ORK < 7) then
		SelectMsg(UID, 2, savenum6, 6163, NPC, 19, 298);
	else
RunQuestExchange(UID,547)
	SaveEvent(UID, 9916);
end
end
end

local savenum7 = 148;

if (EVENT == 302) then -- 42 Level
	SelectMsg(UID, 4, savenum7, 4932, NPC, 22, 303, 23, -1);
end

if (EVENT == 303) then
	SaveEvent(UID, 9927);
end

if (EVENT == 305) then
	SaveEvent(UID, 9929);
end

if (EVENT == 306) then
	OATH = HowmuchItem(UID, 379276000);
	if (OATH < 3) then
		SelectMsg(UID, 2, savenum7, 4932, NPC, 19, 308);
	else
		SelectMsg(UID, 4, savenum7, 4934, NPC, 22, 307, 23, -1);
	end
end

if (EVENT == 308) then
	ShowMap(UID, 110);
end

if (EVENT == 307) then
	QuestStatusCheck = GetQuestStatus(UID, 148) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	OATH = HowmuchItem(UID, 379276000);
	if (OATH < 3) then
		SelectMsg(UID, 2, savenum7, 4932, NPC, 19, 308);
	else
RunQuestExchange(UID,548)
	SaveEvent(UID, 9928);
end
end
end

local savenum8 = 151;

if (EVENT == 312) then -- 43 Level
	SelectMsg(UID, 4, savenum8, 576, NPC, 22, 313, 23, -1);
end

if (EVENT == 313) then
	SaveEvent(UID, 9939);
end

if (EVENT == 315) then
	SaveEvent(UID, 9941);
end

if (EVENT == 316) then
	FEATHER = HowmuchItem(UID, 379272000);
	if (FEATHER < 7) then
		SelectMsg(UID, 2, savenum8, 6187, NPC, 19, 318);
	else
		SelectMsg(UID, 4, savenum8, 6196, NPC, 22, 317, 23, -1);
	end
end

if (EVENT == 318) then
	ShowMap(UID, 112);
end

if (EVENT == 317) then
	QuestStatusCheck = GetQuestStatus(UID, 151) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	FEATHER = HowmuchItem(UID, 379272000);
	if (FEATHER < 7) then
		SelectMsg(UID, 2, savenum8, 6187, NPC, 19, 318);
	else
RunQuestExchange(UID,549)
	SaveEvent(UID, 9940);
end
end
end

local savenum=1373
local talknum=44200
local exchangeid=6161

if (EVENT == 500) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 501, 23, 193);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 193);
	end
end
	
if (EVENT == 501) then
	SaveEvent(UID, 3964);
end

if (EVENT == 506) then
	ITEMA  = HowmuchItem(UID, 810494000);
	if(ITEMA > 4) then
		SaveEvent(UID, 3966)
	end	
end

if (EVENT == 503) then
	ITEMA  = HowmuchItem(UID, 810494000);
	if(ITEMA > 4) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, 504, 27, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 505);
	end
end

if (EVENT == 504)then
	QuestStatusCheck = GetQuestStatus(UID, 1373) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA  = HowmuchItem(UID, 810494000);
	if(ITEMA > 4) then
	RunQuestExchange(UID, exchangeid)
	SaveEvent(UID,3965)
		else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 505);
	end
end
end

if (EVENT == 505) then
ShowMap(UID, 586);
end

local savenum=1374
local talknum=44204
local exchangeid=6162

if (EVENT == 510) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 511, 23, 193);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 193);
	end
end
	
if (EVENT == 511) then
	SaveEvent(UID, 3974);
end

if (EVENT == 516) then
	ITEMA  = HowmuchItem(UID, 810495000);
	if(ITEMA > 19) then
		SaveEvent(UID, 3976)
	end	
end

if (EVENT == 513) then
	ITEMA  = HowmuchItem(UID, 810495000);
	if(ITEMA > 19) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, 514, 27, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 515);
	end
end

if (EVENT == 515) then
ShowMap(UID, 550);
end


if (EVENT == 514)then
	QuestStatusCheck = GetQuestStatus(UID, 1374) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA  = HowmuchItem(UID, 810495000);
	if(ITEMA > 19) then
	RunQuestExchange(UID, exchangeid)
	SaveEvent(UID,3975)
		else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 515);
	end
end
end

local savenum=1375
local talknum=44208
local exchangeid=6163

if (EVENT == 520) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 521, 23, 193);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 193);
	end
end
	
if (EVENT == 521) then
	SaveEvent(UID, 3984);
end

if (EVENT == 526) then
	ITEMA  = HowmuchItem(UID, 810496000);
	if(ITEMA > 14) then
		SaveEvent(UID, 3986)
	end	
end

if (EVENT == 523) then
	ITEMA  = HowmuchItem(UID, 810496000);
	if(ITEMA > 14) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, 524, 27, 193);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 525);
	end
end

if (EVENT == 524)then
	QuestStatusCheck = GetQuestStatus(UID, 1375) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA  = HowmuchItem(UID, 810496000);
	if(ITEMA > 14) then
	RunQuestExchange(UID, exchangeid)
	SaveEvent(UID,3985)
		else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 525);
	end
end
end

if (EVENT == 525) then
	ShowMap(UID, 36);
end

local savenum=1376
local talknum=44212
local exchangeid=6164

if (EVENT == 530) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 531, 23, 193);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 193);
	end
end
	
if (EVENT == 531) then
	SaveEvent(UID, 3994);
end

if (EVENT == 536) then
	SaveEvent(UID, 3996)
end

if (EVENT == 533) then
	ITEMA = HowmuchItem(UID, 810497000);
	if (ITEMA > 2) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, 534, 27, 193);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
end

if (EVENT == 534)then
	QuestStatusCheck = GetQuestStatus(UID, 1376) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 810497000);
	if (ITEMA > 2) then
	RunQuestExchange(UID, exchangeid)
	SaveEvent(UID,3995)
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
end
end

if (EVENT == 1002)then
	SelectMsg(UID, 2, 516, 20122, NPC, 4161, 1003);
end

if (EVENT == 1003)then
	SelectMsg(UID, 2, 516, 20123, NPC, 4552, 1004,6004,-1);
	SaveEvent(UID,11008)
end

if (EVENT == 1004)then
	SelectMsg(UID, 4, 516, 20124, NPC, 4161, 1005,3005,-1);
	SaveEvent(UID,11010)
end

if (EVENT == 1005) then 
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	    SaveEvent(UID,11009);
		SaveEvent(UID, 11014);
		SelectMsg(UID, 2, 516, 20207, NPC,6002, -1);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID,11009);
		SaveEvent(UID, 11019);
		SelectMsg(UID, 2, 516, 20207, NPC,6002, -1);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID,11009);
		SaveEvent(UID, 11024);
		SelectMsg(UID, 2, 516, 20207, NPC,6002, -1);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID,11009);
		SaveEvent(UID, 11029);
		SelectMsg(UID, 2, 516, 20207, NPC,6002, -1);
	end
end

if (EVENT == 1101)then
	SelectMsg(UID, 4, 520, 20009, NPC, 22, 1102,23,-1);
end

if (EVENT == 1102)then
	SaveEvent(UID,11086)
end

if (EVENT == 1106)then
	SaveEvent(UID,11088)
end

if (EVENT == 1103) then
	ITEM_COUNT = HowmuchItem(UID, 910210000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 520, 20009, NPC, 18, -1);
	else
		SelectMsg(UID, 4, 520, 20009, NPC, 22, 1105, 27, -1); 
	end
end

if (EVENT == 1105)then
	QuestStatusCheck = GetQuestStatus(UID, 520) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910210000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 520, 20009, NPC, 18, -1);
	else
RunQuestExchange(UID,3007);
	SaveEvent(UID,11087);
	SaveEvent(UID,11098);
end
end
end

if (EVENT == 1201)then
	SelectMsg(UID, 4, 521, 20011, NPC, 22, 1202,23,-1);
end

if (EVENT == 1202)then
	SaveEvent(UID,11098)
end

if (EVENT == 1206)then
	SaveEvent(UID,11100)
end

if (EVENT == 1203) then
		SelectMsg(UID, 4, 521, 20011, NPC, 22, 1205, 27, -1); 
end

if (EVENT == 1205)then
	QuestStatusCheck = GetQuestStatus(UID, 521) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
RunQuestExchange(UID,3008)
	SaveEvent(UID,11099)
	SaveEvent(UID,11110)
end
end
