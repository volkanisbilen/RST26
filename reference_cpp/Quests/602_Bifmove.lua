-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 602;

if (EVENT == 165) then
	SelectMsg(UID, 2, -1, 4470, NPC, 4200, 166, 4199, -1);		   
end

if (EVENT == 166) then
check = CheckBeefEventLogin(UID)
	if (check == true) then
		Nation = CheckNation(UID);
		if (Nation == 1) then
			ZoneChange(UID, 31, 78, 730)
		else
			ZoneChange(UID, 31, 245, 950);
		end
	else
		SelectMsg(UID, 2, -1, 4471, NPC, 10, -1);	
	end
end