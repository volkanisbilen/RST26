-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24419;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910129000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 40, 4528, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 40, 4529, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, 40, 4530, NPC, 4216, 102, 4217, 103);
end

if (EVENT == 102) then
	ItemA = HowmuchItem(UID, 910129000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 40, 4528, NPC, 10, -1);
	else
	RunQuestExchange(UID, 484);		 
	SaveEvent(UID, 4285);
end
end

if (EVENT == 103) then
	RobItem(UID, 910129000, 1);
	SelectMsg(UID, 2, 40, 4531, NPC, 10, -1);
end