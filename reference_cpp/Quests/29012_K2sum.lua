-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29012;

if (EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 763)	 
	ITEM = HowmuchItem(UID, 900284000)
		if(QuestStatusCheck == 1 and ITEM > 0) then
			EVENT = 101
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
	end
end

if (EVENT == 101)then
	SelectMsg(UID, 2, -1, 22279, NPC, 3000,102,3005,-1);
end

if(EVENT == 102) then
	QuestStatusCheck = GetQuestStatus(UID, 763)	 
	ITEM = HowmuchItem(UID, 900284000)
		if(QuestStatusCheck == 1 and ITEM > 0) then
			SpawnEventSystem(UID,9145,0,71,1895,0,1716);
			ShowEffect(UID, 300391);
			RobItem(UID, 900284000, 1);
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
	end
end