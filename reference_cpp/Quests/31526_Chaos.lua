-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31526;

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 9264, NPC, 7143, 3001, 7139, 3002, 7116, 3003, 7117, 3004, 7118, 3005,7149,3006);
end

if (EVENT == 3001) then
	SelectMsg(UID, 3, -1, 9321, NPC, 7144, 400, 7145, 401, 7146, 402, 7147, 403);
end

if (EVENT == 400) then 
	SelectMsg(UID, 3, -1, 9322, NPC, 2003, 3001, 2002, 404, 7148, 3001);
end

if (EVENT == 404) then
	SelectMsg(UID, 3, -1, 9323, NPC, 2003, 400, 2002, 405, 7148, 3001);
end

if (EVENT == 405) then
	SelectMsg(UID, 3, -1, 9324, NPC, 2003, 404, 2002, 406, 7148, 3001);
end

if (EVENT == 406) then
	SelectMsg(UID, 3, -1, 9325, NPC, 2003, 405, 2002, 407, 7148, 3001);
end

if (EVENT == 407) then
	SelectMsg(UID, 3, -1, 9326, NPC, 2003, 406, 2002, 408, 7148, 3001);
end

if (EVENT == 408) then
	SelectMsg(UID, 3, -1, 9327, NPC, 2003, 407, 2002, 409, 7148, 3001);
end

if (EVENT == 409) then
	SelectMsg(UID, 3, -1, 9328, NPC, 2003, 408, 7145, 401, 7146, 402, 7147, 403);
end

if (EVENT == 401) then
	SelectMsg(UID, 3, -1, 9329, NPC, 2003, 3001, 2002, 410, 7148, 3001);
end

if (EVENT == 410) then
	SelectMsg(UID, 3, -1, 9330, NPC, 2003, 401, 2002, 411, 7148, 3001);
end

if (EVENT == 411) then
	SelectMsg(UID, 3, -1, 9331, NPC, 2003, 410, 2002, 412, 7148, 3001);
end

if (EVENT == 412) then
	SelectMsg(UID, 3, -1, 9332, NPC, 2003, 411, 2002, 413, 7148, 3001);
end

if (EVENT == 413) then
	SelectMsg(UID, 3, -1, 9333, NPC, 2003, 412, 2002, 414, 7148, 3001);
end

if (EVENT == 414) then
	SelectMsg(UID, 3, -1, 9334, NPC, 2003, 413, 2002, 415, 7148, 3001);
end

if (EVENT == 415) then
	SelectMsg(UID, 3, -1, 9335, NPC, 2003, 414, 2002, 416, 7148, 3001);
end

if (EVENT == 416) then
	SelectMsg(UID, 3, -1, 9336, NPC, 2003, 415, 2002, 417, 7148, 3001);
end

if (EVENT == 417) then
	SelectMsg(UID, 3, -1, 9337, NPC, 2003, 416, 2002, 418, 7148, 3001);
end

if (EVENT == 418) then
	SelectMsg(UID, 3, -1, 9358, NPC, 2003, 417, 2002, 419, 7148, 3001);
end

if (EVENT == 419) then
	SelectMsg(UID, 3, -1, 9338, NPC, 2003, 418, 2002, 420, 7148, 3001);
end

if (EVENT == 420) then
	SelectMsg(UID, 3, -1, 9339, NPC, 2003, 419, 2002, 421, 7148, 3001);
end

if (EVENT == 421) then
	SelectMsg(UID, 3, -1, 9340, NPC, 2003, 420, 7144, 400, 7146, 402, 7147, 403);
end

if (EVENT == 402) then
	SelectMsg(UID, 3, -1, 9341, NPC, 2003, 3001, 2002, 422, 7148, 3001);
end

if (EVENT == 422) then
	SelectMsg(UID, 3, -1, 9342, NPC, 2003, 402, 2002, 423, 7148, 3001);
end

if (EVENT == 423) then
	SelectMsg(UID, 3, -1, 9343, NPC, 2003, 422, 2002, 424, 7148, 3001);
end

if (EVENT == 424) then
	SelectMsg(UID, 3, -1, 9344, NPC, 2003, 423, 7144, 400, 7145, 401, 7147, 403);
end

if (EVENT == 403) then
	SelectMsg(UID, 3, -1, 9345, NPC, 2003, 3001, 2002, 425, 7148, 3001);
end

if (EVENT == 425) then
	SelectMsg(UID, 3, -1, 9346, NPC, 2003, 403, 2002, 426, 7148, 3001);
end

if (EVENT == 426) then
	SelectMsg(UID, 3, -1, 9347, NPC, 2003, 425, 7144, 400, 7145, 401, 7146, 402);
end

if (EVENT == 3002) then
	Check = isRoomForItem(UID, 910246000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			SelectMsg(UID, 2, -1, 9265, NPC, 22, 300, 23, -1);
	end
end

if (EVENT == 300) then
	IsTakeToday = GetUserDailyOp(UID,1);
		if (IsTakeToday == 1) then
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			;
	    else
			GiveItem(UID, 910246000, 1,1);
	    end
	 else
		SelectMsg(UID, 2, -1, 9280, NPC, 10, -1);
	end
end

if (EVENT == 3003) then
	ITEM = HowmuchItem(UID, 810150000);
	if (ITEM > 0) then
		SelectMsg(UID, 2, -1, 9265, NPC, 4302, 200, 4303, -1);
	else
		SelectMsg(UID, 2, -1, 9266, NPC, 10, -1);
	end
end

if (EVENT == 200) then
	SelectMsg(UID, 2, -1, 9267, NPC, 65, 201, 66, -1);
end

if (EVENT == 201) then
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	Check = isRoomForItem(UID, 910041000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
	else
		RobItem(UID, 810150000, 1);
		GiveItem(UID, 910246000, 1,1);
		SelectMsg(UID, 2, -1, 9268, NPC, 20, -1);
	end
end   
end

if (EVENT == 3005) then
	SelectMsg(UID, 3, -1, 9269, NPC, 7119, 203, 7120, 204, 7249, 210);
end

if (EVENT == 203) then
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	ANCTEXT = HowmuchItem(UID, 810160000);
	ORACLE = HowmuchItem(UID, 900184000);
	if (ANCTEXT < 1) then
		SelectMsg(UID, 2, -1, 9270, NPC, 10, -1);
	else
	if (ORACLE < 1) then
	SelectMsg(UID, 2, -1, 9269, NPC, 10, -1);
	else
	Roll = RollDice(UID, 4) 
	if Roll == 0 then
    GiveItem(UID,900182674,1,1);
    end
	if Roll == 1 then
    GiveItem(UID,900182675,1,1);
    end
	if Roll == 2 then
    GiveItem(UID,900179670,1,1);
    end
	if Roll == 3 then
    GiveItem(UID,900181671,1,1);
    end
	if Roll == 4 then
    GiveItem(UID,900180669,1,1);
    end

        RobItem(UID, 810160000, 1);
		RobItem(UID, 900184000, 1);
		SelectMsg(UID, 2, -1, 9273, NPC, 20, -1);
	end   
end
end
end

if (EVENT == 204) then
	PARA = HowmuchItem(UID, 900000000);
	Level = CheckLevel(UID);
	if (PARA >= Level * 6000  ) then
		SelectMsg(UID, 2, -1, 9275, NPC, 10, 205, 4005, -1);
	else
		SelectMsg(UID, 2, -1, 9274, NPC, 10, -1);
	end   
end

if (EVENT == 205) then	
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
ORACLE = HowmuchItem(UID, 900184000);
if (ORACLE < 1) then
SelectMsg(UID, 2, -1, 9269, NPC, 10, -1);
else
	Roll = RollDice(UID, 4) 
	if Roll == 0 then
    GiveItem(UID,900183678,1,1);
    end
	if Roll == 1 then
    GiveItem(UID,900183679,1,1);
    end
	if Roll == 2 then
    GiveItem(UID,900183680,1,1);
    end
	if Roll == 3 then
    GiveItem(UID,900183681,1,1);
    end
	if Roll == 4 then
    GiveItem(UID,900183682,1,1);
    end
        RobItem(UID, 900184000, 1);
		GoldLose(UID,6000 * GetLevel(UID));
		SelectMsg(UID, 2, -1, 9273, NPC, 10, -1); 
end
end
end


if (EVENT == 210) then
    ORACLE = HowmuchItem(UID, 900184000);
	DOCUMENT = HowmuchItem(UID, 810161000);
	if (ORACLE < 1) then
	SelectMsg(UID, 2, -1, 9269, NPC, 10, -1);
	else
	if (DOCUMENT < 1) then
	SelectMsg(UID, 2, -1, 9269, NPC, 10, -1);
	else
		SelectMsg(UID, 2, -1, 9269, NPC, 10, 211,4005,-1);
	end
	end
	end
	
	
if (EVENT == 211) then	
SelectMsg(UID, 2, -1, 11038, NPC, 7993, 212,7993,213);
end

if (EVENT == 212) then
SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
         else
RobItem(UID, 900184000, 1);
RobItem(UID, 810161000, 1);
GiveItem(UID, 910248763, 1,1);
SelectMsg(UID, 2, -1, 9273, NPC, 10, -1); 
end
end

if (EVENT == 213) then
SlotCheck = CheckGiveSlot(UID, 2)
     if SlotCheck == false then
       
         else
RobItem(UID, 900184000, 1);
RobItem(UID, 810161000, 1);
GiveItem(UID, 910248763, 1,1);
SelectMsg(UID, 2, -1, 9273, NPC, 10, -1); 
end
end

if (EVENT == 3004) then
	SelectMsg(UID, 3, -1, 9277, NPC, 7121, 206, 7122, 207, 7123, 208, 7124, 209, 7135, 500);
end

if (EVENT == 206) then
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	KingWingTime = GetUserDailyOp(UID, 4);
	King = isKing(UID);
	if (King) then
		if (KingWingTime == 1) then
			Check = isRoomForItem(UID, 910041000);
			if (Check == -1) then
				SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
			Nation = CheckNation(UID);
			else
				if (Nation == 1) then
					GiveItem(UID, 900177663, 1,30);
					SelectMsg(UID, 2, -1, 9279, NPC, 20, -1);
				else
					GiveItem(UID, 900177663, 1,30);
					SelectMsg(UID, 2, -1, 9279, NPC, 20, -1);
				end
			end
		else
			SelectMsg(UID, 2, -1, 9280, NPC, 10, -1);
		end
	else
		SelectMsg(UID, 2, -1, 9279, NPC, 10, -1);
	end
end
end

if (EVENT == 207) then
	--KeeperKiller = isKeeperKiller(UID);
	KeeperTime = GetUserDailyOp(UID, 7);
	if (KeeperTime == 1) then
		--if (KeeperKiller) then
		Check = isRoomForItem(UID, 910041000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			--GiveItem(UID, 900182674, 1)
			SelectMsg(UID, 2, -1, 9282, NPC, 20, -1);
		end
	--else
		--SelectMsg(UID, 2, -1, 9278, NPC, 10, -1);
	--end
	else
		SelectMsg(UID, 2, -1, 9280, NPC, 10 ,-1);
	end
end

if (EVENT == 208) then
	--Captain1Killer = isCaptain1Killer(UID);
	Captain1Time = GetUserDailyOp(UID, 5);
	if (Captain1Time == 1) then
		--if (Captain1Killer) then
		Check = isRoomForItem(UID, 910041000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			--GiveItem(UID, 900182674, 1)
			SelectMsg(UID, 2, -1, 9283, NPC, 20, -1);
		end
	--else
		--SelectMsg(UID, 2, -1, 9278, NPC, 10, -1);
	--end
	else
		SelectMsg(UID, 2, -1, 9280, NPC, 10 ,-1);
	end
end

if (EVENT == 209) then
	--Captain2Killer = isCaptain1Killer(UID);
	Captain2Time = GetUserDailyOp(UID, 5);
	if (Captain2Time == 1) then
		--if (Captain2Killer) then
		Check = isRoomForItem(UID, 910041000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			--GiveItem(UID, 900182674, 1)
			SelectMsg(UID, 2, -1, 9283, NPC, 20, -1);
		end
	--else
		--SelectMsg(UID, 2, -1, 9278, NPC, 10, -1);
	--end
	else
		SelectMsg(UID, 2, -1, 9280, NPC, 10 ,-1);
	end
end

if (EVENT == 500) then
	SelectMsg(UID, 2, -1, 9307, NPC, 10, 501);
end

if (EVENT == 501) then
PersonelRank = GetUserDailyOp(UID, 3);
Rank = GetUserDailyOp(UID, 2);
	if (PersonelRank == 1) then
		RequestPersonalRankReward(UID);
		SelectMsg(UID, 2, -1, 21997, NPC, 20, -1)
	elseif (Rank == 1) then
		RequestReward(UID);
		SelectMsg(UID, 2, -1, 21997, NPC, 20, -1)
	else
		SelectMsg(UID, 2, -1, 9280, NPC, 10 ,-1);
	end
end

if (EVENT == 3006) then
	SelectMsg(UID, 2, -1, 9349, NPC, 7120, 350);
end

if (EVENT == 350) then
SlotCheck = CheckGiveSlot(UID, 1)
     if SlotCheck == false then
       
         else
	CHAOSEMBLEM = HowmuchItem(UID, 900106000);
    if (CHAOSEMBLEM < 5) then	
	SelectMsg(UID, 2, -1, 9348, NPC, 10, -1);
	else
	Roll = RollDice(UID, 4) 
	if Roll == 0 then
    GiveItem(UID,900183678,1,1);
    end
	if Roll == 1 then
    GiveItem(UID,900183679,1,1);
    end
	if Roll == 2 then
    GiveItem(UID,900183680,1,1);
    end
	if Roll == 3 then
    GiveItem(UID,900183681,1,1);
    end
	if Roll == 4 then
    GiveItem(UID,900183682,1,1);
    end
		RobItem(UID, 900106000, 5);
		SelectMsg(UID, 2, -1, 9273, NPC, 20, -1);
end
end
end