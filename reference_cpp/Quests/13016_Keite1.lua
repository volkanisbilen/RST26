-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 13016;

if (EVENT == 500) then
SelectMsg(UID, 3, -1, 4834, NPC ,4263, 2100, 4264, 2200, 4265, 2300,7493,9000,4337,2400,4263,300);
end

if (EVENT == 2100) then 
	SelectMsg(UID, 9, -1,NPC);	
end

if (EVENT == 9000) then 
	SelectMsg(UID, 28, -1,NPC);	
end

if (EVENT == 2200) then
	ITEMA = HowmuchItem(UID, 800090000);
		if (ITEMA > 0) then
			SelectMsg(UID, 9, -1,NPC);	
		else
			SelectMsg(UID, 2, -1, 4833, NPC, 27, 1005);
	end
end	

if (EVENT == 2300) then
	SelectMsg(UID, 14, -1, NPC);
end

if (EVENT == 2400) then
	SelectMsg(UID, 3, -1, 820, NPC,4344,3000,4345,3001,7551,3002,3019);
end

if (EVENT == 3000) then
	MagicBag = HowmuchItem(UID, 800440000);
		if (MagicBag > 0) then
			EVENT = 3007
		else
			SelectMsg(UID, 2, -1, 823, NPC, 10, -1)
	end
end

if (EVENT == 3007) then
	MagicBag = HowmuchItem(UID, 800440000);
		if (MagicBag == 0) then
			SelectMsg(UID, 2, -1, 823, NPC, 10, -1)
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 800440000, 1);
			GiveItem(UID, 700011001, 1,30);
			SelectMsg(UID, 2, -1, 9273, NPC, 10, -1)
		end
	end
end

if (EVENT == 3001) then
	ITEM = HowmuchItem(UID, 800450000);
		if (ITEM > 0) then
			EVENT = 3010
		else
			SelectMsg(UID, 2, -1, 824, NPC, 10, -1)
	end
end

if (EVENT == 3010) then
	ITEM = HowmuchItem(UID, 800450000);
	if (ITEM == 0) then
		SelectMsg(UID, 2, -1, 824, NPC, 10, -1)
	else
		SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		else
			RobItem(UID, 800450000, 1);
			GiveItem(UID, 700012000, 1,30);
			SelectMsg(UID, 2, -1, 9273, NPC, 10,-1);
		end
	end
end

if (EVENT == 3002) then
	SelectMsg(UID, 3, -1, 820, NPC,7552,3003,7553,3005);
end

if (EVENT == 3003) then
	HP = HowmuchItem(UID, 810115000);
		if (HP > 0) then
			EVENT = 3004
		else
			SelectMsg(UID, 2, -1, 10365, NPC, 10, -1);
	end
end

if (EVENT == 3004) then
	HP = HowmuchItem(UID, 810115000);
		if (HP == 0) then
			SelectMsg(UID, 2, -1, 10365, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810115000, 1);	
			GiveItem(UID, 810117000, 1,30);
			SelectMsg(UID, 2, -1, 9273, NPC, 10,-1);
		end
	end
end

if (EVENT == 3005) then
	MP = HowmuchItem(UID, 810116000);
		if (MP > 0) then
			EVENT = 3006
		else
			SelectMsg(UID, 2, -1, 10366, NPC, 10, -1);
	end
end

if (EVENT == 3006) then
	MP = HowmuchItem(UID, 810116000);
		if (MP == 0) then
			SelectMsg(UID, 2, -1, 10366, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810116000, 1);	
			GiveItem(UID, 810118000, 1,30);
			SelectMsg(UID, 2, -1, 9273, NPC, 10,-1);
		end
	end
end

----------PET GOREV-------------------
