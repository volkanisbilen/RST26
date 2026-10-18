-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 29050;

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
SelectMsg(UID, 19, -1, 9488, NPC, 3000, 1010,3005,-1);
end

if (EVENT == 1010) then
SelectMsg(UID, 2, -1, 9483, NPC, 7183, 1002,7184,1013);
end

if (EVENT == 1013) then
SelectMsg(UID, 19, -1, 9488, NPC, 3000, 1014,3005,-1);
end

if (EVENT == 1014) then
	Room = isCswWinnerNembers(UID);
	if (Room) then
		--ZoneChange(UID, 35, 459, 113);
	else
		SelectMsg(UID, 2, -1, 9611, NPC, 10,-1);
	end
end

if (EVENT == 1002) then
	SelectMsg(UID, 2, 812, 9484, NPC, 10, 1005);
end

if (EVENT == 1005) then
	SelectMsg(UID, 4, 812, 9485, NPC, 3000, 1006, 3005, -1);
end

if (EVENT == 1006) then
	SaveEvent(UID, 2721);
end

if (EVENT == 1004) then
	ITEMA = HowmuchItem(UID, 900060000);
	if (ITEMA < 1) then 
	SelectMsg(UID, 2, 812, 9485, NPC, 18, 1008);
	else
    SelectMsg(UID, 4, 812, 9485, NPC, 3000, 1007, 3005, -1);
end
end

if (EVENT == 1008) then
	ShowMap(UID, 1009);
end

if (EVENT == 1003) then
	SaveEvent(UID, 2723);
end

if (EVENT == 1007) then
    ITEMA = HowmuchItem(UID, 900060000);
        if (ITEMA < 1) then 
            SelectMsg(UID, 2, 812, 9485, NPC, 18, 1008);
        else
            RunCountExchange(UID, 1225)
            SaveEvent(UID, 2724); 
    end
end