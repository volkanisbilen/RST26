-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29021;

if (EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 740)	 
	ITEM = HowmuchItem(UID, 900260000)
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
	QuestStatusCheck = GetQuestStatus(UID, 740)	 
	ITEM = HowmuchItem(UID, 900260000)
		if(QuestStatusCheck == 1 and ITEM > 0) then	
	ZONE = GetZoneID(UID);
		if(ZONE == 12) then
			SpawnEventSystem(UID,9194,0,12,175,0,145);
			ShowEffect(UID, 300391);
			RobItem(UID, 900260000, 1);
		elseif(ZONE == 15) then
			SpawnEventSystem(UID,9194,0,15,175,0,145);
			ShowEffect(UID, 300391);
			RobItem(UID, 900260000, 1);
		elseif(ZONE == 16) then
			SpawnEventSystem(UID,9194,0,16,175,0,145);
			ShowEffect(UID, 300391);
			RobItem(UID, 900260000, 1);
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
		end
	end
end