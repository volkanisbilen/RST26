-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24418;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910128000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 39, 4524, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 39, 4525, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, 39, 4526, NPC, 4214, 103, 4215, 102);
end

if (EVENT == 102) then
	ItemA = HowmuchItem(UID, 910128000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 39, 4524, NPC, 10, -1);
	else
	RunQuestExchange(UID, 483);		 
	SaveEvent(UID, 4284);
end
end

if (EVENT == 103) then
	RobItem(UID, 910128000, 1);
	SelectMsg(UID, 2, 39, 4527, NPC, 10, -1);
end