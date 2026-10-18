-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 13010;

if (EVENT == 500) then
	SelectMsg(UID, 2, -1, 4072, NPC, 4071, 501, 4072, -1);
end

if (EVENT == 501) then
	ZoneChange(UID, 21, 249, 306)
end