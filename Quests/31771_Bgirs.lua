local NPC = 31771;

-- v2615 Quest_Helper.tbl contains two compatibility IDs for the same
-- "Eslant Experience Blessing" mission. The selected helper row determines
-- which persistent quest/exchange pair must be used.
local function ResolveQuest(UID)
	local Helper = GetQuestHelperID(UID);
	if (Helper >= 14901 and Helper <= 14905) then
		return 10004, 14902, 14903, 16332;
	end
	return 1788, 14917, 14918, 16331;
end

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 45017, NPC, 27, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 0, NPC);
	else
		EVENT = QuestNum;
	end
end

-- Offer the mission.
if (EVENT == 2010) then
	QuestID, AcceptHelper, CompleteHelper, ExchangeID = ResolveQuest(UID);
	MonsterSub = ExistMonsterQuestSub(UID);
	if (MonsterSub == 0) then
		SelectMsg(UID, 4, QuestID, 45017, NPC, 22, 2011, 23, -1);
	else
		SelectMsg(UID, 2, QuestID, 45017, NPC, 27, -1);
	end
end

if (EVENT == 2011) then
	QuestID, AcceptHelper, CompleteHelper, ExchangeID = ResolveQuest(UID);
	SaveEvent(UID, AcceptHelper);
end

-- Active/ready state. All four v2615 monster groups require five kills.
if (EVENT == 2013) then
	QuestID, AcceptHelper, CompleteHelper, ExchangeID = ResolveQuest(UID);
	Count1 = CountMonsterQuestSub(UID, QuestID, 1);
	Count2 = CountMonsterQuestSub(UID, QuestID, 2);
	Count3 = CountMonsterQuestSub(UID, QuestID, 3);
	Count4 = CountMonsterQuestSub(UID, QuestID, 4);
	if (Count1 < 5 or Count2 < 5 or Count3 < 5 or Count4 < 5) then
		SelectMsg(UID, 2, QuestID, 45017, NPC, 18, 2014);
	else
		SelectMsg(UID, 4, QuestID, 45017, NPC, 41, 2012, 27, -1);
	end
end

if (EVENT == 2014) then
	ShowMap(UID);
end

-- Quest_Helper complete event and the reward button intentionally converge.
if (EVENT == 2012) then
	QuestID, AcceptHelper, CompleteHelper, ExchangeID = ResolveQuest(UID);
	Count1 = CountMonsterQuestSub(UID, QuestID, 1);
	Count2 = CountMonsterQuestSub(UID, QuestID, 2);
	Count3 = CountMonsterQuestSub(UID, QuestID, 3);
	Count4 = CountMonsterQuestSub(UID, QuestID, 4);
	if (Count1 < 5 or Count2 < 5 or Count3 < 5 or Count4 < 5) then
		SelectMsg(UID, 2, QuestID, 45017, NPC, 18, 2014);
	else
		RunQuestExchange(UID, ExchangeID);
		SaveEvent(UID, CompleteHelper);
	end
end
