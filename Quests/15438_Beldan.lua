local NPC = 15438;

if (EVENT == 3000) then
	NpcMsg(UID, 810, NPC);
end

if (EVENT == 3010) then
	SelectMsg(UID, 2, -1, 807, NPC, 67, 3011, 45609, 3020, 68, -1);
end

if (EVENT == 3011) then
	Level = CheckLevel(UID);
	if (Level > 74) then
		SelectMsg(UID, 2, -1, 808, NPC, 2002, 3012);
	else
		SelectMsg(UID, 2, -1, 810, NPC, 10, -1);
	end
end

-- 3 Silvery Gems -> 1 Fortified Sterling Silver Gemstone.
if (EVENT == 3020) then
	SelectMsg(UID, 2, -1, 45608, NPC, 65, 3021, 68, -1);
end

if (EVENT == 3021) then
	if (CheckExistItem(UID, 389196000, 3) == true) then
		if (GiveItem(UID, 811137000, 1) == true) then
			RobItem(UID, 389196000, 3);
			NpcMsg(UID, 45611, NPC);
		else
			NpcMsg(UID, 45612, NPC);
		end
	else
		NpcMsg(UID, 45610, NPC);
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
	SelectMsg(UID, 2, -1, 804, NPC, 10, -1);
	end
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=333 status=255 n_index=693
if (EVENT == 3005) then
	SaveEvent(UID, 695);
end
