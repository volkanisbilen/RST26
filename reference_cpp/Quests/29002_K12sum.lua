-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29002;

if (EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 703)	 
	ITEM = HowmuchItem(UID, 900224000)
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
	QuestStatusCheck = GetQuestStatus(UID, 703)	 
	ITEM = HowmuchItem(UID, 900224000)
		if(QuestStatusCheck == 1 and ITEM > 0) then
	ZONE = GetZoneID(UID);
		if(ZONE == 1) then
			SpawnEventSystem(UID,9085,0,1,728,0,1387);
			ShowEffect(UID, 300391)
			RobItem(UID, 900224000, 1);
		elseif(ZONE == 5) then
			SpawnEventSystem(UID,9085,0,5,728,0,1387);
			ShowEffect(UID, 300391)
			RobItem(UID, 900224000, 1);
		elseif(ZONE == 6) then
			SpawnEventSystem(UID,9085,0,6,728,0,1387);
			ShowEffect(UID, 300391)
			RobItem(UID, 900224000, 1);
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
		end
	end
end