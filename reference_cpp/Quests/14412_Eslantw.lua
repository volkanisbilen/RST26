-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14412;


if (EVENT == 222) then
	ITEM_COUNT1 = HowmuchItem(UID, 910087000);  
	if (ITEM_COUNT1 < 1) then
		SelectMsg(UID, 2, 67, 697, NPC, 18, 100);
	else
		SelectMsg(UID, 2, 67, 705, NPC, 4006, 226, 4005, 193);
	end
end

if (EVENT == 100) then
	ShowMap(UID, 42);
end

if (EVENT == 226) then
	ITEM_COUNT1 = HowmuchItem(UID, 910087000);  
	if (ITEM_COUNT1 < 1) then
		SelectMsg(UID, 2, 67, 697, NPC, 18, 100);
	else
		RobItem(UID, 910087000, 1);
		GoldGain(UID , 1); 
		SaveEvent(UID, 458);
	end
end