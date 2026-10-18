-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 14423;


local savenum = 44;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910133000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, savenum, 4574, NPC, 10, -1);
	else
		SelectMsg(UID, 4, savenum, 4575, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, savenum, 4576, NPC, 4224, 103, 4225, 102);
end

if (EVENT == 102) then
	ItemA = HowmuchItem(UID, 910133000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, savenum, 4574, NPC, 10, -1);
	else
	RunQuestExchange(UID, 488);		 
	SaveEvent(UID, 4303);
end
end

if (EVENT == 103) then
	RobItem(UID, 910133000, 1);
	SelectMsg(UID, 2, savenum, 4577, NPC, 10, -1, -1, -1);
end