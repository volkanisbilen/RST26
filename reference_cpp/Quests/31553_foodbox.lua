-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31553;

if (EVENT == 100) then
	QuestStatusCheck = GetQuestStatus(UID, 633)	
		if(QuestStatusCheck == 1) then
			EVENT = 101
		else
	QuestStatusCheck = GetQuestStatus(UID, 635)	
		if(QuestStatusCheck == 1) then
			EVENT = 102
		else
	QuestStatusCheck = GetQuestStatus(UID, 637)	
		if(QuestStatusCheck == 1) then
			EVENT = 103
		else
	QuestStatusCheck = GetQuestStatus(UID, 639)	
		if(QuestStatusCheck == 1) then
		EVENT = 104
				end
			end
		end
	end
end

if(EVENT == 101) then
SlotCheck = CheckGiveSlot(UID, 3)
     if SlotCheck == false then
       
         else
	COUNTA = HowmuchItem(UID, 900156000) 
	if(COUNTA < 1) then
		GiveItem(UID, 900156000);
		RobItem(UID, 900201000);
		SelectMsg(UID, 2, -1, 21587, NPC, 22,-1);
	end
end
end

if(EVENT == 102) then
SlotCheck = CheckGiveSlot(UID, 3)
     if SlotCheck == false then
       
         else
	COUNTA = HowmuchItem(UID, 900154000) 
	if(COUNTA < 1) then
		GiveItem(UID, 900154000);
		RobItem(UID, 900204000);
		SelectMsg(UID, 2, -1, 21582, NPC, 22,-1);
	end
end
end

if(EVENT == 103) then
SlotCheck = CheckGiveSlot(UID, 3)
     if SlotCheck == false then
       
         else
	COUNTA = HowmuchItem(UID, 900155000) 
	if(COUNTA < 1) then
		GiveItem(UID, 900155000);
		RobItem(UID, 900207000);
		SelectMsg(UID, 2, -1, 21582, NPC, 22,-1);
	end
end
end

if(EVENT == 104) then
SlotCheck = CheckGiveSlot(UID, 3)
     if SlotCheck == false then
       
         else
	COUNTA = HowmuchItem(UID, 900152000) 
	if(COUNTA < 1) then
		GiveItem(UID, 900152000);
		RobItem(UID, 900210000);
		SelectMsg(UID, 2, -1, 21582, NPC, 22,-1);
	end
end
end