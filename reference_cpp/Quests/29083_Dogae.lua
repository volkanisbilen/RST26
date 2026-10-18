-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29083;

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 12370, NPC, 8933, 101, 8934, -1);
end

if (EVENT == 101) then
	SelectMsg(UID, 32, -1, NPC);
end

--22,32,33,34,35,36,37,38,39,40,41 betting