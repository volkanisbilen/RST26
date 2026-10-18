-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14432;

if (EVENT == 160) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 8256, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 8258, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1000) then -- 41 Lard Orc Premium
	SaveEvent(UID, 2079);
end

if (EVENT == 1002) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 414, 8152, NPC, 22, 1003, 23, 1004);
	else
		SelectMsg(UID, 2, 414, 8152, NPC, 10, -1);
	end
end

if (EVENT == 1003) then
	SaveEvent(UID, 2080);
end

if (EVENT == 1004) then
	SaveEvent(UID, 2083);
end

if (EVENT == 1010) then
	SaveEvent(UID, 2082);
end

if (EVENT == 1006) then
	MonsterCount = CountMonsterQuestSub(UID, 414, 1);
	if (MonsterCount < 19) then
		SelectMsg(UID, 2, 414, 8152, NPC, 18, 1007);
	else
		SelectMsg(UID, 4, 414, 8152, NPC, 41, 1008, 27, -1);
	end
end

if (EVENT == 1007) then
	ShowMap(UID, 108);
end

if (EVENT == 1008) then
	QuestStatusCheck = GetQuestStatus(UID, 414) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 414, 1);
	if (MonsterCount < 19) then
		SelectMsg(UID, 2, 414, 8152, NPC, 18, 1007);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1211)
		SaveEvent(UID, 2081);
	else
		RunQuestExchange(UID,1211)
		SaveEvent(UID, 2081);   
	end
end
end
end

local savenum = 144;

if (EVENT == 8750) then -- 41 Level Lard Orc
	SelectMsg(UID, 2, savenum, 8152, NPC, 10, 8751);
end

if (EVENT == 8751) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8700);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8705);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8710);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8715);
	end
end

if (EVENT == 8752) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8152, NPC, 22, 8753, 23, 8754);
	else
		SelectMsg(UID, 2, savenum, 8152, NPC, 10, -1);
	end
end

if (EVENT == 8753) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8701);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8706);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8711);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8716);
	end
end

if (EVENT == 8754) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8704);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8709);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8714);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8719);
	end
end

if (EVENT == 8760) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8703);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8708);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8713);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8718);
	end
end

if (EVENT == 8756) then
	MonsterCount = CountMonsterQuestSub(UID, 144, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 8152, NPC, 18, 8757);
	else
		SelectMsg(UID, 5, savenum, 8152, NPC, 41, 8758,23, -1);
	end
end

if (EVENT == 8757) then
	ShowMap(UID, 108);
end

if (EVENT == 8758) then
	QuestStatusCheck = GetQuestStatus(UID, 144) 
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 144, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 8152, NPC, 18, 8757);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,953,STEP,1);
		SaveEvent(UID, 8702);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,954,STEP,1);
		SaveEvent(UID, 8707);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,955,STEP,1);
		SaveEvent(UID, 8712);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,956,STEP,1);
		SaveEvent(UID, 8717);
end
end
end
end

if (EVENT == 8170) then -- 45 Level Scolar
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8826);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8831);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8836);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8841);
	end
end

if (EVENT == 8172) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 176, 8394, NPC, 10, 8175);
	else
		SelectMsg(UID, 2, 176, 8282, NPC, 10, -1);
	end
end

if (EVENT == 8175) then
	SelectMsg(UID, 4, 176, 8395, NPC, 22, 8173, 23, -1);
end

if (EVENT == 8173) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8827);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8832);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8837);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8842);
	end
end

if (EVENT == 8180) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8829);
		EVENT = 8181
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8834);
		EVENT = 8181
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8839);
		EVENT = 8181
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8844);
		EVENT = 8181
	end
end

if (EVENT == 8181) then
	SelectMsg(UID, 2, 176, 8393, NPC, 3007, -1);
end

if (EVENT == 8176) then
	MonsterCount = CountMonsterQuestSub(UID, 176, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 176, 8394, NPC, 18, 8177);
	else
		SelectMsg(UID, 4, 176, 8396, NPC, 41, 8178, 27, -1);
	end
end

if (EVENT == 8177) then
	ShowMap(UID, 539);
end

if (EVENT == 8178) then
	QuestStatusCheck = GetQuestStatus(UID, 176) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 176, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 176, 8394, NPC, 18, 8177);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,977)
		SaveEvent(UID, 8828);
	elseif (Class == 2 or Class == 7 or Class == 8) then      
RunQuestExchange(UID,978)
		SaveEvent(UID, 8833);
	elseif (Class == 3 or Class == 9 or Class == 10) then    
RunQuestExchange(UID,979)
		SaveEvent(UID, 8838);
	elseif (Class == 4 or Class == 11 or Class == 12) then     
RunQuestExchange(UID,980)
		SaveEvent(UID, 8843);
end
end
end
end

if (EVENT == 9430) then -- 47 Level Macairodus
	SelectMsg(UID, 2, 207, 8761, NPC, 10, 9431);
end

if (EVENT == 9431) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9492);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9497);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9502);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9507);
	end 
end

if (EVENT == 9432) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 207, 8761, NPC, 22, 9433, 23, 9434);
	else
		SelectMsg(UID, 2, 207, 8761, NPC, 10, -1);
	end
end

if (EVENT == 9433) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9493);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9498);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9503);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9508);
	end
end

if (EVENT == 9434) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9496);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9501);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9506);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9511);
	end
end

if (EVENT == 9440) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9495);
		EVENT = 9441
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9500);
		EVENT = 9441
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9505);
		EVENT = 9441
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9510);
		EVENT = 9441
	end
end

if (EVENT == 9441) then
	SelectMsg(UID, 2, 207, 8759, NPC, 3007, -1);
end

if (EVENT == 9436) then
	MonsterCount = CountMonsterQuestSub(UID, 207, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 207, 8384, NPC, 18, 9437);
	else
		SelectMsg(UID, 5, 207, 8396, NPC, 41, 9438, 27, -1);
	end
end

if (EVENT == 9437) then
	ShowMap(UID, 620);
end

if (EVENT == 9438) then
	QuestStatusCheck = GetQuestStatus(UID, 207) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 207, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 207, 8384, NPC, 18, 9437);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,1110,STEP,1);
		SaveEvent(UID, 9494);
	elseif (Class == 2 or Class == 7 or Class == 8) then      
RunQuestExchange(UID,1111,STEP,1);
		SaveEvent(UID, 9499);
	elseif (Class == 3 or Class == 9 or Class == 10) then    
RunQuestExchange(UID,1112,STEP,1);
		SaveEvent(UID, 9504);
	elseif (Class == 4 or Class == 11 or Class == 12) then     
RunQuestExchange(UID,1113,STEP,1);
		SaveEvent(UID, 9509);
end
end
end
end

if (EVENT == 9450) then -- 51 Level Blood Don
	SelectMsg(UID, 2, 228, 8763, NPC, 10, 9451);
end

if (EVENT == 9451) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9534);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9539);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9544);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9549);
	end
end

if (EVENT == 9452) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 228, 8763, NPC, 22, 9453, 23, -1);
	else
		SelectMsg(UID, 2, 228, 8763, NPC, 10, -1);
	end
end

if (EVENT == 9453) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9535);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9540);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9545);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9550);
	end
end

if (EVENT == 9460) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9537);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9542);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9547);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9552);
	end
end

if (EVENT == 9456) then
	MonsterCount = CountMonsterQuestSub(UID, 228, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 228, 8384, NPC, 18, 9457);
	else
		SelectMsg(UID, 5, 228, 8396, NPC, 41, 9458, 27, -1);
	end
end

if (EVENT == 9457) then
	ShowMap(UID, 624);
end

if (EVENT == 9458) then
	QuestStatusCheck = GetQuestStatus(UID, 228) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 228, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 228, 8384, NPC, 18, 9457);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,1118,STEP,1);
		SaveEvent(UID, 9536);
	elseif (Class == 2 or Class == 7 or Class == 8) then      
RunQuestExchange(UID,1119,STEP,1);
		SaveEvent(UID, 9541);
	elseif (Class == 3 or Class == 9 or Class == 10) then    
RunQuestExchange(UID,1120,STEP,1);
		SaveEvent(UID, 9546);
	elseif (Class == 4 or Class == 11 or Class == 12) then     
RunQuestExchange(UID,1121,STEP,1);
		SaveEvent(UID, 9551);
end
end
end
end

if (EVENT == 9060) then -- 56 Level Grell
	SelectMsg(UID, 2, 266, 8228, NPC, 3003, 9061);
end

if (EVENT == 9061) then
	SaveEvent(UID, 9123);
end

if (EVENT == 9062) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 2, 266, 8426, NPC, 10, 9070);
	else
		SelectMsg(UID, 2, 266, 8428, NPC, 10, -1);
	end
end

if (EVENT == 9070) then
	SelectMsg(UID, 4, 266, 8427, NPC, 22, 9063, 23, 9064);
end

if (EVENT == 9063) then
	SaveEvent(UID, 9124);
end

if (EVENT == 9064) then
	SaveEvent(UID, 9127);
end

if (EVENT == 9065) then
	SelectMsg(UID, 2, 266, 8429, NPC, 3014, -1);
	SaveEvent(UID, 9126);
end

if (EVENT == 9067) then
	MonsterCount = CountMonsterQuestSub(UID, 266, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 266, 8430, NPC, 18, 9068);
	else
		SelectMsg(UID, 4, 266, 8431, NPC, 41, 9069, 27, -1);
	end
end

if (EVENT == 9068) then
	ShowMap(UID, 318);
end

if (EVENT == 9069) then
	QuestStatusCheck = GetQuestStatus(UID, 266) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 266, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 266, 8430, NPC, 18, 9068);
	else
RunQuestExchange(UID,1043)
	SaveEvent(UID, 9125);
end
end
end

if (EVENT == 9320) then -- 57 Level Hell Hound
	SelectMsg(UID, 2, 268, 8677, NPC, 10, 9321);
end

if (EVENT == 9321) then
	SaveEvent(UID, 9345);
end

if (EVENT == 9322) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 268, 8677, NPC, 22, 9323, 23, 9324);
	else
		SelectMsg(UID, 2, 268, 8677, NPC, 10, -1);
	end
end

if (EVENT == 9323) then
	SaveEvent(UID, 9346);
end

if (EVENT == 9324) then
	SaveEvent(UID, 9349);
end

if (EVENT == 9325) then
	SaveEvent(UID, 9348);
end

if (EVENT == 9327) then
	MonsterCount = CountMonsterQuestSub(UID, 268, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 268, 8552, NPC, 18, 9328);
	else
		SelectMsg(UID, 4, 268, 8431, NPC, 41, 9329, 27, -1);
	end
end

if (EVENT == 9328) then
	ShowMap(UID, 36);
end

if (EVENT == 9329) then
	QuestStatusCheck = GetQuestStatus(UID, 268) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 268, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 268, 8552, NPC, 18, 9328);
	else
RunQuestExchange(UID,1088)
	SaveEvent(UID, 9347); 
end
end
end

if (EVENT == 9340) then -- 59 Level DTC
	SelectMsg(UID, 2, 270, 8681, NPC, 10, 9341);
end

if (EVENT == 9341) then
	SaveEvent(UID, 9369);
end

if (EVENT == 9342) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 270, 8681, NPC, 22, 9343, 23, 9344);
	else
		SelectMsg(UID, 2, 270, 8681, NPC, 10, -1);
	end
end

if (EVENT == 9343) then
	SaveEvent(UID, 9370);
end

if (EVENT == 9344) then
	SaveEvent(UID, 9373);
end

if (EVENT == 9345) then
	SaveEvent(UID, 9372);
end

if (EVENT == 9347) then
	MonsterCount = CountMonsterQuestSub(UID, 270, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 270, 8566, NPC, 18, 9348);
	else
		SelectMsg(UID, 4, 270, 8431, NPC, 41, 9349, 27, -1);
	end
end

if (EVENT == 9348) then
	ShowMap(UID, 704);
end

if (EVENT == 9349) then
	QuestStatusCheck = GetQuestStatus(UID, 270) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 270, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 270, 8566, NPC, 18, 9348);
	else
RunQuestExchange(UID,1091)
	SaveEvent(UID, 9371); 
end
end
end

if (EVENT == 9140) then -- 60 Level Lich
	SelectMsg(UID, 2, 292, 8212, NPC, 4244, 9141);
end

if (EVENT == 9141) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9276);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9281);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9286);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9291);
	end
end

if (EVENT == 9150) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 292, 8440, NPC, 3018, 9152, 3019, 9159);
	else
		SelectMsg(UID, 2, 292, 8441, NPC, 4242, -1);
	end
end

if (EVENT == 9152) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9277);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9282);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9287);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9292);
	end
end

if (EVENT == 9159) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9280);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9285);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9290);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9295);
	end
end

if (EVENT == 9153) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 9279);
		EVENT = 9154
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 9284);
		EVENT = 9154
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 9289);
		EVENT = 9154
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 9294);
		EVENT = 9154
	end
end

if (EVENT == 9154) then
	SelectMsg(UID, 2, 292, 8439, NPC, 57, -1);
end

if (EVENT == 9155) then
	MonsterCount = CountMonsterQuestSub(UID, 292, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 292, 8442, NPC, 18, 9157);
	else
		SelectMsg(UID, 4, 292, 8443, NPC, 41, 9158, 27, -1);
	end
end

if (EVENT == 9157) then
	ShowMap(UID, 16);
end

if (EVENT == 9158) then
	QuestStatusCheck = GetQuestStatus(UID, 292) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 292, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 292, 8442, NPC, 18, 9157);
	else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,1056)
		SaveEvent(UID, 9278);
    elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,1057)
		SaveEvent(UID, 9283);
    elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,1058)
		SaveEvent(UID, 9288);
    elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,1059)
		SaveEvent(UID, 9293);
  end	 
end
end
end


if (EVENT == 1100) then -- 43 Level Megantilion   Premium
	SaveEvent(UID, 2103);
end

if (EVENT == 1102) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 418, 8310, NPC, 22, 1103, 23, 1104);
	else
		SelectMsg(UID, 2, 418, 8310, NPC, 10, 153);
	end
end

if (EVENT == 1103) then
	SaveEvent(UID, 2104);
end

if (EVENT == 1104) then
	SaveEvent(UID, 2107);
end

if (EVENT == 1105) then
	SaveEvent(UID, 2106);
end

if (EVENT == 1106) then
	MonsterCount = CountMonsterQuestSub(UID, 418, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 418, 8310, NPC, 18, 1107);
	else
		SelectMsg(UID, 4, 418, 8310, NPC, 41, 1108, 23, 1107);
	end
end

if (EVENT == 1107) then
	ShowMap(UID, 29);
end

if (EVENT == 1108) then
	QuestStatusCheck = GetQuestStatus(UID, 418) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 418, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, 418, 8310, NPC, 18, 1107);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1213)
		SaveEvent(UID, 2105);
	else
		RunQuestExchange(UID,1213)
		SaveEvent(UID, 2105);
	end
end
end
end

if (EVENT == 1200) then -- 45 Level Scolar  Premium
	SaveEvent(UID, 2127);
end

if (EVENT == 1202) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 422, 1215, NPC, 22, 1203, 23, 1204);
	else
		SelectMsg(UID, 2, 422, 1215, NPC, 10, 153);
	end
end

if (EVENT == 1203) then
	SaveEvent(UID, 2128);
end

if (EVENT == 1204) then
	SaveEvent(UID, 2131);
end

if (EVENT == 1205) then
	SaveEvent(UID, 2130);
end

if (EVENT == 1206) then
	MonsterCount = CountMonsterQuestSub(UID, 422, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 422, 1215, NPC, 18, 1207);
	else
		SelectMsg(UID, 4, 422, 1215, NPC, 41, 1208, 23, 1207);
	end
	end
	
if (EVENT == 1207) then
	ShowMap(UID, 29);
end

if (EVENT == 1208) then
	QuestStatusCheck = GetQuestStatus(UID, 422) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 422, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 422, 1215, NPC, 18, 1207);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,1215)
		SaveEvent(UID, 2129);
	else
		RunQuestExchange(UID,1215)
		SaveEvent(UID, 2129);
	end
end
end
end

if (EVENT == 200) then -- 56 Level Grell  Premium
	SaveEvent(UID, 2307);
end

if (EVENT == 202) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 476, 8170, NPC, 22, 203, 23, 204);
	else
		SelectMsg(UID, 2, 476, 8170, NPC, 10, 153);
	end
end

if (EVENT == 203) then
	SaveEvent(UID, 2308);
end

if (EVENT == 204) then
	SaveEvent(UID, 2311);
end

if (EVENT == 205) then
	SaveEvent(UID, 2310);
end

if (EVENT == 207) then
	MonsterCount = CountMonsterQuestSub(UID, 476, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 476, 8170, NPC, 10, 206);
	else
		SelectMsg(UID, 4, 476, 8170, NPC, 41, 208, 23, 206);
	end
	end
	
if (EVENT == 208) then
	QuestStatusCheck = GetQuestStatus(UID, 476) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 476, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 476, 8170, NPC, 10, 206);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,21043)
		SaveEvent(UID, 2309);
	else
		RunQuestExchange(UID,21043)
		SaveEvent(UID, 2309);
	end
end
end
end

if (EVENT == 300) then -- 56 Level Hell Hound  Premium
	SaveEvent(UID, 2331);
end

if (EVENT == 302) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 479, 8677, NPC, 22, 303, 23, 304);
	else
		SelectMsg(UID, 2, 479, 8677, NPC, 10, 153);
	end
end

if (EVENT == 303) then
	SaveEvent(UID, 2332);
end

if (EVENT == 304) then
	SaveEvent(UID, 2335);
end

if (EVENT == 305) then
	SaveEvent(UID, 2334);
end

if (EVENT == 307) then
	MonsterCount = CountMonsterQuestSub(UID, 479, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 479, 8677, NPC, 10, 306);
	else
		SelectMsg(UID, 4, 479, 8677, NPC, 41, 308, 23, 306);
	end
	end
	
if (EVENT == 308) then
	QuestStatusCheck = GetQuestStatus(UID, 479) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 479, 1);
	if (MonsterCount < 30) then
		SelectMsg(UID, 2, 479, 8677, NPC, 10, 306);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,11088)
		SaveEvent(UID, 2333);
	else
		RunQuestExchange(UID,11088)
		SaveEvent(UID, 2333);
	end
end
end
end

if (EVENT == 400) then -- 58 Level Manticore  Premium
	SaveEvent(UID, 2355);
end

if (EVENT == 402) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 482, 8681, NPC, 22, 403, 23, 404);
	else
		SelectMsg(UID, 2, 482, 8681, NPC, 10, 153);
	end
end

if (EVENT == 403) then
	SaveEvent(UID, 2356);
end

if (EVENT == 404) then
	SaveEvent(UID, 2359);
end

if (EVENT == 405) then
	SaveEvent(UID, 2358);
end

if (EVENT == 407) then
	MonsterCount = CountMonsterQuestSub(UID, 482, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 482, 8681, NPC, 10, 406);
	else
		SelectMsg(UID, 4, 482, 8681, NPC, 41, 408, 23, 406);
	end
	end
	
if (EVENT == 408) then
	QuestStatusCheck = GetQuestStatus(UID, 482) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 482, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 482, 8681, NPC, 10, 406);
	else
	Prem = GetPremium(UID);
	if (Prem > 0) then
		RunQuestExchange(UID,11091)
		SaveEvent(UID, 2357);
	else
		RunQuestExchange(UID,11091)
		SaveEvent(UID, 2357);
	end
end
end
end

local savenum = 150;

if (EVENT == 8950) then -- 43 Level Megantilion
	SelectMsg(UID, 2, savenum, 8310, NPC, 10, 8951);
end 

if (EVENT == 8951) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8784);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8789);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8794);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8799);
	end
end

if (EVENT == 8952) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8310, NPC, 22, 8953, 23, 8954);
	else
		SelectMsg(UID, 2, savenum, 8310, NPC, 10, -1);
	end
end

if (EVENT == 8953) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8785);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8790);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8795);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8800);
	end
end

if (EVENT == 8954) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8788);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8793);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8798);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8803);
	end
end

if (EVENT == 8960) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8787);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8792);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8797);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8802);
	end
end

if (EVENT == 8956) then
	MonsterCount = CountMonsterQuestSub(UID, 150, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 8310, NPC, 18, 8957);
	else
		SelectMsg(UID, 4, savenum, 8310, NPC, 41, 8958, 23, -1);
	end
end

if (EVENT == 8958) then
	QuestStatusCheck = GetQuestStatus(UID, 150) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 150, 1);
	if (MonsterCount < 20) then
		SelectMsg(UID, 2, savenum, 8310, NPC, 18, 8957);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,969)
		SaveEvent(UID, 8791);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,970)
		SaveEvent(UID, 8786);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,971)
		SaveEvent(UID, 8796);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,972)
		SaveEvent(UID, 8801);
end
end
end
end

local savenum=393
local talknum=8761


if (EVENT == 9900) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 9901, 23, 193);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 193);
	end
end

if(EVENT == 9901) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1313);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1316);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1319);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1322);
	end
end

if(EVENT == 9905) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1314);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1317);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1320);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1323);
	end
end

if(EVENT == 9903) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 2) then
		SelectMsg(UID, 5, savenum, talknum, NPC, 41, 9906, 27, 153);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
end

if (EVENT == 9906) then
	QuestStatusCheck = GetQuestStatus(UID, savenum) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 2) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	RunQuestExchange(UID,11110)
	SaveEvent(UID, 1315);
	elseif (Class == 2 or Class == 7 or Class == 8) then
	RunQuestExchange(UID,11111)
	SaveEvent(UID, 1318);
	elseif (Class == 3 or Class == 9 or Class == 10) then
	RunQuestExchange(UID,11112)
	SaveEvent(UID, 1321);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	RunQuestExchange(UID,11113)
	SaveEvent(UID, 1324);
		else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
	end
end
end

local savenum=397
local talknum=8763


if (EVENT == 9800) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, talknum, NPC, 22, 9801, 23, 193);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 193);
	end
end

if(EVENT == 9801) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1409);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1412);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1415);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1418);
	end
end

if(EVENT == 9805) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 1410);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 1413);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 1416);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 1419);
	end
end

if(EVENT == 9803) then
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 19) then
		SelectMsg(UID, 5, savenum, talknum, NPC, 41, 9806, 27, 153);
	else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
end

if (EVENT == 9806) then
	QuestStatusCheck = GetQuestStatus(UID, 397) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount1 = CountMonsterQuestSub(UID, savenum, 1);
	if (MonsterCount1 > 19) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	RunQuestExchange(UID,11118)
	SaveEvent(UID, 1411);
	elseif (Class == 2 or Class == 7 or Class == 8) then
	RunQuestExchange(UID,11119)
	SaveEvent(UID, 1414);
	elseif (Class == 3 or Class == 9 or Class == 10) then
	RunQuestExchange(UID,11120)
	SaveEvent(UID, 1417);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	RunQuestExchange(UID,11121)
	SaveEvent(UID, 1420);
		else
		SelectMsg(UID, 2, savenum, talknum, NPC, 10, 217);
	end
	end
end
end