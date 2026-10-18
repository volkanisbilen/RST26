local NPC = 21510;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 1252, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 1252, NPC)
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
		SelectMsg(UID, 2, -1, 962, NPC, 27, -1);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
	-- Captain Fargo's quest instance is the dedicated Stone 2 spawn set.
	-- monster_stone_respawn_list: ZoneID=82, Family=71 (s_index 913..942).
	MonsterStoneQuestJoin(UID,199,82,71);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
	else
		RunQuestExchange(UID,94,STEP,1);
		SaveEvent(UID, 6041);
	end 
end

if (EVENT == 7006) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
	else
RunQuestExchange(UID,95,STEP,1);
	SaveEvent(UID, 6047); 
end
end

if (EVENT == 7007) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
	else
RunQuestExchange(UID,96,STEP,1);
	SaveEvent(UID, 6053);
end
end

if (EVENT == 7008) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
		SelectMsg(UID, 2, -1, 962, NPC, 27, -1);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
    RunQuestExchange(UID,53)
	SaveEvent(UID, 7107);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
	SelectMsg(UID, 4, 543, 20054, NPC, 22, 1803, 27, -1);
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
		SelectMsg(UID, 2, 543, 20054, NPC, 18,1804);
	else
		SelectMsg(UID, 5, 543, 20054, NPC, 22, 1806,27, -1);
end
end	

if (EVENT == 1804 ) then
	ShowMap(UID, 510)
end

if (EVENT == 1806)then
	QuestStatusCheck = GetQuestStatus(UID, 543) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 508107000);   
	if (ITEM1_COUNT < 5) then
		SelectMsg(UID, 2, 543, 20054, NPC, 18,1804);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
		SelectMsg(UID, 2, -1, 1179, NPC, 10, -1);
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
		SelectMsg(UID, 4, 555, 20499, NPC, 22, 2103, 23, -1);
	else
		SelectMsg(UID, 2, 555, 4072, NPC, 10, -1);
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
		SelectMsg(UID, 4, 555, 20499, NPC, 41, 2104, 27, -1);
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

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=199 status=2 n_index=6041
if (EVENT == 193) then
	QuestStatusCheck = GetQuestStatus(UID, 199)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		SelectMsg(UID, 5, 199, 6070, NPC, 4006, 7004, 4005, -1);
	end
end

-- [AUTO-GEN] quest=438 status=2 n_index=7107
if (EVENT == 240) then
	QuestStatusCheck = GetQuestStatus(UID, 438)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 53);
		SaveEvent(UID, 7109);
	end
end

-- [AUTO-GEN] quest=439 status=255 n_index=7115
if (EVENT == 405) then
	SaveEvent(UID, 7116);
end

-- [AUTO-GEN] quest=220 status=255 n_index=4204
if (EVENT == 530) then
	SaveEvent(UID, 4205);
end

-- [AUTO-GEN] quest=522 status=255 n_index=11102
if (EVENT == 1100) then
	SaveEvent(UID, 11103);
end

-- [AUTO-GEN] quest=525 status=255 n_index=11138
if (EVENT == 1200) then
	SaveEvent(UID, 11139);
end

-- [AUTO-GEN] quest=526 status=255 n_index=11150
if (EVENT == 1300) then
	SaveEvent(UID, 11151);
end

-- [AUTO-GEN] quest=527 status=255 n_index=11162
if (EVENT == 1400) then
	SaveEvent(UID, 11163);
end

-- [AUTO-GEN] quest=535 status=255 n_index=11258
if (EVENT == 1500) then
	SaveEvent(UID, 11259);
end

-- [AUTO-GEN] quest=536 status=255 n_index=11270
if (EVENT == 1600) then
	SaveEvent(UID, 11271);
end

-- [AUTO-GEN] quest=542 status=255 n_index=11342
if (EVENT == 1700) then
	SaveEvent(UID, 11343);
end

-- [AUTO-GEN] quest=543 status=255 n_index=11354
if (EVENT == 1800) then
	SaveEvent(UID, 11355);
end

-- [AUTO-GEN] quest=544 status=255 n_index=11366
if (EVENT == 1900) then
	SaveEvent(UID, 11367);
end

-- [AUTO-GEN] quest=545 status=255 n_index=11378
if (EVENT == 2000) then
	SaveEvent(UID, 11379);
end

-- [AUTO-GEN] quest=555 status=255 n_index=11528
if (EVENT == 2100) then
	SaveEvent(UID, 11529);
end

-- [AUTO-GEN] quest=556 status=255 n_index=11540
if (EVENT == 2200) then
	SaveEvent(UID, 11541);
end

-- [AUTO-GEN] quest=556 status=0 n_index=11541
if (EVENT == 2202) then
	SelectMsg(UID, 4, 556, 20080, NPC, 3080, 2203, 23, -1);
end

-- [AUTO-GEN] quest=556 status=0 n_index=11541
if (EVENT == 2203) then
	SaveEvent(UID, 11542);
end

-- [AUTO-GEN] quest=556 status=1 n_index=11542
if (EVENT == 2205) then
	ItemA = HowmuchItem(UID, 910198000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 556, 20080, NPC, 18, 2206);
	else
		SelectMsg(UID, 4, 556, 20080, NPC, 41, 2208, 27, -1);
	end
end

-- [AUTO-GEN] quest=556 status=1 n_index=11542
if (EVENT == 2206) then
	ShowMap(UID, 1);
end

-- [AUTO-GEN] quest=556 status=1 n_index=11542
if (EVENT == 2208) then
	QuestStatusCheck = GetQuestStatus(UID, 556)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3046);
		SaveEvent(UID, 11543);
	end
end

-- [AUTO-GEN] quest=563 status=255 n_index=11624
if (EVENT == 2300) then
	SaveEvent(UID, 11625);
end

-- [AUTO-GEN] quest=563 status=0 n_index=11625
if (EVENT == 2302) then
	SelectMsg(UID, 4, 563, 20094, NPC, 3094, 2303, 23, -1);
end

-- [AUTO-GEN] quest=563 status=0 n_index=11625
if (EVENT == 2303) then
	SaveEvent(UID, 11626);
end

-- [AUTO-GEN] quest=563 status=1 n_index=11626
if (EVENT == 2305) then
	ItemA = HowmuchItem(UID, 910231000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 563, 20094, NPC, 18, 2306);
	else
		SelectMsg(UID, 4, 563, 20094, NPC, 41, 2308, 27, -1);
	end
end

-- [AUTO-GEN] quest=563 status=1 n_index=11626
if (EVENT == 2306) then
	ShowMap(UID, 11);
end

-- [AUTO-GEN] quest=563 status=1 n_index=11626
if (EVENT == 2308) then
	QuestStatusCheck = GetQuestStatus(UID, 563)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3053);
		SaveEvent(UID, 11627);
	end
end

-- [AUTO-GEN] quest=565 status=255 n_index=11648
if (EVENT == 2400) then
	SaveEvent(UID, 11649);
end

-- [AUTO-GEN] quest=565 status=0 n_index=11649
if (EVENT == 2402) then
	SelectMsg(UID, 4, 565, 20098, NPC, 3098, 2403, 23, -1);
end

-- [AUTO-GEN] quest=565 status=0 n_index=11649
if (EVENT == 2403) then
	SaveEvent(UID, 11650);
end

-- [AUTO-GEN] quest=565 status=1 n_index=11650
if (EVENT == 2405) then
	ItemA = HowmuchItem(UID, 910233000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 565, 20098, NPC, 18, 2406);
	else
		SelectMsg(UID, 4, 565, 20098, NPC, 41, 2408, 27, -1);
	end
end

-- [AUTO-GEN] quest=565 status=1 n_index=11650
if (EVENT == 2406) then
	ShowMap(UID, 1);
end

-- [AUTO-GEN] quest=565 status=1 n_index=11650
if (EVENT == 2408) then
	QuestStatusCheck = GetQuestStatus(UID, 565)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3055);
		SaveEvent(UID, 11651);
	end
end

-- [AUTO-GEN] quest=566 status=255 n_index=11660
if (EVENT == 2500) then
	SaveEvent(UID, 11661);
end

-- [AUTO-GEN] quest=566 status=0 n_index=11661
if (EVENT == 2502) then
	SelectMsg(UID, 4, 566, 20100, NPC, 3100, 2503, 23, -1);
end

-- [AUTO-GEN] quest=566 status=0 n_index=11661
if (EVENT == 2503) then
	SaveEvent(UID, 11662);
end

-- [AUTO-GEN] quest=566 status=1 n_index=11662
if (EVENT == 2505) then
	ItemA = HowmuchItem(UID, 910199000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 566, 20100, NPC, 18, 2506);
	else
		SelectMsg(UID, 4, 566, 20100, NPC, 41, 2508, 27, -1);
	end
end

-- [AUTO-GEN] quest=566 status=1 n_index=11662
if (EVENT == 2506) then
	ShowMap(UID, 1);
end

-- [AUTO-GEN] quest=566 status=1 n_index=11662
if (EVENT == 2508) then
	QuestStatusCheck = GetQuestStatus(UID, 566)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3056);
		SaveEvent(UID, 11663);
	end
end

-- [AUTO-GEN] quest=574 status=255 n_index=11755
if (EVENT == 2600) then
	SaveEvent(UID, 11756);
end

-- [AUTO-GEN] quest=574 status=0 n_index=11756
if (EVENT == 2602) then
	SelectMsg(UID, 4, 574, 20116, NPC, 3116, 2603, 23, -1);
end

-- [AUTO-GEN] quest=574 status=0 n_index=11756
if (EVENT == 2603) then
	SaveEvent(UID, 11757);
end

-- [AUTO-GEN] quest=574 status=1 n_index=11757
if (EVENT == 2605) then
	ItemA = HowmuchItem(UID, 910235000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 574, 20116, NPC, 18, 2606);
	else
		SelectMsg(UID, 4, 574, 20116, NPC, 41, 2608, 27, -1);
	end
end

-- [AUTO-GEN] quest=574 status=1 n_index=11757
if (EVENT == 2606) then
	ShowMap(UID, 1);
end

-- [AUTO-GEN] quest=574 status=1 n_index=11757
if (EVENT == 2608) then
	QuestStatusCheck = GetQuestStatus(UID, 574)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3064);
		SaveEvent(UID, 11758);
	end
end

-- [AUTO-GEN] quest=575 status=255 n_index=11767
if (EVENT == 2700) then
	SaveEvent(UID, 11768);
end

-- [AUTO-GEN] quest=575 status=0 n_index=11768
if (EVENT == 2702) then
	SelectMsg(UID, 4, 575, 20118, NPC, 3118, 2703, 23, -1);
end

-- [AUTO-GEN] quest=575 status=0 n_index=11768
if (EVENT == 2703) then
	SaveEvent(UID, 11769);
end

-- [AUTO-GEN] quest=575 status=1 n_index=11769
if (EVENT == 2705) then
	ItemA = HowmuchItem(UID, 910200000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 575, 20118, NPC, 18, 2706);
	else
		SelectMsg(UID, 4, 575, 20118, NPC, 41, 2708, 27, -1);
	end
end

-- [AUTO-GEN] quest=575 status=1 n_index=11769
if (EVENT == 2706) then
	ShowMap(UID, 1);
end

-- [AUTO-GEN] quest=575 status=1 n_index=11769
if (EVENT == 2708) then
	QuestStatusCheck = GetQuestStatus(UID, 575)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3065);
		SaveEvent(UID, 11770);
	end
end

-- [AUTO-GEN] quest=580 status=255 n_index=11828
if (EVENT == 2800) then
	SaveEvent(UID, 11829);
end

-- [AUTO-GEN] quest=580 status=0 n_index=11829
if (EVENT == 2802) then
	SelectMsg(UID, 4, 580, 20730, NPC, 3128, 2803, 23, -1);
end

-- [AUTO-GEN] quest=580 status=1 n_index=11830
if (EVENT == 2803) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 580, 20730, NPC, 18, 2805);
	else
		SelectMsg(UID, 4, 580, 20730, NPC, 41, 2804, 27, -1);
	end
end

-- [AUTO-GEN] quest=580 status=1 n_index=11830
if (EVENT == 2804) then
	QuestStatusCheck = GetQuestStatus(UID, 580)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3070);
		SaveEvent(UID, 11831);
	end
end

-- [AUTO-GEN] quest=580 status=3 n_index=11832
if (EVENT == 2805) then
	SelectMsg(UID, 2, 580, 20730, NPC, 10, -1);
end

-- [AUTO-GEN] quest=586 status=255 n_index=11900
if (EVENT == 2900) then
	SaveEvent(UID, 11901);
end

-- [AUTO-GEN] quest=586 status=0 n_index=11901
if (EVENT == 2902) then
	SelectMsg(UID, 4, 586, 20742, NPC, 3140, 2903, 23, -1);
end

-- [AUTO-GEN] quest=586 status=1 n_index=11902
if (EVENT == 2903) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 586, 20742, NPC, 18, 2905);
	else
		SelectMsg(UID, 4, 586, 20742, NPC, 41, 2904, 27, -1);
	end
end

-- [AUTO-GEN] quest=586 status=1 n_index=11902
if (EVENT == 2904) then
	QuestStatusCheck = GetQuestStatus(UID, 586)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3076);
		SaveEvent(UID, 11903);
	end
end

-- [AUTO-GEN] quest=586 status=3 n_index=11904
if (EVENT == 2905) then
	SelectMsg(UID, 2, 586, 20742, NPC, 10, -1);
end

-- [AUTO-GEN] quest=587 status=255 n_index=11912
if (EVENT == 3000) then
	SaveEvent(UID, 11913);
end

-- [AUTO-GEN] quest=587 status=0 n_index=11913
if (EVENT == 3002) then
	SelectMsg(UID, 4, 587, 20744, NPC, 3142, 3003, 23, -1);
end

-- [AUTO-GEN] quest=587 status=0 n_index=11913
if (EVENT == 3003) then
	SaveEvent(UID, 11914);
end

-- [AUTO-GEN] quest=587 status=1 n_index=11914
if (EVENT == 3005) then
	ItemA = HowmuchItem(UID, 910205000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 587, 20744, NPC, 18, 3006);
	else
		SelectMsg(UID, 4, 587, 20744, NPC, 41, 3008, 27, -1);
	end
end

-- [AUTO-GEN] quest=587 status=1 n_index=11914
if (EVENT == 3006) then
	ShowMap(UID, 1);
end

-- [AUTO-GEN] quest=587 status=1 n_index=11914
if (EVENT == 3008) then
	QuestStatusCheck = GetQuestStatus(UID, 587)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3077);
		SaveEvent(UID, 11915);
	end
end

-- [AUTO-GEN] quest=594 status=255 n_index=11984
if (EVENT == 3100) then
	SaveEvent(UID, 11985);
end

-- [AUTO-GEN] quest=594 status=0 n_index=11985
if (EVENT == 3102) then
	SelectMsg(UID, 4, 594, 20756, NPC, 3154, 3103, 23, -1);
end

-- [AUTO-GEN] quest=594 status=0 n_index=11985
if (EVENT == 3103) then
	SaveEvent(UID, 11986);
end

-- [AUTO-GEN] quest=594 status=1 n_index=11986
if (EVENT == 3105) then
	ItemA = HowmuchItem(UID, 910237000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 594, 20756, NPC, 18, 3106);
	else
		SelectMsg(UID, 4, 594, 20756, NPC, 41, 3106, 27, -1);
	end
end

-- [AUTO-GEN] quest=594 status=1 n_index=11986
if (EVENT == 3106) then
	QuestStatusCheck = GetQuestStatus(UID, 594)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3083);
		SaveEvent(UID, 11987);
	end
end

-- [AUTO-GEN] quest=597 status=255 n_index=12020
if (EVENT == 3200) then
	SaveEvent(UID, 12021);
end

-- [AUTO-GEN] quest=597 status=0 n_index=12021
if (EVENT == 3202) then
	SelectMsg(UID, 4, 597, 20762, NPC, 3160, 3203, 23, -1);
end

-- [AUTO-GEN] quest=597 status=1 n_index=12022
if (EVENT == 3203) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 597, 20762, NPC, 18, 3205);
	else
		SelectMsg(UID, 4, 597, 20762, NPC, 41, 3204, 27, -1);
	end
end

-- [AUTO-GEN] quest=597 status=1 n_index=12022
if (EVENT == 3204) then
	QuestStatusCheck = GetQuestStatus(UID, 597)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3086);
		SaveEvent(UID, 12023);
	end
end

-- [AUTO-GEN] quest=597 status=3 n_index=12024
if (EVENT == 3205) then
	SelectMsg(UID, 2, 597, 20762, NPC, 10, -1);
end

-- [AUTO-GEN] quest=598 status=255 n_index=12032
if (EVENT == 3300) then
	SaveEvent(UID, 12033);
end

-- [AUTO-GEN] quest=598 status=0 n_index=12033
if (EVENT == 3302) then
	SelectMsg(UID, 4, 598, 20764, NPC, 3162, 3303, 23, -1);
end

-- [AUTO-GEN] quest=598 status=0 n_index=12033
if (EVENT == 3303) then
	SaveEvent(UID, 12034);
end

-- [AUTO-GEN] quest=598 status=1 n_index=12034
if (EVENT == 3305) then
	ItemA = HowmuchItem(UID, 910206000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 598, 20764, NPC, 18, 3306);
	else
		SelectMsg(UID, 4, 598, 20764, NPC, 41, 3308, 27, -1);
	end
end

-- [AUTO-GEN] quest=598 status=1 n_index=12034
if (EVENT == 3306) then
	ShowMap(UID, 1);
end

-- [AUTO-GEN] quest=598 status=1 n_index=12034
if (EVENT == 3308) then
	QuestStatusCheck = GetQuestStatus(UID, 598)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3087);
		SaveEvent(UID, 12035);
	end
end

-- [AUTO-GEN] quest=607 status=255 n_index=12140
if (EVENT == 3400) then
	SaveEvent(UID, 12141);
end

-- [AUTO-GEN] quest=607 status=0 n_index=12141
if (EVENT == 3402) then
	SelectMsg(UID, 4, 607, 20782, NPC, 3180, 3403, 23, -1);
end

-- [AUTO-GEN] quest=607 status=0 n_index=12141
if (EVENT == 3403) then
	SaveEvent(UID, 12142);
end

-- [AUTO-GEN] quest=607 status=1 n_index=12142
if (EVENT == 3405) then
	ItemA = HowmuchItem(UID, 910241000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 607, 20782, NPC, 18, 3406);
	else
		SelectMsg(UID, 4, 607, 20782, NPC, 41, 3406, 27, -1);
	end
end

-- [AUTO-GEN] quest=607 status=1 n_index=12142
if (EVENT == 3406) then
	QuestStatusCheck = GetQuestStatus(UID, 607)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3096);
		SaveEvent(UID, 12143);
	end
end

-- [AUTO-GEN] quest=608 status=255 n_index=12152
if (EVENT == 3500) then
	SaveEvent(UID, 12153);
end

-- [AUTO-GEN] quest=608 status=0 n_index=12153
if (EVENT == 3502) then
	SelectMsg(UID, 4, 608, 20784, NPC, 3182, 3503, 23, -1);
end

-- [AUTO-GEN] quest=608 status=0 n_index=12153
if (EVENT == 3503) then
	SaveEvent(UID, 12154);
end

-- [AUTO-GEN] quest=608 status=1 n_index=12154
if (EVENT == 3505) then
	ItemA = HowmuchItem(UID, 910207000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 608, 20784, NPC, 18, 3506);
	else
		SelectMsg(UID, 4, 608, 20784, NPC, 41, 3506, 27, -1);
	end
end

-- [AUTO-GEN] quest=608 status=1 n_index=12154
if (EVENT == 3506) then
	QuestStatusCheck = GetQuestStatus(UID, 608)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3097);
		SaveEvent(UID, 12155);
	end
end

-- [AUTO-GEN] quest=614 status=255 n_index=12224
if (EVENT == 3600) then
	SaveEvent(UID, 12225);
end

-- [AUTO-GEN] quest=614 status=0 n_index=12225
if (EVENT == 3602) then
	SelectMsg(UID, 4, 614, 20796, NPC, 3194, 3603, 23, -1);
end

-- [AUTO-GEN] quest=614 status=1 n_index=12226
if (EVENT == 3603) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 614, 20796, NPC, 18, 3605);
	else
		SelectMsg(UID, 4, 614, 20796, NPC, 41, 3604, 27, -1);
	end
end

-- [AUTO-GEN] quest=614 status=1 n_index=12226
if (EVENT == 3604) then
	QuestStatusCheck = GetQuestStatus(UID, 614)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3103);
		SaveEvent(UID, 12227);
	end
end

-- [AUTO-GEN] quest=614 status=3 n_index=12228
if (EVENT == 3605) then
	SelectMsg(UID, 2, 614, 20796, NPC, 10, -1);
end

-- [AUTO-GEN] quest=617 status=255 n_index=12260
if (EVENT == 3700) then
	SaveEvent(UID, 12261);
end

-- [AUTO-GEN] quest=617 status=0 n_index=12261
if (EVENT == 3702) then
	SelectMsg(UID, 4, 617, 20802, NPC, 3200, 3703, 23, -1);
end

-- [AUTO-GEN] quest=617 status=0 n_index=12261
if (EVENT == 3703) then
	SaveEvent(UID, 12262);
end

-- [AUTO-GEN] quest=617 status=1 n_index=12262
if (EVENT == 3705) then
	ItemA = HowmuchItem(UID, 910244000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 617, 20802, NPC, 18, 3706);
	else
		SelectMsg(UID, 4, 617, 20802, NPC, 41, 3706, 27, -1);
	end
end

-- [AUTO-GEN] quest=617 status=1 n_index=12262
if (EVENT == 3706) then
	QuestStatusCheck = GetQuestStatus(UID, 617)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3106);
		SaveEvent(UID, 12263);
	end
end

-- [AUTO-GEN] quest=618 status=255 n_index=12272
if (EVENT == 3800) then
	SaveEvent(UID, 12273);
end

-- [AUTO-GEN] quest=618 status=0 n_index=12273
if (EVENT == 3802) then
	SelectMsg(UID, 4, 618, 20804, NPC, 3202, 3803, 23, -1);
end

-- [AUTO-GEN] quest=618 status=0 n_index=12273
if (EVENT == 3803) then
	SaveEvent(UID, 12274);
end

-- [AUTO-GEN] quest=618 status=1 n_index=12274
if (EVENT == 3805) then
	ItemA = HowmuchItem(UID, 910208000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 618, 20804, NPC, 18, 3806);
	else
		SelectMsg(UID, 4, 618, 20804, NPC, 41, 3806, 27, -1);
	end
end

-- [AUTO-GEN] quest=618 status=1 n_index=12274
if (EVENT == 3806) then
	QuestStatusCheck = GetQuestStatus(UID, 618)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3107);
		SaveEvent(UID, 12275);
	end
end

-- [AUTO-GEN] quest=199 status=255 n_index=6038
if (EVENT == 6090) then
	SaveEvent(UID, 6039);
end
