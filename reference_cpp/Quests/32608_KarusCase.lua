-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32608;

if (EVENT == 100) then
	NATION = CheckNation(UID);
	if (NATION == 1) then
		SelectMsg(UID, 2, -1, 1028, NPC, 10, -1);
	else
		SelectMsg(UID, 2, -1, 1032, NPC, 10, 101);
	end
end

if (EVENT == 101) then
	Copy_of_Information_Paper = HowmuchItem(UID, 900035000);
	if (Copy_of_Information_Paper > 0) then
		SelectMsg(UID, 2, -1, 1028, NPC, 10, -1);
	else
		RobItem(UID, 900074000);
		GiveItem(UID, 900035000,1);
	end
end