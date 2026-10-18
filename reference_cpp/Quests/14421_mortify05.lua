-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14421;

local savenum = 42;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910131000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, savenum, 4566, NPC, 10, -1);
	else
		SelectMsg(UID, 4, savenum, 4567, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, savenum, 4568, NPC, 4220, 102, 4221, 103);
end

if (EVENT == 102) then
	ItemA = HowmuchItem(UID, 910131000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, savenum, 4566, NPC, 10, -1);
	else
	RunQuestExchange(UID, 486);		 
	SaveEvent(UID, 4301);
end
end

if (EVENT == 103) then
	RobItem(UID, 910131000, 1);
	SelectMsg(UID, 2, savenum, 4569, NPC, 10, -1);
end