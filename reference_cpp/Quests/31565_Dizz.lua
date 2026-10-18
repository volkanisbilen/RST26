-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31565;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then 
		SelectMsg(UID, 2, -1, 4703, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then 
		NpcMsg(UID, 8060, NPC)
	else
		EVENT = QuestNum
	end
end


if (EVENT == 1001) then
SelectMsg(UID, 4, 653, 21287, NPC, 22, 1002, 23, -1);
end

if (EVENT == 1002) then
	QuestStatus = GetQuestStatus(UID, 653)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12642);
	end
end

if (EVENT == 1006) then
	QuestStatus = GetQuestStatus(UID, 653)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389410000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 653, 21287, NPC, 18,1004);
		else
			SaveEvent(UID, 12644);
		end
	end
end

if (EVENT == 1005) then
	QuestStatus = GetQuestStatus(UID, 653)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389410000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 653, 21287, NPC, 18,1004);
		else
			SelectMsg(UID, 4, 653, 21287, NPC, 22, 1007, 27, -1);
		end
	end
end	

if (EVENT == 1004) then
	ShowMap(UID, 810);
end

if (EVENT == 1007)then
	QuestStatus = GetQuestStatus(UID, 653)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389410000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 653, 21287, NPC, 18,1004);
		else
			RunQuestExchange(UID,3138);
			SaveEvent(UID,12643);
			SaveEvent(UID,12654);
			SelectMsg(UID, 2, 653, 21758, NPC, 10,-1);
		end
	end
end

if (EVENT == 1101) then
SelectMsg(UID, 4, 654, 21289, NPC, 22, 1102, 23, -1);
end

if (EVENT == 1102) then
	QuestStatus = GetQuestStatus(UID, 654)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12654);
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 654)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389083000);   
		if (ITEM1_COUNT < 2) then
			SelectMsg(UID, 2, 654, 21289, NPC, 18,1104);
		else
			SaveEvent(UID, 12656);
		end
	end
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 654)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389083000);   
		if (ITEM1_COUNT < 2) then
			SelectMsg(UID, 2, 654, 21289, NPC, 18,1104);
		else
			SelectMsg(UID, 4, 654, 21289, NPC, 22, 1107, 27, -1);
		end
	end
end	

if (EVENT == 1104) then
	ShowMap(UID, 415);
end

if (EVENT == 1107)then
	QuestStatus = GetQuestStatus(UID, 654)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389083000);   
		if (ITEM1_COUNT < 2) then
			SelectMsg(UID, 2, 654, 21289, NPC, 18,1104);
		else
			RunQuestExchange(UID,3139);
			SaveEvent(UID,12655);
			SaveEvent(UID,12666);
			SelectMsg(UID, 2, 654, 21781, NPC, 10,-1);
		end
	end
end

if (EVENT == 1201) then
SelectMsg(UID, 4, 655, 21291, NPC, 22, 1202, 23, -1);
end

if (EVENT == 1202) then
	QuestStatus = GetQuestStatus(UID, 655)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12666);
	end
end

if (EVENT == 1206) then
	QuestStatus = GetQuestStatus(UID, 655)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389490000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 655, 21291, NPC, 18,1204);
		else
			SaveEvent(UID, 12668);
		end
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 655)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389490000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 655, 21291, NPC, 18,1204);
		else
			SelectMsg(UID, 4, 655, 21291, NPC, 22, 1207, 27, -1);
		end
	end
end	

if (EVENT == 1204) then
	ShowMap(UID, 811);
end

if (EVENT == 1207)then
	QuestStatus = GetQuestStatus(UID, 655)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389490000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 655, 21291, NPC, 18,1204);
		else
			RunQuestExchange(UID,3140);
			SaveEvent(UID,12667);
			SaveEvent(UID,12678);
			SelectMsg(UID, 2, 655, 21696, NPC, 10,-1);
		end
	end
end

if (EVENT == 1301) then
SelectMsg(UID, 4, 656, 21293, NPC, 22, 1302, 23, -1);
end

if (EVENT == 1302) then
	QuestStatus = GetQuestStatus(UID, 656)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12678);
	end
end

if (EVENT == 1306) then
	QuestStatus = GetQuestStatus(UID, 656)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389450000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 656, 21293, NPC, 18,1304);
		else
			SaveEvent(UID, 12680);
		end
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 656)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389450000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 656, 21293, NPC, 18,1304);
		else
			SelectMsg(UID, 4, 656, 21293, NPC, 22, 1307, 27, -1);
		end
	end
end	

if (EVENT == 1204) then
	ShowMap(UID, 811);
end

if (EVENT == 1307)then
	QuestStatus = GetQuestStatus(UID, 656)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 389450000);   
		if (ITEM1_COUNT < 5) then
			SelectMsg(UID, 2, 656, 21293, NPC, 18,1304);
		else
			RunQuestExchange(UID,3141);
			SaveEvent(UID,12679);
			SaveEvent(UID,12690);
		end
	end
end

if (EVENT == 1401) then
SelectMsg(UID, 4, 657, 21295, NPC, 22, 1402, 23, -1);
end

if (EVENT == 1402) then
	QuestStatus = GetQuestStatus(UID, 657)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12690);
	end
end

if (EVENT == 1406) then
	QuestStatus = GetQuestStatus(UID, 657)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900167000);   
	ITEM2_COUNT = HowmuchItem(UID, 900166000);   
		if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 657, 21295, NPC, 18,-1);
		else
			SaveEvent(UID, 12692);
		end
	end
end

if (EVENT == 1405) then
	QuestStatus = GetQuestStatus(UID, 657)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900167000);   
	ITEM2_COUNT = HowmuchItem(UID, 900166000);   
		if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 657, 21295, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 657, 21295, NPC, 22, 1407, 27, -1);
		end
	end
end

if (EVENT == 1407)then
	QuestStatus = GetQuestStatus(UID, 657)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900167000);   
	ITEM2_COUNT = HowmuchItem(UID, 900166000);   
		if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 657, 21295, NPC, 18,-1);
		else
			RunQuestExchange(UID,3142);
			SaveEvent(UID,12691);
			SaveEvent(UID,12702);
			SelectMsg(UID, 2, 657, 21819, NPC, 10,-1);
		end
	end
end

if (EVENT == 1501) then
SelectMsg(UID, 4, 658, 21297, NPC, 22, 1502, 23, -1);
end

if (EVENT == 1502) then
	QuestStatus = GetQuestStatus(UID, 658)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12702);
	end
end

if (EVENT == 1506) then
	QuestStatus = GetQuestStatus(UID, 658)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389420000);    
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 658, 21297, NPC, 18,1504);
		else
			SaveEvent(UID, 12704);
		end
	end
end

if (EVENT == 1505) then
	QuestStatus = GetQuestStatus(UID, 658)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389420000);    
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 658, 21297, NPC, 18,1504);
		else
			SelectMsg(UID, 4, 658, 21297, NPC, 22, 1507, 27, -1);
		end
	end
end

if (EVENT == 1504) then
	ShowMap(UID, 815);
end

if (EVENT == 1507)then
	QuestStatus = GetQuestStatus(UID, 658)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389420000);    
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 658, 21297, NPC, 18,1504);
		else
			RunQuestExchange(UID,3143);
			SaveEvent(UID,12703);
			SaveEvent(UID,12714);
			SelectMsg(UID, 2, 658, 21831, NPC, 10,-1);
		end
	end
end

if (EVENT == 1601) then
SelectMsg(UID, 4, 659, 21299, NPC, 22, 1602, 23, -1);
end

if (EVENT == 1602) then
	QuestStatus = GetQuestStatus(UID, 659)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12714);
	end
end

if (EVENT == 1606) then
	QuestStatus = GetQuestStatus(UID, 659)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389530000);    
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 659, 21299, NPC, 18,1604);
		else
			SaveEvent(UID, 12716);
		end
	end
end

if (EVENT == 1605) then
	QuestStatus = GetQuestStatus(UID, 659)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389530000);    
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 659, 21299, NPC, 18,1604);
		else
			SelectMsg(UID, 4, 659, 21299, NPC, 22, 1607, 27, -1);
		end
	end
end

if (EVENT == 1604) then
	ShowMap(UID, 814);
end

if (EVENT == 1607)then
	QuestStatus = GetQuestStatus(UID, 659)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389530000);    
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 659, 21299, NPC, 18,1604);
		else
			RunQuestExchange(UID,3144);
			SaveEvent(UID,12715);
			SaveEvent(UID,12726);
		end
	end
end