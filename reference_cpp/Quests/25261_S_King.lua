-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25261;

if (EVENT == 100) then;
	SelectMsg(UID, 2, -1, 44358, NPC, 40483, 101);
end

if(EVENT == 101)then
	DrakiRiftChange(UID, 3, 4);
	DrakiTowerNpcOut(UID);
end