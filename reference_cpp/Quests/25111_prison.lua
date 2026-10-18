-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25111;

if (EVENT == 100) then
	QuestStatus = GetQuestStatus(UID, 1341)	
		if(QuestStatus == 1) then
			EVENT = 101
	end
end

if (EVENT == 101) then		
	SelectMsg(UID, 2, -1, 43914, NPC, 40348, 102,40349,-1);
end

if (EVENT == 102) then		
	SelectMsg(UID, 2, -1, 44079, NPC, 65, 103);
end

if (EVENT == 103) then
	SlotCheck = CheckGiveSlot(UID, 1)
		if SlotCheck == false then
			
        else		
			SelectMsg(UID, 2, -1, 44081, NPC, 10, -1);
			GiveItem(UID, 900685000,1);
			
	end
end