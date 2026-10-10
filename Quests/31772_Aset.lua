local Ret = 0;
local NPC = 31772;

-- Shop transactions reconstructed from the original Quest_Talk/Quest_Menu
-- entries. GetCoins/CheckWeight/GiveItem/GoldLose are server Lua bindings.
local function AsetBuy(item_id, count, price, priest_only)
	if (priest_only == 1 and isPriest(UID) ~= true) then
		SelectMsg(UID, 2, 0, 45048, NPC, 10, 3001);
		Ret = 1;
		return;
	end
	if (GetCoins(UID) < price) then
		SelectMsg(UID, 2, 0, 45048, NPC, 10, 3001);
		Ret = 1;
		return;
	end
	-- CheckWeight reads stale weight fields from CharacterInfo in this server.
	-- The live slot query is authoritative for whether the reward fits.
	if (isRoomForItem(UID, item_id, count) < 0) then
		SelectMsg(UID, 2, 0, 45046, NPC, 10, 3001);
		Ret = 1;
		return;
	end
	GoldLose(UID, price);
	if (GiveItem(UID, item_id, count) ~= true) then
		GoldGain(UID, price);
		SelectMsg(UID, 2, 0, 45046, NPC, 10, 3001);
	end
	Ret = 1;
end

local function AsetBuyBundle(item_ids, price)
	for _, item_id in ipairs(item_ids) do
		if (isRoomForItem(UID, item_id, 1) < 0) then
			SelectMsg(UID, 2, 0, 45046, NPC, 10, 3001);
			Ret = 1;
			return;
		end
	end
	if (GetCoins(UID) < price) then
		SelectMsg(UID, 2, 0, 45048, NPC, 10, 3001);
		Ret = 1;
		return;
	end
	GoldLose(UID, price);
	local delivered = {};
	for _, item_id in ipairs(item_ids) do
		if (GiveItem(UID, item_id, 1) ~= true) then
			for _, delivered_id in ipairs(delivered) do RobItem(UID, delivered_id, 1); end
			GoldGain(UID, price);
			SelectMsg(UID, 2, 0, 45046, NPC, 10, 3001);
			Ret = 1;
			return;
		end
		delivered[#delivered + 1] = item_id;
	end
	Ret = 1;
end

-- [Event Manager] Aset
-- Auto-generated from sniffer capture (dialog_builder v4)
-- 55 menus, 115 mapped, 23 inferred, 4 unknown [actions=QUEST]

-- ROOT: header=45054 flag=2
if (EVENT == 100) then
	SelectMsg(UID, 2, 0, 45054, NPC, 45398, 102, 45616, 108, 45123, 109, 45378, 112, 45468, 113, 45469, 115, 45579, 122);
end

-- ROOT: header=45054 flag=2
if (EVENT == 101) then
	SelectMsg(UID, 2, 0, 45054, NPC, 45398, 102, 45616, 108, 45123, 109, 45378, 112, 45468, 113, 45469, 115, 45579, 122);
end

-- header=45054 flag=2
if (EVENT == 102) then
	SelectMsg(UID, 2, 0, 45054, NPC, 45393, 103, 45394, 105, 45395, 106, 45385, 107, 45386, 105);
end

-- header=45522 flag=2
if (EVENT == 103) then
	SelectMsg(UID, 2, 0, 45522, NPC, 4161, 5101, 4162, 3001);
end

-- header=45048 flag=2
if (EVENT == 104) then
	SelectMsg(UID, 2, 0, 45048, NPC, 10, 3001);
end

-- header=45081 flag=2
if (EVENT == 105) then
	SelectMsg(UID, 2, 0, 45081, NPC, 10, 5102);
end

-- header=45527 flag=2
if (EVENT == 106) then
	SelectMsg(UID, 2, 0, 45527, NPC, 4161, 5103, 4162, 3001);
end

-- header=45501 flag=2
if (EVENT == 107) then
	SelectMsg(UID, 2, 0, 45501, NPC, 4161, 5104, 4162, 3001);
end

-- header=45763 flag=2
if (EVENT == 108) then
	SelectMsg(UID, 2, 0, 45763, NPC, 4161, 5105, 4162, 3001);
end

-- header=45086 flag=2
if (EVENT == 109) then
	SelectMsg(UID, 2, 0, 45086, NPC, 45121, 110, 45122, 111);
end

-- header=45088 flag=2
if (EVENT == 110) then
	SelectMsg(UID, 2, 0, 45088, NPC, 4527, 5106, 4528, 3001);
end

-- header=45087 flag=2
if (EVENT == 111) then
	SelectMsg(UID, 2, 0, 45087, NPC, 4527, 5107, 4528, 3001);
end

-- header=45492 flag=2
if (EVENT == 112) then
	SelectMsg(UID, 2, 0, 45492, NPC, 4161, 5108, 4162, 3001);
end

-- header=45054 flag=2
if (EVENT == 113) then
	SelectMsg(UID, 2, 0, 45054, NPC, 45376, 114);
end

-- header=45485 flag=2
if (EVENT == 114) then
	SelectMsg(UID, 2, 0, 45485, NPC, 4161, 3001);
end

-- header=45054 flag=2
if (EVENT == 115) then
	SelectMsg(UID, 2, 0, 45054, NPC, 45192, 116, 45227, 129, 45195, 133, 45400, 134, 45426, 137, 45412, 119);
end

-- header=45181 flag=2
if (EVENT == 116) then
	SelectMsg(UID, 2, 0, 45181, NPC, 45193, 117, 45194, 128);
end

-- header=45182 flag=2
if (EVENT == 117) then
	SelectMsg(UID, 2, 0, 45182, NPC, 4161, 5109, 4162, 3001);
end

-- header=45046 flag=2
if (EVENT == 118) then
	SelectMsg(UID, 2, 0, 45046, NPC, 10, 3001);
end

-- header=45552 flag=2
if (EVENT == 119) then
	SelectMsg(UID, 2, 0, 45552, NPC, 45405, 120, 45406, 121, 45407, 138, 45408, 139, 45409, 140, 45410, 141);
end

-- header=45553 flag=2
if (EVENT == 120) then
	SelectMsg(UID, 2, 0, 45553, NPC, 4161, 5110, 4162, 3001);
end

-- header=45554 flag=2
if (EVENT == 121) then
	SelectMsg(UID, 2, 0, 45554, NPC, 4161, 5111, 4162, 3001);
end

-- header=45054 flag=2
if (EVENT == 122) then
	SelectMsg(UID, 2, 0, 45054, NPC, 45093, 125, 45535, 126, 45157, 127, 45241, 142, 45311, 150, 45188, 124, 45191, 123);
end

-- header=45732 flag=2
if (EVENT == 123) then
	SelectMsg(UID, 2, 0, 45732, NPC, 45189, 5112, 45190, 5113);
end

-- header=45731 flag=2
if (EVENT == 124) then
	SelectMsg(UID, 2, 0, 45731, NPC, 4161, 5114, 4162, 3001);
end

-- header=45137 flag=3
if (EVENT == 125) then
	-- The TBL text describes an event-participation quest, but has no reward
	-- or exchange ID; do not show a false “insufficient Noah” response.
	SelectMsg(UID, 3, 0, 45137, NPC, 45580, 3001, 45581, 3001);
end

-- header=45749 flag=2
if (EVENT == 126) then
	SelectMsg(UID, 2, 0, 45749, NPC, 45536, 5140, 45537, 5141, 45538, 5142, 45539, 5143, 45540, 5144, 45541, 5145, 45542, 5146);
end

-- header=45721 flag=2
if (EVENT == 127) then
	SelectMsg(UID, 2, 0, 45721, NPC, 4161, 3001);
end

-- header=45184 flag=2
if (EVENT == 128) then
	SelectMsg(UID, 2, 0, 45184, NPC, 4161, 5115, 4162, 3001);
end

-- header=45237 flag=2
if (EVENT == 129) then
	SelectMsg(UID, 2, 0, 45237, NPC, 45237, 130, 45238, 131, 45239, 132);
end

-- header=45238 flag=2
if (EVENT == 130) then
	SelectMsg(UID, 2, 0, 45238, NPC, 4161, 5116, 4162, 3001);
end

-- header=45239 flag=2
if (EVENT == 131) then
	SelectMsg(UID, 2, 0, 45239, NPC, 4161, 5117, 4162, 3001);
end

-- header=45240 flag=2
if (EVENT == 132) then
	SelectMsg(UID, 2, 0, 45240, NPC, 4161, 5118, 4162, 3001);
end

-- header=45187 flag=2
if (EVENT == 133) then
	SelectMsg(UID, 2, 0, 45187, NPC, 4161, 5119, 4162, 3001);
end

-- header=45564 flag=2
if (EVENT == 134) then
	SelectMsg(UID, 2, 0, 45564, NPC, 45404, 135, 45403, 136);
end

-- header=45549 flag=2
if (EVENT == 135) then
	SelectMsg(UID, 2, 0, 45549, NPC, 4161, 5120, 4162, 3001);
end

-- header=45546 flag=2
if (EVENT == 136) then
	SelectMsg(UID, 2, 0, 45546, NPC, 4161, 5121, 4162, 3001);
end

-- header=45582 flag=2
if (EVENT == 137) then
	SelectMsg(UID, 2, 0, 45582, NPC, 4161, 5122, 4162, 3001);
end

-- header=45555 flag=2
if (EVENT == 138) then
	SelectMsg(UID, 2, 0, 45555, NPC, 4161, 5123, 4162, 3001);
end

-- header=45556 flag=2
if (EVENT == 139) then
	SelectMsg(UID, 2, 0, 45556, NPC, 4161, 5124, 4162, 3001);
end

-- header=45557 flag=2
if (EVENT == 140) then
	SelectMsg(UID, 2, 0, 45557, NPC, 4161, 5125, 4162, 3001);
end

-- header=45558 flag=2
if (EVENT == 141) then
	SelectMsg(UID, 2, 0, 45558, NPC, 4161, 5126, 4162, 3001);
end

-- header=45722 flag=2
if (EVENT == 142) then
	SelectMsg(UID, 2, 0, 45722, NPC, 45242, 143, 45243, 144, 45244, 145, 45245, 146, 45246, 147, 45247, 148, 45248, 149);
end

-- header=45723 flag=2
if (EVENT == 143) then
	SelectMsg(UID, 2, 0, 45723, NPC, 4161, 5127, 4162, 3001);
end

-- header=45724 flag=2
if (EVENT == 144) then
	SelectMsg(UID, 2, 0, 45724, NPC, 4161, 5128, 4162, 3001);
end

-- header=45725 flag=2
if (EVENT == 145) then
	SelectMsg(UID, 2, 0, 45725, NPC, 4161, 5129, 4162, 3001);
end

-- header=45726 flag=2
if (EVENT == 146) then
	SelectMsg(UID, 2, 0, 45726, NPC, 4161, 5130, 4162, 3001);
end

-- header=45727 flag=2
if (EVENT == 147) then
	SelectMsg(UID, 2, 0, 45727, NPC, 4161, 5131, 4162, 3001);
end

-- header=45728 flag=2
if (EVENT == 148) then
	SelectMsg(UID, 2, 0, 45728, NPC, 4161, 5132, 4162, 3001);
end

-- header=45729 flag=2
if (EVENT == 149) then
	SelectMsg(UID, 2, 0, 45729, NPC, 4161, 5133, 4162, 3001);
end

-- header=45730 flag=2
if (EVENT == 150) then
	SelectMsg(UID, 2, 0, 45730, NPC, 4161, 5135, 4162, 3001);
end

-- header=45579 flag=2
if (EVENT == 151) then
	SelectMsg(UID, 2, 0, 45579, NPC, 4161, 3001);
end

-- header=45625 flag=2
if (EVENT == 152) then
	SelectMsg(UID, 2, 0, 45625, NPC, 4161, 5134, 4162, 3001);
end

-- header=45054 flag=2
if (EVENT == 153) then
	SelectMsg(UID, 2, 0, 45054, NPC, 45393, 103, 45395, 106, 45386, 105);
end

-- header=45763 flag=2
if (EVENT == 154) then
	SelectMsg(UID, 2, 0, 45763, NPC, 4161, 5105, 4162, 3001);
end

-- ═══ Transactions mapped from the original Aset TBL dialog text ═══
-- Rosetta/Ruler boxes are sold here; their random opening is an item-use
-- action, not an NPC exchange, so selecting “open” does not sell another box.
if (EVENT == 5101) then AsetBuy(978053000, 1, 9000000, 0); end
if (EVENT == 5102) then Ret = 1; end
if (EVENT == 5103) then Ret = 1; end -- Rosetta equipment exchange requires exact equipped-piece recipes.
if (EVENT == 5104) then AsetBuy(978049000, 1, 9000000, 0); end
if (EVENT == 5105) then AsetBuy(811063000, 100, 200000000, 0); end
if (EVENT == 5106) then AsetBuy(899996000, 1, 26950000, 0); end
if (EVENT == 5107) then AsetBuy(899997000, 1, 57750000, 0); end
if (EVENT == 5108) then AsetBuy(700007000, 1, 29000000, 0); end
if (EVENT == 5109) then AsetBuy(389070000, 1, 70000000, 0); end
if (EVENT == 5110) then AsetBuy(800003000, 1, 20000000, 0); end
if (EVENT == 5111) then AsetBuy(800004000, 1, 20000000, 0); end
if (EVENT == 5112) then
	AsetBuyBundle({811004000, 811003000}, 78000000);
end
if (EVENT == 5113) then
	AsetBuyBundle({811004000, 811003000, 811005000}, 99000000);
end
if (EVENT == 5114) then AsetBuy(508114000, 2, 78000000, 0); end
if (EVENT == 5115) then AsetBuy(389130000, 1, 105000000, 0); end
if (EVENT == 5116) then AsetBuy(978011000, 1, 50000000, 0); end
if (EVENT == 5117) then AsetBuy(978012000, 1, 50000000, 0); end
if (EVENT == 5118) then AsetBuy(978013000, 1, 60000000, 0); end
if (EVENT == 5119) then AsetBuy(811010000, 1, 60000000, 0); end
if (EVENT == 5120) then AsetBuy(811074000, 1, 90000000, 0); end
if (EVENT == 5121) then AsetBuy(811073000, 1, 90000000, 0); end
if (EVENT == 5122) then AsetBuy(811142000, 10, 5000000, 0); end
if (EVENT == 5123) then AsetBuy(800005000, 1, 20000000, 0); end
if (EVENT == 5124) then AsetBuy(800006000, 1, 13000000, 0); end
if (EVENT == 5125) then AsetBuy(800007000, 1, 10000000, 0); end
if (EVENT == 5126) then AsetBuy(800008000, 1, 30000000, 0); end
if (EVENT == 5127) then AsetBuy(811025000, 1, 39000000, 0); end
if (EVENT == 5128) then AsetBuy(811026000, 1, 39000000, 0); end
if (EVENT == 5129) then AsetBuy(811027000, 1, 39000000, 0); end
if (EVENT == 5130) then AsetBuy(811028000, 1, 39000000, 0); end
if (EVENT == 5131) then AsetBuy(811029000, 1, 39000000, 0); end
if (EVENT == 5132) then AsetBuy(811030000, 1, 39000000, 0); end
if (EVENT == 5133) then AsetBuy(811031000, 1, 39000000, 0); end
if (EVENT == 5134) then
	if (HowmuchItem(UID, 811153000) >= 1) then
		local nation = GetNation(UID);
		local output = 0;
		if (nation == 1) then output = 811057000; end
		if (nation == 2) then output = 811056000; end
		if (output > 0 and isRoomForItem(UID, output, 1) >= 0) then
			RobItem(UID, 811153000, 1);
			if (GiveItem(UID, output, 1) ~= true) then
				GiveItem(UID, 811153000, 1);
			end
		else
			SelectMsg(UID, 2, 0, 45046, NPC, 10, 3001);
		end
	else
		SelectMsg(UID, 2, 0, 45048, NPC, 10, 3001);
	end
	Ret = 1;
end
if (EVENT == 5135) then AsetBuy(811049000, 1, 39000000, 1); end
if (EVENT == 5140) then AsetBuy(900181000, 1, 120000000, 0); end -- Wing of Steel
if (EVENT == 5141) then AsetBuy(900179000, 1, 120000000, 0); end -- Wing of Assassin
if (EVENT == 5142) then AsetBuy(900180000, 1, 120000000, 0); end -- Wing of Pitch Black
if (EVENT == 5143) then AsetBuy(900178000, 1, 120000000, 0); end -- Wing of Darkness (NP)
if (EVENT == 5144) then AsetBuy(900178000, 1, 120000000, 0); end -- Wing of Darkness (defense)
if (EVENT == 5145) then AsetBuy(900178000, 1, 120000000, 0); end -- Wing of Darkness (HP)
if (EVENT == 5146) then AsetBuy(900182000, 1, 120000000, 0); end -- Wing of Warrior

-- Close dialog
if (EVENT == 3001) then
	Ret = 1;
end

-- ═══════════════════════════════════════════════════════════════════
-- AUTO-GENERATED EVENT HANDLERS (ko-quest-gen)
-- ═══════════════════════════════════════════════════════════════════

-- [AUTO-GEN] quest=1694 status=0 n_index=14235
if (EVENT == 22000) then
	SelectMsg(UID, 4, 1694, 0, NPC, 22, 22001, 23, -1);
end

-- [AUTO-GEN] quest=1694 status=0 n_index=14235
if (EVENT == 22001) then
	SaveEvent(UID, 14235);
end

