-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29008;

if (EVENT == 100)then
	QuestStatus = GetQuestStatus(UID, 739)	 
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
	QuestStatus = GetQuestStatus(UID, 739)	 
	ITEM = HowmuchItem(UID, 900260000)
		if(QuestStatusCheck == 1 and ITEM > 0) then
	ZONE = GetZoneID(UID);
		if(ZONE == 11) then
			SpawnEventSystem(UID,9121,0,11,173,0,135);
			ShowEffect(UID, 300391);
			RobItem(UID, 900260000, 1);
		elseif(ZONE == 13) then
			SpawnEventSystem(UID,9121,0,13,173,0,135);
			ShowEffect(UID, 300391);
			RobItem(UID, 900260000, 1);
		elseif(ZONE == 14) then
			SpawnEventSystem(UID,9121,0,14,173,0,135);
			ShowEffect(UID, 300391);
			RobItem(UID, 900260000, 1);
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
		end
	end
end