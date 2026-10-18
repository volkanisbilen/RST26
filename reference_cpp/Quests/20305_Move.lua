-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 20305;

if (EVENT == 165) then
	NATION = CheckNation(UID)
	if (NATION == 1) then
		SelectMsg(UID, 2, -1, 4631, NPC, 10, -1);        
	else
		Capture = CheckMiddleStatueCapture(UID)
		if (Capture == 1) then
			SelectMsg(UID, 2, -1, 4634, NPC, 4226, 169, 4227, -1);        
		else
			SelectMsg(UID, 2, -1, 4633, NPC, 10, -1);        
		end
	end
end

if (EVENT == 169) then
	NATION = CheckNation(UID)
		if (NATION == 1) then
			SelectMsg(UID, 2, -1, 4631, NPC, 10, -1);        
		else 
	Capture = CheckMiddleStatueCapture(UID)
		if (Capture == 0) then
			SelectMsg(UID, 2, -1, 4633, NPC, 10, -1); 
		else
			MoveMiddleStatue(UID)
		end
	end
end