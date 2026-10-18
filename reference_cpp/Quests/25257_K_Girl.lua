-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25257;

if (EVENT == 100) then;
	SelectMsg(UID, 2, -1, 12401, NPC, 65, 101);
end

if(EVENT == 101)then
	DrakiRiftChange(UID, 1, 4);
	DrakiTowerNpcOut(UID);
end