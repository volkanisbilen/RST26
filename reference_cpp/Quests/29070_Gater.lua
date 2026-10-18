-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC= 29070;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 23057, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 23057, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1001)then
	SelectMsg(UID, 2, 780, 23057, NPC, 10, 1002);
end

if (EVENT == 1002)then
	SelectMsg(UID, 2, 780, 23058, NPC, 3000, 1003,3005,-1);
	SaveEvent(UID, 13711);
end

if (EVENT == 1003)then
	SelectMsg(UID, 4, 780, 22986, NPC, 3000, 1004,3005,-1);
	SaveEvent(UID, 13713);
end

if (EVENT == 1004)then
	SelectMsg(UID, 2, 780, 23075, NPC, 3000,1005,4005,-1);
	SaveEvent(UID, 13712);
	SaveEvent(UID, 13723);
end

if (EVENT == 1005)then
	SelectMsg(UID, 2, 780, 23075, NPC, 3000,1006,4005,-1);
	SaveEvent(UID, 13712);
	SaveEvent(UID, 13723);
end

if (EVENT == 1007)then
	SelectMsg(UID, 2, 780, 23075, NPC, 3000,1006,4005,-1);
end

if (EVENT == 1101)then
	SelectMsg(UID, 4, 781, 22988, NPC, 3000,1102,4005,-1);
end

if (EVENT == 1102)then
	SaveEvent(UID, 13723);
end

if (EVENT == 1106)then
	SaveEvent(UID, 13725);
end

if (EVENT == 1006)then
MonsterStoneQuestJoin(UID,781);
end

if (EVENT == 1105)then
	QuestStatusCheck = GetQuestStatus(UID, 781)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, 780, 23075, NPC, 3000,1006,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900318000);  
    ITEM2_COUNT = HowmuchItem(UID, 900319000); 	
	if (ITEM1_COUNT < 4 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 780, 23075, NPC, 3000,1006,4005,-1);
	else
	SelectMsg(UID, 4, 781, 22988, NPC, 22, 1107, 27, -1);
end
end
end

if (EVENT == 1107)then
RunQuestExchange(UID,3229)
	SaveEvent(UID,13724)
	SaveEvent(UID,13735)
	end
	
if (EVENT == 1201)then
	SelectMsg(UID, 4, 783, 22992, NPC, 3000,1202,4005,-1);
end

if (EVENT == 1202)then
	SaveEvent(UID, 13747);
end

if (EVENT == 1206)then
	SaveEvent(UID, 13749);
end

if(EVENT == 1205) then
	ITEMA = HowmuchItem(UID, 900338000)	
	if(ITEMA < 1) then
		SelectMsg(UID, 2, 783, 22990, NPC, 18, -1);
	else
		SelectMsg(UID, 4, 783, 22990, NPC, 22, 1207,23,-1);
	end
end

if (EVENT == 1207)then
SelectMsg(UID, 2, 784, 23113, NPC, 10,1307,4005,-1);
RunQuestExchange(UID,3231)
	SaveEvent(UID,13748)
	SaveEvent(UID,13759)
	end
	
if (EVENT == 1301)then
	SelectMsg(UID, 4, 784, 22994, NPC, 3000,1302,4005,-1);
end

if (EVENT == 1302)then
	SaveEvent(UID, 13759);
end

if (EVENT == 1306)then
	SaveEvent(UID, 13761);
end

if (EVENT == 1305)then
	QuestStatusCheck = GetQuestStatus(UID, 784)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, 784, 23113, NPC, 10,1307,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900321000);  
    ITEM2_COUNT = HowmuchItem(UID, 900322000); 	
	if (ITEM1_COUNT < 4 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 784, 23113, NPC, 10,1307,4005,-1);
	else
	SelectMsg(UID, 4, 784, 22988, NPC, 22, 1308, 27, -1);
end
end
end

if (EVENT == 1307)then
	MonsterStoneQuestJoin(UID,784);
end

if (EVENT == 1308)then
RunQuestExchange(UID,3232)
	SaveEvent(UID,13760)
	SaveEvent(UID,13771)
	end
	
	
	
if (EVENT == 1401)then
	SelectMsg(UID, 2, 791, 23057, NPC, 22, 1402);
end

if (EVENT == 1402)then
	SelectMsg(UID, 2, 791, 23058, NPC, 3000, 1403,3005,-1);
	SaveEvent(UID, 13833);
end

if (EVENT == 1403)then
	SelectMsg(UID, 4, 791, 22986, NPC, 3000, 1404,3005,-1);
	SaveEvent(UID, 13835);
end

if (EVENT == 1404)then
	SelectMsg(UID, 2, 791, 23185, NPC, 10,-1);
	SaveEvent(UID, 13834);
	SaveEvent(UID, 13845);
	RunQuestExchange(UID,3239)
end

if (EVENT == 1501) then
	SelectMsg(UID, 4, 797, 23014, NPC, 22, 1502, 27, -1);
end

if (EVENT == 1502) then
	SaveEvent(UID, 13881);
end

if (EVENT == 1506) then
	SaveEvent(UID, 13883);
end

	if(EVENT == 1505) then
	ITEMA = HowmuchItem(UID, 370005000)	
	ITEMB = HowmuchItem(UID, 900000000)	
	if(ITEMA < 20 and ITEMA < 100000) then
		SelectMsg(UID, 2, 797, 23014, NPC, 18,1507);
	else
		SelectMsg(UID, 4, 797, 23014, NPC, 22, 1508,23,-1);
	end
end

if (EVENT == 1507 ) then
	ShowMap(UID, 414)
end

if (EVENT == 1508)then
	SelectMsg(UID, 2, 797, 23243, NPC, 10,-1);
	SaveEvent(UID, 13882);
	SaveEvent(UID, 13893);
	RunQuestExchange(UID,3243);
	GiveItem(UID, 900337525);
end