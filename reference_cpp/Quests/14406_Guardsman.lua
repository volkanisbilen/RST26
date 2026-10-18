-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14406;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 559, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 560, NPC)
	else
		EVENT = QuestNum
	end
end

local savenum = 428;

if (EVENT == 1000) then -- 47 Level Hornet Premium
	SaveEvent(UID, 2163);
end

if (EVENT == 1002) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8158, NPC, 22, 1003, 23, -1);
	else
		SelectMsg(UID, 2, savenum, 8158, NPC, 10, -1);
	end
end

if (EVENT == 1003) then
	SaveEvent(UID, 2164);
end

if (EVENT == 1004) then
	SaveEvent(UID, 2167);
end

if (EVENT == 1005) then
	SelectMsg(UID, 2, savenum, 8418, NPC, 3007, -1);
	SaveEvent(UID, 2166);
end

if (EVENT == 1007) then
	MonsterCount = CountMonsterQuestSub(UID, 428, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 8419, NPC, 18, 1008);
	else
		SelectMsg(UID, 4, savenum, 8248, NPC, 41, 1009, 27, -1);
	end
end

if (EVENT == 1008) then
	ShowMap(UID, 27);
end

if (EVENT == 1009) then
	QuestStatusCheck = GetQuestStatus(UID, 428) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 428, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 8419, NPC, 18, 1008);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1218)
		SaveEvent(UID, 2165);
	else
		RunQuestExchange(UID,1218)
		SaveEvent(UID, 2165);
	end
end
end
end

if (EVENT == 8050) then -- 47 Level Hornet
	SelectMsg(UID, 2, 204, 8245, NPC, 3003, 8051);
end

if (EVENT == 8051) then
	SaveEvent(UID, 8979);
end

if (EVENT == 8052) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 204, 8246, NPC, 10, 8060);
	else
		SelectMsg(UID, 2, 204, 8254, NPC, 10, -1);
	end
end

if (EVENT == 8060) then
	SelectMsg(UID, 4, 204, 8247, NPC, 22, 8053, 23, -1);
end

if (EVENT == 8053) then
	SaveEvent(UID, 8980);
end

if (EVENT == 8054) then
	SaveEvent(UID, 8983);
end

if (EVENT == 8055) then
	SelectMsg(UID, 2, 204, 8418, NPC, 3007, -1);
	SaveEvent(UID, 8982);
end

if (EVENT == 8057) then
	MonsterCount = CountMonsterQuestSub(UID, 204, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 204, 8419, NPC, 18, 8058);
	else
		SelectMsg(UID, 4, 204, 8248, NPC, 41, 8059, 27, -1);
	end
end

if (EVENT == 8058) then
	ShowMap(UID, 27);
end

if (EVENT == 8059) then
	QuestStatusCheck = GetQuestStatus(UID, 204) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 204, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 204, 8419, NPC, 18, 8058);
	else
RunQuestExchange(UID,952)
	SaveEvent(UID, 8981);
end
end
end

if (EVENT == 9510) then -- 48 Level Gray Oz
	SelectMsg(UID, 2, 211, 8235, NPC, 3003, 9511);
end

if (EVENT == 9511) then
	SaveEvent(UID, 9660);
end

if (EVENT == 9512) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 211, 8769, NPC, 10, 9515);
	else
		SelectMsg(UID, 2, 211, 8254, NPC, 10, -1);
	end
end

if (EVENT == 9515) then
	SelectMsg(UID, 4, 211, 8769, NPC, 22, 9513, 23, -1);
end

if (EVENT == 9513) then
Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then --Warrior
	SaveEvent(UID, 9661);
	elseif (Class == 2 or Class == 7 or Class == 8) then --Rogue
	SaveEvent(UID, 9666);
	elseif (Class == 3 or Class == 9 or Class == 10) then --Mage
	SaveEvent(UID, 9671);
	elseif (Class == 4 or Class == 11 or Class == 12) then --Priest
	SaveEvent(UID, 9676);
end
end

if (EVENT == 9520) then
Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then --Warrior
	SaveEvent(UID, 9663);
	SelectMsg(UID, 2, -1, 8418, NPC, 3007, -1);
	elseif (Class == 2 or Class == 7 or Class == 8) then --Rogue
	SaveEvent(UID, 9668);
	SelectMsg(UID, 2, -1, 8418, NPC, 3007, -1);
	elseif (Class == 3 or Class == 9 or Class == 10) then --Mage
	SaveEvent(UID, 9673);
	SelectMsg(UID, 2, -1, 8418, NPC, 3007, -1);
	elseif (Class == 4 or Class == 11 or Class == 12) then --Priest
	SaveEvent(UID, 9678);
	SelectMsg(UID, 2, -1, 8418, NPC, 3007, -1);
end
end


if (EVENT == 9516) then
	MonsterCount = CountMonsterQuestSub(UID, 211, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 211, 8419, NPC, 18, 9517);
	else
		SelectMsg(UID, 5, 211, 8238, NPC, 41, 9518, 27, -1);
	end
end

if (EVENT == 9517) then
	ShowMap(UID, 507);
end

if (EVENT == 9518) then
	QuestStatusCheck = GetQuestStatus(UID, 211) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 211, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 211, 8419, NPC, 18, 9517);
	else
Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then --Warrior
	RunQuestExchange(UID,1142,STEP,1); 
	SaveEvent(UID, 9662);
	elseif (Class == 2 or Class == 7 or Class == 8) then --Rogue
	RunQuestExchange(UID,1143,STEP,1);
	SaveEvent(UID, 9667);
	elseif (Class == 3 or Class == 9 or Class == 10) then --Mage
	RunQuestExchange(UID,1144,STEP,1);
	SaveEvent(UID, 9672);
	elseif (Class == 4 or Class == 11 or Class == 12) then --Priest
	RunQuestExchange(UID,1145,STEP,1);
	SaveEvent(UID, 9677);
end
end
end
end

if (EVENT == 200) then -- 52 Level Haunga Warrior Premium
	SaveEvent(UID, 2223);
end

if (EVENT == 202) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 462, 8166, NPC, 22, 203, 23, -1);
	else
		SelectMsg(UID, 2, 462, 8166, NPC, 10, -1);
	end
end

if (EVENT == 203) then
	SaveEvent(UID, 2224);
end

if (EVENT == 204) then
	SaveEvent(UID, 2227);
end

if (EVENT == 205) then
	SaveEvent(UID, 2226);
end

if (EVENT == 207) then
	MonsterCount = CountMonsterQuestSub(UID, 462, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 462, 8166, NPC, 18, 208);
	else
		SelectMsg(UID, 4, 462, 8166, NPC, 41, 209, 27, -1);
	end
end

if (EVENT == 208) then
	ShowMap(UID, 59);
end

if (EVENT == 209) then
	QuestStatusCheck = GetQuestStatus(UID, 462) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 462, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 462, 8166, NPC, 18, 208);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1941)
		SaveEvent(UID, 2225);
	else
		RunQuestExchange(UID,1941)
		SaveEvent(UID, 2225);
	end
end
end
end

if (EVENT == 8450) then -- 52 Level Haunga Warrior
	SelectMsg(UID, 2, 226, 8249, NPC, 3003, 8451);
end

if (EVENT == 8451) then
	SaveEvent(UID, 9039);
end

if (EVENT == 8452) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 226, 8250, NPC, 10, 8460);
	else
		SelectMsg(UID, 2, 226, 8254, NPC, 10, -1);
	end
end

if (EVENT == 8460) then
	SelectMsg(UID, 4, 226, 8251, NPC, 22, 8453, 23, -1);
end

if (EVENT == 8453) then
	SaveEvent(UID, 9040);
end

if (EVENT == 8454) then
	SaveEvent(UID, 9043);
end

if (EVENT == 8455) then
	SelectMsg(UID, 2, 226, 8418, NPC, 3014, -1);
	SaveEvent(UID, 9042);
end

if (EVENT == 8457) then
	MonsterCount = CountMonsterQuestSub(UID, 226, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 226, 8419, NPC, 18, 8458);
	else
		SelectMsg(UID, 5, 226, 8252, NPC, 41, 8459, 27, -1);
	end
end

if (EVENT == 8458) then
	ShowMap(UID, 59);
end

if (EVENT == 8459) then
	QuestStatusCheck = GetQuestStatus(UID, 226) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 226, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 226, 8419, NPC, 18, 8458);
	else
RunQuestExchange(UID,941,STEP,1) ;
 SaveEvent(UID, 9041);
end
end
end

if (EVENT == 300) then -- 57 Level Phantom Premium
	SaveEvent(UID, 2319);
end

if (EVENT == 302) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 477, 8166, NPC, 22, 303, 23, -1);
	else
		SelectMsg(UID, 2, 477, 8166, NPC, 10, -1);
	end
end

if (EVENT == 303) then
	SaveEvent(UID, 2320);
end

if (EVENT == 304) then
	SaveEvent(UID, 2323);
end

if (EVENT == 305) then
	SaveEvent(UID, 2322);
end

if (EVENT == 307) then
	MonsterCount = CountMonsterQuestSub(UID, 477, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 477, 8166, NPC, 18, 308);
	else
		SelectMsg(UID, 4, 477, 8166, NPC, 41, 309, 27, -1);
	end
end

if (EVENT == 308) then
	ShowMap(UID, 702);
end

if (EVENT == 309) then
	QuestStatusCheck = GetQuestStatus(UID, 477) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 477, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 477, 8166, NPC, 18, 308);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,11090)
		SaveEvent(UID, 2321);
	else
		RunQuestExchange(UID,11090)
		SaveEvent(UID, 2321);
	end
end
end
end

if (EVENT == 400) then -- 58 Level Groom Hound Premium
	SaveEvent(UID, 2343);
end

if (EVENT == 402) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 480, 8166, NPC, 22, 403, 23, -1);
	else
		SelectMsg(UID, 2, 480, 8166, NPC, 10, -1);
	end
end

if (EVENT == 403) then
	SaveEvent(UID, 2344);
end

if (EVENT == 404) then
	SaveEvent(UID, 2347);
end

if (EVENT == 405) then
	SaveEvent(UID, 2346);
end

if (EVENT == 407) then
	MonsterCount = CountMonsterQuestSub(UID, 480, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 480, 8166, NPC, 18, 408);
	else
		SelectMsg(UID, 4, 480, 8166, NPC, 41, 409, 27, -1);
	end
end

if (EVENT == 408) then
	ShowMap(UID, 601);
end

if (EVENT == 409) then
	QuestStatusCheck = GetQuestStatus(UID, 480) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 480, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 480, 8166, NPC, 18, 408);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,11093)
		SaveEvent(UID, 2345);
	else
		RunQuestExchange(UID,11093)
		SaveEvent(UID, 2345);
	end
end
end
end

if (EVENT == 9330) then -- 58 Level Phantom
	SelectMsg(UID, 2, 447, 8245, NPC, 3003, 9331);
end

if (EVENT == 9331) then
	SaveEvent(UID, 9357);
end

if (EVENT == 9332) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 447, 8679, NPC, 10, 9340);
	else
		SelectMsg(UID, 2, 447, 8254, NPC, 10, -1);
	end
end

if (EVENT == 9340) then
	SelectMsg(UID, 4, 447, 8679, NPC, 22, 9333, 23, -1);
end

if (EVENT == 9333) then
	SaveEvent(UID, 9358);
end

if (EVENT == 9334) then
	SaveEvent(UID, 9361);
end

if (EVENT == 9335) then
	SaveEvent(UID, 9360);
	SelectMsg(UID, 2, 447, 8418, NPC, 3014, -1);
end

if (EVENT == 9337) then
	MonsterCount = CountMonsterQuestSub(UID, 447, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 447, 8576, NPC, 18, 9338);
	else
		SelectMsg(UID, 4, 447, 8582, NPC, 41, 9339, 27, -1);
	end
end

if (EVENT == 9338) then
	ShowMap(UID, 702);
end

if (EVENT == 9339) then
	QuestStatusCheck = GetQuestStatus(UID, 447) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 447, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 447, 8576, NPC, 18, 9338);
	else
RunQuestExchange(UID,1090)
	SaveEvent(UID, 9359);
end
end
end


if (EVENT == 9350) then -- 58 Level Groom Hound
	SelectMsg(UID, 2, 272, 8245, NPC, 3003, 9351);
end

if (EVENT == 9351) then
	SaveEvent(UID, 9381);
end

if (EVENT == 9352) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 272, 8683, NPC, 10, 9360);
	else
		SelectMsg(UID, 2, 272, 8254, NPC, 10, -1);
	end
end

if (EVENT == 9360) then
	SelectMsg(UID, 4, 272, 8247, NPC, 22, 9353, 23, -1);
end

if (EVENT == 9353) then
	SaveEvent(UID, 9382);
end

if (EVENT == 9354) then
	SaveEvent(UID, 9385);
end

if (EVENT == 9355) then
	SaveEvent(UID, 9384);
	SelectMsg(UID, 2, 272, 8418, NPC, 3014, -1);
end

if (EVENT == 9357) then
	MonsterCount = CountMonsterQuestSub(UID, 272, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 272, 8558, NPC, 18, 9358);
	else
		SelectMsg(UID, 4, 272, 8560, NPC, 41, 9359, 27, -1);
	end
end

if (EVENT == 9358) then
	ShowMap(UID, 601);
end

if (EVENT == 9359) then
	QuestStatusCheck = GetQuestStatus(UID, 272) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 272, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 272, 8558, NPC, 18, 9358);
	else
RunQuestExchange(UID,1093)
	SaveEvent(UID, 9383);
end
end
end