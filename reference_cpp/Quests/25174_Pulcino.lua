-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25174;

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 9088, NPC, 8707, 115);
end
if (EVENT == 115) then 
	SelectMsg(UID, 2, -1, 12133, NPC, 8708, 116);
end

if (EVENT == 116) then
	EMBLEM = HowmuchItem(UID, 910553000);
	if (EMBLEM < 1) then
		SelectMsg(UID, 2, -1, 12133, NPC, 10, -1);
	else
		SelectMsg(UID, 2, -1, 12060, NPC, 4006, 117, 4005, -1);
	end
end

if (EVENT == 117) then
	EMBLEM = HowmuchItem(UID, 910553000);
		if (EMBLEM < 1) then
			SelectMsg(UID, 2, -1, 12133, NPC, 10, -1);
		else
			RobItem(UID, 910553000, 1);
			GiveItem(UID, 810554924, 1,15);
	end
end