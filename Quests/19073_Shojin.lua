local Ret = 0;
local NPC = 19073;

-- [Hepa Pupil] Shozin
-- Auto-generated from sniffer capture (dialog_builder v4)
-- 12 menus, 19 mapped, 3 inferred, 1 unknown [actions=QUEST]

-- ROOT: header=845 flag=20
if (EVENT == 100) then
	SelectMsg(UID, 20, -1, 845, NPC, 4520, 101, 4521, 102, 4526, 103, 40368, 105, 4522, 111, 4523, 3001);
end

-- header=846 flag=19
if (EVENT == 101) then
	SelectMsg(UID, 19, -1, 846, NPC, 4419, 100);
end

-- header=847 flag=19
if (EVENT == 102) then
	SelectMsg(UID, 19, -1, 847, NPC, 4419, 100);
end

-- header=849 flag=19
if (EVENT == 103) then
	SelectMsg(UID, 19, -1, 849, NPC, 4527, 104, 4528, 3001);
end

-- header=851 flag=19
if (EVENT == 104) then
	SelectMsg(UID, 19, -1, 851, NPC, 10, 3001);
end

-- header=44228 flag=2
if (EVENT == 105) then
	SelectMsg(UID, 2, 1745, 44228, NPC, 40365, 106, 40369, 107, 40367, 109);
end

-- header=45328 flag=2
if (EVENT == 106) then
	SelectMsg(UID, 2, 1745, 45328, NPC, 10, 3001);
end

-- header=44221 flag=2
if (EVENT == 107) then
	SelectMsg(UID, 2, 1745, 44221, NPC, 40147, 108);
end

-- header=44222 flag=2
if (EVENT == 108) then
	SelectMsg(UID, 2, 1506, 44222, NPC, 22, 5000, 23, 3001);
end

-- header=44225 flag=2
if (EVENT == 109) then
	SelectMsg(UID, 2, 1745, 44225, NPC, 4161, 110, 4162, 3001);
end

-- header=44227 flag=3
if (EVENT == 110) then
	SelectMsg(UID, 3, 1745, 44227, NPC, 27, 3001);
end

-- header=848 flag=19
if (EVENT == 111) then
	SelectMsg(UID, 19, -1, 848, NPC, 4524, 3001, 4525, 3001 --[[ TODO: unknown ]]);
end

-- ═══ Action handlers (sniffer-verified) ═══

-- QUEST action (btn_text=22)
if (EVENT == 5000) then
	-- TODO: wire quest logic (SaveEvent, RunExchange, etc.)
	Ret = 1;
end

-- Close dialog
if (EVENT == 3001) then
	Ret = 1;
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=1506 status=0 n_index=8385
if (EVENT == 146) then
	SelectMsg(UID, 4, 1506, 44224, NPC, 3495, 147, 23, -1);
end

-- [AUTO-GEN] quest=1506 status=0 n_index=8385
if (EVENT == 147) then
	SaveEvent(UID, 8386);
end

-- [AUTO-GEN] quest=1506 status=1 n_index=8386
if (EVENT == 149) then
	QuestStatusCheck = GetQuestStatus(UID, 1506)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6230)) then
			SaveEvent(UID, 8387);
		end
	end
end

-- [AUTO-GEN] quest=1506 status=1 n_index=8386
if (EVENT == 150) then
	ItemA = HowmuchItem(UID, 379107000);
	if (ItemA < 0) then
		SelectMsg(UID, 2, 1506, 44224, NPC, 18, 151);
	else
		SelectMsg(UID, 4, 1506, 44224, NPC, 41, 149, 27, -1);
	end
end

-- [AUTO-GEN] quest=1506 status=1 n_index=8386
if (EVENT == 151) then
	ShowMap(UID, 21);
end

-- [AUTO-GEN] quest=1506 status=2 n_index=8387
if (EVENT == 200) then
	QuestStatusCheck = GetQuestStatus(UID, 1506)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		if (RunQuestExchange(UID, 6230)) then
			SaveEvent(UID, 8389);
		end
	end
end

-- [AUTO-GEN] quest=931 status=0 n_index=6740
if (EVENT == 1000) then
	SelectMsg(UID, 4, 931, 0, NPC, 22, 1001, 23, -1);
end

-- [AUTO-GEN] quest=931 status=0 n_index=6740
if (EVENT == 1001) then
	SaveEvent(UID, 6741);
end

-- [AUTO-GEN] quest=931 status=1 n_index=6741
if (EVENT == 1002) then
	SelectMsg(UID, 2, 931, 0, NPC, 10, -1);
end

-- [AUTO-GEN] quest=931 status=1 n_index=6741
if (EVENT == 1003) then
	QuestStatusCheck = GetQuestStatus(UID, 931)
	if(QuestStatusCheck == 2) then
		SelectMsg(UID, 2, -1, 8779, NPC, 10, -1);
	else
		SaveEvent(UID, 6742);
	end
end

-- [AUTO-GEN] quest=931 status=1 n_index=6741
if (EVENT == 1004) then
	ShowMap(UID, 21);
end

