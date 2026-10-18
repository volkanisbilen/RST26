-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 19003;

if (EVENT == 150) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 8255, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 8257, NPC)
	else
		EVENT = QuestNum
	end
end


local savenum = 101;

if (EVENT == 8230) then -- 24 Level Spoiler
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8231);
end

if (EVENT == 8231) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 7917);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 7922);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 7927);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 7932);
	end
end

if (EVENT == 8232) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8233, 23, 8234);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8233) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 7918);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 7923);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 7928);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 7933);
	end
end

if (EVENT == 8234) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 7921);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 7926);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 7931);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 7936);
	end
end

if (EVENT == 8240) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 7920);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 7925);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 7930);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 7935);
	end
end

if (EVENT == 8236) then
	MonsterCount = CountMonsterQuestSub(UID, 101,1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8237);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8238, 23, -1);
	end
end

if (EVENT == 8237) then
	ShowMap(UID, 521);
end

if (EVENT == 8238) then
	QuestStatusCheck = GetQuestStatus(UID, 101) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 101,1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8237);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	RunQuestExchange(UID,824)
		SaveEvent(UID, 7919);
	elseif (Class == 2 or Class == 7 or Class == 8) then
	RunQuestExchange(UID,825)
		SaveEvent(UID, 7924);
   elseif (Class == 3 or Class == 9 or Class == 10) then
   RunQuestExchange(UID,826)
		SaveEvent(UID, 7929);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	RunQuestExchange(UID,827)
		SaveEvent(UID, 7934);
end
end
end
end

local savenum = 103;

if (EVENT == 8330) then -- 25 Level Scorpion
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8331);
end

if (EVENT == 8331) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 7959);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 7964);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 7969);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 7974);
	end
end

if (EVENT == 8332) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8333, 23, 8334);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8333) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 7960);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 7965);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 7970);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 7975);
	end
end

if (EVENT == 8334) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 7963);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 7968);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 7973);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 7978);
	end
end

if (EVENT == 8340) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 7962);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 7967);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 7972);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 7977);
	end
end

if (EVENT == 8336) then
	MonsterCount = CountMonsterQuestSub(UID, 103,1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8337);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8338, 23, -1);
	end
end

if (EVENT == 8337) then
	ShowMap(UID, 567);
end

if (EVENT == 8338) then
	QuestStatusCheck = GetQuestStatus(UID, 103) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 103,1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8337);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,814)
		SaveEvent(UID, 7961);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,815)
		SaveEvent(UID, 7966);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,816)
		SaveEvent(UID, 7971);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,817)
		SaveEvent(UID, 7976);
end
end
end
end

local savenum = 104;

if (EVENT == 8430) then -- 26 Level Lycaon
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8431);
end

if (EVENT == 8431) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8013);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8018);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8023);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8028);
	end
end

if (EVENT == 8432) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8433, 23, 8434);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8433) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8014);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8019);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8024);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8029);
	end
end

if (EVENT == 8434) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8017);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8022);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8027);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8032);
	end
end

if (EVENT == 8440) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8016);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8021);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8026);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8031);
	end
end

if (EVENT == 8436) then
	MonsterCount = CountMonsterQuestSub(UID, 104,1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8437);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8438, 23, -1);
	end
end

if (EVENT == 8437) then
	ShowMap(UID, 61);
end

if (EVENT == 8438) then
	QuestStatusCheck = GetQuestStatus(UID, 104) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 104,1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8437);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,820)
		SaveEvent(UID, 8015);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,821)
		SaveEvent(UID, 8020);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,822)
		SaveEvent(UID, 8025);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,823)
		SaveEvent(UID, 8030);
end
end
end
end

local savenum = 105;

if (EVENT == 8530) then -- 27 Level Loup Garou
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8531);
end

if (EVENT == 8531) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8055);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8060);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8065);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8070);
	end
end

if (EVENT == 8532) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8533, 23, 8534);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8533) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8056);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8061);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8066);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8071);
	end
end

if (EVENT == 8534) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8059);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8064);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8069);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8074);
	end
end

if (EVENT == 8540) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8058);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8063);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8068);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8073);
	end
end

if (EVENT == 8536) then
	MonsterCount = CountMonsterQuestSub(UID, 105,1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8537);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8538, 23, -1);
	end
end

if (EVENT == 8537) then
	ShowMap(UID, 68);
end

if (EVENT == 8538) then
	QuestStatusCheck = GetQuestStatus(UID, 105)
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then	
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 105,1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8537);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,832)
		SaveEvent(UID, 8057);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,833)
		SaveEvent(UID, 8062);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,834)
		SaveEvent(UID, 8067);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,835)
		SaveEvent(UID, 8072);
end
end
end
end

local savenum = 109;

if (EVENT == 215) then -- 28 Level Skull
	SelectMsg(UID, 2, savenum, 1323, NPC, 10, 216);
end

if (EVENT == 216) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 483);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 488);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 493);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 498);
	end
end

if (EVENT == 217) then
	SelectMsg(UID, 4, savenum, 1323, NPC, 22, 218, 23, 219);
end

if (EVENT == 218) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 484);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 489);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 494);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 499);
	end
end

if (EVENT == 219) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 487);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 492);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 497);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 502);
	end
end

if (EVENT == 221) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 486);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 491);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 496);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 501);
	end
end

if (EVENT == 224) then
	ITEMSKULL = HowmuchItem(UID, 810418000);
	if (ITEMSKULL < 3) then
		SelectMsg(UID, 2, savenum, 1323, NPC, 18, 225);
	else
		SelectMsg(UID, 5, savenum, 1323, NPC, 41, 227,23,-1);
	end
end

if (EVENT == 225) then
	ShowMap(UID, 23);
end

if (EVENT == 227) then
	QuestStatusCheck = GetQuestStatus(UID, 109) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMSKULL = HowmuchItem(UID, 810418000);
	if (ITEMSKULL < 3) then
		SelectMsg(UID, 2, savenum, 1323, NPC, 18, 225);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,47,STEP,1); 
		SaveEvent(UID, 485);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,48,STEP,1); 
		SaveEvent(UID, 490);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,49,STEP,1);
		SaveEvent(UID, 495);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,50,STEP,1);
		SaveEvent(UID, 500);
end
end
end
end

local savenum = 108;

if (EVENT == 8130) then -- 28 Level Dark Eyes
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8131);
end

if (EVENT == 8131) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	SaveEvent(UID, 8103);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8108);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8113);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8118);
	end
end

if (EVENT == 8132) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8133, 23, 8134);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8133) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8104);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8109);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8114);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8119);
	end
end

if (EVENT == 8134) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8107);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8112);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8117);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8122);
	end
end

if (EVENT == 8140) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8106);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8111);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8116);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8121);
	end
end

if (EVENT == 8136) then
	MonsterCount = CountMonsterQuestSub(UID, 108, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8137);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8138, 23, -1);
	end
end

if (EVENT == 8137) then
	ShowMap(UID, 568);
end

if (EVENT == 8138) then
	QuestStatusCheck = GetQuestStatus(UID, 108) 
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 108, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8137);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,828)
		SaveEvent(UID, 8105);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,829)
		SaveEvent(UID, 8110);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,830)
		SaveEvent(UID, 8115);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,831)
		SaveEvent(UID, 8120);
end
end
end
end

if (EVENT == 9300) then -- 29 Level Keilan
	SaveEvent(UID, 5316);
end

if (EVENT == 9305) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 100, 8673, NPC, 22, 9306, 23, 9307);
	else
		SelectMsg(UID, 2, 100, 8673, NPC, 10, -1);
	end
end

if (EVENT == 9306) then
	SaveEvent(UID, 5317);
end

if (EVENT == 9307) then
	SaveEvent(UID, 5320);
end

if (EVENT == 9310) then
	SaveEvent(UID, 5319);
end

if (EVENT == 9315) then
	MonsterCount = CountMonsterQuestSub(UID, 100,1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 100, 8673, NPC, 18, 9308);
	else
		SelectMsg(UID, 4, 100, 8673, NPC, 41, 9309, 27, -1);
	end
end

if (EVENT == 9308) then
	ShowMap(UID, 527);
end

if (EVENT == 9309) then
	QuestStatusCheck = GetQuestStatus(UID, 100) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 100,1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 100, 8673, NPC, 18, 9308);
	else
	RunQuestExchange(UID, 1086)
	SaveEvent(UID, 5318);
end
end
end

local savenum = 113;

if (EVENT == 8730) then -- 30 Level Skeleton
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8731);
end

if (EVENT == 8731) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8187);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8192);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8197);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8202);
	end
end

if (EVENT == 8732) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8733, 23, 8734);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8733) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8188);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8193);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8198);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8203);
	end
end

if (EVENT == 8734) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8191);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8196);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8201);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8206);
	end
end

if (EVENT == 8740) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8190);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8195);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8200);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8205);
	end
end

if (EVENT == 8736) then
	MonsterCount = CountMonsterQuestSub(UID, 113, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8737);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8738, 23, -1);
	end
end

if (EVENT == 8737) then
	ShowMap(UID, 23);
end

if (EVENT == 8738) then
	QuestStatusCheck = GetQuestStatus(UID, 113) 
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 113, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8737);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
	RunQuestExchange(UID, 848)
		SaveEvent(UID, 8189);
	elseif (Class == 2 or Class == 7 or Class == 8) then
	RunQuestExchange(UID, 849)
		SaveEvent(UID, 8194);
	elseif (Class == 3 or Class == 9 or Class == 10) then
	RunQuestExchange(UID, 850)
		SaveEvent(UID, 8199);
	elseif (Class == 4 or Class == 11 or Class == 12) then
	RunQuestExchange(UID, 851)
		SaveEvent(UID, 8204);
end
end
end
end


local savenum = 116;

if (EVENT == 8830) then -- 31 Level Paralyzer
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8831);
end

if (EVENT == 8831) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8229);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8234);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8239);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8244);
	end
end

if (EVENT == 8832) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8833, 23, 8834);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8833) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8230);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8235);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8240);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8245);
	end
end

if (EVENT == 8834) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8233);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8238);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8243);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8248);
	end
end

if (EVENT == 8840) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8232);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8237);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8242);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8247);
	end
end

if (EVENT == 8836) then
	MonsterCount = CountMonsterQuestSub(UID ,116, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8837);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8838, 23, -1);
	end
end

if (EVENT == 8837) then
	ShowMap(UID, 570);
end

if (EVENT == 8838) then
	QuestStatusCheck = GetQuestStatus(UID, 116)
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then	
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID ,116, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8837);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,856)
		SaveEvent(UID, 8231);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,857)
		SaveEvent(UID, 8236);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,858)
		SaveEvent(UID, 8241);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,859)
		SaveEvent(UID, 8246);
end
end
end
end

local savenum = 118;

if (EVENT == 8930) then -- 32 Level Dire Wolf
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8931);
end

if (EVENT == 8931) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8271);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8276);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8281);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8286);
	end
end

if (EVENT == 8932) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8933, 23, 8934);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8933) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8272);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8277);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8282);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8287);
	end
end

if (EVENT == 8934) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8275);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8280);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8285);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8290);
	end
end

if (EVENT == 8940) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8274);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8279);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8284);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8289);
	end
end

if (EVENT == 8936) then
	MonsterCount = CountMonsterQuestSub(UID, 118, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8937);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8938, 23, -1);
	end
end

if (EVENT == 8937) then
	ShowMap(UID, 523);
end

if (EVENT == 8938) then
	QuestStatusCheck = GetQuestStatus(UID, 118)
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then	
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 118, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8937);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,864)
		SaveEvent(UID, 8273);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,865)
		SaveEvent(UID, 8278);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,866)
		SaveEvent(UID, 8283);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,867)
		SaveEvent(UID, 8288);
end
end
end
end


local savenum = 120;

if (EVENT == 8900) then -- 33 Level Smildon
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8901);
end

if (EVENT == 8901) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8313);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8318);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8323);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8328);
	end
end

if (EVENT == 8902) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8903, 23, 8904);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8903) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8314);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8319);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8324);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8329);
	end
end

if (EVENT == 8904) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8317);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8322);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8327);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8332);
	end
end

if (EVENT == 8910) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8316);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8321);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8326);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8331);
	end
end

if (EVENT == 8906) then
	MonsterCount = CountMonsterQuestSub(UID, 120, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8907);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8908, 23, -1);
	end
end

if (EVENT == 8907) then
	ShowMap(UID, 525);
end

if (EVENT == 8908) then
	QuestStatusCheck = GetQuestStatus(UID, 120)
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then	
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 120, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8907);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,872)
		SaveEvent(UID, 8315);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,873)
		SaveEvent(UID, 8320);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,874)
		SaveEvent(UID, 8325);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,875)
		SaveEvent(UID, 8330);
end
end
end
end

local savenum = 122;

if (EVENT == 8800) then -- 34 Level Wild Smildon
	SelectMsg(UID, 2, savenum, 8034, NPC, 10, 8801);
end

if (EVENT == 8801) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8355);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8360);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8365);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8370);
	end
end

if (EVENT == 8802) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 8034, NPC, 22, 8803, 23, 8804);
	else
		SelectMsg(UID, 2, savenum, 8034, NPC, 10, -1);
	end
end

if (EVENT == 8803) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8356);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8361);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8366);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8371);
	end
end

if (EVENT == 8804) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8359);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8364);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8369);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8374);
	end
end

if (EVENT == 8810) then
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 8358);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 8363);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 8368);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 8373);
	end
end

if (EVENT == 8806) then
	MonsterCount = CountMonsterQuestSub(UID, 122, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8807);
	else
		SelectMsg(UID, 4, savenum, 8034, NPC, 41, 8808, 23, -1);
	end
end

if (EVENT == 8807) then
	ShowMap(UID, 572);
end

if (EVENT == 8808) then
	QuestStatusCheck = GetQuestStatus(UID, 122) 
	SlotCheck = CheckGiveSlot(UID, 2)
	if SlotCheck == false then
	elseif(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 122, 1);
	if (MonsterCount < 5) then
		SelectMsg(UID, 2, savenum, 8034, NPC, 18, 8807);
	else
	Class = CheckClass(UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
RunQuestExchange(UID,880)
		SaveEvent(UID, 8357);
	elseif (Class == 2 or Class == 7 or Class == 8) then
RunQuestExchange(UID,881)
		SaveEvent(UID, 8362);
	elseif (Class == 3 or Class == 9 or Class == 10) then
RunQuestExchange(UID,882)
		SaveEvent(UID, 8367);
	elseif (Class == 4 or Class == 11 or Class == 12) then
RunQuestExchange(UID,883)
		SaveEvent(UID, 8372);
end
end
end
end