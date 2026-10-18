-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29024;

if (EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 758)	
	ITEM = HowmuchItem(UID, 900278000)
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
	QuestStatusCheck = GetQuestStatus(UID, 758)	
	ITEM = HowmuchItem(UID, 900278000)
		if(QuestStatusCheck == 1 and ITEM > 0) then	
	ZONE = GetZoneID(UID);
		if(ZONE == 12) then
			SpawnEventSystem(UID,9207,0,12,754,0,242);
			ShowEffect(UID, 300391);
			RobItem(UID, 900278000, 1);
		elseif(ZONE == 15) then
			SpawnEventSystem(UID,9207,0,15,754,0,242);
			ShowEffect(UID, 300391);
			RobItem(UID, 900278000, 1);
		elseif(ZONE == 16) then
			SpawnEventSystem(UID,9207,0,16,754,0,242);
			ShowEffect(UID, 300391);
			RobItem(UID, 900278000, 1);
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
		end
	end
end