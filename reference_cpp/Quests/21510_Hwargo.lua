-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 21510;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 664, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 664, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 195) then -- 44 Level Recons
	SaveEvent(UID, 467);
end

if (EVENT == 200) then
	SelectMsg(UID, 2, 177, 1173, NPC, 10, 201);
end

if (EVENT == 201) then
	SelectMsg(UID, 4, 177, 1174, NPC, 22, 202, 23, -1);
end

if (EVENT == 202) then
	Check = isRoomForItem(UID, 910044000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
	else
		GiveItem(UID, 910044000, 1);
		SaveEvent(UID, 468);
	end
end

if (EVENT == 205) then
	SelectMsg(UID, 2, 177, 1175, NPC, 10, -1);
	SaveEvent(UID, 470);
end

if (EVENT == 210) then
	ITEMA = HowmuchItem(UID, 910040000);
	ITEMB = HowmuchItem(UID, 910041000);
	if (ITEMA < 3) then 
		SelectMsg(UID, 2, 177, 1177, NPC, 18, 213);
	elseif (ITEMB < 1) then
		SelectMsg(UID, 2, 177, 1178, NPC, 18, 213);
	elseif (ITEMA > 2 and ITEMB > 0) then
		SelectMsg(UID, 4, 177, 1179, NPC, 41, 214, 27, -1);
	end
end

if (EVENT == 213) then
	ShowMap(UID, 46);
end

if (EVENT == 214) then
	QuestStatusCheck = GetQuestStatus(UID, 177) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 910040000);
	ITEMB = HowmuchItem(UID, 910041000);
	if (ITEMA < 3) then 
		SelectMsg(UID, 2, 177, 1177, NPC, 18, 213);
	elseif (ITEMB < 1) then
		SelectMsg(UID, 2, 177, 1178, NPC, 18, 213);
		else
RunQuestExchange(UID,88)
	SaveEvent(UID, 469);
end
end
end

local savenum = 199;

if (EVENT == 6092) then -- 46 Level Quest Area
	SelectMsg(UID, 2, savenum, 6065, NPC, 6007, 6093);
end

if (EVENT == 6093) then
	ITEM_COUNT = HowmuchItem(UID, 910135000);   
	ITEM_COUNT1 = HowmuchItem(UID, 910138000);
	if (ITEM_COUNT < 1 or ITEM_COUNT1 < 3) then
		SelectMsg(UID, 4, savenum, 6067, NPC, 4543, 6094, 4191, -1);
	elseif (ITEM_COUNT > 0 and ITEM_COUNT1 > 2) then
		SelectMsg(UID, 5, savenum, 6070, NPC, 4006, 7004,4005, -1);
	end
end

if (EVENT == 6094) then
	MonsterStoneQuestJoin(UID,199);
	EVENT = 6095
end

if (EVENT == 6095) then
	Class = CheckClass (UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 6040);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 6046);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 6052);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 6058);
	end
end

if (EVENT == 7000) then
	Class = CheckClass (UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 6042);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 6048);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 6054);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 6060);
	end
end

if (EVENT == 7004) then
	Class = CheckClass (UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		EVENT = 7005
	elseif (Class == 2 or Class == 7 or Class == 8) then
		EVENT = 7006
	elseif (Class == 3 or Class == 9 or Class == 10) then
		EVENT = 7007
	elseif (Class == 4 or Class == 11 or Class == 12) then
		EVENT = 7008
	end
end

if (EVENT == 7005) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
		RunQuestExchange(UID,94,STEP,1);
		SaveEvent(UID, 6041);
	end 
end

if (EVENT == 7006) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
RunQuestExchange(UID,95,STEP,1);
	SaveEvent(UID, 6047); 
end
end

if (EVENT == 7007) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
RunQuestExchange(UID,96,STEP,1);
	SaveEvent(UID, 6053);
end
end

if (EVENT == 7008) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
RunQuestExchange(UID,97,STEP,1);
	SaveEvent(UID, 6059); 
end
end

if (EVENT == 532) then -- 50 Level 7 Keys Quest
	SelectMsg(UID, 4, 220, 4196, NPC, 22, 533, 23, -1);
end

if (EVENT == 533) then
	Check = isRoomForItem(UID, 910050000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
	else
		GiveItem(UID, 910050000, 1);
		SaveEvent(UID, 4206);
	end
end

if (EVENT == 534) then
	SaveEvent(UID, 4209);
end

if (EVENT == 538) then
	SaveEvent(UID, 4208);
end

if (EVENT == 536) then
	ITEM7 = HowmuchItem(UID, 910057000);
	if (ITEM7 > 0) then
		SelectMsg(UID, 4, 220, 4197, NPC, 4172, 537, 4173, -1);
	else
		SelectMsg(UID, 2, 220, 4198, NPC, 18, 192);
	end
end

if (EVENT == 192) then
	ShowMap(UID, 432);
end

if (EVENT == 537) then
	QuestStatusCheck = GetQuestStatus(UID, 220) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM7 = HowmuchItem(UID, 910057000);
	if (ITEM7 > 0) then
	RunQuestExchange(UID,470)
	SaveEvent(UID, 4207);
		else
		SelectMsg(UID, 2, 220, 4198, NPC, 18, 192);
	end
end
end

if (EVENT == 1000) then -- 47 Level Border Security Scroll
	SaveEvent(UID, 2452);
end

if (EVENT == 1001) then
	SelectMsg(UID, 4, 494, 9238, NPC, 22, 1002, 23, -1);
end

if (EVENT == 1002) then
	SaveEvent(UID, 2453);
end   

if (EVENT == 1003) then
	SaveEvent(UID, 2456);
end

if (EVENT == 1006) then
	SaveEvent(UID, 2455);
end

if (EVENT == 1007) then
	ITEMBDW = HowmuchItem(UID, 900143000);
	if (ITEMBDW < 1) then
		SelectMsg(UID, 2, 494, 9238, NPC, 18, 191);
	else
		SelectMsg(UID, 4, 494, 9238, NPC, 4006, 1008, 4005, -1);
	end
end

if (EVENT == 191) then
	ShowMap(UID, 726);
end

if (EVENT == 1008) then
	QuestStatusCheck = GetQuestStatus(UID, 494) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEMBDW = HowmuchItem(UID, 900143000);
	if (ITEMBDW < 1) then
		SelectMsg(UID, 2, 494, 9238, NPC, 18, 191);
	else
RunQuestExchange(UID,222)
	SaveEvent(UID, 2454);
end
end
end


if (EVENT == 400) then
	SelectMsg(UID, 4, 438, 6195, NPC, 10, 401, 4005, -1);
end

if (EVENT == 401) then
    SelectMsg(UID, 15, -1, -1, NPC);
	SaveEvent(UID, 7107);
    RunQuestExchange(UID,53)
end


----------------------------------
if (EVENT == 410) then
	SelectMsg(UID, 2, 439, 4985, NPC, 10, 411, 4005, -1);
end

if (EVENT == 411) then
	SaveEvent(UID, 7118);
end
----------------------------------
if (EVENT == 412) then
	SelectMsg(UID, 2, 443, 4985, NPC, 10, 413, 4005, -1);
end
----------------------------------
if (EVENT == 413) then
	SaveEvent(UID, 7140);
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 522, 20013, NPC, 22, 1103, 27, -1);
end

if (EVENT == 1103) then
	SaveEvent(UID, 11104);
end

if (EVENT == 1104) then
		SelectMsg(UID, 4, 522, 20013, NPC, 22, 1105, 27, -1);
		SaveEvent(UID, 11106);
end

if (EVENT == 1105) then
SelectMsg(UID, 2, 522, 20209, NPC, 10, -1);
	SaveEvent(UID, 11105);
	SaveEvent(UID, 11116);
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 525, 20019, NPC, 22, 1203, 27, -1);
end

if (EVENT == 1203) then
	SaveEvent(UID, 11140);
end

if (EVENT == 1208) then
	SaveEvent(UID, 11142);
end

if (EVENT == 1205) then
	ITEM_COUNT = HowmuchItem(UID, 910214000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 525, 20019, NPC, 18, -1);
	else
		SelectMsg(UID, 4, 525, 20019, NPC, 22, 1207, 27, -1); 
	end
end

if (EVENT == 1207)then
	QuestStatusCheck = GetQuestStatus(UID, 525) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910214000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 525, 20019, NPC, 18, -1);
	else
RunQuestExchange(UID,3012)
	SaveEvent(UID,11141)
	SaveEvent(UID,11152)
end
end
end

if (EVENT == 1302) then
	SelectMsg(UID, 4, 526, 20021, NPC, 22, 1303, 27, -1);
end

if (EVENT == 1303) then
	SaveEvent(UID, 11152);
end

if (EVENT == 1308) then
	SaveEvent(UID, 11154);
end

if (EVENT == 1305) then
	ITEM_COUNT = HowmuchItem(UID, 910195000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 526, 20021, NPC, 18,1306);
	else
		SelectMsg(UID, 4, 526, 20021, NPC, 22, 1307, 27, -1); 
	end
end

if (EVENT == 1306) then
	ShowMap(UID, 728);
end

if (EVENT == 1307)then
	QuestStatusCheck = GetQuestStatus(UID, 526) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910195000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 526, 20021, NPC, 18,1306);
	else
RunQuestExchange(UID,3013)
	SaveEvent(UID,11153)
end
end
end

if (EVENT == 1402) then
	SelectMsg(UID, 2, 527, 20212, NPC, 4161, 1403);
end

if (EVENT == 1403) then
	SelectMsg(UID, 2, 527, 20213, NPC, 4552, 1404);
end

if (EVENT == 1404) then
	SelectMsg(UID, 4, 527, 20213, NPC, 22,1405,27,-1);
	SaveEvent(UID,11164)
	SaveEvent(UID,11166)
end

if (EVENT == 1405) then
	SaveEvent(UID,11165)
	SaveEvent(UID,11176)
end

if (EVENT == 1502) then
	SelectMsg(UID, 2, 535, 20039, NPC, 4161, 1504);
end

if (EVENT == 1503) then
	SelectMsg(UID, 2, 535, 20269, NPC, 4552, 1504);
end

if (EVENT == 1504) then
	SelectMsg(UID, 4, 535, 20039, NPC, 22,1505,27,-1);
	SaveEvent(UID,11260)
	SaveEvent(UID,11262)
end

if (EVENT == 1505) then
	SaveEvent(UID,11261)
	SaveEvent(UID,11272)
end

if (EVENT == 1602) then
	SelectMsg(UID, 4, 536, 20041, NPC, 22, 1603, 27, -1);
end

if (EVENT == 1603) then
	SaveEvent(UID,11272)
end

if (EVENT == 1608) then
	SaveEvent(UID,11274)
end

if (EVENT == 1605) then
	ITEM_COUNT = HowmuchItem(UID, 910196000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 536, 20041, NPC, 18,1606);
	else
		SelectMsg(UID, 4, 536, 20041, NPC, 22, 1607,27, -1); 
	end
end

if (EVENT == 1606) then
	ShowMap(UID, 730);
end

if (EVENT == 1607) then
	QuestStatusCheck = GetQuestStatus(UID, 536) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910196000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 536, 20041, NPC, 18,1606);
	else
RunQuestExchange(UID,3023);
SaveEvent(UID,11273);
end
end
end

if (EVENT == 1702) then
	SelectMsg(UID, 4, 542, 20053, NPC, 22, 1703, 27, -1);
end

if (EVENT == 1703) then
	SaveEvent(UID,11344)
end

if (EVENT == 1708) then
	SaveEvent(UID,11346)
end

if (EVENT == 1705) then
	ITEM1_COUNT = HowmuchItem(UID, 910227000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 542, 20053, NPC, 18,-1);
	else
		SelectMsg(UID, 4, 542, 20053, NPC, 22, 1706, 27, -1);
end
end	

if (EVENT == 1706)then
	QuestStatusCheck = GetQuestStatus(UID, 542) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 910227000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 542, 20053, NPC, 18,-1);
	else
RunQuestExchange(UID,3029)
	SaveEvent(UID,11345)
	SaveEvent(UID,11356)
end
end
end

if (EVENT == 1802) then
	SelectMsg(UID, 4, 543, 20055, NPC, 22, 1803, 27, -1);
end

if (EVENT == 1803) then
	SaveEvent(UID,11356)
end

if (EVENT == 1808) then
	SaveEvent(UID,11358)
end

if (EVENT == 1805) then
	ITEM1_COUNT = HowmuchItem(UID, 508107000);   
	if (ITEM1_COUNT < 5) then
		SelectMsg(UID, 2, 543, 20055, NPC, 18,1804);
	else
		SelectMsg(UID, 5, 543, 20055, NPC, 22, 1806,27, -1);
end
end	

if (EVENT == 1804 ) then
	ShowMap(UID, 510)
end

if (EVENT == 1806)then
	QuestStatusCheck = GetQuestStatus(UID, 543) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 508107000);   
	if (ITEM1_COUNT < 5) then
		SelectMsg(UID, 2, 543, 20055, NPC, 18,1804);
	else
RunQuestExchange(UID,3030,STEP,1);
	SaveEvent(UID,11357)
	SaveEvent(UID,11368)
end
end
end

if (EVENT == 1902) then
	SelectMsg(UID, 4, 544, 20057, NPC, 22, 1903, 27, -1);
end

if (EVENT == 1903) then
	SaveEvent(UID,11368)
end

if (EVENT == 1908) then
	SaveEvent(UID,11370)
end

if (EVENT == 1905) then
	MonsterCount = CountMonsterQuestSub(UID, 544, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 544, 20057, NPC, 18, 1904);
	else
		SelectMsg(UID, 4, 544, 20057, NPC, 22, 1907, 23, -1);
	end
end

if (EVENT == 1904 ) then
	ShowMap(UID, 375)
end

if (EVENT == 1907)then
	QuestStatusCheck = GetQuestStatus(UID, 544) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 544, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 544, 20057, NPC, 18, 1904);
	else
RunQuestExchange(UID,3031)
	SaveEvent(UID,11369)
	SaveEvent(UID,11380)
end
end
end

if (EVENT == 2002) then
	SelectMsg(UID, 4, 545, 20059, NPC, 22, 2003, 27, -1);
end

if (EVENT == 2003) then
	SaveEvent(UID,11380)
end

if (EVENT == 2008) then
	SaveEvent(UID,11382)
end

if (EVENT == 2005) then
	ITEM1_COUNT = HowmuchItem(UID, 910197000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 545, 20059, NPC, 18,2004);
	else
		SelectMsg(UID, 4, 545, 20059, NPC, 22, 2006,27, -1);
end
end	

if (EVENT == 2004 ) then
	ShowMap(UID, 733)
end

if (EVENT == 2006)then
	QuestStatusCheck = GetQuestStatus(UID, 545) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 187, NPC, 10, -1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 910197000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 545, 20059, NPC, 18,2004);
	else
		RunQuestExchange(UID,3032);
		SaveEvent(UID,11381);
end
end
end

if (EVENT == 2102) then
	QuestStatus = ExistMonsterQuestSub(UID,555);
	if (QuestStatus == 0) then
		SelectMsg(UID, 4, 555, 20494, NPC, 22, 2103, 23, 193);
	else
		SelectMsg(UID, 2, 555, 4071, NPC, 10, 193);
	end
end
	
if (EVENT == 2103) then
	SaveEvent(UID, 11530);
end

if (EVENT == 2108) then
	SaveEvent(UID, 11532);
end

if (EVENT == 2105) then
	OrderReturn  = HowmuchItem(UID, 910230000);
	if(OrderReturn > 0) then
		SelectMsg(UID, 4, 555, 20494, NPC, 41, 2104, 27, 193);
	else
		SelectMsg(UID, 2, 555, 11380, NPC, 10, -1);
	end
end

if (EVENT == 2104) then
	OrderReturn  = HowmuchItem(UID, 910230000);
	if(OrderReturn > 0) then
	RunQuestExchange(UID, 3045)
	SaveEvent(UID, 11531)
	SaveEvent(UID, 11542)
	end
end