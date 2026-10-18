local NPC = 31518;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 21715, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 21715, NPC)
	else
		EVENT = QuestNum
	end
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=558 status=2 n_index=11567
if (EVENT == 101) then
	QuestStatusCheck = GetQuestStatus(UID, 558)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3048);
		SaveEvent(UID, 11569);
	end
end

-- [AUTO-GEN] quest=558 status=255 n_index=11564
if (EVENT == 1000) then
	SaveEvent(UID, 11565);
end

-- [AUTO-GEN] quest=558 status=0 n_index=11565
if (EVENT == 1002) then
	SelectMsg(UID, 4, 558, 20084, NPC, 3084, 1003, 23, -1);
end

-- [AUTO-GEN] quest=558 status=1 n_index=11566
if (EVENT == 1003) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 558, 20084, NPC, 18, 1005);
	else
		SelectMsg(UID, 4, 558, 20084, NPC, 41, 1004, 27, -1);
	end
end

-- [AUTO-GEN] quest=558 status=1 n_index=11566
if (EVENT == 1004) then
	QuestStatusCheck = GetQuestStatus(UID, 558)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3048);
		SaveEvent(UID, 11567);
	end
end

-- [AUTO-GEN] quest=558 status=3 n_index=11568
if (EVENT == 1005) then
	SelectMsg(UID, 2, 558, 20084, NPC, 10, -1);
end

-- [AUTO-GEN] quest=559 status=255 n_index=11576
if (EVENT == 1100) then
	SaveEvent(UID, 11577);
end

-- [AUTO-GEN] quest=559 status=0 n_index=11577
if (EVENT == 1102) then
	SelectMsg(UID, 4, 559, 20086, NPC, 3086, 1103, 23, -1);
end

-- [AUTO-GEN] quest=559 status=0 n_index=11577
if (EVENT == 1103) then
	SaveEvent(UID, 11578);
end

-- [AUTO-GEN] quest=559 status=1 n_index=11578
if (EVENT == 1105) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 5, 559, 20086, NPC, 22, 1106, 23, -1);
	else
		SelectMsg(UID, 2, 559, 20086, NPC, 18, 1106);
	end
end

-- [AUTO-GEN] quest=559 status=1 n_index=11578
if (EVENT == 1106) then
	QuestStatusCheck = GetQuestStatus(UID, 559)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3049);
		SaveEvent(UID, 11579);
	end
end

-- [AUTO-GEN] quest=560 status=255 n_index=11588
if (EVENT == 1200) then
	SaveEvent(UID, 11589);
end

-- [AUTO-GEN] quest=560 status=0 n_index=11589
if (EVENT == 1202) then
	SelectMsg(UID, 4, 560, 20088, NPC, 3088, 1203, 23, -1);
end

-- [AUTO-GEN] quest=560 status=0 n_index=11589
if (EVENT == 1203) then
	SaveEvent(UID, 11590);
end

-- [AUTO-GEN] quest=560 status=1 n_index=11590
if (EVENT == 1205) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 5, 560, 20088, NPC, 22, 1206, 23, -1);
	else
		SelectMsg(UID, 2, 560, 20088, NPC, 18, 1206);
	end
end

-- [AUTO-GEN] quest=560 status=1 n_index=11590
if (EVENT == 1206) then
	QuestStatusCheck = GetQuestStatus(UID, 560)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3050);
		SaveEvent(UID, 11591);
	end
end

-- [AUTO-GEN] quest=561 status=255 n_index=11600
if (EVENT == 1300) then
	SaveEvent(UID, 11601);
end

-- [AUTO-GEN] quest=561 status=0 n_index=11601
if (EVENT == 1302) then
	SelectMsg(UID, 4, 561, 20090, NPC, 3090, 1303, 23, -1);
end

-- [AUTO-GEN] quest=561 status=0 n_index=11601
if (EVENT == 1303) then
	SaveEvent(UID, 11602);
end

-- [AUTO-GEN] quest=561 status=1 n_index=11602
if (EVENT == 1305) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 561, 20090, NPC, 22, 1306, 23, -1);
	else
		SelectMsg(UID, 2, 561, 20090, NPC, 18, 1306);
	end
end

-- [AUTO-GEN] quest=561 status=1 n_index=11602
if (EVENT == 1306) then
	QuestStatusCheck = GetQuestStatus(UID, 561)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		RunQuestExchange(UID, 3051);
		SaveEvent(UID, 11603);
	end
end

