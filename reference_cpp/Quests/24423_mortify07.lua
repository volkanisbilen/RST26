-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24423;

if (EVENT == 100) then
	ItemA = HowmuchItem(UID, 910133000);
	if (ItemA == 0) then
		SelectMsg(UID, 2, 44, 4544, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 44, 4545, NPC, 22, 101, 23, -1);
	end
end

if (EVENT == 101) then
	SelectMsg(UID, 2, 44, 4546, NPC, 4224, 102, 4225, 103);
end

if (EVENT == 103) then
	ItemA = HowmuchItem(UID, 910133000);
		if (ItemA == 0) then
			SelectMsg(UID, 2, 44, 4544, NPC, 10, -1);
		else
			RunQuestExchange(UID, 488);		 
			SaveEvent(UID, 4289);
	end
end

if (EVENT == 102) then
	RobItem(UID, 910133000, 1);
	SelectMsg(UID, 2, 44, 4547, NPC, 10, -1);
end