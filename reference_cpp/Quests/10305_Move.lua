-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 10305;

if (EVENT == 165) then
	NATION = CheckNation(UID);
	if (NATION == 2) then
		SelectMsg(UID, 2, -1, 4632, NPC, 10, -1);        
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
	NATION = CheckNation(UID);
		if (NATION == 2) then
			SelectMsg(UID, 2, -1, 4632, NPC, 10, -1);        
		else
	Capture = CheckMiddleStatueCapture(UID)
		if (Capture == 1) then
			MoveMiddleStatue(UID)
		else
			SelectMsg(UID, 2, -1, 4633, NPC, 10, -1);  
		end
	end
end