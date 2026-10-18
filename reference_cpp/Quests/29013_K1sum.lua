-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29013;

if (EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 769)	 
	ITEM = HowmuchItem(UID, 900290000)
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
	QuestStatusCheck = GetQuestStatus(UID, 769)	 
	ITEM = HowmuchItem(UID, 900290000)
		if(QuestStatusCheck == 1 and ITEM > 0) then
			SpawnEventSystem(UID,9146,0,71,1579,0,540);
			ShowEffect(UID, 300391);
			RobItem(UID, 900290000, 1);
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
	end
end