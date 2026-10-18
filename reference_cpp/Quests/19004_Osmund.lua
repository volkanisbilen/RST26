-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 19004;

if (EVENT == 165) then
	QuestStatusCheck = GetQuestStatus(UID, 697)	
	ITEM1_COUNT = HowmuchItem(UID, 900217000);  
	if(QuestStatusCheck == 1 and ITEM1_COUNT < 1) then
		EVENT = 2000
	else
		NpcMsg(UID, 568, NPC)
end
end

local savenum = 102;

if (EVENT == 195) then -- 24 Level Princers Scorpion
	SaveEvent(UID, 302);
end

if (EVENT == 200) then
		SelectMsg(UID, 4, savenum, 568, NPC, 22, 201, 23, 202);
end

if (EVENT == 201) then
	SaveEvent(UID, 303);
end

if (EVENT == 202) then
	SaveEvent(UID, 306);
end

if (EVENT == 205) then
	SelectMsg(UID, 2, savenum, 568, NPC, 10, -1);
	SaveEvent(UID, 305);
end

if (EVENT == 210) then
	ITEMA = HowmuchItem(UID, 810418000);
	if (ITEMA < 2) then
		SelectMsg(UID, 2, savenum, 568, NPC, 18, 206);
	else
		SelectMsg(UID, 5, savenum, 568, NPC, 41, 207, 27, -1);
	end
end

if (EVENT == 206) then
	ShowMap(UID, 32);
end

if (EVENT == 207) then
	QuestStatusCheck = GetQuestStatus(UID, 102) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 810418000);
	if (ITEMA < 2) then
		SelectMsg(UID, 2, savenum, 568, NPC, 18, 206);
	else
RunQuestExchange(UID,46,STEP,1);
SaveEvent(UID, 304);
end
end
end

local savenum = 110;

if (EVENT == 410) then -- 28 Level Undying Bone
	SaveEvent(UID, 820);
end

if (EVENT == 412) then
	SelectMsg(UID, 4, savenum, 723, NPC, 22, 413, 23, 414);
end

if (EVENT == 413) then
	SaveEvent(UID, 821);
end

if (EVENT == 414) then
	SaveEvent(UID, 824);
end

if (EVENT == 416) then
	SelectMsg(UID, 2, savenum, 723, NPC, 10, -1);
	SaveEvent(UID, 823);
end

if (EVENT == 417) then
	ITEMA = HowmuchItem(UID, 810418000);
	if (ITEMA < 1) then
		SelectMsg(UID, 2, savenum, 723, NPC, 18, 418);
	else
		SelectMsg(UID, 4, savenum, 723, NPC, 41, 419, 27, -1);
	end
end

if (EVENT == 418) then
	ShowMap(UID, 22);
end

if (EVENT == 419) then
	QuestStatusCheck = GetQuestStatus(UID, 110) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 810418000);
	if (ITEMA < 1) then
		SelectMsg(UID, 2, savenum, 723, NPC, 18, 418);
	else
RunQuestExchange(UID,132)
	SaveEvent(UID, 822);
end
end
end

local savenum = 112;

if (EVENT == 290) then -- 29 Level Beozar of Glyptodont
	SaveEvent(UID, 510);
end

if (EVENT == 292) then
	SelectMsg(UID, 4, savenum, 1338, NPC, 22, 293, 23, 294);
end

if (EVENT == 293) then
	SaveEvent(UID, 511);
end

if (EVENT == 294) then
	SaveEvent(UID, 514);
end

if (EVENT == 297) then
	SelectMsg(UID, 2, savenum, 1338, NPC, 10, -1);
	SaveEvent(UID, 513);
end

if (EVENT == 298) then
	ITEM = HowmuchItem(UID, 810418000);
	if (ITEM < 2) then
		SelectMsg(UID, 2, savenum, 1338, NPC, 18, 299);
	else
		SelectMsg(UID, 5, savenum, 1338, NPC, 41, 300, 27, -1);
	end
end

if (EVENT == 299) then
	ShowMap(UID, 595);
end

if (EVENT == 300) then
	QuestStatusCheck = GetQuestStatus(UID, 112) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM = HowmuchItem(UID, 810418000);
	if (ITEM < 2) then
		SelectMsg(UID, 2, savenum, 1338, NPC, 18, 299);
	else
RunQuestExchange(UID,52,STEP,1);
	SaveEvent(UID, 512);
end
end
end

local savenum = 114;

if (EVENT == 301) then -- 30 Level Boss Hunt
	SaveEvent(UID, 332);
end

if (EVENT == 303) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, savenum, 584, NPC, 22, 304, 23, 305);
	else
		SelectMsg(UID, 2, savenum, 584, NPC, 10, -1);
	end
end

if (EVENT == 304) then
	SaveEvent(UID, 333);
end

if (EVENT == 305) then
	SaveEvent(UID, 336);
end

if (EVENT == 307) then
	SelectMsg(UID, 2, savenum, 584, NPC, 10, -1);
	SaveEvent(UID, 335);
end

if (EVENT == 308) then
	MonsterCount01 = CountMonsterQuestSub(UID, 114, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 114, 2);
	MonsterCount03 = CountMonsterQuestSub(UID, 114, 3);
	MonsterCount04 = CountMonsterQuestSub(UID, 114, 4);
	if (MonsterCount01 > 0 and MonsterCount02 > 0 and MonsterCount03 > 0 and MonsterCount04 > 0) then 
	SelectMsg(UID, 4, savenum, 584, NPC, 41, 310, 27, -1);
	else
		SelectMsg(UID, 2, savenum, 584, NPC, 15, 309);
	end
end

if (EVENT == 309) then
	ShowMap(UID, 694);
end

if (EVENT == 310) then
	QuestStatusCheck = GetQuestStatus(UID, 114) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount01 = CountMonsterQuestSub(UID, 114, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 114, 2);
	MonsterCount03 = CountMonsterQuestSub(UID, 114, 3);
	MonsterCount04 = CountMonsterQuestSub(UID, 114, 4);
	if (MonsterCount01 < 1 and MonsterCount02 < 1 and MonsterCount03 < 1 and MonsterCount04 < 1) then 
	SelectMsg(UID, 2, savenum, 584, NPC, 15, 309);
		else
	RunQuestExchange(UID,125);
	SaveEvent(UID, 334);
end
end
end

local savenum = 115;

if (EVENT == 400) then -- 30 Level Keilan Scale
	SaveEvent(UID, 9944);
end

if (EVENT == 402) then
	SelectMsg(UID, 4, savenum, 1232, NPC, 22, 403, 23, 404);
end

if (EVENT == 403) then
	SaveEvent(UID, 9945);
end

if (EVENT == 404) then
	SaveEvent(UID, 9948);
end

if (EVENT == 405) then
	SelectMsg(UID, 2, savenum, 1232, NPC, 10, -1);
	SaveEvent(UID, 9947);
end

if (EVENT == 407) then
	ITEMB = HowmuchItem(UID, 810418000);
	if (ITEMB < 2) then
		SelectMsg(UID, 2, savenum, 1232, NPC, 18, 408);
	else
		SelectMsg(UID, 5, savenum, 1232, NPC, 41, 409, 27, -1);
	end
end

if (EVENT == 408) then
	ShowMap(UID, 527);
end

if (EVENT == 409) then
	QuestStatusCheck = GetQuestStatus(UID, 115) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMB = HowmuchItem(UID, 810418000);
	if (ITEMB < 2) then
		SelectMsg(UID, 2, savenum, 1232, NPC, 18, 408);
	else
RunQuestExchange(UID,535,STEP,1);
	SaveEvent(UID, 9946);
end
end
end

if (EVENT == 2000)then
SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
         else
SelectMsg(UID, 2, -1, 22270, NPC, 22,-1);
GiveItem(UID, 900217000,1);
end
end