--######################
--#######CYBERACS#######
--######################
--######################
--##Created By TriLogy##
--######################
local NPC = 31774;
------------------
if (EVENT == 100) then      
	SelectMsg(UID, 3, -1, 45147, NPC,45202,101);
	--SelectMsg(UID, 3, -1, 45147, NPC,); --iptal edildi.
end

if (EVENT == 101) then 
	SelectMsg(UID, 3, -1, 45136, NPC,45162,102,64,103);
end

if (EVENT == 102) then 
	SelectMsg(UID, 3, -1, 45136, NPC,45163,103,45164,104,45165,105);
end

if (EVENT == 103) then 
	--SelectMsg(UID, 3, -1, 45144, NPC,10,-1);
	SelectMsg(UID, 58, -1, -1, NPC);
end

if (EVENT == 104) then 
	SelectMsg(UID, 3, -1, 45145, NPC,10,-1);
end

if (EVENT == 105) then 
	SelectMsg(UID, 3, -1, 45146, NPC,10,-1);
end

if (EVENT == 106) then
	Check = isRoomForItem(UID, 1);
		if (Check == 1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			SelectMsg(UID, 2, -1, 45136, NPC, 22, 107, 23, -1);
	end
end
if (EVENT == 107) then --fonksiyon eklencek
	IsTakeToday = GetUserDailyOp(UID,9);
		if (IsTakeToday == 9) then
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			;
	    else
			--GiveItem(UID, 900790000, 5,1);
			GiveItem(UID, 810554924, 5,1);
	    end
	 else
		SelectMsg(UID, 2, -1, 9280, NPC, 10, -1);
	end
end

