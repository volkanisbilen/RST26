-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC= 24408;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4187, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 4188, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 125) then 
	SelectMsg(UID, 2, 216, 4192, NPC, 4170, 140, 4169, -1);
end

if (EVENT == 140) then 
	ItemA = HowmuchItem(UID, 910085000);  
	if (ItemA == 0) then 
		Check = isRoomForItem(UID, 910085000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			GiveItem(UID, 910085000, 1);
			--SaveEvent(UID, 4191);
		end	
	else
		SelectMsg(UID, 2, 216, 4193, NPC, 10, -1);
	end
end