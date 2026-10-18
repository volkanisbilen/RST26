-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- 29191_Santa Quest Finish...
-- =============================================
local NPC = 29191;

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 12030, NPC,40648,105,40454,120);
end

if (EVENT == 105) then
	SelectMsg(UID, 3, -1, 44612, NPC,40649,106,40650,107,40651,108,40652,109,40653,110,40654,111,40655,112,40656,113,40657,118);
end

if (EVENT == 106) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 5) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 914008000,5);
			GiveItem(UID, 910249000,3);
    	end
    end
end

if (EVENT == 107) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 5) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 914008000,5);
			GiveItem(UID, 910250000,3);
    	end
    end
end

if (EVENT == 108) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 5) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 914008000,5);
			GiveItem(UID, 910251000,3);
    	end
    end
end

if (EVENT == 109) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 5) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 914008000,5);
			GiveItem(UID, 910252000,3);
    	end
    end
end

if (EVENT == 110) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 20) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 914008000,20);
			GiveItem(UID, 919731147,1);
    	end
    end
end

if (EVENT == 111) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 40) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 914008000,40);
			GiveItem(UID, 379181000,10);
    	end
    end
end

if (EVENT == 112) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 50) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 914008000,50);
			GiveItem(UID, 700011000,1,7);
    	end
    end
end

if (EVENT == 113) then
	WINTER = HowmuchItem(UID, 914008000);
	if (WINTER < 60) then
		SelectMsg(UID, 2, -1, 44330, NPC, 10,-1);
	else
		SelectMsg(UID, 3, -1, 12060, NPC, 7786,114,7787,115,7788,116,7789,117);
	end
end

if (EVENT == 114) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 60) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
		
		else
			RobItem(UID, 914008000, 60);
			GiveItem(UID, 810510902, 1,7);
		end
    end
end

if (EVENT == 115) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 60) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 914008000, 60);
			GiveItem(UID, 810510903, 1,7);
		end
    end
end

if (EVENT == 116) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 60) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 914008000, 60);
			GiveItem(UID, 810510905, 1,7);
		end
    end
end

if (EVENT == 117) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 60) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 914008000, 60);
			GiveItem(UID, 810510904, 1,7);
		end
    end
end

if (EVENT == 118) then
	WINTER = HowmuchItem(UID, 914008000);
		if (WINTER < 100) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 914008000,100);
			GiveItem(UID, 910941725,1,15);
    	end
    end
end

if (EVENT == 120) then
	SelectMsg(UID, 3, -1, 44329, NPC,40455,121,40456,122,40457,123);
end

if (EVENT == 121) then
	WINTER = HowmuchItem(UID, 379181000);
		if (WINTER < 10) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 379181000,10);
			GiveItem(UID, 930520722,1);
    	end
    end
end

if (EVENT == 122) then
	WINTER = HowmuchItem(UID, 379181000);
		if (WINTER < 20) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 379181000,20);
			GiveItem(UID, 930530723,1);
    	end
    end
end

if (EVENT == 123) then
	WINTER = HowmuchItem(UID, 379181000);
		if (WINTER < 30) then
			SelectMsg(UID, 2, -1, 44330, NPC, 10, -1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
	    else
			RobItem(UID, 379181000,30);
			GiveItem(UID, 930540724,1);
    	end
    end
end