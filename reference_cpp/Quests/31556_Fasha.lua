-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31556;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then 
		SelectMsg(UID, 2, -1, 4703, NPC, 10, 193);
	elseif (QuestNum > 1 and  QuestNum < 100) then 
		NpcMsg(UID, 8060, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1001) then
	SelectMsg(UID, 2, 642, 21617, NPC, 10, 1002);
end

if (EVENT == 1002) then
	SelectMsg(UID, 2, 642, 21618, NPC, 3000, 1003,3005,-1);
	SaveEvent(UID, 12504);
end

if (EVENT == 1003) then
	SelectMsg(UID, 4, 642, 21619, NPC, 3000, 1004,3005,-1);
	SaveEvent(UID, 12506);
end

if (EVENT == 1004) then
	SelectMsg(UID, 2, 642, 21620, NPC, 10,-1);
	SaveEvent(UID, 12505);
	SaveEvent(UID, 12516);
end