local Ret = 0;
local NPC = 29235;

-- [VIP Manager] Juliane
-- Auto-generated from sniffer capture (dialog_builder v4)
-- 39 menus, 72 mapped, 5 inferred, 37 unknown [actions=QUEST]

-- ROOT: header=11624 flag=2
if (EVENT == 100) then
	SelectMsg(UID, 2, 0, 11624, NPC, 40790, 102, 40375, 105, 4637, 109, 8365, 113, 8534, 124, 8603, 129, 7582, 5000, 45419, 5001, 9018, 135);
end

-- ROOT: header=11624 flag=2
if (EVENT == 101) then
	SelectMsg(UID, 2, -1, 11624, NPC, 45217, 3001 --[[ TODO: unknown ]], 45441, 3001 --[[ TODO: unknown ]], 65555, 3001 --[[ TODO: unknown ]], 65556, 3001 --[[ TODO: unknown ]], 65550, 3001 --[[ TODO: unknown ]], 45216, 3001 --[[ TODO: unknown ]], 45446, 3001 --[[ TODO: unknown ]], 45548, 3001 --[[ TODO: unknown ]]);
end

-- Moon God titles: Grace of the Moon God (930661000) -> achievement
if (EVENT == 102) then
	SelectMsg(UID, 3, -1, 44727, NPC, 40791, 110, 40792, 111, 40793, 112, 40794, 113, 40795, 114);
end

local TITLE_COST = { [110] = {463, 1}, [111] = {464, 5}, [112] = {465, 10}, [113] = {466, 10}, [114] = {467, 10} };
if (TITLE_COST[EVENT] ~= nil) then
	local r = ExchangeItemForAchievement(UID, 930661000, TITLE_COST[EVENT][2], TITLE_COST[EVENT][1]);
	if (r == 1) then
		SelectMsg(UID, 3, -1, 9425, NPC, 27, 3001);
	else
		SelectMsg(UID, 3, -1, 12208, NPC, 27, 3001);
	end
end

-- header=9425 flag=3
if (EVENT == 103) then
	SelectMsg(UID, 3, -1, 9425, NPC, 27, 3001);
end

-- header=12208 flag=3
if (EVENT == 104) then
	SelectMsg(UID, 3, -1, 12208, NPC, 27, 3001);
end

-- header=45350 flag=2
if (EVENT == 105) then
	SelectMsg(UID, 2, -1, 45350, NPC, 40376, 106, 40377, 108);
end

-- header=45351 flag=2
if (EVENT == 106) then
	SelectMsg(UID, 2, -1, 45351, NPC, 40143, 107, 66, 3001);
end

-- header=45353 flag=2
if (EVENT == 107) then
	SelectMsg(UID, 2, -1, 45353, NPC, 27, 3001);
end

-- header=45352 flag=2
if (EVENT == 108) then
	SelectMsg(UID, 2, -1, 45352, NPC, 40143, 107, 66, 3001);
end

-- header=11624 flag=3
if (EVENT == 109) then
	SelectMsg(UID, 3, -1, 11624, NPC, 8359, 110, 7166, 111);
end

-- header=11801 flag=3
if (EVENT == 110) then
	SelectMsg(UID, 3, -1, 11801, NPC, 10, 3001);
end

-- header=9427 flag=2
if (EVENT == 111) then
	SelectMsg(UID, 2, -1, 9427, NPC, 10, 112);
end

-- header=11065 flag=3
if (EVENT == 112) then
	SelectMsg(UID, 3, -1, 11065, NPC, 10, 3001);
end

-- header=11624 flag=3
if (EVENT == 113) then
	SelectMsg(UID, 3, -1, 11624, NPC, 40606, 114, 9001, 116, 8473, 3001 --[[ TODO: unknown ]], 8290, 119, 8466, 112, 8467, 112, 45291, 123);
end

-- header=12006 flag=2
if (EVENT == 114) then
	SelectMsg(UID, 2, -1, 12006, NPC, 40928, 3001, 40865, 115, 40744, 3001 --[[ TODO: unknown ]], 45453, 118, 45082, 3001 --[[ TODO: unknown ]]);
end

-- header=44685 flag=2
if (EVENT == 115) then
	SelectMsg(UID, 2, -1, 44685, NPC, 7477, 3001);
end

-- header=12446 flag=2
if (EVENT == 116) then
	SelectMsg(UID, 2, -1, 12446, NPC, 9003, 117, 9004, 3001 --[[ TODO: unknown ]], 9005, 3001 --[[ TODO: unknown ]], 9006, 3001 --[[ TODO: unknown ]], 9007, 3001 --[[ TODO: unknown ]], 9008, 3001 --[[ TODO: unknown ]], 9009, 3001 --[[ TODO: unknown ]], 9010, 3001 --[[ TODO: unknown ]], 9011, 3001 --[[ TODO: unknown ]], 9012, 3001 --[[ TODO: unknown ]]);
end

-- header=12447 flag=3
if (EVENT == 117) then
	SelectMsg(UID, 3, -1, 12447, NPC, 27, 3001);
end

-- header=44685 flag=2
if (EVENT == 118) then
	SelectMsg(UID, 2, -1, 44685, NPC, 27, 3001);
end

-- header=11624 flag=3
if (EVENT == 119) then
	SelectMsg(UID, 3, -1, 11624, NPC, 45104, 120, 9017, 3001 --[[ TODO: unknown ]]);
end

-- header=45409 flag=2
if (EVENT == 120) then
	SelectMsg(UID, 2, -1, 45409, NPC, 45328, 121);
end

-- header=45587 flag=2
if (EVENT == 121) then
	SelectMsg(UID, 2, -1, 45587, NPC, 45428, 122, 45429, 3001 --[[ TODO: unknown ]]);
end

-- header=11739 flag=2
if (EVENT == 122) then
	SelectMsg(UID, 2, -1, 11739, NPC, 10, 3001);
end

-- header=11860 flag=2
if (EVENT == 123) then
	SelectMsg(UID, 2, -1, 11860, NPC, 4161, 118, 4162, 3001);
end

-- header=11972 flag=3
if (EVENT == 124) then
	SelectMsg(UID, 3, -1, 11972, NPC, 2002, 125, 8535, 126, 8536, 128);
end

-- header=11973 flag=3
if (EVENT == 125) then
	SelectMsg(UID, 3, -1, 11973, NPC, 2002, 126, 8535, 126, 8536, 3001 --[[ TODO: unknown ]]);
end

-- header=11974 flag=3
if (EVENT == 126) then
	SelectMsg(UID, 3, -1, 11974, NPC, 4161, 127, 4162, 3001, 8536, 3001 --[[ TODO: unknown ]]);
end

-- header=11975 flag=3
if (EVENT == 127) then
	SelectMsg(UID, 3, -1, 11975, NPC, 10, 3001);
end

-- header=11976 flag=3
if (EVENT == 128) then
	SelectMsg(UID, 3, -1, 11976, NPC, 4161, 127, 4162, 3001);
end

-- header=12006 flag=2
if (EVENT == 129) then
	SelectMsg(UID, 2, 0, 12006, NPC, 8604, 130, 8605, 132, 8606, 132, 8607, 132);
end

-- header=11807 flag=2
if (EVENT == 130) then
	SelectMsg(UID, 2, -1, 11807, NPC, 8564, 131, 8565, 3001 --[[ TODO: unknown ]], 8566, 3001 --[[ TODO: unknown ]], 8567, 131, 8568, 3001 --[[ TODO: unknown ]], 8569, 3001 --[[ TODO: unknown ]], 8570, 3001 --[[ TODO: unknown ]], 8571, 3001 --[[ TODO: unknown ]], 8572, 3001 --[[ TODO: unknown ]]);
end

-- header=12007 flag=2
if (EVENT == 131) then
	SelectMsg(UID, 2, -1, 12007, NPC, 10, 3001);
end

-- header=12008 flag=2
if (EVENT == 132) then
	SelectMsg(UID, 2, 0, 12008, NPC, 8564, 3001 --[[ TODO: unknown ]], 8565, 3001 --[[ TODO: unknown ]], 8566, 3001 --[[ TODO: unknown ]], 8567, 131, 8568, 3001 --[[ TODO: unknown ]], 8569, 131, 8570, 3001, 8571, 131, 8572, 3001 --[[ TODO: unknown ]]);
end

-- header=12378 flag=2
if (EVENT == 133) then
	SelectMsg(UID, 2, -1, 12378, NPC, 27, 3001);
end

-- header=45410 flag=2
if (EVENT == 134) then
	SelectMsg(UID, 2, -1, 45410, NPC, 27, 3001);
end

-- header=12467 flag=2
if (EVENT == 135) then
	SelectMsg(UID, 2, -1, 12467, NPC, 9030, 136);
end

-- header=12468 flag=2
if (EVENT == 136) then
	SelectMsg(UID, 2, -1, 12468, NPC, 9031, 137, 27, 3001);
end

-- header=12469 flag=2
if (EVENT == 137) then
	SelectMsg(UID, 2, -1, 12469, NPC, 9032, 138, 9033, 138, 9034, 138, 9035, 3001 --[[ TODO: unknown ]]);
end

-- header=12474 flag=2
if (EVENT == 138) then
	SelectMsg(UID, 2, -1, 12474, NPC, 27, 3001);
end

-- ═══ Action handlers (sniffer-verified) ═══

-- QUEST action (btn_text=7582)
if (EVENT == 5000) then
	-- TODO: wire quest logic (SaveEvent, RunExchange, etc.)
	Ret = 1;
end

-- QUEST action (btn_text=45419)
if (EVENT == 5001) then
	-- TODO: wire quest logic (SaveEvent, RunExchange, etc.)
	Ret = 1;
end

-- Close dialog
if (EVENT == 3001) then
	Ret = 1;
end
