-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24421;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910131000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 42, 4536, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 42, 4537, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, 42, 4538, NPC, 4220, 102, 4221, 103);
end

if (EVENT == 102) then
	ItemA = HowmuchItem(UID, 910131000);
		if (ItemA == 0) then
			SelectMsg(UID, 2, 42, 4536, NPC, 10, -1);
		else
			RunQuestExchange(UID, 486);		 
			SaveEvent(UID, 4287);
	end
end

if (EVENT == 103) then
	RobItem(UID, 910131000, 1);
	SelectMsg(UID, 2, 42, 4539, NPC, 10, -1);
end