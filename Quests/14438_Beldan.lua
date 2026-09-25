local NPC = 14438;

if (EVENT == 3000) then
	SelectMsg(UID, 2, -1, 807, NPC, 67, 3011, 68, -1);
end

if (EVENT == 3010) then
	SelectMsg(UID, 2, -1, 807, NPC, 67, 3011, 68, -1);
end

if (EVENT == 3011) then
	Level = CheckLevel(UID);
	if (Level > 69) then
		SelectMsg(UID, 2, -1, 808, NPC, 2002, 3012);
	else
		SelectMsg(UID, 2, -1, 815, NPC, 10, -1);
	end
end

if (EVENT == 3012) then
	SelectMsg(UID, 2, -1, 809, NPC, 65, 3013);
end

if (EVENT == 3013) then
JURADTIME = CheckJuraidMountainTime(UID);
	if (JURADTIME == true or JURADTIME == 1) then
	JoinEvent(UID);
	SaveEvent(UID, 695);
else
	SelectMsg(UID, 2, -1, 808, NPC, 10, -1);
	end
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=333 status=255 n_index=693
if (EVENT == 3005) then
	SaveEvent(UID, 695);
end
