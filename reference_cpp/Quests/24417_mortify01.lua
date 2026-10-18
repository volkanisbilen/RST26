-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24417;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910127000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 38, 4520, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 38, 4521, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, 38, 4522, NPC, 4212, 102, 4213, 103);
end

if (EVENT == 102) then
	ItemA = HowmuchItem(UID, 910127000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 38, 4520, NPC, 10, -1);
	else
	RunQuestExchange(UID, 482);		 
	SaveEvent(UID, 4283);
end
end

if (EVENT == 103) then
	RobItem(UID, 910127000, 1);
	SelectMsg(UID, 2, 38, 4523, NPC, 10, -1);
end