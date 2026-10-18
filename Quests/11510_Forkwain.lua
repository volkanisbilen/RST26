local NPC = 11510;

if (EVENT == 190) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 664, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 664, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 195) then
	SaveEvent(UID, 439);
end

if (EVENT == 200) then
	SelectMsg(UID, 2, 177, 667, NPC, 10, 201);
end

if (EVENT == 201) then
	SelectMsg(UID, 4, 177, 668, NPC, 22, 202, 23, -1);
end

if (EVENT == 202) then
	Check = isRoomForItem(UID, 910044000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 6481, NPC, 27, -1);
	else
		GiveItem(UID, 910044000, 1);
		SaveEvent(UID, 440);
	end
end

if (EVENT == 205) then
	SelectMsg(UID, 2, 177, 669, NPC, 10, -1);
	SaveEvent(UID, 442);
end

if (EVENT == 210) then
	ITEMA = HowmuchItem(UID, 910040000);
	ITEMB = HowmuchItem(UID, 910041000);
	if (ITEMA < 3) then 
		SelectMsg(UID, 2, 177, 671, NPC, 18, 213);
	elseif (ITEMB < 1) then
		SelectMsg(UID, 2, 177, 672, NPC, 18, 213);
	elseif (ITEMA > 2 and ITEMB > 0) then
		SelectMsg(UID, 4, 177, 673, NPC, 41, 214, 27, -1);
	end
end

if (EVENT == 213) then
	ShowMap(UID, 40);
end

if (EVENT == 214) then
	QuestStatusCheck = GetQuestStatus(UID, 177) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEMA = HowmuchItem(UID, 910040000);
	ITEMB = HowmuchItem(UID, 910041000);
	if (ITEMA < 3) then 
		SelectMsg(UID, 2, 177, 671, NPC, 18, 213);
	elseif (ITEMB < 1) then
		SelectMsg(UID, 2, 177, 672, NPC, 18, 213);
	else
	RunQuestExchange(UID,88)
	SaveEvent(UID, 441);
end
end
end

local savenum = 199;

if (EVENT == 6092) then
	SelectMsg(UID, 2, savenum, 6041, NPC, 6007, 6093, 4005, -1);
end

if (EVENT == 6093) then
	ITEM_COUNT = HowmuchItem(UID, 910135000);   
	ITEM_COUNT1 = HowmuchItem(UID, 910138000);
	if (ITEM_COUNT < 1 or ITEM_COUNT1 < 3) then
		SelectMsg(UID, 4, savenum, 6043, NPC, 4543, 6095, 4191, -1);
	elseif (ITEM_COUNT > 0 and ITEM_COUNT1 > 2) then
		SelectMsg(UID, 5, savenum, 6049, NPC, 4006, 7004,4005, -1);
	end
end

if (EVENT == 6095) then
	MonsterStoneQuestJoin(UID,199);
	EVENT = 6096
end

if (EVENT == 6096) then
	Class = CheckClass (UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 6079);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 6085);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 6091);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 6097);
	end
end

if (EVENT == 7000) then
	Class = CheckClass (UID);
	if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then
		SaveEvent(UID, 6081);
	elseif (Class == 2 or Class == 7 or Class == 8) then
		SaveEvent(UID, 6087);
	elseif (Class == 3 or Class == 9 or Class == 10) then
		SaveEvent(UID, 6093);
	elseif (Class == 4 or Class == 11 or Class == 12) then
		SaveEvent(UID, 6099);
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
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910135000);   
	ITEM_COUNT1 = HowmuchItem(UID, 910138000);
	if (ITEM_COUNT < 1 or ITEM_COUNT1 < 3) then
		SelectMsg(UID, 4, savenum, 6043, NPC, 4543, 6095, 4191, -1);
		else
	RunQuestExchange(UID,94,STEP,1);
	SaveEvent(UID, 6080);
end 
end
end

if (EVENT == 7006) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910135000);   
	ITEM_COUNT1 = HowmuchItem(UID, 910138000);
	if (ITEM_COUNT < 1 or ITEM_COUNT1 < 3) then
		SelectMsg(UID, 4, savenum, 6043, NPC, 4543, 6095, 4191, -1);
		else
	RunQuestExchange(UID,95,STEP,1);
	SaveEvent(UID, 6086); 
end
end
end

if (EVENT == 7007) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910135000);   
	ITEM_COUNT1 = HowmuchItem(UID, 910138000);
	if (ITEM_COUNT < 1 or ITEM_COUNT1 < 3) then
		SelectMsg(UID, 4, savenum, 6043, NPC, 4543, 6095, 4191, -1);
		else
	RunQuestExchange(UID,96,STEP,1);
	SaveEvent(UID, 6092);
end
end
end

if (EVENT == 7008) then
	QuestStatusCheck = GetQuestStatus(UID, 199) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910135000);   
	ITEM_COUNT1 = HowmuchItem(UID, 910138000);
	if (ITEM_COUNT < 1 or ITEM_COUNT1 < 3) then
		SelectMsg(UID, 4, savenum, 6043, NPC, 4543, 6095, 4191, -1);
		else
	RunQuestExchange(UID,97,STEP,1);
	SaveEvent(UID, 6098); 
end
end
end

if (EVENT == 532) then
	SelectMsg(UID, 4, 220, 4296, NPC, 22, 533, 23, -1);
end

if (EVENT == 533) then
	Check = isRoomForItem(UID, 910050000);
	if (Check == -1) then
		SelectMsg(UID, 2, -1, 6481, NPC, 27, -1);
	else
		GiveItem(UID, 910050000, 1);
		SaveEvent(UID, 4211);
	end
end

if (EVENT == 534) then
	SaveEvent(UID, 4214);
end

if (EVENT == 538) then
	SaveEvent(UID, 4213);
end

if (EVENT == 536) then
	ITEM7 = HowmuchItem(UID, 910057000);
	if (ITEM7 > 0) then
		SelectMsg(UID, 4, 220, 4297, NPC, 4172, 537, 4173, -1);
	else
		SelectMsg(UID, 2, 220, 4298, NPC, 18, 192);
	end
end 

if (EVENT == 192) then
	ShowMap(UID, 439);
end

if (EVENT == 537) then
	QuestStatusCheck = GetQuestStatus(UID, 220) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM7 = HowmuchItem(UID, 910057000);
	if (ITEM7 == 0) then
	SelectMsg(UID, 2, 220, 4298, NPC, 18, 192);
	else
	RunQuestExchange(UID,470)
	SaveEvent(UID, 4212);
end
end
end

if (EVENT == 1000) then 
	SaveEvent(UID, 2458);
end

if (EVENT == 1001) then
	SelectMsg(UID, 4, 494, 9239, NPC, 22, 1002, 23, -1);
end

if (EVENT == 1002) then
	SaveEvent(UID, 2459);
end   

if (EVENT == 1003) then
	SaveEvent(UID, 2462);
end

if (EVENT == 1006) then
	SaveEvent(UID, 2461);
end

if (EVENT == 1007) then
	ITEMBDW = HowmuchItem(UID, 900143000);
	if (ITEMBDW < 1) then
		SelectMsg(UID, 2, 494, 9239, NPC, 18, 191);
	else
		SelectMsg(UID, 4, 494, 9239, NPC, 4006, 1008, 4005, -1);
	end
end

if (EVENT == 191) then
	ShowMap(UID, 39);
end

if (EVENT == 1008) then
	QuestStatusCheck = GetQuestStatus(UID, 494) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEMBDW = HowmuchItem(UID, 900143000);
	if (ITEMBDW < 1 or ITEMBDW == 0) then
		SelectMsg(UID, 2, 494, 9239, NPC, 18, 191);
	else
	RunQuestExchange(UID,222)
	SaveEvent(UID, 2460);
end
end
end

if (EVENT == 400) then
	SelectMsg(UID, 4, 438, 4997, NPC, 10, 401, 4005, -1);
end

if (EVENT == 401) then
	QuestStatusCheck = GetQuestStatus(UID, 438) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
    SelectMsg(UID, 15, -1, -1, NPC);
    RunQuestExchange(UID,54)
	SaveEvent(UID, 7112);
end
end

if (EVENT == 410) then
	SelectMsg(UID, 2, 439, 4985, NPC, 10, 411, 4005, -1);
end

if (EVENT == 411) then
	SaveEvent(UID, 7129);
end

if (EVENT == 412) then
	SelectMsg(UID, 2, 443, 4985, NPC, 10, 413, 4005, -1);
end

if (EVENT == 413) then
	SaveEvent(UID, 7151);
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 522, 20013, NPC, 22, 1103, 27, -1);
end

if (EVENT == 1103) then
	SaveEvent(UID, 11110);
end

if (EVENT == 1104) then
		SelectMsg(UID, 4, 522, 20013, NPC, 22, 1105, 27, -1);
		SaveEvent(UID, 11112);
end

if (EVENT == 1105) then
SelectMsg(UID, 2, 522, 20209, NPC, 10, -1);
	SaveEvent(UID, 11111);
	SaveEvent(UID, 11122);
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 525, 20019, NPC, 22, 1203, 27, -1);
end

if (EVENT == 1203) then
	SaveEvent(UID, 11146);
end

if (EVENT == 1208) then
	SaveEvent(UID, 11148);
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
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910214000);   
	if (ITEM_COUNT < 1 or ITEM_COUNT == 0) then
		SelectMsg(UID, 2, 525, 20019, NPC, 18, -1);
	else
	RunQuestExchange(UID,3012);
	SaveEvent(UID,11147);
	SaveEvent(UID,11158);
end
end
end

if (EVENT == 1302) then
	SelectMsg(UID, 4, 526, 20021, NPC, 22, 1303, 27, -1);
end

if (EVENT == 1303) then
	SaveEvent(UID, 11158);
end

if (EVENT == 1308) then
	SaveEvent(UID, 11160);
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
	ShowMap(UID, 729);
end

if (EVENT == 1307)then
	QuestStatusCheck = GetQuestStatus(UID, 526) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910195000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 526, 20021, NPC, 18,1306);
	else
	RunQuestExchange(UID,3013);
	SaveEvent(UID,11159);
end
end
end

if (EVENT == 1402) then
	SelectMsg(UID, 2, 527, 20217, NPC, 4161, 1403);
end

if (EVENT == 1403) then
	SelectMsg(UID, 2, 527, 20218, NPC, 4552, 1404);
end

if (EVENT == 1404) then
	SelectMsg(UID, 4, 527, 20218, NPC, 22,1405,27,-1);
	SaveEvent(UID,11170);
	SaveEvent(UID,11172);
end

if (EVENT == 1405) then
	SaveEvent(UID,11171);
	SaveEvent(UID,11182);
end

if (EVENT == 1502) then
	SelectMsg(UID, 2, 535, 20039, NPC, 4161, 1504);
end

if (EVENT == 1503) then
	SelectMsg(UID, 2, 535, 20273, NPC, 4552, 1504);
end

if (EVENT == 1504) then
	SelectMsg(UID, 4, 535, 20039, NPC, 22,1505,27,-1);
	SaveEvent(UID,11266);
	SaveEvent(UID,11268);
end

if (EVENT == 1505) then
	SaveEvent(UID,11267);
	SaveEvent(UID,11278);
end

if (EVENT == 1602) then
	SelectMsg(UID, 4, 536, 20041, NPC, 22, 1603, 27, -1);
end

if (EVENT == 1603) then
	SaveEvent(UID,11278);
end

if (EVENT == 1608) then
	SaveEvent(UID,11280);
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
	ShowMap(UID, 731);
end

if (EVENT == 1607) then
	QuestStatusCheck = GetQuestStatus(UID, 536) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM_COUNT = HowmuchItem(UID, 910196000);   
	if (ITEM_COUNT < 1) then
		SelectMsg(UID, 2, 536, 20041, NPC, 18,1606);
	else
RunQuestExchange(UID,3023);
SaveEvent(UID,11279);
end
end
end

if (EVENT == 1702) then
	SelectMsg(UID, 4, 542, 20053, NPC, 22, 1703, 27, -1);
end

if (EVENT == 1703) then
	SaveEvent(UID,11350);
end

if (EVENT == 1708) then
	SaveEvent(UID,11352);
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
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 910227000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 542, 20053, NPC, 18,-1);
	else
	RunQuestExchange(UID,3029);
	SaveEvent(UID,11351);
	SaveEvent(UID,11362);
end
end
end

if (EVENT == 1802) then
	SelectMsg(UID, 4, 543, 20055, NPC, 22, 1803, 27, -1);
end

if (EVENT == 1803) then
	SaveEvent(UID,11362);
end

if (EVENT == 1808) then
	SaveEvent(UID,11364);
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
	ShowMap(UID, 509)
end

if (EVENT == 1806)then
	QuestStatusCheck = GetQuestStatus(UID, 543) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 508107000);   
	if (ITEM1_COUNT < 5) then
		SelectMsg(UID, 2, 543, 20055, NPC, 18,1804);
	else
	RunQuestExchange(UID,3030,STEP,1);
	SaveEvent(UID,11363);
	SaveEvent(UID,11374);
end
end
end

if (EVENT == 1902) then
	SelectMsg(UID, 4, 544, 20057, NPC, 22, 1903, 27, -1);
end

if (EVENT == 1903) then
	SaveEvent(UID,11374);
end

if (EVENT == 1908) then
	SaveEvent(UID,11376);
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
	ShowMap(UID, 374)
end

if (EVENT == 1907)then
	QuestStatusCheck = GetQuestStatus(UID, 544) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 544, 1);
	if (MonsterCount < 40) then
		SelectMsg(UID, 2, 544, 20057, NPC, 18, 1904);
	else
	RunQuestExchange(UID,3031);
	SaveEvent(UID,11375);
	SaveEvent(UID,11386);
end
end
end

if (EVENT == 2002) then
	SelectMsg(UID, 4, 545, 20059, NPC, 22, 2003, 27, -1);
end

if (EVENT == 2003) then
	SaveEvent(UID,11386);
end

if (EVENT == 2008) then
	SaveEvent(UID,11388);
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
	ShowMap(UID, 733);
end

if (EVENT == 2006)then
	QuestStatusCheck = GetQuestStatus(UID, 545) 
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 673, NPC, 10, -1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 910197000);   
	if (ITEM1_COUNT < 1) then
		SelectMsg(UID, 2, 545, 20059, NPC, 18,2004);
	else
	RunQuestExchange(UID,3032);
	SaveEvent(UID,11387);
end
end
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=199 status=2 n_index=6080
if (EVENT == 193) then
	QuestStatusCheck = GetQuestStatus(UID, 199)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		SelectMsg(UID, 5, 199, 6049, NPC, 4006, 7004, 4005, -1);
	end
end

-- [AUTO-GEN] quest=438 status=2 n_index=7112
if (EVENT == 240) then
	QuestStatusCheck = GetQuestStatus(UID, 438)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 54);
		SaveEvent(UID, 7114);
	end
end

-- [AUTO-GEN] quest=439 status=255 n_index=7126
if (EVENT == 405) then
	SaveEvent(UID, 7127);
end

-- [AUTO-GEN] quest=220 status=255 n_index=5099
if (EVENT == 530) then
	SaveEvent(UID, 4210);
end

-- [AUTO-GEN] quest=522 status=255 n_index=11108
if (EVENT == 1100) then
	SaveEvent(UID, 11109);
end

-- [AUTO-GEN] quest=525 status=255 n_index=11144
if (EVENT == 1200) then
	SaveEvent(UID, 11145);
end

-- [AUTO-GEN] quest=526 status=255 n_index=11156
if (EVENT == 1300) then
	SaveEvent(UID, 11157);
end

-- [AUTO-GEN] quest=527 status=255 n_index=11168
if (EVENT == 1400) then
	SaveEvent(UID, 11169);
end

-- [AUTO-GEN] quest=535 status=255 n_index=11264
if (EVENT == 1500) then
	SaveEvent(UID, 11265);
end

-- [AUTO-GEN] quest=536 status=255 n_index=11276
if (EVENT == 1600) then
	SaveEvent(UID, 11277);
end

-- [AUTO-GEN] quest=542 status=255 n_index=11348
if (EVENT == 1700) then
	SaveEvent(UID, 11349);
end

-- [AUTO-GEN] quest=543 status=255 n_index=11360
if (EVENT == 1800) then
	SaveEvent(UID, 11361);
end

-- [AUTO-GEN] quest=544 status=255 n_index=11372
if (EVENT == 1900) then
	SaveEvent(UID, 11373);
end

-- [AUTO-GEN] quest=545 status=255 n_index=11384
if (EVENT == 2000) then
	SaveEvent(UID, 11385);
end

-- [AUTO-GEN] quest=555 status=255 n_index=11534
if (EVENT == 2100) then
	SaveEvent(UID, 11535);
end

-- [AUTO-GEN] quest=555 status=0 n_index=11535
if (EVENT == 2102) then
	SelectMsg(UID, 4, 555, 20079, NPC, 3079, 2103, 23, -1);
end

-- [AUTO-GEN] quest=555 status=0 n_index=11535
if (EVENT == 2103) then
	SaveEvent(UID, 11536);
end

-- [AUTO-GEN] quest=555 status=1 n_index=11536
if (EVENT == 2105) then
	ItemA = HowmuchItem(UID, 910230000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 555, 20079, NPC, 18, 2106);
	else
		SelectMsg(UID, 4, 555, 20079, NPC, 41, 2108, 27, -1);
	end
end

-- [AUTO-GEN] quest=555 status=1 n_index=11536
if (EVENT == 2106) then
	ShowMap(UID, 12);
end

-- [AUTO-GEN] quest=555 status=1 n_index=11536
if (EVENT == 2108) then
	QuestStatusCheck = GetQuestStatus(UID, 555)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3045);
		SaveEvent(UID, 11537);
	end
end

-- [AUTO-GEN] quest=556 status=255 n_index=11546
if (EVENT == 2200) then
	SaveEvent(UID, 11547);
end

-- [AUTO-GEN] quest=556 status=0 n_index=11547
if (EVENT == 2202) then
	SelectMsg(UID, 4, 556, 20081, NPC, 3081, 2203, 23, -1);
end

-- [AUTO-GEN] quest=556 status=0 n_index=11547
if (EVENT == 2203) then
	SaveEvent(UID, 11548);
end

-- [AUTO-GEN] quest=556 status=1 n_index=11548
if (EVENT == 2205) then
	ItemA = HowmuchItem(UID, 910198000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 556, 20081, NPC, 18, 2206);
	else
		SelectMsg(UID, 4, 556, 20081, NPC, 41, 2208, 27, -1);
	end
end

-- [AUTO-GEN] quest=556 status=1 n_index=11548
if (EVENT == 2206) then
	ShowMap(UID, 2);
end

-- [AUTO-GEN] quest=556 status=1 n_index=11548
if (EVENT == 2208) then
	QuestStatusCheck = GetQuestStatus(UID, 556)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3046);
		SaveEvent(UID, 11549);
	end
end

-- [AUTO-GEN] quest=563 status=255 n_index=11630
if (EVENT == 2300) then
	SaveEvent(UID, 11631);
end

-- [AUTO-GEN] quest=563 status=0 n_index=11631
if (EVENT == 2302) then
	SelectMsg(UID, 4, 563, 20095, NPC, 3095, 2303, 23, -1);
end

-- [AUTO-GEN] quest=563 status=0 n_index=11631
if (EVENT == 2303) then
	SaveEvent(UID, 11632);
end

-- [AUTO-GEN] quest=563 status=1 n_index=11632
if (EVENT == 2305) then
	ItemA = HowmuchItem(UID, 910231000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 563, 20095, NPC, 18, 2306);
	else
		SelectMsg(UID, 4, 563, 20095, NPC, 41, 2308, 27, -1);
	end
end

-- [AUTO-GEN] quest=563 status=1 n_index=11632
if (EVENT == 2306) then
	ShowMap(UID, 12);
end

-- [AUTO-GEN] quest=563 status=1 n_index=11632
if (EVENT == 2308) then
	QuestStatusCheck = GetQuestStatus(UID, 563)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3053);
		SaveEvent(UID, 11633);
	end
end

-- [AUTO-GEN] quest=565 status=255 n_index=11654
if (EVENT == 2400) then
	SaveEvent(UID, 11655);
end

-- [AUTO-GEN] quest=565 status=0 n_index=11655
if (EVENT == 2402) then
	SelectMsg(UID, 4, 565, 20099, NPC, 3099, 2403, 23, -1);
end

-- [AUTO-GEN] quest=565 status=0 n_index=11655
if (EVENT == 2403) then
	SaveEvent(UID, 11656);
end

-- [AUTO-GEN] quest=565 status=1 n_index=11656
if (EVENT == 2405) then
	ItemA = HowmuchItem(UID, 910233000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 565, 20099, NPC, 18, 2406);
	else
		SelectMsg(UID, 4, 565, 20099, NPC, 41, 2408, 27, -1);
	end
end

-- [AUTO-GEN] quest=565 status=1 n_index=11656
if (EVENT == 2406) then
	ShowMap(UID, 2);
end

-- [AUTO-GEN] quest=565 status=1 n_index=11656
if (EVENT == 2408) then
	QuestStatusCheck = GetQuestStatus(UID, 565)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3055);
		SaveEvent(UID, 11657);
	end
end

-- [AUTO-GEN] quest=566 status=255 n_index=11666
if (EVENT == 2500) then
	SaveEvent(UID, 11667);
end

-- [AUTO-GEN] quest=566 status=0 n_index=11667
if (EVENT == 2502) then
	SelectMsg(UID, 4, 566, 20101, NPC, 3101, 2503, 23, -1);
end

-- [AUTO-GEN] quest=566 status=0 n_index=11667
if (EVENT == 2503) then
	SaveEvent(UID, 11668);
end

-- [AUTO-GEN] quest=566 status=1 n_index=11668
if (EVENT == 2505) then
	ItemA = HowmuchItem(UID, 910199000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 566, 20101, NPC, 18, 2506);
	else
		SelectMsg(UID, 4, 566, 20101, NPC, 41, 2508, 27, -1);
	end
end

-- [AUTO-GEN] quest=566 status=1 n_index=11668
if (EVENT == 2506) then
	ShowMap(UID, 2);
end

-- [AUTO-GEN] quest=566 status=1 n_index=11668
if (EVENT == 2508) then
	QuestStatusCheck = GetQuestStatus(UID, 566)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3056);
		SaveEvent(UID, 11669);
	end
end

-- [AUTO-GEN] quest=574 status=255 n_index=11761
if (EVENT == 2600) then
	SaveEvent(UID, 11762);
end

-- [AUTO-GEN] quest=574 status=0 n_index=11762
if (EVENT == 2602) then
	SelectMsg(UID, 4, 574, 20117, NPC, 3117, 2603, 23, -1);
end

-- [AUTO-GEN] quest=574 status=0 n_index=11762
if (EVENT == 2603) then
	SaveEvent(UID, 11763);
end

-- [AUTO-GEN] quest=574 status=1 n_index=11763
if (EVENT == 2605) then
	ItemA = HowmuchItem(UID, 910235000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 574, 20117, NPC, 18, 2606);
	else
		SelectMsg(UID, 4, 574, 20117, NPC, 41, 2608, 27, -1);
	end
end

-- [AUTO-GEN] quest=574 status=1 n_index=11763
if (EVENT == 2606) then
	ShowMap(UID, 2);
end

-- [AUTO-GEN] quest=574 status=1 n_index=11763
if (EVENT == 2608) then
	QuestStatusCheck = GetQuestStatus(UID, 574)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3064);
		SaveEvent(UID, 11764);
	end
end

-- [AUTO-GEN] quest=575 status=255 n_index=11773
if (EVENT == 2700) then
	SaveEvent(UID, 11774);
end

-- [AUTO-GEN] quest=575 status=0 n_index=11774
if (EVENT == 2702) then
	SelectMsg(UID, 4, 575, 20119, NPC, 3119, 2703, 23, -1);
end

-- [AUTO-GEN] quest=575 status=0 n_index=11774
if (EVENT == 2703) then
	SaveEvent(UID, 11775);
end

-- [AUTO-GEN] quest=575 status=1 n_index=11775
if (EVENT == 2705) then
	ItemA = HowmuchItem(UID, 910200000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 575, 20119, NPC, 18, 2706);
	else
		SelectMsg(UID, 4, 575, 20119, NPC, 41, 2708, 27, -1);
	end
end

-- [AUTO-GEN] quest=575 status=1 n_index=11775
if (EVENT == 2706) then
	ShowMap(UID, 2);
end

-- [AUTO-GEN] quest=575 status=1 n_index=11775
if (EVENT == 2708) then
	QuestStatusCheck = GetQuestStatus(UID, 575)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3065);
		SaveEvent(UID, 11776);
	end
end

-- [AUTO-GEN] quest=580 status=255 n_index=11834
if (EVENT == 2800) then
	SaveEvent(UID, 11835);
end

-- [AUTO-GEN] quest=580 status=0 n_index=11835
if (EVENT == 2802) then
	SelectMsg(UID, 4, 580, 20731, NPC, 3129, 2803, 23, -1);
end

-- [AUTO-GEN] quest=580 status=1 n_index=11836
if (EVENT == 2803) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 580, 20731, NPC, 18, 2805);
	else
		SelectMsg(UID, 4, 580, 20731, NPC, 41, 2804, 27, -1);
	end
end

-- [AUTO-GEN] quest=580 status=1 n_index=11836
if (EVENT == 2804) then
	QuestStatusCheck = GetQuestStatus(UID, 580)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3070);
		SaveEvent(UID, 11837);
	end
end

-- [AUTO-GEN] quest=580 status=3 n_index=11838
if (EVENT == 2805) then
	SelectMsg(UID, 2, 580, 20731, NPC, 10, -1);
end

-- [AUTO-GEN] quest=586 status=255 n_index=11906
if (EVENT == 2900) then
	SaveEvent(UID, 11907);
end

-- [AUTO-GEN] quest=586 status=0 n_index=11907
if (EVENT == 2902) then
	SelectMsg(UID, 4, 586, 20743, NPC, 3141, 2903, 23, -1);
end

-- [AUTO-GEN] quest=586 status=1 n_index=11908
if (EVENT == 2903) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 586, 20743, NPC, 18, 2905);
	else
		SelectMsg(UID, 4, 586, 20743, NPC, 41, 2904, 27, -1);
	end
end

-- [AUTO-GEN] quest=586 status=1 n_index=11908
if (EVENT == 2904) then
	QuestStatusCheck = GetQuestStatus(UID, 586)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3076);
		SaveEvent(UID, 11909);
	end
end

-- [AUTO-GEN] quest=586 status=3 n_index=11910
if (EVENT == 2905) then
	SelectMsg(UID, 2, 586, 20743, NPC, 10, -1);
end

-- [AUTO-GEN] quest=587 status=255 n_index=11918
if (EVENT == 3000) then
	SaveEvent(UID, 11919);
end

-- [AUTO-GEN] quest=587 status=0 n_index=11919
if (EVENT == 3002) then
	SelectMsg(UID, 4, 587, 20745, NPC, 3143, 3003, 23, -1);
end

-- [AUTO-GEN] quest=587 status=0 n_index=11919
if (EVENT == 3003) then
	SaveEvent(UID, 11920);
end

-- [AUTO-GEN] quest=587 status=1 n_index=11920
if (EVENT == 3005) then
	ItemA = HowmuchItem(UID, 910205000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 587, 20745, NPC, 18, 3006);
	else
		SelectMsg(UID, 4, 587, 20745, NPC, 41, 3008, 27, -1);
	end
end

-- [AUTO-GEN] quest=587 status=1 n_index=11920
if (EVENT == 3006) then
	ShowMap(UID, 2);
end

-- [AUTO-GEN] quest=587 status=1 n_index=11920
if (EVENT == 3008) then
	QuestStatusCheck = GetQuestStatus(UID, 587)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3077);
		SaveEvent(UID, 11921);
	end
end

-- [AUTO-GEN] quest=594 status=255 n_index=11990
if (EVENT == 3100) then
	SaveEvent(UID, 11991);
end

-- [AUTO-GEN] quest=594 status=0 n_index=11991
if (EVENT == 3102) then
	SelectMsg(UID, 4, 594, 20757, NPC, 3155, 3103, 23, -1);
end

-- [AUTO-GEN] quest=594 status=0 n_index=11991
if (EVENT == 3103) then
	SaveEvent(UID, 11992);
end

-- [AUTO-GEN] quest=594 status=1 n_index=11992
if (EVENT == 3105) then
	ItemA = HowmuchItem(UID, 910237000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 594, 20757, NPC, 18, 3106);
	else
		SelectMsg(UID, 4, 594, 20757, NPC, 41, 3106, 27, -1);
	end
end

-- [AUTO-GEN] quest=594 status=1 n_index=11992
if (EVENT == 3106) then
	QuestStatusCheck = GetQuestStatus(UID, 594)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3083);
		SaveEvent(UID, 11993);
	end
end

-- [AUTO-GEN] quest=597 status=255 n_index=12026
if (EVENT == 3200) then
	SaveEvent(UID, 12027);
end

-- [AUTO-GEN] quest=597 status=0 n_index=12027
if (EVENT == 3202) then
	SelectMsg(UID, 4, 597, 20763, NPC, 3161, 3203, 23, -1);
end

-- [AUTO-GEN] quest=597 status=1 n_index=12028
if (EVENT == 3203) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 597, 20763, NPC, 18, 3205);
	else
		SelectMsg(UID, 4, 597, 20763, NPC, 41, 3204, 27, -1);
	end
end

-- [AUTO-GEN] quest=597 status=1 n_index=12028
if (EVENT == 3204) then
	QuestStatusCheck = GetQuestStatus(UID, 597)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3086);
		SaveEvent(UID, 12029);
	end
end

-- [AUTO-GEN] quest=597 status=3 n_index=12030
if (EVENT == 3205) then
	SelectMsg(UID, 2, 597, 20763, NPC, 10, -1);
end

-- [AUTO-GEN] quest=598 status=255 n_index=12038
if (EVENT == 3300) then
	SaveEvent(UID, 12039);
end

-- [AUTO-GEN] quest=598 status=0 n_index=12039
if (EVENT == 3302) then
	SelectMsg(UID, 4, 598, 20765, NPC, 3163, 3303, 23, -1);
end

-- [AUTO-GEN] quest=598 status=0 n_index=12039
if (EVENT == 3303) then
	SaveEvent(UID, 12040);
end

-- [AUTO-GEN] quest=598 status=1 n_index=12040
if (EVENT == 3305) then
	ItemA = HowmuchItem(UID, 910206000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 598, 20765, NPC, 18, 3306);
	else
		SelectMsg(UID, 4, 598, 20765, NPC, 41, 3308, 27, -1);
	end
end

-- [AUTO-GEN] quest=598 status=1 n_index=12040
if (EVENT == 3306) then
	ShowMap(UID, 2);
end

-- [AUTO-GEN] quest=598 status=1 n_index=12040
if (EVENT == 3308) then
	QuestStatusCheck = GetQuestStatus(UID, 598)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3087);
		SaveEvent(UID, 12041);
	end
end

-- [AUTO-GEN] quest=607 status=255 n_index=12146
if (EVENT == 3400) then
	SaveEvent(UID, 12147);
end

-- [AUTO-GEN] quest=607 status=0 n_index=12147
if (EVENT == 3402) then
	SelectMsg(UID, 4, 607, 20783, NPC, 3181, 3403, 23, -1);
end

-- [AUTO-GEN] quest=607 status=0 n_index=12147
if (EVENT == 3403) then
	SaveEvent(UID, 12148);
end

-- [AUTO-GEN] quest=607 status=1 n_index=12148
if (EVENT == 3405) then
	ItemA = HowmuchItem(UID, 910241000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 607, 20783, NPC, 18, 3406);
	else
		SelectMsg(UID, 4, 607, 20783, NPC, 41, 3406, 27, -1);
	end
end

-- [AUTO-GEN] quest=607 status=1 n_index=12148
if (EVENT == 3406) then
	QuestStatusCheck = GetQuestStatus(UID, 607)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3096);
		SaveEvent(UID, 12149);
	end
end

-- [AUTO-GEN] quest=608 status=255 n_index=12158
if (EVENT == 3500) then
	SaveEvent(UID, 12159);
end

-- [AUTO-GEN] quest=608 status=0 n_index=12159
if (EVENT == 3502) then
	SelectMsg(UID, 4, 608, 20785, NPC, 3183, 3503, 23, -1);
end

-- [AUTO-GEN] quest=608 status=0 n_index=12159
if (EVENT == 3503) then
	SaveEvent(UID, 12160);
end

-- [AUTO-GEN] quest=608 status=1 n_index=12160
if (EVENT == 3505) then
	ItemA = HowmuchItem(UID, 910207000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 608, 20785, NPC, 18, 3506);
	else
		SelectMsg(UID, 4, 608, 20785, NPC, 41, 3506, 27, -1);
	end
end

-- [AUTO-GEN] quest=608 status=1 n_index=12160
if (EVENT == 3506) then
	QuestStatusCheck = GetQuestStatus(UID, 608)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3097);
		SaveEvent(UID, 12161);
	end
end

-- [AUTO-GEN] quest=614 status=255 n_index=12230
if (EVENT == 3600) then
	SaveEvent(UID, 12231);
end

-- [AUTO-GEN] quest=614 status=0 n_index=12231
if (EVENT == 3602) then
	SelectMsg(UID, 4, 614, 20797, NPC, 3195, 3603, 23, -1);
end

-- [AUTO-GEN] quest=614 status=1 n_index=12232
if (EVENT == 3603) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 614, 20797, NPC, 18, 3605);
	else
		SelectMsg(UID, 4, 614, 20797, NPC, 41, 3604, 27, -1);
	end
end

-- [AUTO-GEN] quest=614 status=1 n_index=12232
if (EVENT == 3604) then
	QuestStatusCheck = GetQuestStatus(UID, 614)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3103);
		SaveEvent(UID, 12233);
	end
end

-- [AUTO-GEN] quest=614 status=3 n_index=12234
if (EVENT == 3605) then
	SelectMsg(UID, 2, 614, 20797, NPC, 10, -1);
end

-- [AUTO-GEN] quest=617 status=255 n_index=12266
if (EVENT == 3700) then
	SaveEvent(UID, 12267);
end

-- [AUTO-GEN] quest=617 status=0 n_index=12267
if (EVENT == 3702) then
	SelectMsg(UID, 4, 617, 20803, NPC, 3201, 3703, 23, -1);
end

-- [AUTO-GEN] quest=617 status=0 n_index=12267
if (EVENT == 3703) then
	SaveEvent(UID, 12268);
end

-- [AUTO-GEN] quest=617 status=1 n_index=12268
if (EVENT == 3705) then
	ItemA = HowmuchItem(UID, 910244000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 617, 20803, NPC, 18, 3706);
	else
		SelectMsg(UID, 4, 617, 20803, NPC, 41, 3706, 27, -1);
	end
end

-- [AUTO-GEN] quest=617 status=1 n_index=12268
if (EVENT == 3706) then
	QuestStatusCheck = GetQuestStatus(UID, 617)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3106);
		SaveEvent(UID, 12269);
	end
end

-- [AUTO-GEN] quest=618 status=255 n_index=12278
if (EVENT == 3800) then
	SaveEvent(UID, 12279);
end

-- [AUTO-GEN] quest=618 status=0 n_index=12279
if (EVENT == 3802) then
	SelectMsg(UID, 4, 618, 20805, NPC, 3203, 3803, 23, -1);
end

-- [AUTO-GEN] quest=618 status=0 n_index=12279
if (EVENT == 3803) then
	SaveEvent(UID, 12280);
end

-- [AUTO-GEN] quest=618 status=1 n_index=12280
if (EVENT == 3805) then
	ItemA = HowmuchItem(UID, 910208000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 618, 20805, NPC, 18, 3806);
	else
		SelectMsg(UID, 4, 618, 20805, NPC, 41, 3806, 27, -1);
	end
end

-- [AUTO-GEN] quest=618 status=1 n_index=12280
if (EVENT == 3806) then
	QuestStatusCheck = GetQuestStatus(UID, 618)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3107);
		SaveEvent(UID, 12281);
	end
end

-- [AUTO-GEN] quest=199 status=255 n_index=6077
if (EVENT == 6090) then
	SaveEvent(UID, 6078);
end

