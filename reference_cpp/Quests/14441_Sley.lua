-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14441;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 3003, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 3200, NPC)
	else
		EVENT = QuestNum
	end
end

local savenum = 404;

if (EVENT == 1000) then -- 36 Level Rotten Eyes Premium
	SelectMsg(UID, 2, savenum, 841, NPC, 10, 1001);
end

if (EVENT == 1001) then
	SaveEvent(UID, 2019);
end

if (EVENT == 1002) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 841, NPC, 22, 1003, 23, 1004);
	else
		SelectMsg(UID, 2, savenum, 841, NPC, 10, -1);
	end
end

if (EVENT == 1003) then
	SaveEvent(UID, 2020);
end

if (EVENT == 1004) then
	SaveEvent(UID, 2023);
end

if (EVENT == 1010) then
	SaveEvent(UID, 2022);
end

if (EVENT == 1006) then
	MonsterCount = CountMonsterQuestSub(UID, 404, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 732, NPC, 18, 1007);
	else
		SelectMsg(UID, 4, savenum, 8467, NPC, 41, 1008, 23, -1);
	end
end

if (EVENT == 1007) then
	ShowMap(UID, 98);
end

if (EVENT == 1008) then
	QuestStatusCheck = GetQuestStatus(UID, 404) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 404, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 732, NPC, 18, 1007);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 1206)
		SaveEvent(UID, 2021);   
	else
		RunQuestExchange(UID, 1206)
		SaveEvent(UID, 2021);
	end
end
end
end

local savenum = 416;

if (EVENT == 1100) then
	SelectMsg(UID, 2, savenum, 841, NPC, 10, 1101);
end

if (EVENT == 1101) then
	SaveEvent(UID, 2091);
end

if (EVENT == 1102) then -- 42 Level Battalion Premium
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 841, NPC, 22, 1103, 23, 1104);
	else
		SelectMsg(UID, 2, savenum, 841, NPC, 10, -1);
	end
end

if (EVENT == 1103) then
	SaveEvent(UID, 2092);
end

if (EVENT == 1104) then
	SaveEvent(UID, 2095);
end

if (EVENT == 1110) then
	SaveEvent(UID, 2094);
end

if (EVENT == 1106) then
	MonsterCount = CountMonsterQuestSub(UID, 416, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 732, NPC, 18, 1107);
	else
		SelectMsg(UID, 4, savenum, 8467, NPC, 41, 1108, 23, -1);
	end
end

if (EVENT == 1107) then
	ShowMap(UID, 110);
end

if (EVENT == 1108) then
	QuestStatusCheck = GetQuestStatus(UID, 416) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 416, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 732, NPC, 18, 1107);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, 1212)
		SaveEvent(UID, 2093);   
	else
		RunQuestExchange(UID, 1212)
		SaveEvent(UID, 2093);
	end
end
end
end

local savenum=404
local talknum=797
local exchangeid=1206

if (EVENT == 1002) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 1003, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end
	
if (EVENT == 1003) then
	SaveEvent(UID, 2020);
end

if (EVENT == 1010) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 1) then
		SaveEvent(UID, 2022)
	end	
end

if (EVENT == 1006) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 1) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, 1005, 27, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
end

if (EVENT == 1005)then
	QuestStatusCheck = GetQuestStatus(UID, 404) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 1) then
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, exchangeid, 4);
	else
		RunQuestExchange(UID, exchangeid, 1)
	end
	SaveEvent(UID,2021)
		else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
end
end

local savenum=416
local talknum=841
local exchangeid=1212

if (EVENT == 1102) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 1103, 23, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, -1);
	end
end
	
if (EVENT == 1103) then
	SaveEvent(UID, 2092);
end

if (EVENT == 1110) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 1) then
		SaveEvent(UID, 2094)
	end	
end

if (EVENT == 1106) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 1) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, 1105, 27, -1);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
end

if (EVENT == 1105)then
	QuestStatusCheck = GetQuestStatus(UID, 416) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 1) then
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID, exchangeid, 4)
	else
		RunQuestExchange(UID, exchangeid, 1)
	end
	SaveEvent(UID,2093)
		else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
end
end


local savenum=129
local talknum=797
local exchangeid=907

if (EVENT == 8402) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 8403);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 153);
	end
end

if (EVENT == 8403) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 8404, 23, 8405);
end

if (EVENT == 8404) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8491);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8496);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8501);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8506);
	end
end

if (EVENT == 8405) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8494);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8499);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8504);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8509);
	end
end

if (EVENT == 8410) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8493);
		EVENT = 8411
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8498);
		EVENT = 8411
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8503);
		EVENT = 8411
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8508);
		EVENT = 8411
	end
end

if (EVENT == 8411) then
	SelectMsg(UID, 2, savenum, talknum, NPC, 3002, 153);
end

if (EVENT == 8406) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount > 14) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, 8408, 27, 153);
	else 
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, 8407);
	end
end

if (EVENT == 8407) then
	ShowMap(UID, 511);
end

if (EVENT == 8408) then
	QuestStatusCheck = GetQuestStatus(UID, 129) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount > 14) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		RunQuestExchange(UID, 904)
		SaveEvent(UID, 8492);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		RunQuestExchange(UID, 905)
		SaveEvent(UID, 8497);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		RunQuestExchange(UID, 906)
		SaveEvent(UID, 8502);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		RunQuestExchange(UID, 907)
		SaveEvent(UID, 8507);
			else 
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, 8407);
	end
end
end
end

local savenum=147
local talknum=841
local exchangeid=961

if (EVENT == 8852) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 8853);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 153);
	end
end

if (EVENT == 8853) then
	SelectMsg(UID, 4, savenum, talknum, NPC, 22, 8854, 23, 8855);
end

if (EVENT == 8854) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8743);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8748);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8753);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8758);
	end
end

if (EVENT == 8855) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8746);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8751);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8756);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8761);
	end
end

if (EVENT == 8860) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8745);
		EVENT = 8861
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8750);
		EVENT = 8861
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8755);
		EVENT = 8861
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8760);
		EVENT = 8861
	end
end

if (EVENT == 8861) then
	SelectMsg(UID, 2, savenum, talknum, NPC, 3002, 153);
end

if (EVENT == 8856) then
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount > 1) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 41, 8858, 27, 153);
	else 
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, 8857);
	end
end

if (EVENT == 8857) then
	ShowMap(UID, 511);
end

if (EVENT == 8858) then
	QuestStatusCheck = GetQuestStatus(UID, 147) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount > 1) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		RunQuestExchange(UID, 961)
		SaveEvent(UID, 8744);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		RunQuestExchange(UID, 962)
		SaveEvent(UID, 8749);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		RunQuestExchange(UID, 963)
		SaveEvent(UID, 8754);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		RunQuestExchange(UID, 964)
		SaveEvent(UID, 8759);
			else 
		SelectMsg(UID, 2, savenum, talknum, NPC, 18, 8857);
	end
	end
end
end
