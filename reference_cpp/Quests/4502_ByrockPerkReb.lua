local UserClass;

local QuestNum;

local Ret = 0;

local NPC =4502;



if (EVENT == 500) then -- Exchange [Perks Point]
    SelectMsg(UID, 3, -1, 45318, NPC, 45560, 504,45558, 502, 45557, -1);
end
if (EVENT == 504) then -- Exchange [Perks Point]
    SelectMsg(UID, 3, -1, 45314, NPC, 45556, 501);
end

if (EVENT == 501) then
    ITEM1 = HowmuchItem(UID, 389068000);-- 1milenium bar
    ITEM2 = HowmuchItem(UID, 700089000);--2k kc
    if (ITEM1 < 1 or ITEM1 == 0 or ITEM2 < 1 or ITEM2 == 0) then
        SelectMsg(UID, 2, -1, 45315, NPC, 45556 -1);
    else
    RobItem(UID, 389068000, 1); -- 1milenium bar
    RobItem(UID, 700089000, 1); --2k kc
    PerkUseItem(UID, 0, 0, 1)
    end
end
if (EVENT == 502) then
    SelectMsg(UID, 3, -1, 45316, NPC, 45559, 505);
end
if (EVENT == 503) then
	SelectMsg(UID, 3, -1, 45316, NPC, 45559, 503);
	EXP = GetExpPercent(UID);
   	LEVEL = GetLevel(UID);
	NP = CheckLoyalty(UID);
	MONEY = HowmuchItem(UID, 900000000);
	SlotCheck = CheckGiveSlot(UID, 1)
	if SlotCheck == false then
	else
		if(LEVEL == 83 and NP >= 10000  and EXP == 100 and MONEY>=100000000 ) then 
		RobLoyalty(UID,10000);
		GoldLose(UID, 100000000);
		GiveItem(UID,900579000,1);
		SelectMsg(UID, 48, -1, -1, NPC);
		else
		SelectMsg(UID, 2, -1, 45317, NPC, 45555, -1);
		end
		
	end
end

if (EVENT == 505) then
	SelectMsg(UID, 3, -1, 45316, NPC, 45559, 503);
	SlotCheck = CheckGiveSlot(UID, 1)
	REBITEM = HowmuchItem(UID, 900579000);
	if SlotCheck == false then
	else
		if(REBITEM >0 or REBITEM==1 ) then 
		SelectMsg(UID, 48, -1, -1, NPC);
		else
	SelectMsg(UID, 3, -1, 45316, NPC, 45559, 503);
		end
		
	end
end


if (EVENT == 506) then
	SelectMsg(UID, 3, -1, 45316, NPC, 45559, 503);
	EXP = GetExpPercent(UID);
   	LEVEL = GetLevel(UID);
	NP = CheckLoyalty(UID);
	MONEY = HowmuchItem(UID, 900000000);
	SlotCheck = CheckGiveSlot(UID, 1)
	REBITEM = HowmuchItem(UID, 900579000);--2k kc
	if SlotCheck == false then
	else
		if(LEVEL == 83 and NP >= 10000  and EXP == 100 and MONEY>=100000000 ) then 
		RobLoyalty(UID,10000);
		GoldLose(UID, 100000000);
		GiveItem(UID,900579000,1);
		SelectMsg(UID, 48, -1, -1, NPC);
		else
		SelectMsg(UID, 2, -1, 45317, NPC, 45555, -1);
		end
		
	end
end