-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25022;


if(EVENT == 100)then
	QuestStatusCheck = GetQuestStatus(UID, 1211)	
		if(QuestStatusCheck == 1) then
			EVENT = 101
		else
	QuestStatusCheck = GetQuestStatus(UID, 1214)	
		if(QuestStatusCheck == 1) then
			EVENT = 105
		else
			SelectMsg(UID, 2, -1, 43671, NPC, 10, -1);
		end
	end
end

if(EVENT == 101)then
	SelectMsg(UID, 2, -1, 43671, NPC, 40164, 102);
end

if(EVENT == 102)then
	ShowEffect(UID, 300435);
	SelectMsg(UID, 2, -1, 43672, NPC, 40159, 103);
end

if(EVENT == 103)then
	SelectMsg(UID, 2, -1, 43673, NPC, 40160, 104);
end

if(EVENT == 104)then
	QuestStatusCheck = GetQuestStatus(UID, 1211)	
		if(QuestStatusCheck == 1) then
	ZONE = GetZoneID(UID);
		if(ZONE == 21) then
			ShowEffect(UID, 300438);
			SpawnEventSystem(UID,9666,0,21,467,0,519);
			SpawnEventSystem(UID,9666,0,21,469,0,523);
		elseif(ZONE == 22) then
			ShowEffect(UID, 300438);
			SpawnEventSystem(UID,9666,0,22,467,0,519);
			SpawnEventSystem(UID,9666,0,22,469,0,523);
		elseif(ZONE == 23) then
			ShowEffect(UID, 300438);
			SpawnEventSystem(UID,9666,0,23,467,0,519);
			SpawnEventSystem(UID,9666,0,23,469,0,523);
		elseif(ZONE == 24) then
			ShowEffect(UID, 300438);
			SpawnEventSystem(UID,9666,0,24,467,0,519);
			SpawnEventSystem(UID,9666,0,24,469,0,523);
		elseif(ZONE == 25) then
			ShowEffect(UID, 300438);
			SpawnEventSystem(UID,9666,0,25,467,0,519);
			SpawnEventSystem(UID,9666,0,25,469,0,523);
		else
			SelectMsg(UID, 2, -1, 43671, NPC, 10, -1);
		end
	end
end

if(EVENT == 105)then
	SelectMsg(UID, 2, -1, 43681, NPC, 40166, 106);
end

if(EVENT == 106)then
	SelectMsg(UID, 2, -1, 43682, NPC, 40167, 107);
end

if(EVENT == 107)then
	SelectMsg(UID, 2, -1, 43683, NPC, 40169, 108);
end

if(EVENT == 108)then
	SlotCheck = CheckGiveSlot(UID, 2)
		if SlotCheck == false then
			
        else
	QuestStatusCheck = GetQuestStatus(UID, 1214)	
		if(QuestStatusCheck == 2) then
			SelectMsg(UID, 2, -1, 43671, NPC, 10, -1);
		else
			GiveItem(UID, 900632000, 1);
			GiveItem(UID, 900631000, 1);
		end
	end
end