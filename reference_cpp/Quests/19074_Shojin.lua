-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC =19074;

if (EVENT == 100) then
	SelectMsg(UID, 20, -1, 845, NPC, 4520, 101, 4521, 102, 4526, 103,4522, 104, 4523, 105);--40368,125,
end

if EVENT == 104 then
   SelectMsg(UID, 19, -1, 848, NPC, 4524, 106, 4525, -1);	
end

if EVENT == 105 then
   SelectMsg(UID, 21, -1, -1, NPC, -1, -1 );	
end

if EVENT == 106 then
SlotCheck = CheckGiveSlot(UID, 2)
		if SlotCheck == false then
			SelectMsg(UID, 2,-1,8898,NPC,10,-1)
        else
   SelectMsg(UID, 18, -1, -1, NPC);	
end
end
if (EVENT == 101) then
	SelectMsg(UID, 2, -1, 846, NPC, 2003, 100);
end

if (EVENT == 102) then
	SelectMsg(UID, 2, -1, 847, NPC, 2003, 100);
end

if (EVENT == 103) then
	SelectMsg(UID, 2, -1, 849, NPC, 4527, 200, 4528, 201);
end

if (EVENT == 200) then
	Loyalty = CheckLoyalty(UID);
	Money = HowmuchItem(UID, 900000000);
	if (Loyalty < 100) then
		SelectMsg(UID, 2, -1, 852, NPC, 18, 202);
	elseif (Money < 1000000) then
		SelectMsg(UID, 2, -1, 851, NPC, 18, 203);
	else
		RobLoyalty(UID, 100);
		GoldLose(UID, 1000000);
		GiveItem(UID, 389132000, 1);
	end
end

if (EVENT == 202) then
	ShowMap(UID, 338);
end

if (EVENT == 203) then
	ShowMap(UID, 336);
end

if (EVENT == 201) then
	SelectMsg(UID, 2, -1, 854, NPC, 10, 168);
end

if (EVENT == 125) then
	SelectMsg(UID, 3, -1, 44228, NPC, 40365, 126,40369,128,40367,135);
end

if (EVENT == 126) then
	SelectMsg(UID, 2, -1, 44218, NPC,40366,127,13,-1 );
end

if (EVENT == 127) then
	SlotCheck = CheckGiveSlot(UID, 4)
		if SlotCheck == false then
			SelectMsg(UID, 2,-1,8898,NPC,10,-1)
        else
	NP = CheckLoyalty(UID);
		if (NP < 1000) then
			SelectMsg(UID, 2, -1, 44219, NPC,10,-1);	 
	    else
			EVENT = 137
		end
	end
end

if (EVENT == 137) then
	SlotCheck = CheckGiveSlot(UID, 4)
		if SlotCheck == false then
			SelectMsg(UID, 2,-1,8898,NPC,10,-1)
        else
	NP = CheckLoyalty(UID);
		if (NP < 10000) then
			SelectMsg(UID, 2, -1, 44219, NPC,10,-1);	 
	    else
	Roll = RollDice(UID, 35) 
		if Roll == 0 then
			GiveItem(UID,810664000,1);
		end
		if Roll == 1 then
			GiveItem(UID,810665000,1);
		end
		if Roll == 2 then
			GiveItem(UID,810666000,1);
		end
		if Roll == 3 then
			GiveItem(UID,810667000,1);
		end
		if Roll == 4 then
			GiveItem(UID,810664000,1);
			GiveItem(UID,810665000,1);
		end
		if Roll == 5 then
			GiveItem(UID,810665000,1);
			GiveItem(UID,810666000,1);
		end
		if Roll == 6 then
			GiveItem(UID,810666000,1);
			GiveItem(UID,810667000,1);
		end
		if Roll == 7 then
			GiveItem(UID,810664000,1);
			GiveItem(UID,810666000,1);
		end
		if Roll == 8 then
			GiveItem(UID,810665000,1);
			GiveItem(UID,810667000,1);
		end
		if Roll == 9 then
			GiveItem(UID,810664000,1);
			GiveItem(UID,810667000,1);
		end
		if Roll == 10 then
			GiveItem(UID,810664000,1);
			GiveItem(UID,810665000,1);
			GiveItem(UID,810666000,1);
		end
		if Roll == 11 then
			GiveItem(UID,810665000,1);
			GiveItem(UID,810666000,1);
			GiveItem(UID,810667000,1);
		end
		if Roll == 12 then
			GiveItem(UID,810664000,1);
			GiveItem(UID,810665000,1);
			GiveItem(UID,810666000,1);
			GiveItem(UID,810667000,1);
		end
		if Roll == 13 then
			GiveItem(UID,810664000,1);
			GiveItem(UID,389070000,50);
		end
		if Roll == 14 then
			GiveItem(UID,810665000,1);
			GiveItem(UID,389070000,50);
		end
		if Roll == 15 then
			GiveItem(UID,810666000,1);
			GiveItem(UID,389070000,50);
		end
		if Roll == 16 then
			GiveItem(UID,810667000,1);
			GiveItem(UID,389070000,50);
		end
		if Roll == 17 then
			GiveItem(UID,810664000,1);
			GiveItem(UID,389130000,50);
		end
		if Roll == 18 then
			GiveItem(UID,810665000,1);
			GiveItem(UID,389130000,50);
		end
		if Roll == 19 then
			GiveItem(UID,810666000,1);
			GiveItem(UID,389130000,50);
		end
		if Roll == 20 then
			GiveItem(UID,810667000,1);
			GiveItem(UID,389130000,50);
		end
		if Roll == 21 then
			GiveItem(UID,810664000,1);
			GiveItem(UID,389130000,50);
			GiveItem(UID,389070000,50);
		end
		if Roll == 22 then
			GiveItem(UID,810665000,1);
			GiveItem(UID,389130000,50);
			GiveItem(UID,389070000,50);
		end
		if Roll == 23 then
			GiveItem(UID,810666000,1);
			GiveItem(UID,389130000,50);
			GiveItem(UID,389070000,50);
		end
		if Roll == 24 then
			GiveItem(UID,810667000,1);
			GiveItem(UID,389130000,50);
			GiveItem(UID,389070000,50);
		end
		if Roll == 25 then
			GiveItem(UID,389130000,50);
			GiveItem(UID,389070000,50);
		end
		if Roll == 26 then
			GiveItem(UID,389070000,50);
		end
		if Roll == 27 then
			GiveItem(UID,389130000,50);
		end
		if Roll == 28 then
			GiveItem(UID,810664000,1);
		end
		if Roll == 29 then
			GiveItem(UID,810665000,1);
		end
		if Roll == 30 then
			GiveItem(UID,810666000,1);
		end
		if Roll == 31 then
			GiveItem(UID,810667000,1);
		end
		if Roll == 32 then
			GiveItem(UID,810664000,1);
		end
		if Roll == 33 then
			GiveItem(UID,810665000,1);
		end
		if Roll == 34 then
			GiveItem(UID,810666000,1);
		end
		if Roll == 35 then
			GiveItem(UID,810667000,1);
		end
			RobLoyalty(UID, 10000);
			SelectMsg(UID, 2, -1, 44219, NPC,27,-1);
		end
	end
end

if (EVENT == 128) then
	QuestStatusCheck = GetQuestStatus(UID, 1506)	
		if(QuestStatusCheck == 1) then
			EVENT = 131	
		elseif(QuestStatusCheck == 3) then
			EVENT = 132
		else
			SelectMsg(UID, 2, 1506, 44221, NPC, 40147, 129);
	end
end

if (EVENT == 129) then	
	SelectMsg(UID, 2, 1506, 44222, NPC, 22, 130,23-1);
end

if (EVENT == 130) then	
	SaveEvent(UID, 1516);
end

if (EVENT == 149) then	
	SaveEvent(UID, 1518);
end

if (EVENT == 131) then	
	SelectMsg(UID, 2, -1, 44229, NPC, 10,-1);
end

if (EVENT == 132) then
	ITEMA = HowmuchItem(UID, 379107000);
	ITEMB = HowmuchItem(UID, 810671000);
		if (ITEMA < 1 and ITEMB < 1) then 
			SelectMsg(UID, 2, -1, 44229, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 44229, NPC, 3000,133,3005,-1);
	end
end

if (EVENT == 133) then	
	SelectMsg(UID, 2, 1506, 44223, NPC, 40143, 134,27,-1);
end

if (EVENT == 134) then
	ITEMA = HowmuchItem(UID, 379107000);
	ITEMB = HowmuchItem(UID, 810671000);
		if (ITEMA < 1 and ITEMB < 1) then 
			SelectMsg(UID, 2, -1, 44229, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			GiveItem(UID,998017000,5,15);
			RobItem(UID, 379107000, 1);
			RobItem(UID, 810671000, 1);
			SaveEvent(UID, 1519);
		end
	end
end

if (EVENT == 135) then
	SelectMsg(UID, 2, -1, 44225, NPC,3000,136,3005,-1 );
end

if (EVENT == 136) then
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
	ITEM = HowmuchItem(UID, 810673000);
		if (ITEM < 1) then
			SelectMsg(UID, 2, -1, 44227, NPC,10,-1);	 
	    else
	Roll = RollDice(UID, 5) 
		if Roll == 0 then
			GiveItem(UID,810668000,2);
		end 
		if Roll == 1 then
			GiveItem(UID,810668000,3);
		end 
		if Roll == 2 then
			GiveItem(UID,810668000,4);
		end 
		if Roll == 3 then
			GiveItem(UID,810668000,5);
		end 
		if Roll == 4 then
			GiveItem(UID,810668000,6);
		end 
		if Roll == 5 then
			GiveItem(UID,810668000,7);
		end 
			RobItem(UID, 810673000, 1);
			SelectMsg(UID, 2, -1, 44226, NPC,27,-1);
		end
	end
end