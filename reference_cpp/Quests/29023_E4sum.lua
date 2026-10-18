-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29023;

if (EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 752)	
	ITEM = HowmuchItem(UID, 900272000)
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
	QuestStatusCheck = GetQuestStatus(UID, 752)	
	ITEM = HowmuchItem(UID, 900272000)
		if(QuestStatusCheck == 1 and ITEM > 0) then	
	ZONE = GetZoneID(UID);
		if(ZONE == 12) then
			SpawnEventSystem(UID,9206,0,12,727,0,851);
			ShowEffect(UID, 300391);
			RobItem(UID, 900272000, 1);
		elseif(ZONE == 15) then
			SpawnEventSystem(UID,9206,0,15,727,0,851);
			ShowEffect(UID, 300391);
			RobItem(UID, 900272000, 1);
		elseif(ZONE == 16) then
			SpawnEventSystem(UID,9206,0,16,727,0,851);
			ShowEffect(UID, 300391);
			RobItem(UID, 900272000, 1);
		else
			SelectMsg(UID, 2, -1, 22279, NPC,10,-1);
		end
	end
end