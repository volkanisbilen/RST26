-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29020;

if (EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 734)	
	ITEM = HowmuchItem(UID, 900254000)
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
	QuestStatusCheck = GetQuestStatus(UID, 734)	
	ITEM = HowmuchItem(UID, 900254000)
		if(QuestStatusCheck == 1 and ITEM > 0) then	
	ZONE = GetZoneID(UID);
		if(ZONE == 2) then
			SpawnEventSystem(UID,9183,0,2,662,0,1837);
			ShowEffect(UID, 300391);
			RobItem(UID, 900254000, 1);
		elseif(ZONE == 7) then
			SpawnEventSystem(UID,9183,0,7,662,0,1837);
			ShowEffect(UID, 300391);
			RobItem(UID, 900254000, 1);
		elseif(ZONE == 8) then
			SpawnEventSystem(UID,9183,0,8,662,0,1837);
			ShowEffect(UID, 300391)
			RobItem(UID, 900254000, 1);
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
		end
	end
end