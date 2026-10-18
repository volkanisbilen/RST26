-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31718;

if EVENT == 100 then
    SelectMsg(UID, 2, -1, 44595, NPC, 40772,200,40752,300,40753,400,40644, 110,27,-1);
end

if (EVENT == 2000) then
	Check = isRoomForItem(UID, 914057000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			SelectMsg(UID, 2, -1, 44694, NPC, 22, 201, 23, -1);
	end
end

if (EVENT == 201) then
	IsTakeToday = GetUserDailyOp(UID,9);
		if (IsTakeToday == 1) then
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			;
	    else
			GiveItem(UID, 914057000, 1,1);
	    end
	 else
		SelectMsg(UID, 2, -1, 44695, NPC, 10, -1);
	end
end

if EVENT == 400 then
	SelectMsg(UID, 3, -1, 44689, NPC, 10, -1);
end

if EVENT == 110 then
    SelectMsg(UID, 3, -1, 12075, NPC, 40636,115,40637,120,40638,125,40639,130,40640,135,40641,140,40642,145,40643,150);
end

if (EVENT == 300) then
    SelectMsg(UID, 57, -1, -1, NPC);
end

if (EVENT == 115) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 2) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 12075, NPC, 8518, 116);
	end
end

if (EVENT == 116) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 2) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
        else
			RobItem(UID, 810977000,2);
			GiveItem(UID,900144023);
			SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
		end
	end
end

if (EVENT == 120) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 4) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 12075, NPC, 8518, 121);
	end
end

if (EVENT == 121) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 4) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 2)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810977000,4);
			GiveItem(UID,900144023);
			GiveItem(UID,900144023);
			SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
		end
	end
end

if (EVENT == 125) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 5) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 12075, NPC, 8518, 126);
	end
end

if (EVENT == 126) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 5) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810977000,5);
			GiveItem(UID,379154000,1);
			SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
		end
	end
end

if (EVENT == 130) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 8) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 12075, NPC, 8518, 131);
	end
end

if (EVENT == 131) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 8) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810977000,8);
			GiveItem(UID,379156000,1);
			SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
		end
	end
end

if (EVENT == 135) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 10) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 12075, NPC, 8518, 136);
	end
end

if (EVENT == 136) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 10) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else
			RobItem(UID, 810977000,10);
			GiveItem(UID,810636000,1);
			SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
		end
	end
end

if (EVENT == 140) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 15) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 12075, NPC, 8518, 141);
	end
end

if (EVENT == 141) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 15) then
				SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
				
		else
	Roll = RollDice(UID, 5) 
		if Roll == 0 then
				GiveItem(UID,389196000,3);
				RobItem(UID, 810977000,15);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 1 then
				GiveItem(UID,389197000,3);
				RobItem(UID, 810977000,15);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 2 then
				GiveItem(UID,389198000,3);
				RobItem(UID, 810977000,15);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 3 then
				GiveItem(UID,389199000,3);
				RobItem(UID, 810977000,15);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 4 then
				GiveItem(UID,389201000,3);
				RobItem(UID, 810977000,15);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 5 then
				GiveItem(UID,389205000,3);
				RobItem(UID, 810977000,15);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end
		end 
	end
end

if (EVENT == 145) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 20) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 12075, NPC, 8518, 146);
	end
end

if (EVENT == 146) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 20) then
				SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
       
        else
	Roll = RollDice(UID, 6) 
		if Roll == 0 then
				GiveItem(UID,389160000,5);
				RobItem(UID, 810977000,20);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 1 then
				GiveItem(UID,389161000,5);
				RobItem(UID, 810977000,20);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 2 then
				GiveItem(UID,389162000,5);
				RobItem(UID, 810977000,20);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 3 then
				GiveItem(UID,389163000,5);
				RobItem(UID, 810977000,20);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 4 then
				GiveItem(UID,389164000,5);
				RobItem(UID, 810977000,20);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 5 then
				GiveItem(UID,389165000,5);
				RobItem(UID, 810977000,20);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
		if Roll == 6 then
				GiveItem(UID,389166000,5);
				RobItem(UID, 810977000,20);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
			end
		end
	end
end

if (EVENT == 150) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 30) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
			SelectMsg(UID, 2, -1, 12075, NPC, 8518, 151);
	end
end

if (EVENT == 151) then
	ITEMA = HowmuchItem(UID, 810977000);
		if (ITEMA < 30) then
			SelectMsg(UID, 2, -1, 44595, NPC, 10,-1);
		else
	SlotCheck = CheckGiveSlot(UID, 3)
		if SlotCheck == false then
				
        else
	Roll = RollDice(UID, 6) 
		if Roll == 0 then
				GiveItem(UID,810341000,2);
				GiveItem(UID,389160000,1);
				GiveItem(UID,389196000,1);
				RobItem(UID, 810977000,30);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 1 then
				GiveItem(UID,810341000,2);
				GiveItem(UID,389161000,1);
				GiveItem(UID,389197000,1);
				RobItem(UID, 810977000,30);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 2 then
				GiveItem(UID,810341000,2);
				GiveItem(UID,389162000,1);
				GiveItem(UID,389198000,1);
				RobItem(UID, 810977000,30);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 3 then
				GiveItem(UID,810341000,2);
				GiveItem(UID,389163000,1);
				GiveItem(UID,389199000,1);
				RobItem(UID, 810977000,30);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 4 then
				GiveItem(UID,810341000,2);
				GiveItem(UID,389164000,1);
				GiveItem(UID,389201000,1);
				RobItem(UID, 810977000,30);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
		if Roll == 5 then
				GiveItem(UID,810341000,2);
				GiveItem(UID,389165000,1);
				GiveItem(UID,389205000,1);
				RobItem(UID, 810977000,30);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
		if Roll == 6 then
				GiveItem(UID,810341000,2);
				GiveItem(UID,389166000,1);
				GiveItem(UID,389198000,1);
				RobItem(UID, 810977000,30);
				SelectMsg(UID, 2, -1, 9110, NPC, 10, -1);
				end 
			end
		end
	end
end