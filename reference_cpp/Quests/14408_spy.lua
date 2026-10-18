-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC= 14408;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 4287, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 4288, NPC)
	else
		EVENT = QuestNum
	end
end

local savenum = 216;

if (EVENT == 125) then 
	SelectMsg(UID, 2, -1, 4292, NPC, 4170, 130, 4169, -1);
end

if (EVENT == 130) then 
	ItemA = HowmuchItem(UID, 910085000);  
	if (ItemA == 0) then
		Check = isRoomForItem(UID, 910085000);
		if (Check == -1) then
			SelectMsg(UID, 2, -1, 832, NPC, 27, -1);
		else
			GiveItem(UID, 910085000, 1);
		end	  
	else
		SelectMsg(UID, 2, -1, 4293, NPC, 10, -1);
	end
end