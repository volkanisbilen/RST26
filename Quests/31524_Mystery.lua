local Ret = 0;
local NPC = 31524;

-- Mysterious old man Â
-- Auto-generated from sniffer capture (dialog_builder v4)
-- 4 menus, 7 mapped, 0 inferred, 1 unknown

-- ROOT: header=21215 flag=3
if (EVENT == 100) then
	-- Route the old-man item-disassembly option to the native client window.
	SelectMsg(UID, 3, 1745, 21215, NPC, 7494, 104, 8351, 101, 8788, 102, 45307, 101);
end

-- header=21215 flag=70
if (EVENT == 101) then
	SelectMsg(UID, 70, 616, 21215, NPC, 10, 3001);
end

-- header=12247 flag=2
if (EVENT == 102) then
	SelectMsg(UID, 2, 1745, 12247, NPC, 8785, 103, 8786, 3001 --[[ TODO: unknown ]]);
end

-- header=12248 flag=2
if (EVENT == 103) then
	SelectMsg(UID, 2, 1745, 12248, NPC, 8787, 3001);
end

-- Native item disassembly window (same flag 27 flow used by the reference Lua).
if (EVENT == 104) then
	SelectMsg(UID, 27, -1, -1, NPC);
end

-- Open the client's item-smash/disassembly window at Mysterious Narki.
-- The server validates the selected item, Moradon location, fee and reward.
if (EVENT == 200) then
	SelectMsg(UID, 53, -1, NPC);
end

-- Close dialog
if (EVENT == 3001) then
	Ret = 1;
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=615 status=255 n_index=12236
if (EVENT == 1000) then
	SaveEvent(UID, 12237);
end

-- [AUTO-GEN] quest=615 status=0 n_index=12237
if (EVENT == 1002) then
	SelectMsg(UID, 4, 615, 20798, NPC, 3196, 1003, 23, -1);
end

-- [AUTO-GEN] quest=615 status=1 n_index=12238
if (EVENT == 1003) then
	ItemA = HowmuchItem(UID, 900012000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 615, 20798, NPC, 18, 1005);
	else
		SelectMsg(UID, 4, 615, 20798, NPC, 41, 1004, 27, -1);
	end
end

-- [AUTO-GEN] quest=615 status=1 n_index=12238
if (EVENT == 1004) then
	QuestStatusCheck = GetQuestStatus(UID, 615)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 3104)) then
			SaveEvent(UID, 12239);
		end
	end
end

-- [AUTO-GEN] quest=615 status=3 n_index=12240
if (EVENT == 1005) then
	SelectMsg(UID, 2, 615, 20798, NPC, 10, -1);
end

-- [AUTO-GEN] quest=616 status=255 n_index=12248
if (EVENT == 1100) then
	SaveEvent(UID, 12249);
end

-- [AUTO-GEN] quest=616 status=0 n_index=12249
if (EVENT == 1102) then
	SelectMsg(UID, 4, 616, 20800, NPC, 3198, 1103, 23, -1);
end

-- [AUTO-GEN] quest=616 status=0 n_index=12249
if (EVENT == 1103) then
	SaveEvent(UID, 12250);
end

-- [AUTO-GEN] quest=616 status=1 n_index=12250
if (EVENT == 1105) then
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, 616, 20800, NPC, 22, 1106, 23, -1);
	else
		SelectMsg(UID, 2, 616, 20800, NPC, 18, 1106);
	end
end

-- [AUTO-GEN] quest=616 status=1 n_index=12250
if (EVENT == 1106) then
	QuestStatusCheck = GetQuestStatus(UID, 616)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 3105)) then
			SaveEvent(UID, 12251);
		end
	end
end
