-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 11610;

if (EVENT == 240) then
	SelectMsg(UID, 3, -1, 4250, NPC, 4150, 100, 4151, 360, 4154, -1);
end

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 6421, NPC, 4333, 280, 4334, 110, 4335, 120, 4328, 350, 4154, -1);
end

if (EVENT == 110) then
	GetClanRank = CheckKnight(UID)
	if (GetClanRank == 2) then
		GetClanGrade = CheckClanGrade(UID)
		if (GetClanGrade == 1) then
			Check = isClanLeader(UID)
			if (Check) then
				SelectMsg(UID, 3, -1, 6420, NPC, 4157, 111, 4158, 114, 4159, 115);
			else
				SelectMsg(UID, 2, -1, 4264, NPC, 10, -1);
			end
		else
			SelectMsg(UID, 2, -1, 6424, NPC, 10, -1);
		end
	else
		SelectMsg(UID, 2, -1, 6443, NPC, 10, -1);
	end
end

if (EVENT == 111) then
	SelectMsg(UID, 2, -1, 6426, NPC, 4160, 112, 27, -1);
end

if (EVENT == 112) then
	SelectMsg(UID, 2, -1, 6427, NPC, 4160, 113, 27, -1);
end

if (EVENT == 113) then
	SelectMsg(UID, 2, -1, 6428, NPC, 27, -1);
end

if (EVENT == 114) then
	SelectMsg(UID, 4, 22, 6429, NPC, 4161, 116, 4162, -1);
end

if (EVENT == 115) then
	ZoneChangeClan(UID, 93, 63, 474)
end

if (EVENT == 116) then
	ItemA = HowmuchItem(UID, 900000000);
	ItemB = HowmuchItem(UID, 389221000);
	if (ItemA < 10000000) then
		SelectMsg(UID, 2, -1, 6430, NPC, 10, -1);
	elseif (ItemB == 0) then
		SelectMsg(UID, 2, -1, 6431, NPC, 18, 244);
	else
		ShowEffect(UID, 300391);
		RobItem(UID, 389221000, 1);
		GoldLose(UID, 10000000);
		PromoteKnight(UID, 3);
		SelectMsg(UID, 2, -1, 6432, NPC, 10, -1);
	end
end

if (EVENT == 244) then
	ShowMap(UID, 667);
end

if (EVENT == 120) then
	GetClanRank = CheckKnight(UID)
	if (GetClanRank == 7) then
		Check = isClanLeader(UID)
		if (Check) then
			SelectMsg(UID, 3, -1, 6433, NPC, 4157, 121, 4158, 124, 4159, 125);
		else
			SelectMsg(UID, 2, -1, 4264, NPC, 10, -1);
		end
	else
		SelectMsg(UID, 2, -1, 6434, NPC, 10, -1);
	end
end

if (EVENT == 121) then
	SelectMsg(UID, 2, -1, 6435, NPC, 4160, 122, 27, -1);
end

if (EVENT == 122) then
	SelectMsg(UID, 2, -1, 6436, NPC, 4160, 123, 27, -1);
end

if (EVENT == 123) then
	SelectMsg(UID, 2, -1, 6437, NPC, 27, -1);
end

if (EVENT == 124) then
	SelectMsg(UID, 4, 23, 6438, NPC, 4161, 126, 4162, -1);
end

if (EVENT == 125) then
   ZoneChangeClan(UID, 94, 110, 20)
end

if (EVENT == 126) then
	ItemA = HowmuchItem(UID, 900000000);
	ItemB = HowmuchItem(UID, 389222000); 
	if (ItemA < 100000000) then
		SelectMsg(UID, 2, -1, 6439, NPC, 10, -1);
	elseif (ItemB == 0) then
		SelectMsg(UID, 2, -1, 6440, NPC, 18, 245);
	else
		ShowEffect(UID, 300391);
		RobItem(UID, 389222000, 1);
		GoldLose(UID, 100000000);
		PromoteKnight(UID, 8);
		SelectMsg(UID, 2, -1, 6441, NPC, 10, -1);
	end
end

if (EVENT == 245) then
	ShowMap(UID, 668);
end

if (EVENT == 280) then
	GetClanRank = CheckKnight(UID)
	if (GetClanRank == 1) then
		GetClanGrade = CheckClanGrade(UID)
		if (GetClanGrade < 4) then
			Check = isClanLeader(UID)
			if (Check) then
				SelectMsg(UID, 3, -1, 4265, NPC, 4157, 281, 4158, 286, 4159, 285);
			else
				SelectMsg(UID, 2, -1, 4264, NPC, 10, -1);
			end
		else
			SelectMsg(UID, 2, -1, 4263, NPC, 10, -1);
		end
	else
		SelectMsg(UID, 2, -1, 6442, NPC, 10, -1);
	end
end

if (EVENT == 281) then
	SelectMsg(UID, 2, -1, 4266, NPC, 4160, 282, 27, -1);
end

if (EVENT == 282) then
	SelectMsg(UID, 2, -1, 4267, NPC, 4160, 283, 27, -1);
end

if (EVENT == 283) then
	SelectMsg(UID, 2, -1, 4268, NPC, 27, -1);
end

if (EVENT == 285) then
	ZoneChangeClan(UID, 54, 150, 150, 50)
end

if (EVENT == 286) then
	SelectMsg(UID, 4, 21, 4272, NPC, 4161, 287, 4162, -1);
end

if (EVENT == 287) then
	ItemA = HowmuchItem(UID, 900000000);
	ItemB = HowmuchItem(UID, 910045000);
	if (ItemA < 10000000) then
		SelectMsg(UID, 2, -1, 4270, NPC, 10, -1);
	elseif (ItemB == 0) then
		SelectMsg(UID, 2, -1, 4271, NPC, 18, 242);
	else
		ShowEffect(UID, 300391);
		RobItem(UID , 910045000, 1);
		GoldLose(UID, 10000000);
		PromoteKnight(UID, 2);
	end
end

if (EVENT == 242) then
	ShowMap(UID, 422);
end

if (EVENT == 360) then
	Loyalty = CheckLoyalty(UID)
	if (Loyalty == 0 or Loyalty == 100) then
		SelectMsg(UID, 3, -1, 4257, NPC, 4152, 361, 4153, 363, 4154, -1);
	else
		SelectMsg(UID, 2, -1, 4256, NPC, 10, -1);
	end
end

if (EVENT == 361) then
	ITEM_COUNT = HowmuchItem(UID, 900000000);
	NATION = CheckLoyalty(UID)
	if (ITEM_COUNT < 1500000) then
		SelectMsg(UID, 2, 19, 4260, NPC, 10, -1);
	elseif(NATION == 0 or NATION == 100) then
	    SelectMsg(UID, 4, 19, 4258, NPC, 22, 362, 23, -1);
	else
		SelectMsg(UID, 2, -1, 4256, NPC, 10, -1);
	end
end

if (EVENT == 362) then 
	ITEM_COUNT = HowmuchItem(UID, 900000000);
	NATION = CheckLoyalty(UID);
	if (ITEM_COUNT < 1500000) then
		SelectMsg(UID, 2, 19, 4260, NPC, 10, -1);
    elseif(NATION == 0 or NATION == 100) then
		GoldLose(UID, 1500000);
		GiveLoyalty(UID, 500);
	else
		SelectMsg(UID, 2, 19, 4256, NPC, 10, -1);
	end
end

if (EVENT == 363) then
	ITEM_COUNT = HowmuchItem(UID, 900000000);
	NATION = CheckLoyalty(UID)
	if (ITEM_COUNT < 350000 and NATION > 500) then
		SelectMsg(UID, 2, 20, 4261, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 20, 4259, NPC, 22, 364, 23, -1);
	end
end

if (EVENT == 364) then
	ITEM_COUNT = HowmuchItem(UID, 900000000);
	NATION = CheckLoyalty(UID)
	if (ITEM_COUNT < 350000) then
		SelectMsg(UID, 2, 20, 4261, NPC, 10, -1);
	elseif (NATION == 500 or NATION == 0) then
		GoldLose(UID, 350000)
		GiveLoyalty(UID, 100)
	else
		SelectMsg(UID, 2, 20, 4259, NPC, 10, -1);
	end
end

local Reward;

if (EVENT == 370) then
	Reward = RequestPersonalRankReward(UID)
	if (Reward == 0) then
		SelectMsg(UID, 2, -1, 4254, NPC, 10, -1);
	elseif (Reward == 2) then
		SelectMsg(UID, 2, -1, 4255, NPC, 10, -1);
	end
end

local Reward;

if (EVENT == 380) then
	Reward = RequestReward(UID)
	if (Reward == 0) then
		SelectMsg(UID, 2, -1, 4252, NPC, 10, -1);
	elseif (Reward == 2) then
		SelectMsg(UID, 2, -1, 4253, NPC, 10, -1);
	end
end

if (EVENT == 350) then
	GetClanRank = CheckKnight(UID)
	if (GetClanRank == 3) then -- ACC 5
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6470, NPC, 4330, 371, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 4) then -- ACC 4
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6469, NPC, 4330, 372, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 5) then -- ACC 3
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6468, NPC, 4330, 368, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 6) then -- ACC 2
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6467, NPC, 4330, 373, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 7) then -- ACC 1
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6446, NPC, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 8) then -- Royal 5
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6466, NPC, 4330, 369, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 9) then -- Royal 4
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6465, NPC, 4330, 365, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 10) then -- Royal 3
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6464, NPC, 4330, 366, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 11) then -- Royal 2
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6449, NPC, 4330, 367, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank == 12) then -- Royal 1
		Leader = isClanLeader(UID)
		if (Leader) then
			SelectMsg(UID, 2, -1, 6448, NPC, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	elseif (GetClanRank < 3) then -- Training veya Clansiz olanlar
		Leader = isClanLeader(UID)
		if (Leader) then 
			SelectMsg(UID, 2, -1, 6424, NPC, 27, -1);
		else
			SelectMsg(UID, 2, -1, 6423, NPC, 27, -1);
		end
	end
end

if (EVENT == 371) then -- ACC 4 ICIN 126K CP
	CP = CheckClanPoint(UID)
	if (CP < 252000) then
		SelectMsg(UID, 2, -1, 6458, NPC, 27, -1);
	else
		ShowEffect(UID, 300391)
		RobClanPoint(UID, 252000)
		PromoteKnight(UID, 4)
		SelectMsg(UID, 2, -1, 6450, NPC, 27, -1);
	end
end

if (EVENT == 372) then -- ACC 3 ICIN 180K CP
	CP = CheckClanPoint(UID)
	if (CP < 360000) then
		SelectMsg(UID, 2, -1, 6457, NPC, 27, -1);
	else
		ShowEffect(UID, 300391)
		RobClanPoint(UID, 360000)
		PromoteKnight(UID, 5)
		SelectMsg(UID, 2, -1, 6450, NPC, 27, -1);
	end
end

if (EVENT == 368) then -- ACC 2 ICIN 270K CP
	CP = CheckClanPoint(UID)
	if (CP < 540000) then
		SelectMsg(UID, 2, -1, 6456, NPC, 27, -1);
	else
		ShowEffect(UID, 300391)
		RobClanPoint(UID, 540000)
		PromoteKnight(UID, 6)
		SelectMsg(UID, 2, -1, 6450, NPC, 27, -1);
	end
end

if (EVENT == 373) then -- ACC 1 ICIN 360K CP
	CP = CheckClanPoint(UID)
	if (CP < 720000) then
		SelectMsg(UID, 2, -1, 6455, NPC, 27, -1);
	else
		ShowEffect(UID, 300391)
		RobClanPoint(UID, 720000)
		PromoteKnight(UID, 7)
		SelectMsg(UID, 2, -1, 6450, NPC, 27, -1);
	end
end

if (EVENT == 369) then -- Royal 4 ICIN 540K CP
	CP = CheckClanPoint(UID)
	if (CP < 1080000) then
		SelectMsg(UID, 2, -1, 6454, NPC, 27, -1);
	else
		ShowEffect(UID, 300391)
		RobClanPoint(UID, 1080000)
		PromoteKnight(UID, 9)
		SelectMsg(UID, 2, -1, 6450, NPC, 27, -1);
	end
end

if (EVENT == 365) then -- Royal 3 ICIN 630K CP
	CP = CheckClanPoint(UID)
	if (CP < 1260000) then
		SelectMsg(UID, 2, -1, 6453, NPC, 27, -1);
	else
		ShowEffect(UID, 300391)
		RobClanPoint(UID, 1260000)
		PromoteKnight(UID, 10)
		SelectMsg(UID, 2, -1, 6450, NPC, 27, -1);
	end
end

if (EVENT == 366) then -- Royal 2 ICIN 720K CP
	CP = CheckClanPoint(UID)
	if (CP < 1440000) then
		SelectMsg(UID, 2, -1, 6452, NPC, 27, -1);
	else
		ShowEffect(UID, 300391)
		RobClanPoint(UID, 1440000)
		PromoteKnight(UID, 11)
		SelectMsg(UID, 2, -1, 6450, NPC, 27, -1);
	end
end

if (EVENT == 367) then -- Royal 1 ICIN 810K CP
	CP = CheckClanPoint(UID)
	if (CP < 1620000) then
		SelectMsg(UID, 2, -1, 6451, NPC, 27, -1);
	else
		ShowEffect(UID, 300391)
		RobClanPoint(UID, 1620000)
		PromoteKnight(UID,12)
		SelectMsg(UID, 2, -1, 6450, NPC, 27, -1);
	end
end