-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================

if (EVENT == 100) then
	SelectMsg(UID, 2, -1, 9489, NPC, 10, 101);
end

if (EVENT == 101) then
	KEY = HowmuchItem(UID, 900061000);
	if (KEY < 1) then
		SelectMsg(UID, 2, -1, 9490, NPC, 10, -1);
	else
		SelectMsg(UID, 2, -1, 9491, NPC, 3000, 102,3005,-1);
	end
end

if (EVENT == 102) then
    RobItem(UID, 900061000, 1);
	ZoneChange(UID, 35, 335, 416);
end