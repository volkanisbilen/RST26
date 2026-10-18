-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 31574;

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
	SelectMsg(UID, 2, 631, 21243, NPC, 10, 1002);
end

if (EVENT == 1002) then
	SelectMsg(UID, 2, 631, 21243, NPC, 3000, 1003,3005,-1);
	SaveEvent(UID, 12378);
	
end

if (EVENT == 1003) then
	SelectMsg(UID, 4, 631, 21243, NPC, 3000, 1004,3005,-1);
	SaveEvent(UID, 12380);
	
end

if (EVENT == 1004) then
	SelectMsg(UID, 2, 631, 21455, NPC, 10,-1);
	SaveEvent(UID, 12379);
	SaveEvent(UID, 12390);
end

if (EVENT == 1101) then
	SelectMsg(UID, 4, 632, 21245, NPC, 22, 1102, 27, -1);
end

if (EVENT == 1102) then
	QuestStatus = GetQuestStatus(UID, 632)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12390);
	end
end

if (EVENT == 1106) then
	QuestStatus = GetQuestStatus(UID, 632)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 389520000);   
		if (ITEM1_COUNT1 < 3) then
			SelectMsg(UID, 2, 632, 21245, NPC, 18,1107);
		else
			SaveEvent(UID, 12392);
		end
	end
end

if (EVENT == 1105) then
	QuestStatus = GetQuestStatus(UID, 632)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 389520000);   
		if (ITEM1_COUNT1 < 3) then
			SelectMsg(UID, 2, 632, 21245, NPC, 18,1107);
		else
			SelectMsg(UID, 4, 632, 21245, NPC, 22, 1108,27, -1);
		end
	end
end


if (EVENT == 1107) then
	ShowMap(UID, 108);
end

if (EVENT == 1108)then
	QuestStatus = GetQuestStatus(UID, 632)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 389520000);   
		if (ITEM1_COUNT1 < 3) then
			SelectMsg(UID, 2, 632, 21245, NPC, 18,1107);
		else
			SelectMsg(UID, 2, 632, 21477, NPC, 10,-1);
			RunQuestExchange(UID,3117);
			SaveEvent(UID,12391);
			SaveEvent(UID,12402);
		end
	end
end

if (EVENT == 1201) then
	SelectMsg(UID, 4, 633, 21247, NPC, 22, 1202, 27, -1);
end

if (EVENT == 1202) then
	QuestStatus = GetQuestStatus(UID, 633)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12402);
	end
end

if (EVENT == 1206) then
	QuestStatus = GetQuestStatus(UID, 633)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900151000);   
	ITEM2_COUNT = HowmuchItem(UID, 900156000);  
	ITEM3_COUNT = HowmuchItem(UID, 900165000); 
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1208);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1209);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1210);
		else
			SaveEvent(UID, 12404);
		end
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 633)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900151000);   
	ITEM2_COUNT = HowmuchItem(UID, 900156000);  
	ITEM3_COUNT = HowmuchItem(UID, 900165000);
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1208);
        elseif (ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1209);
		elseif (ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1210);
		else
			SelectMsg(UID, 4, 633, 21247, NPC, 22, 1207, 27, -1);
		end
	end
end

if (EVENT == 1208) then
	ShowMap(UID, 776);
end
if (EVENT == 1209) then
	ShowMap(UID, 777);
end
if (EVENT == 1210) then
	ShowMap(UID, 778);
end

if (EVENT == 1207)then
	QuestStatus = GetQuestStatus(UID, 633)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900151000);   
	ITEM2_COUNT = HowmuchItem(UID, 900156000);  
	ITEM3_COUNT = HowmuchItem(UID, 900165000);
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1208);
        elseif (ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1209);
		elseif (ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 633, 21247, NPC, 18, 1210);
		else
			SelectMsg(UID, 2, 633, 21481, NPC, 10,-1);
			RunQuestExchange(UID,3118);
			SaveEvent(UID,12403);
			SaveEvent(UID,12414);
		end
	end
end


if (EVENT == 1301) then
	SelectMsg(UID, 4, 634, 21249, NPC, 22, 1302, 27, -1);
end

if (EVENT == 1302) then
	QuestStatus = GetQuestStatus(UID, 634)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12414);
	end
end

if (EVENT == 1306) then
	QuestStatus = GetQuestStatus(UID, 634)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 389460000);   
		if (ITEM1_COUNT1 < 5) then
			SelectMsg(UID, 2, 634, 21249, NPC, 18,1307);
		else
			SaveEvent(UID, 12416);
		end
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 634)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 389460000);   
		if (ITEM1_COUNT1 < 5) then
			SelectMsg(UID, 2, 634, 21249, NPC, 18,1307);
		else
			SelectMsg(UID, 4, 634, 21249, NPC, 22, 1308,27, -1);
		end
	end
end

if (EVENT == 1307) then
	ShowMap(UID, 1145);
end

if (EVENT == 1308)then
	QuestStatus = GetQuestStatus(UID, 634)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT1 = HowmuchItem(UID, 389460000);   
		if (ITEM1_COUNT1 < 5) then
			SelectMsg(UID, 2, 634, 21249, NPC, 18,1307);
		else
			SelectMsg(UID, 2, 634, 21503, NPC, 10,-1);
			RunQuestExchange(UID,3119);
			SaveEvent(UID,12415);
			SaveEvent(UID,12426);
		end
	end
end

if (EVENT == 1401) then
	SelectMsg(UID, 4, 635, 21251, NPC, 22, 1402, 27, -1);
end

if (EVENT == 1402) then
	QuestStatus = GetQuestStatus(UID, 635)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12426);
	end
end

if (EVENT == 1406) then
	QuestStatus = GetQuestStatus(UID, 635)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900149000);   
	ITEM2_COUNT = HowmuchItem(UID, 900154000);  
	ITEM3_COUNT = HowmuchItem(UID, 900163000);
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1408);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1409);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1410);	
		else
			SaveEvent(UID, 12428);
		end
	end
end

if (EVENT == 1405) then
	QuestStatus = GetQuestStatus(UID, 635)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900149000);   
	ITEM2_COUNT = HowmuchItem(UID, 900154000);  
	ITEM3_COUNT = HowmuchItem(UID, 900163000);  
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1408);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1409);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1410);
		else
			SelectMsg(UID, 4, 635, 21251, NPC, 22, 1407, 27, -1);	
		end
	end
end

if (EVENT == 1408) then
	ShowMap(UID, 776);
end
if (EVENT == 1409) then
	ShowMap(UID, 777);
end
if (EVENT == 1410) then
	ShowMap(UID, 778);
end

if (EVENT == 1407)then
	QuestStatus = GetQuestStatus(UID, 635)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900149000);   
	ITEM2_COUNT = HowmuchItem(UID, 900154000);  
	ITEM3_COUNT = HowmuchItem(UID, 900163000);  
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1408);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1409);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, -1, 21251, NPC, 18, 1410);
		else
			SelectMsg(UID, 2, 635, 21516, NPC, 10,-1);
			RunQuestExchange(UID,3120);
			SaveEvent(UID,12427);
			SaveEvent(UID,12438);
		end
	end
end

if (EVENT == 1501) then
	SelectMsg(UID, 4, 636, 21253, NPC, 22, 1502, 27, -1);
end

if (EVENT == 1502) then
	QuestStatus = GetQuestStatus(UID, 636)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
		SaveEvent(UID, 12438);
	end
end

if (EVENT == 1506) then
	QuestStatus = GetQuestStatus(UID, 636)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389540000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, -1, 21253, NPC, 18,1507);
		else
			SaveEvent(UID, 12440);
		end
	end
end

if (EVENT == 1505) then
	QuestStatus = GetQuestStatus(UID, 636)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389540000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, -1, 21253, NPC, 18,1507);
		else
			SelectMsg(UID, 4, 636, 21253, NPC, 22, 1508, 27, -1);
		end
	end
end

if (EVENT == 1507) then
	ShowMap(UID, 245);
end

if (EVENT == 1508)then
	QuestStatus = GetQuestStatus(UID, 636)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389540000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, -1, 21253, NPC, 18,1507);
		else
			SelectMsg(UID, 2, 636, 21519, NPC, 10,-1);
			RunQuestExchange(UID,3121);
			SaveEvent(UID,12439);
			SaveEvent(UID,12450);
		end
	end
end

if (EVENT == 1601) then
	SelectMsg(UID, 4, 637, 21255, NPC, 22, 1602, 27, -1);
end

if (EVENT == 1602) then
	QuestStatus = GetQuestStatus(UID, 637)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12450);
	end
end

if (EVENT == 1606) then
	QuestStatus = GetQuestStatus(UID, 637)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900164000);   
	ITEM2_COUNT = HowmuchItem(UID, 900155000);  
	ITEM3_COUNT = HowmuchItem(UID, 900150000); 
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1608);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1609);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1610);
		else
			SaveEvent(UID, 12452);
		end
	end
end

if (EVENT == 1605) then
	QuestStatus = GetQuestStatus(UID, 637)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900164000);   
	ITEM2_COUNT = HowmuchItem(UID, 900155000);  
	ITEM3_COUNT = HowmuchItem(UID, 900150000); 
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1608);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1609);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1610);
		else
			SelectMsg(UID, 4, 637, 21255, NPC, 22, 1607, 27, -1);	
		end
	end
end

if (EVENT == 1608) then
	ShowMap(UID, 776);
end
if (EVENT == 1609) then
	ShowMap(UID, 777);
end
if (EVENT == 1610) then
	ShowMap(UID, 778);
end

if (EVENT == 1607)then
	QuestStatus = GetQuestStatus(UID, 637)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900164000);   
	ITEM2_COUNT = HowmuchItem(UID, 900155000);  
	ITEM3_COUNT = HowmuchItem(UID, 900150000); 
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1608);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1609);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 637, 21255, NPC, 18, 1610);
		else
			RunQuestExchange(UID,3122);
			SaveEvent(UID,12451);
			SaveEvent(UID,12462);
		end
	end
end

if (EVENT == 1701) then
	SelectMsg(UID, 4, 638, 21257, NPC, 22, 1702, 27, -1);
end

if (EVENT == 1702) then
	QuestStatus = GetQuestStatus(UID, 638)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12462);
	end
end

if (EVENT == 1706) then
	QuestStatus = GetQuestStatus(UID, 638)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389430000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 638, 21257, NPC, 18,-1);
		else
			SaveEvent(UID, 12464);
		end
	end
end

if (EVENT == 1705) then
	QuestStatus = GetQuestStatus(UID, 638)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389430000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 638, 21257, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 638, 21257, NPC, 22, 1708, 27, -1); 
		end
	end
end

if (EVENT == 1708)then
	QuestStatus = GetQuestStatus(UID, 638)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389430000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 638, 21257, NPC, 18,-1);
		else
			RunQuestExchange(UID,3123);
			SaveEvent(UID,12463);
			SaveEvent(UID,12474);
		end
	end
end

if (EVENT == 1801) then
	SelectMsg(UID, 4, 639, 21259, NPC, 22, 1802, 27, -1);
end

if (EVENT == 1802) then
	QuestStatus = GetQuestStatus(UID, 639)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12474);
	end
end

if (EVENT == 1806) then
	QuestStatus = GetQuestStatus(UID, 639)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900148000);   
	ITEM2_COUNT = HowmuchItem(UID, 900152000);  
	ITEM3_COUNT = HowmuchItem(UID, 900159000); 
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1808);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1809);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1810);
		else
		SaveEvent(UID, 12476);
		end
	end
end

if (EVENT == 1805) then
	QuestStatus = GetQuestStatus(UID, 639)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900148000);   
	ITEM2_COUNT = HowmuchItem(UID, 900152000);  
	ITEM3_COUNT = HowmuchItem(UID, 900159000); 
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1808);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1809);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1810);
		else		
			SelectMsg(UID, 4, 639, 21259, NPC, 22, 1807, 27, -1);	
		end
	end
end

if (EVENT == 1808) then
	ShowMap(UID, 776);
end
if (EVENT == 1809) then
	ShowMap(UID, 777);
end
if (EVENT == 1810) then
	ShowMap(UID, 778);
end

if (EVENT == 1807)then
	QuestStatus = GetQuestStatus(UID, 639)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM1_COUNT = HowmuchItem(UID, 900148000);   
	ITEM2_COUNT = HowmuchItem(UID, 900152000);  
	ITEM3_COUNT = HowmuchItem(UID, 900159000); 
		if (ITEM1_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1808);
        elseif ( ITEM2_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1809);
		elseif ( ITEM3_COUNT < 1) then
			SelectMsg(UID, 2, 639, 21259, NPC, 18, 1810);
		else
			RunQuestExchange(UID,3124);
			SaveEvent(UID,12475);
			SaveEvent(UID,12486);
		end
	end
end

if (EVENT == 1901) then
	SelectMsg(UID, 4, 640, 21261, NPC, 22, 1902, 27, -1);
end

if (EVENT == 1902) then
	QuestStatus = GetQuestStatus(UID, 640)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 12486);
	end
end

if (EVENT == 1906) then
	QuestStatus = GetQuestStatus(UID, 640)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389550000);   
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, -1, 21261, NPC, 18,1907);
		else
			SaveEvent(UID, 12488);
		end
	end
end

if (EVENT == 1905) then
	QuestStatus = GetQuestStatus(UID, 640)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389550000);   
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, -1, 21261, NPC, 18,1907);
		else
			SelectMsg(UID, 4, 640, 21261, NPC, 22, 1908, 27, -1); 
		end
	end
end

if (EVENT == 1907) then
	ShowMap(UID, 820);
end

if (EVENT == 1908)then
	QuestStatus = GetQuestStatus(UID, 640)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 389550000);   
		if (ITEM_COUNT < 3) then
			SelectMsg(UID, 2, -1, 21261, NPC, 18,1907);
		else
			RunQuestExchange(UID,3125);
			SaveEvent(UID,12487);
			SaveEvent(UID,12498);
		end
	end
end