-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29001;

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 43679, NPC, 7087, 101, 3005, -1);
end   

if (EVENT == 101) then
	ZoneChange(UID, 21, 780, 52)
end