local Ret = 0;
local NPC = 31737;

-- [Priest of Full Moon]: inside the Full Moon Rift, Monster Coins buy the defence skills.
-- 914058000 Monster Coin. Item ids: 914059000 Light Feet, 914060000 Drain Restore, 914061000 Fire Strike,
-- 914062000 Bloody Arrow, 914063000 Ankle Freeze, 914064000 Inferior Guard Summon, 914065000 Guard Summon,
-- 914066000 Summon Guard (default), 914077000 Tear of Goddess, 914078000 God's Shield
local COIN = 914058000;

local SHOP = {
	[110] = {914059000, 3},
	[111] = {914060000, 2},
	[112] = {914061000, 4},
	[113] = {914062000, 3},
	[114] = {914063000, 5},
	[115] = {914064000, 20},
	[116] = {914065000, 30},
	[117] = {914066000, 8},
	[118] = {914077000, 1},
	[119] = {914078000, 2},
};

if (EVENT == 100) then
	SelectMsg(UID, 3, -1, 44690, NPC, 40760, 110, 40761, 111, 40762, 112, 40763, 113, 40764, 114, 40765, 115, 40766, 116, 40767, 117, 40768, 118, 40769, 119);
end

if (SHOP[EVENT] ~= nil) then
	local item = SHOP[EVENT][1];
	local cost = SHOP[EVENT][2];
	if (HowmuchItem(UID, COIN) < cost) then
		SelectMsg(UID, 2, -1, 44691, NPC, 10, 3001);
	elseif (CheckGiveSlot(UID, 1) == false) then
		SelectMsg(UID, 2, -1, 44692, NPC, 10, 3001);
	else
		RobItem(UID, COIN, cost);
		GiveItem(UID, item, 1);
		SelectMsg(UID, 2, -1, 44693, NPC, 10, 3001);
	end
end

if (EVENT == 3001) then
	Ret = 1;
end
