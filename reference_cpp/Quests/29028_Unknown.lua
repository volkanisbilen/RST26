-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
if (EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 775)	 
	if(QuestStatusCheck == 1) then
		EVENT = 101
	else
		QuestStatusCheck = GetQuestStatus(UID, 776)	 
		if(QuestStatusCheck == 1) then
			EVENT = 102	
		else
			SelectMsg(UID, 2, -1, 22281, NPC,10,-1);
		end
	end
end

if (EVENT == 101)then
SelectMsg(UID, 2, -1, 22281, NPC, 3000,103,3005,-1);
end

if (EVENT == 102)then
SelectMsg(UID, 2, -1, 22281, NPC, 3000,104,3005,-1);
end

if(EVENT == 103) then
	ZONE = GetZoneID(UID);
	if(ZONE == 21) then
		SpawnEventSystem(UID,9157,0,21,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	elseif(ZONE == 22) then
		SpawnEventSystem(UID,9157,0,22,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	elseif(ZONE == 23) then
		SpawnEventSystem(UID,9157,0,23,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	elseif(ZONE == 24) then
		SpawnEventSystem(UID,9157,0,24,334,0,65);
		ShowEffect(UID, 300391)
		RobItem(UID, 900295000, 1);
	elseif(ZONE == 25) then
		SpawnEventSystem(UID,9157,0,25,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	end
end

if(EVENT == 104) then
	ZONE = GetZoneID(UID);
	if(ZONE == 21) then
		SpawnEventSystem(UID,9230,0,21,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	elseif(ZONE == 22) then
		SpawnEventSystem(UID,9230,0,22,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	elseif(ZONE == 23) then
		SpawnEventSystem(UID,9230,0,23,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	elseif(ZONE == 24) then
		SpawnEventSystem(UID,9230,0,24,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	elseif(ZONE == 25) then
		SpawnEventSystem(UID,9230,0,25,334,0,65);
		ShowEffect(UID, 300391);
		RobItem(UID, 900295000, 1);
	end
end