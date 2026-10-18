-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29052;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 9488, NPC, 10, 193);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 9488, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1001) then
SelectMsg(UID, 2, -1, 9477, NPC, 7181, 1002,7182,1010);
end

if (EVENT == 1010) then
ZONEKONTROL = GetZoneID(UID);
--ZONEKONTROL2 = GetZoneID(UID);
if (ZONEKONTROL == 35) then
ZoneChange(UID, 30, 510, 252);
--else
--if (ZONEKONTROL2 == 91) then
--ZoneChange(UID, 21, 510, 252);
--end
end
end

if (EVENT == 1002) then
	SelectMsg(UID, 2, 811, 9478, NPC, 10, 1005);
end

if (EVENT == 1005) then
	SelectMsg(UID, 4, 811, 9494, NPC, 3000, 1006, 3005, -1);
end

if (EVENT == 1006) then
	SaveEvent(UID, 2716);
end

if (EVENT == 1004) then
	ITEMA = HowmuchItem(UID, 900060000);
	if (ITEMA < 5) then 
		SelectMsg(UID, 2, 811, 9494, NPC, 18, 1008);
	else
		SelectMsg(UID, 4, 811, 9494, NPC, 3000, 1007, 3005, -1);
	end
end

if (EVENT == 1008) then
	ShowMap(UID, 1008);
end

if (EVENT == 1003) then
	SaveEvent(UID, 2718);
end

if (EVENT == 1007) then
	ITEMA = HowmuchItem(UID, 900060000);
		if (ITEMA < 5) then 
			SelectMsg(UID, 2, 811, 9494, NPC, 18, 1008);
		else
			RunQuestExchange(UID,1224);
			SaveEvent(UID, 2719);
	end
end