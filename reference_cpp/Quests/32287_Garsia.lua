-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 32287;

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 906, NPC, 4076, 102, 4154, -1);
end

if (EVENT == 102) then
	ZoneChange(UID, 71, 630, 919)
end