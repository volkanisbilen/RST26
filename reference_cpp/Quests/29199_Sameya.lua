-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29199;

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 10503, NPC, 7588, 101, 7587, -1);
end

if (EVENT == 101) then
NATION = CheckNation(UID);
if (NATION == 2) then
	ZoneChange(UID, 2, 1606, 401)
	else
	ZoneChange(UID, 1, 425, 1641)
end
end