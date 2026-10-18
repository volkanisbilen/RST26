-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32556;

if (EVENT == 100) then
	NATION = CheckNation(UID);
	if (NATION == 2) then
		GiveItem(UID, 900071000, 1);
		SelectMsg(UID, 2, -1, 9846, NPC, 10, -1);
	else
	    GiveItem(UID, 900071000, 1);
		SelectMsg(UID, 2, -1, 1028, NPC, 10, -1);
	end
end