-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 15002;

if (EVENT == 165) then
	SelectMsg(UID, 2, -1, 4132, NPC, 4073, 169, 4074, -1);
end

if (EVENT == 169) then
	ZoneChange(UID, 48, 133, 118)
end
