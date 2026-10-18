local Ret = 0;
local NPC = 31506;

-- [Lunar Lady] Magpie
-- Auto-generated from sniffer capture (dialog_builder v4)
-- 5 menus, 6 mapped, 0 inferred, 1 unknown

-- ROOT: header=45268 flag=3
if (EVENT == 100) then
	SelectMsg(UID, 3, 1745, 45268, NPC, 40600, 101, 40599, 103);
end

-- header=44507 flag=2
if (EVENT == 101) then
	SelectMsg(UID, 2, 1745, 44507, NPC, 8419, 102, 40158, 3001 --[[ TODO: unknown ]]);
end

-- header=44509 flag=3
if (EVENT == 102) then
	SelectMsg(UID, 3, 1745, 44509, NPC, 27, 3001);
end

-- header=44506 flag=3
if (EVENT == 103) then
	SelectMsg(UID, 3, 1745, 44506, NPC, 4006, 104);
end

-- header=44510 flag=3
if (EVENT == 104) then
	SelectMsg(UID, 3, 1745, 44510, NPC, 27, 3001);
end

-- Close dialog
if (EVENT == 3001) then
	Ret = 1;
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=1745 status=0 n_index=14610
if (EVENT == 500) then
	SelectMsg(UID, 4, 1745, 45496, NPC, 3574, 501, 23, -1);
end

-- [AUTO-GEN] quest=1745 status=0 n_index=14610
if (EVENT == 501) then
	SaveEvent(UID, 14611);
end

-- [AUTO-GEN] quest=1745 status=1 n_index=14611
if (EVENT == 502) then
	QuestStatusCheck = GetQuestStatus(UID, 1745)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 16264)) then
			SaveEvent(UID, 14612);
		end
	end
end

-- [AUTO-GEN] quest=1745 status=1 n_index=14611
if (EVENT == 503) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1745, 45496, NPC, 22, 502, 23, -1);
	else
		SelectMsg(UID, 2, 1745, 45496, NPC, 18, 504);
	end
end

-- [AUTO-GEN] quest=1745 status=1 n_index=14611
if (EVENT == 504) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=960 status=0 n_index=6887
if (EVENT == 1000) then
	SelectMsg(UID, 4, 960, 0, NPC, 22, 1001, 23, -1);
end

-- [AUTO-GEN] quest=960 status=0 n_index=6887
if (EVENT == 1001) then
	SaveEvent(UID, 6888);
end

-- [AUTO-GEN] quest=960 status=1 n_index=6888
if (EVENT == 1002) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1681 status=0 n_index=14078
if (EVENT == 1301) then
	SelectMsg(UID, 4, 1681, 0, NPC, 22, 1302, 23, -1);
end

-- [AUTO-GEN] quest=1681 status=0 n_index=14078
if (EVENT == 1302) then
	SaveEvent(UID, 14079);
end

-- [AUTO-GEN] quest=1681 status=1 n_index=14079
if (EVENT == 1303) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1474 status=1 n_index=7660
if (EVENT == 1413) then
	QuestStatusCheck = GetQuestStatus(UID, 1474)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6225)) then
			SaveEvent(UID, 7661);
		end
	end
end

-- [AUTO-GEN] quest=1475 status=1 n_index=7665
if (EVENT == 1423) then
	QuestStatusCheck = GetQuestStatus(UID, 1475)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6226)) then
			SaveEvent(UID, 7666);
		end
	end
end

-- [AUTO-GEN] quest=1476 status=1 n_index=7670
if (EVENT == 1433) then
	QuestStatusCheck = GetQuestStatus(UID, 1476)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6227)) then
			SaveEvent(UID, 7671);
		end
	end
end

-- [AUTO-GEN] quest=1739 status=0 n_index=14568
if (EVENT == 3000) then
	SelectMsg(UID, 4, 1739, 45482, NPC, 3570, 3001, 23, -1);
end

-- [AUTO-GEN] quest=1662 status=0 n_index=10966
if (EVENT == 21000) then
	SelectMsg(UID, 4, 1662, 0, NPC, 22, 21001, 23, -1);
end

-- [AUTO-GEN] quest=1662 status=0 n_index=10966
if (EVENT == 21001) then
	SaveEvent(UID, 10966);
end

-- [AUTO-GEN] quest=1663 status=0 n_index=10968
if (EVENT == 22000) then
	SelectMsg(UID, 4, 1663, 0, NPC, 22, 22001, 23, -1);
end

-- [AUTO-GEN] quest=1663 status=0 n_index=10968
if (EVENT == 22001) then
	SaveEvent(UID, 10968);
end

-- [AUTO-GEN] quest=1664 status=0 n_index=10970
if (EVENT == 23000) then
	SelectMsg(UID, 4, 1664, 44905, NPC, 3543, 23001, 23, -1);
end

-- [AUTO-GEN] quest=1664 status=0 n_index=10970
if (EVENT == 23001) then
	SaveEvent(UID, 10971);
end

-- [AUTO-GEN] quest=1664 status=1 n_index=10971
if (EVENT == 23003) then
	QuestStatusCheck = GetQuestStatus(UID, 1664)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6823)) then
			SaveEvent(UID, 10972);
		end
	end
end

-- [AUTO-GEN] quest=1664 status=1 n_index=10971
if (EVENT == 23004) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1664, 44905, NPC, 22, 23003, 23, -1);
	else
		SelectMsg(UID, 2, 1664, 44905, NPC, 18, 23005);
	end
end

-- [AUTO-GEN] quest=1664 status=1 n_index=10971
if (EVENT == 23005) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1665 status=0 n_index=10975
if (EVENT == 24000) then
	SelectMsg(UID, 4, 1665, 44908, NPC, 3544, 24001, 23, -1);
end

-- [AUTO-GEN] quest=1665 status=0 n_index=10975
if (EVENT == 24001) then
	SaveEvent(UID, 10976);
end

-- [AUTO-GEN] quest=1665 status=1 n_index=10976
if (EVENT == 24003) then
	QuestStatusCheck = GetQuestStatus(UID, 1665)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6824)) then
			SaveEvent(UID, 10977);
		end
	end
end

-- [AUTO-GEN] quest=1665 status=1 n_index=10976
if (EVENT == 24004) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1665, 44908, NPC, 22, 24003, 23, -1);
	else
		SelectMsg(UID, 2, 1665, 44908, NPC, 18, 24005);
	end
end

-- [AUTO-GEN] quest=1665 status=1 n_index=10976
if (EVENT == 24005) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1666 status=0 n_index=10980
if (EVENT == 25000) then
	SelectMsg(UID, 4, 1666, 44908, NPC, 3545, 25001, 23, -1);
end

-- [AUTO-GEN] quest=1666 status=0 n_index=10980
if (EVENT == 25001) then
	SaveEvent(UID, 10981);
end

-- [AUTO-GEN] quest=1666 status=1 n_index=10981
if (EVENT == 25003) then
	QuestStatusCheck = GetQuestStatus(UID, 1666)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6825)) then
			SaveEvent(UID, 10982);
		end
	end
end

-- [AUTO-GEN] quest=1666 status=1 n_index=10981
if (EVENT == 25004) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1666, 44908, NPC, 22, 25003, 23, -1);
	else
		SelectMsg(UID, 2, 1666, 44908, NPC, 18, 25005);
	end
end

-- [AUTO-GEN] quest=1666 status=1 n_index=10981
if (EVENT == 25005) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1667 status=0 n_index=10985
if (EVENT == 26000) then
	SelectMsg(UID, 4, 1667, 44909, NPC, 3546, 26001, 23, -1);
end

-- [AUTO-GEN] quest=1667 status=0 n_index=10985
if (EVENT == 26001) then
	SaveEvent(UID, 10986);
end

-- [AUTO-GEN] quest=1667 status=1 n_index=10986
if (EVENT == 26003) then
	QuestStatusCheck = GetQuestStatus(UID, 1667)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6826)) then
			SaveEvent(UID, 10987);
		end
	end
end

-- [AUTO-GEN] quest=1667 status=1 n_index=10986
if (EVENT == 26004) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1667, 44909, NPC, 22, 26003, 23, -1);
	else
		SelectMsg(UID, 2, 1667, 44909, NPC, 18, 26005);
	end
end

-- [AUTO-GEN] quest=1667 status=1 n_index=10986
if (EVENT == 26005) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1668 status=0 n_index=10990
if (EVENT == 27000) then
	SelectMsg(UID, 4, 1668, 44910, NPC, 3547, 27001, 23, -1);
end

-- [AUTO-GEN] quest=1668 status=0 n_index=10990
if (EVENT == 27001) then
	SaveEvent(UID, 10991);
end

-- [AUTO-GEN] quest=1668 status=1 n_index=10991
if (EVENT == 27003) then
	QuestStatusCheck = GetQuestStatus(UID, 1668)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6827)) then
			SaveEvent(UID, 10992);
		end
	end
end

-- [AUTO-GEN] quest=1668 status=1 n_index=10991
if (EVENT == 27004) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1668, 44910, NPC, 22, 27003, 23, -1);
	else
		SelectMsg(UID, 2, 1668, 44910, NPC, 18, 27005);
	end
end

-- [AUTO-GEN] quest=1668 status=1 n_index=10991
if (EVENT == 27005) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1669 status=0 n_index=10995
if (EVENT == 28000) then
	SelectMsg(UID, 4, 1669, 44911, NPC, 3548, 28001, 23, -1);
end

-- [AUTO-GEN] quest=1669 status=0 n_index=10995
if (EVENT == 28001) then
	SaveEvent(UID, 10996);
end

-- [AUTO-GEN] quest=1669 status=1 n_index=10996
if (EVENT == 28003) then
	QuestStatusCheck = GetQuestStatus(UID, 1669)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6828)) then
			SaveEvent(UID, 10997);
		end
	end
end

-- [AUTO-GEN] quest=1669 status=1 n_index=10996
if (EVENT == 28004) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1669, 44911, NPC, 22, 28003, 23, -1);
	else
		SelectMsg(UID, 2, 1669, 44911, NPC, 18, 28005);
	end
end

-- [AUTO-GEN] quest=1669 status=1 n_index=10996
if (EVENT == 28005) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1670 status=0 n_index=14000
if (EVENT == 29000) then
	SelectMsg(UID, 4, 1670, 44912, NPC, 3549, 29001, 23, -1);
end

-- [AUTO-GEN] quest=1670 status=0 n_index=14000
if (EVENT == 29001) then
	SaveEvent(UID, 14001);
end

-- [AUTO-GEN] quest=1670 status=1 n_index=14001
if (EVENT == 29003) then
	QuestStatusCheck = GetQuestStatus(UID, 1670)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6829)) then
			SaveEvent(UID, 14002);
		end
	end
end

-- [AUTO-GEN] quest=1670 status=1 n_index=14001
if (EVENT == 29004) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 1670, 44912, NPC, 22, 29003, 23, -1);
	else
		SelectMsg(UID, 2, 1670, 44912, NPC, 18, 29005);
	end
end

-- [AUTO-GEN] quest=1670 status=1 n_index=14001
if (EVENT == 29005) then
	ShowMap(UID, 21);
end

