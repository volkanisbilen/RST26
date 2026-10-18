-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24422;

local savenum = 43;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910132000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 43, 4540, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 43, 4541, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, 43, 4542, NPC, 4222, 102, 4223, 103);
end

if (EVENT == 102) then
	ItemA = HowmuchItem(UID, 910132000);
		if (ItemA == 0) then
			SelectMsg(UID, 2, 43, 4540, NPC, 10, -1);
		else
			RunQuestExchange(UID, 487);		 
			SaveEvent(UID, 4288);
	end
end

if (EVENT == 103) then
	RobItem(UID, 910132000, 1);
	SelectMsg(UID, 2, savenum, 4543, NPC, 10, -1);
end