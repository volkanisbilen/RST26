-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 23523;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 50000, NPC, 51317, 1, 59594, 2, 50121, 3);
end

if (EVENT == 1) then
	SelectMsg(UID, 3, -1, 50001, NPC, 51318, 4, 51319, 5, 51320, 6, 51321, 7, 51322, 8);
end

if (EVENT == 2) then
	SelectMsg(UID, 3, -1, 50003, NPC, 51312, 9, 51313, 10, 51314, 11, 51315, 12, 51316, 13);
end

if (EVENT == 3) then
	SelectMsg(UID, 3, -1, 44828, NPC, 17203, 14, 17204, 15, 17205, 16, 17206, 17, 17207, 18);
end

if (EVENT == 4) then
	Coin = HowmuchItem(UID, 900000000);  
	if (Coin < 20000000) then
		SelectMsg(UID, 2, -1, 50002, NPC, 10, -1);
	else
		GiveCash(UID, 100);
		GoldLose(UID, 20000000);
	end
end

if (EVENT == 5) then
	Coin = HowmuchItem(UID, 900000000);  
	if (Coin < 40000000) then
		SelectMsg(UID, 2, -1, 50002, NPC, 10, -1);
	else
		GiveCash(UID, 350);
		GoldLose(UID, 40000000);
	end	
end

if (EVENT == 6) then
	Coin = HowmuchItem(UID, 900000000);  
	if (Coin < 60000000) then
		SelectMsg(UID, 2, -1, 50002, NPC, 10, -1);
	else
		GiveCash(UID, 500);
		GoldLose(UID, 60000000);
	end
end

if (EVENT == 7) then
	Coin = HowmuchItem(UID, 900000000);  
	if (Coin < 110000000) then
		SelectMsg(UID, 2, -1, 50002, NPC, 10, -1);
	else
		GiveCash(UID, 1000);
		GoldLose(UID, 110000000);
	end
end

if (EVENT == 8) then
	Coin = HowmuchItem(UID, 900000000);  
	if (Coin < 220000000) then
		SelectMsg(UID, 2, -1, 50002, NPC, 10, -1);
	else
		GiveCash(UID, 2000);
		GoldLose(UID, 220000000);
	end
end

-------------------------------------------------------------

if (EVENT == 9) then
	Loyalty = CheckLoyalty(UID, 900000000);  
	if (Loyalty < 20000) then
		SelectMsg(UID, 2, -1, 50004, NPC, 10, -1);
	else
		GiveCash(UID, 100);
		RobLoyalty(UID, 20000);
	end
end

if (EVENT == 10) then
	Loyalty = CheckLoyalty(UID, 900000000);  
	if (Loyalty < 40000) then
		SelectMsg(UID, 2, -1, 50004, NPC, 10, -1);
	else
		GiveCash(UID, 350);
		RobLoyalty(UID, 40000);
	end	
end

if (EVENT == 11) then
	Loyalty = CheckLoyalty(UID, 900000000);  
	if (Loyalty < 60000) then
		SelectMsg(UID, 2, -1, 50004, NPC, 10, -1);
	else
		GiveCash(UID, 500);
		RobLoyalty(UID, 60000);
	end
end

if (EVENT == 12) then
	Loyalty = CheckLoyalty(UID, 900000000);  
	if (Loyalty < 110000) then
		SelectMsg(UID, 2, -1, 50004, NPC, 10, -1);
	else
		GiveCash(UID, 1000);
		RobLoyalty(UID, 110000);
	end
end

if (EVENT == 13) then
	Loyalty = CheckLoyalty(UID, 900000000);  
	if (Loyalty < 220000) then
		SelectMsg(UID, 2, -1, 50004, NPC, 10, -1);
	else
		GiveCash(UID, 2000);
		RobLoyalty(UID, 220000);
	end
end

-------------------------------------------------------------------

if (EVENT == 14) then
	ItemCount = HowmuchItem(UID, 700082000);  
	if (ItemCount < 1) then
		SelectMsg(UID, 2, -1, 44826, NPC, 10, -1);
	else
		GiveCash(UID, 100);
		RobItem(UID, 700082000, 1);
	end
end

if (EVENT == 15) then
	ItemCount = HowmuchItem(UID, 700083000);  
	if (ItemCount < 1) then
		SelectMsg(UID, 2, -1, 44826, NPC, 10, -1);
	else
		GiveCash(UID, 250);
		RobItem(UID, 700083000, 1);
	end
end

if (EVENT == 16) then
	ItemCount = HowmuchItem(UID, 700084000);  
	if (ItemCount < 1) then
		SelectMsg(UID, 2, -1, 44826, NPC, 10, -1);
	else
		GiveCash(UID, 500);
		RobItem(UID, 700084000, 1);
	end
end

if (EVENT == 17) then
	ItemCount = HowmuchItem(UID, 700085000);  
	if (ItemCount < 1) then
		SelectMsg(UID, 2, -1, 44826, NPC, 10, -1);
	else
		GiveCash(UID, 1000);
		RobItem(UID, 700085000, 1);
	end
end

if (EVENT == 18) then
	ItemCount = HowmuchItem(UID, 700081000);  
	if (ItemCount < 1) then
		SelectMsg(UID, 2, -1, 44826, NPC, 10, -1);
	else
		GiveCash(UID, 2000);
		RobItem(UID, 700081000, 1);
	end
end