local NPC = 29190;

if (EVENT == 100) then -- normal hali 100
	SelectMsg(UID, 3, -1, 10502, NPC,7586,101,7636,-1,7587,-1);
end

if (EVENT == 101) then
	Check = CheckUnderTheCastleOpen(UID);
	-- Rust's CheckUnderTheCastleOpen binding returns 0/1, while the
	-- historical C++ binding returned a Lua boolean.
	if (Check == 1) then
		-- The historical Lua VM re-entered the script after assigning EVENT.
		-- Rust executes one requested event per invocation, so complete the
		-- entry branch here instead of assigning EVENT = 102.
		Count = CheckUnderTheCastleUserCount(UID);
		if (Count < 300) then
			ZoneChange(UID, 86, 69, 64);
		else
			SelectMsg(UID, 2, -1, 10542, NPC, 10, -1);
		end
		else
		SelectMsg(UID, 2, -1, 11792, NPC, 10, -1);
	end
end

if (EVENT == 102) then
	Count = CheckUnderTheCastleUserCount(UID);
	if (Count < 300) then
		ZoneChange(UID, 86, 69, 64);
		else
		SelectMsg(UID, 2, -1, 10542, NPC, 10, -1);
	end
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=957 status=255 n_index=6885
if (EVENT == 105) then
	SaveEvent(UID, 6870);
end

-- [AUTO-GEN] quest=932 status=0 n_index=6745
if (EVENT == 1000) then
	SelectMsg(UID, 4, 932, 0, NPC, 22, 1001, 23, -1);
end

-- [AUTO-GEN] quest=932 status=0 n_index=6745
if (EVENT == 1001) then
	SaveEvent(UID, 6746);
end

-- [AUTO-GEN] quest=932 status=1 n_index=6746
if (EVENT == 1002) then
	SelectMsg(UID, 2, 932, 0, NPC, 10, -1);
end

-- [AUTO-GEN] quest=932 status=1 n_index=6746
if (EVENT == 1003) then
	QuestStatusCheck = GetQuestStatus(UID, 932)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		SaveEvent(UID, 6747);
	end
end

-- [AUTO-GEN] quest=932 status=1 n_index=6746
if (EVENT == 1004) then
	ShowMap(UID, 3);
end

