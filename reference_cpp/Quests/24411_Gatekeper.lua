-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24411;

if (EVENT == 217) then
	ITEM = HowmuchItem(UID, 389620000);
	if (ITEM < 1) then
		SelectMsg(UID, 2, -1, 1269, NPC, 18, 218);
	else
		SelectMsg(UID, 2, -1, 1270, NPC, 58, 221, 4005, -1);
	end
end

if (EVENT == 218) then
	ShowMap(UID, 76);
end

if (EVENT == 221) then
	SlotCheck = CheckGiveSlot(UID, 2)
		if SlotCheck == false then
			
        else
		--RobItem(UID, 389620000, 1);
				ExpChange(UID, 5000);
		--SaveEvent(UID, 612);
				GiveItem(UID, 910087000, 1);
				SelectMsg(UID, 2, 66, 1272, NPC, 56, -1);
	end
 end