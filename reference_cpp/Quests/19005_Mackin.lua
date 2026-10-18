-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 19005;

if (EVENT == 100) then
	SelectMsg(UID, 3, 974, 8082, NPC, 4481, 101, 3019, 203);
	--SelectMsg(UID, 48, -1, -1, NPC);
end

if (EVENT == 101) then
	MONEY = HowmuchItem(UID, 900000000);
	if (MONEY > 9999) then
		SelectMsg(UID, 2, 974, 8083, NPC, 4484, 102, 4296, -1);
	else
		SelectMsg(UID, 2, 974, 8084, NPC, 18, 5000);
	end
end

if (EVENT == 102) then
	MONEY = HowmuchItem(UID, 900000000);
		if (MONEY < 10000) then
			SelectMsg(UID, 2, 974, 8084, NPC, 18, 5000);
		else
	IsTakeToday = GetUserDailyOp(UID,8);
		if (IsTakeToday == 1) then
			GoldLose(UID, 10000);
			SelectMsg(UID, 16, 974, NPC);
		else
			SelectMsg(UID, 2, -1, 11584, NPC, 10, -1);
		end
	end
end

if (EVENT == 301) then
	SelectMsg(UID, 3, -1, 11705, NPC, 8246, 310,8254, 340, 8258, 370, 8265,400);
end

if(EVENT == 310)then
	QUEST1 = GetQuestStatus(UID,1119);
	EXP = GetExpPercent(UID);
	if(QUEST1 == 1) then
		SelectMsg(UID, 2, -1, 11706, NPC, 65, 319);
	elseif(QUEST1 == 2)then
		SelectMsg(UID, 2, -1, 11706, NPC, 10, -1);
	elseif (QUEST1 == 0) then
		if(EXP == 100)then
			SelectMsg(UID, 2, -1, 11686, NPC, 8247, 311);
		else
			SelectMsg(UID, 2, -1, 11694, NPC, 10, -1);
		end
	end
end

if(EVENT == 311)then
	SelectMsg(UID, 2, -1, 11687, NPC, 8248, 312);
end

if(EVENT == 312)then
	SelectMsg(UID, 2, -1, 11688, NPC, 8249, 313);
end

if(EVENT == 313)then
	SelectMsg(UID, 2, -1, 11689, NPC, 8250, 314);
end

if(EVENT == 314)then
	SelectMsg(UID, 2, -1, 11690, NPC, 8251, 315);
end

if(EVENT == 315)then
	SelectMsg(UID, 2, -1, 11691, NPC, 8252, 316);
end

if(EVENT == 316)then
	SelectMsg(UID, 2, -1, 11692, NPC, 8253, 317);
end

if(EVENT == 317)then
	SaveEvent(UID,7273);
end

if(EVENT == 319) then
	LEVEL = GetLevel(UID);
	EXP = GetExpPercent(UID);	
	if(LEVEL == 83 and EXP == 100) then
		SaveEvent(UID,7274);
	else
		EVENT = 310;
	end
end
if(EVENT == 340) then
	QUEST1 = GetQuestStatus(UID,1119);
	QUEST2 = GetQuestStatus(UID,1120);
	if (QUEST1 ~= 2)then
		SelectMsg(UID, 2, -1, 11710, NPC, 10, -1);
	elseif(QUEST2 == 0) then
		SelectMsg(UID, 2, -1, 11696, NPC, 8256, 341);
	elseif(QUEST2 == 1) then
		SelectMsg(UID, 2, -1, 11698, NPC, 65, 343);
	else if(QUEST2 == 2) then
		SelectMsg(UID, 2, -1, 11706, NPC, 10, -1);
	else
		SelectMsg(UID, 2, -1, 11706, NPC, 10, -1);
		end
	end	
end
if(EVENT == 341) then
	SelectMsg(UID, 2, -1, 11697, NPC, 65, 342);
end

if(EVENT == 342) then
	SaveEvent(UID,7278);
end

if(EVENT == 343) then
	LEVEL = GetLevel(UID)
	MONEY = HowmuchItem(UID, 900000000)
	if(LEVEL == 83 and MONEY >= 100000000) then 
		GoldLose(UID,100000000);
		SaveEvent(UID,7279);
	else
		SelectMsg(UID, 2, -1, 11698, NPC, 10, -1);
	end
end

if(EVENT == 370) then
	QUEST1 = GetQuestStatus(UID,1119);
	QUEST2 = GetQuestStatus(UID,1120);
	QUEST3 = GetQuestStatus(UID,1121);
	if (QUEST1 ~= 2)then
		SelectMsg(UID, 2, -1, 11710, NPC, 10, -1);
	elseif (QUEST2 ~= 2)then
		SelectMsg(UID, 2, -1, 11711, NPC, 10, -1);
	elseif(QUEST3 == 0)then
		SelectMsg(UID, 2, -1, 11699, NPC, 8259, 371);
	elseif(QUEST3 == 1) then
		SelectMsg(UID, 2, -1, 11703, NPC, 8261, 375);
	end	
end

if(EVENT == 371) then
	SelectMsg(UID, 2, -1, 11700, NPC, 8260, 372);
end

if(EVENT == 372) then
	SelectMsg(UID, 2, -1, 11701, NPC, 8256, 373);
end

if(EVENT == 373) then
	SelectMsg(UID, 2, -1, 11702, NPC, 65, 374);
end

if(EVENT == 374) then
	SaveEvent(UID,7283);
end

if(EVENT == 375) then
	LEVEL = GetLevel(UID);
	NP = CheckLoyalty(UID);
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	else
	if(LEVEL == 83 and NP >= 10000) then 
		RobLoyalty(UID,10000);
		GiveItem(UID,900579000,1);
		SaveEvent(UID,7276);
		SaveEvent(UID,7281);
		SaveEvent(UID,7286);
		SaveEvent(UID,7288);
	else
		SelectMsg(UID, 2, -1, 11708, NPC, 10, -1);
		end
	end
end

if(EVENT == 400) then
QUEST1 = GetQuestStatus(UID,1122);
	if (QUEST1 == 1) then
		SelectMsg(UID, 2, -1, 11704, NPC, 8263, 401, 8264,402);
	else
		SelectMsg(UID, 2, -1, 11706, NPC, 10, -1);
	end
end

if(EVENT == 401) then
	ITEM = HowmuchItem(UID,900579000)
	if(ITEM > 0) then
		SelectMsg(UID, 48, -1, -1, NPC);	
	else
		SelectMsg(UID, 2, 974, 11704, NPC, 27, 402);	
	end
end

if (EVENT == 5000) then
	ShowMap(UID, 336);
end