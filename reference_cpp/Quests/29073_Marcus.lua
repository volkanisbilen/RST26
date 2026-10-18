-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29073;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 23021, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 23021, NPC)
	else
		EVENT = QuestNum
	end
	end


if (EVENT == 1001)then
	SelectMsg(UID, 2, 801, 23020, NPC, 10, 1003);
	SaveEvent(UID, 13923);
end


if (EVENT == 1003)then
	SelectMsg(UID, 4, 801, 23020, NPC, 3000, 1004,3005,-1);
	SaveEvent(UID, 13925);
end

if (EVENT == 1004)then
	SelectMsg(UID, 2, 801, 23020, NPC, 3000, 1006,3005,-1);
	SaveEvent(UID, 13924);
	SaveEvent(UID, 13935);
end

if (EVENT == 1006)then
MonsterStoneQuestJoin(UID,802);
end

if (EVENT == 1101)then
	SelectMsg(UID, 4, 802, 23022, NPC, 3000,1102,4005,-1);
end

if (EVENT == 1102)then
	SaveEvent(UID, 13935);
end

if (EVENT == 1106)then
	SaveEvent(UID, 13937);
end

if (EVENT == 1105)then
	QuestStatusCheck = GetQuestStatus(UID, 802)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, 802, 23020, NPC, 3000, 1006,3005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900331000);  
    ITEM2_COUNT = HowmuchItem(UID, 900332000); 	
	if (ITEM1_COUNT < 10 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 802, 23020, NPC, 3000, 1006,3005,-1);
	else
	SelectMsg(UID, 4, 802, 22988, NPC, 22, 1107, 27, -1);
end
end
end

if (EVENT == 1107) then
    RunQuestExchange(UID, 3247)
    SaveEvent(UID, 13936);
	SaveEvent(UID, 13947);
end

if (EVENT == 1201)then
	SelectMsg(UID, 4, 805, 23029, NPC, 3000,1202,4005,-1);
end

if (EVENT == 1202)then
	SaveEvent(UID, 13971);
end

if (EVENT == 1206)then
	SaveEvent(UID, 13973);
end

if(EVENT == 1205) then
	ITEMA = HowmuchItem(UID, 900334000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 805, 23029, NPC, 18,-1);
	else
		SelectMsg(UID, 4, 805, 23029, NPC, 22, 1207,23,-1);
	end
end
	
if (EVENT == 1207) then
    RunQuestExchange(UID, 3250)
    SaveEvent(UID, 13972);
	SaveEvent(UID, 13983);
	SelectMsg(UID, 2, 805, 23334, NPC, 10, 1208,4005,-1);
end

if (EVENT == 1208)then
ZoneChange(UID, 21, 244, 342)
end


if (EVENT == 1301)then
	SelectMsg(UID, 4, 807, 23030, NPC, 3000,1302,4005,-1);
end

if (EVENT == 1302)then
	SaveEvent(UID, 13983);
end

if (EVENT == 1306)then
	SaveEvent(UID, 13985);
end

if (EVENT == 1305)then
	QuestStatusCheck = GetQuestStatus(UID, 807)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, 807, 23334, NPC, 10, 1208,4005,-1);
	else
	MonsterCount = CountMonsterQuestSub(UID, 807, 1);
	if (MonsterCount < 1) then
	SelectMsg(UID, 2, 807, 23334, NPC, 10, 1208,4005,-1);
	else
	SelectMsg(UID, 4, 807, 23030, NPC, 22, 1308, 27, -1);
end
end
end

if (EVENT == 1308) then
    RunQuestExchange(UID, 3251);
    SaveEvent(UID, 13984);
end