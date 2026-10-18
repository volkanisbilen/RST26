-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
-- Kontrol Edilecek.
-- =============================================
local NPC = 29014;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 23029, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 23029, NPC)
	else
		EVENT = QuestNum
	end
	end
	
if (EVENT == 1001)then
SelectMsg(UID, 2, 696, 22293, NPC, 10,1002);
end

if (EVENT == 1002)then
SelectMsg(UID, 2, 696, 22299, NPC, 3000, 1003,3005,-1);
SaveEvent(UID, 13003);
end

if (EVENT == 1003)then
SelectMsg(UID, 4, 696, 22299, NPC, 3000, 1004,3005,-1);
SaveEvent(UID, 13005);
end

if (EVENT == 1004)then
SelectMsg(UID, 2, 696, 22301, NPC, 10, -1);
SaveEvent(UID, 13004);
SaveEvent(UID, 13015);
end

if (EVENT == 1101)then
	SelectMsg(UID, 4, 697, 22158, NPC, 3000,1102,4005,-1);
end

if (EVENT == 1102)then
	SaveEvent(UID, 13015);
end

if (EVENT == 1106)then
	SaveEvent(UID, 13017);
end

if (EVENT == 1105)then
	ITEM1_COUNT = HowmuchItem(UID, 900216000);  
    ITEM2_COUNT = HowmuchItem(UID, 900217000); 	
	if (ITEM1_COUNT < 1) then
	SelectMsg(UID, 2, 697, 22158, NPC, 18, 1108);
	elseif (ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 697, 22158, NPC, 18, 1109);
	else
	SelectMsg(UID, 4, 697, 22158, NPC, 22, 1107, 27, -1);
end
end

if (EVENT == 1108) then
	ShowMap(UID, 5);
end

if (EVENT == 1109) then
	ShowMap(UID, 674);
end

if (EVENT == 1107) then
    SaveEvent(UID, 13016);
	SaveEvent(UID, 13027);
    RunQuestExchange(UID, 3170)
	SelectMsg(UID, 2, -1, 22313, NPC, 10, -1);
end

if (EVENT == 1201)then
	SelectMsg(UID, 4, 698, 22160, NPC, 3000,1202,4005,-1);
end

if (EVENT == 1202)then
	SaveEvent(UID, 13027);
end

if (EVENT == 1206)then
	SaveEvent(UID, 13029);
end

if (EVENT == 1205)then
	ITEM1_COUNT = HowmuchItem(UID, 900218000);  
	if (ITEM1_COUNT < 1) then
	SelectMsg(UID, 2, 698, 22160, NPC, 18, -1);
	else
	SelectMsg(UID, 4, 698, 22160, NPC, 22, 1207, 27, -1);
end
end

if (EVENT == 1208) then
	ShowMap(UID, 1340);
end

if (EVENT == 1207) then
    SaveEvent(UID, 13028);
	SaveEvent(UID, 13039);
    RunQuestExchange(UID, 3171);
	SelectMsg(UID, 2, -1, 22334, NPC, 10, 1208);
end

if (EVENT == 1301)then
	SelectMsg(UID, 4, 699, 22160, NPC, 3000,1302,4005,-1);
end

if (EVENT == 1302)then
	SaveEvent(UID, 13039);
end

if (EVENT == 1306)then
	SaveEvent(UID, 13041);
end

if (EVENT == 1305) then
	MonsterCount01 = CountMonsterQuestSub(UID, 699, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 699, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 5, 699, 22160, NPC,3000,1309,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 699, 22160, NPC, 18, 1307);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 699, 22160, NPC, 18, 1308);
		end
	end
end

if (EVENT == 1307) then
	ShowMap(UID, 858);
end

if (EVENT == 1308) then
	ShowMap(UID, 860);
end

if (EVENT == 1309) then
    SaveEvent(UID, 13040);
	SaveEvent(UID, 13051);
    RunQuestExchange(UID, 3172,STEP,1);
	ShowEffect(UID, 300391)
	SelectMsg(UID, 2, -1, 22777, NPC, 10, -1);
end

if (EVENT == 1401)then
	SelectMsg(UID, 4, 701, 22164, NPC, 3000,1402,4005,-1);
end

if (EVENT == 1402)then
	SaveEvent(UID, 13051);
end

if (EVENT == 1406)then
	SaveEvent(UID, 13053);
end

if (EVENT == 1405)then
	ITEM1_COUNT = HowmuchItem(UID, 900219000);  
	ITEM2_COUNT = HowmuchItem(UID, 900220000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 701, 22164, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 701, 22164, NPC, 22, 1407, 27, -1);
end
end

if (EVENT == 1407) then
    SaveEvent(UID, 13052);
	SaveEvent(UID, 13063);
    RunQuestExchange(UID, 3173)
	SelectMsg(UID, 2, -1, 22349, NPC, 10, 1508,4005,-1);
end

if (EVENT == 1501)then
	SelectMsg(UID, 4, 702, 22166, NPC, 3000,1502,4005,-1);
end

if (EVENT == 1502)then
	SaveEvent(UID, 13063);
end

if (EVENT == 1506)then
	SaveEvent(UID, 13065);
end

if (EVENT == 1505)then
	QuestStatusCheck = GetQuestStatus(UID, 702)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22349, NPC, 10, 1508,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900221000);  
    ITEM2_COUNT = HowmuchItem(UID, 900222000); 
    ITEM3_COUNT = HowmuchItem(UID, 900223000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22349, NPC, 10, 1508,4005,-1);
	else
	SelectMsg(UID, 5, 702, 22166, NPC,3000,1507,3005,-1);
end
end
end

if (EVENT == 1508)then
	MonsterStoneQuestJoin(UID,702);
end

if (EVENT == 1507) then
    SaveEvent(UID, 13064);
	SaveEvent(UID, 13075);
    RunQuestExchange(UID, 3174,STEP,1);
	SelectMsg(UID, 2, -1, 22370, NPC, 10, -1);
	RobItem(UID, 900221000, 1);
	RobItem(UID, 900222000, 1);
	RobItem(UID, 900223000, 1);
end

if (EVENT == 1601)then
	SelectMsg(UID, 4, 703, 22168, NPC, 3000,1602,4005,-1);
end

if (EVENT == 1602)then
	SaveEvent(UID, 13075);
end

if (EVENT == 1606)then
	SaveEvent(UID, 13077);
end

if (EVENT == 1605) then
	MonsterCount = CountMonsterQuestSub(UID, 703, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 703, 22168, NPC, 18, 1607);
	else
		SelectMsg(UID, 4, 703, 22168, NPC, 3000, 1608, 27, -1);
	end
end

if (EVENT == 1607) then
	ShowMap(UID, 829);
end

if (EVENT == 1608) then
    SaveEvent(UID, 13076);
	SaveEvent(UID, 13087);
	RunQuestExchange(UID,3175)
	SelectMsg(UID, 2, -1, 22382, NPC, 10, -1);
end

if (EVENT == 1701)then
	SelectMsg(UID, 4, 705, 22170, NPC, 3000,1702,4005,-1);
end

if (EVENT == 1702)then
	SaveEvent(UID, 13087);
end

if (EVENT == 1706)then
	SaveEvent(UID, 13089);
end

if (EVENT == 1705) then
	MonsterCount01 = CountMonsterQuestSub(UID, 705, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 705, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 5, 705, 22170, NPC,3000,1709,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 705, 22170, NPC, 18, 1707);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 705, 22170, NPC, 18, 1708);
		end
	end
end

if (EVENT == 1707) then
	ShowMap(UID, 859);
end

if (EVENT == 1708) then
	ShowMap(UID, 861);
end

if (EVENT == 1709) then
    SaveEvent(UID, 13088);
	SaveEvent(UID, 13099);
    RunQuestExchange(UID, 3176,STEP,1);
	ShowEffect(UID, 300391)
	SelectMsg(UID, 2, -1, 22777, NPC, 10, -1);
end

if (EVENT == 1801)then
	SelectMsg(UID, 4, 707, 22172, NPC, 3000,1802,4005,-1);
end

if (EVENT == 1802)then
	SaveEvent(UID, 13099);
end

if (EVENT == 1806)then
	SaveEvent(UID, 13101);
end

if (EVENT == 1805)then
	ITEM1_COUNT = HowmuchItem(UID, 900225000);  
	ITEM2_COUNT = HowmuchItem(UID, 900226000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 707, 22172, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 707, 22172, NPC, 22, 1807, 27, -1);
end
end

if (EVENT == 1807) then
    SaveEvent(UID, 13100);
	SaveEvent(UID, 13111);
    RunQuestExchange(UID, 3177)
	SelectMsg(UID, 2, -1, 22397, NPC, 10, 1908,4005,-1);
end

if (EVENT == 1901)then
	SelectMsg(UID, 4, 708, 22172, NPC, 3000,1902,4005,-1);
end

if (EVENT == 1902)then
	SaveEvent(UID, 13111);
end

if (EVENT == 1906)then
	SaveEvent(UID, 13113);
end

if (EVENT == 1908)then
	MonsterStoneQuestJoin(UID,708);
end

if (EVENT == 1905)then
	QuestStatusCheck = GetQuestStatus(UID, 708)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22397, NPC, 10, 1908,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900227000);  
    ITEM2_COUNT = HowmuchItem(UID, 900228000); 
    ITEM3_COUNT = HowmuchItem(UID, 900229000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22397, NPC, 10, 1908,4005,-1);
	else
	SelectMsg(UID, 5, 708, 22172, NPC,3000,1907,3005,-1);
end
end
end

if (EVENT == 1907) then
    SaveEvent(UID, 13112);
	SaveEvent(UID, 13123);
    RunQuestExchange(UID, 3178,STEP,1);
	RobItem(UID, 900227000, 1);
	RobItem(UID, 900228000, 1);
	RobItem(UID, 900229000, 1);
	SelectMsg(UID, 2, -1, 22409, NPC, 10, -1);
end

if (EVENT == 2001)then
	SelectMsg(UID, 4, 709, 22176, NPC, 3000,2002,4005,-1);
end

if (EVENT == 2002)then
	SaveEvent(UID, 13123);
end

if (EVENT == 2006)then
	SaveEvent(UID, 13125);
end

if (EVENT == 2005) then
	MonsterCount = CountMonsterQuestSub(UID, 709, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 709, 22176, NPC, 18, 2007);
	else
		SelectMsg(UID, 4, 709, 22176, NPC, 3000, 2008, 27, -1);
	end
end

if (EVENT == 2007) then
	ShowMap(UID, 830);
end

if (EVENT == 2008) then
	SaveEvent(UID, 13124);
	SaveEvent(UID, 13135);
    RunQuestExchange(UID, 3179)
	SelectMsg(UID, 2, -1, 22409, NPC, 10, -1);
end

if (EVENT == 2101)then
	SelectMsg(UID, 4, 711, 22178, NPC, 3000,2102,4005,-1);
end

if (EVENT == 2102)then
	SaveEvent(UID, 13135);
end

if (EVENT == 2106)then
	SaveEvent(UID, 13137);
end

if (EVENT == 2105) then
	MonsterCount01 = CountMonsterQuestSub(UID, 711, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 711, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 5, 711, 22178, NPC,3000,2109,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 711, 22178, NPC, 18, 2107);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 711, 22178, NPC, 18, 2108);
		end
	end
end

if (EVENT == 2107) then
	ShowMap(UID, 870);
end

if (EVENT == 2108) then
	ShowMap(UID, 872);
end

if (EVENT == 2109) then
    SaveEvent(UID, 13136);
	SaveEvent(UID, 13147);
    RunQuestExchange(UID, 3180,STEP,1);
	ShowEffect(UID, 300391)
	--SelectMsg(UID, 2, -1, 22777, NPC, 10, -1);
end

if (EVENT == 2201)then
	SelectMsg(UID, 4, 713, 22180, NPC, 3000,2202,4005,-1);
end

if (EVENT == 2202)then
	SaveEvent(UID, 13147);
end

if (EVENT == 2206)then
	SaveEvent(UID, 13149);
end

if (EVENT == 2205)then
	ITEM1_COUNT = HowmuchItem(UID, 900231000);  
	ITEM2_COUNT = HowmuchItem(UID, 900232000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 713, 22180, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 713, 22180, NPC, 22, 2207, 27, -1);
end
end

if (EVENT == 2207) then
    SaveEvent(UID, 13148);
	SaveEvent(UID, 13159);
    RunQuestExchange(UID, 3181)
	SelectMsg(UID, 2, -1, 22456, NPC, 10, 2308,4005,-1);
end

if (EVENT == 2301)then
	SelectMsg(UID, 4, 714, 22182, NPC, 3000,2302,4005,-1);
end

if (EVENT == 2302)then
	SaveEvent(UID, 13159);
end

if (EVENT == 2306)then
	SaveEvent(UID, 13161);
end

if (EVENT == 2308)then
	MonsterStoneQuestJoin(UID,714);
end

if (EVENT == 2305)then
	QuestStatusCheck = GetQuestStatus(UID, 714)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22456, NPC, 10, 2308,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900233000);  
    ITEM2_COUNT = HowmuchItem(UID, 900234000); 
    ITEM3_COUNT = HowmuchItem(UID, 900235000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22456, NPC, 10, 2308,4005,-1);
	else
	SelectMsg(UID, 5, 714, 22182, NPC,3000,2307,3005,-1);
end
end
end

if (EVENT == 2307) then
    SaveEvent(UID, 13160);
	SaveEvent(UID, 13171);
    RunQuestExchange(UID, 3182,STEP,1);
	RobItem(UID, 900233000, 1);
	RobItem(UID, 900234000, 1);
	RobItem(UID, 900235000, 1);
	SelectMsg(UID, 2, -1, 22468, NPC, 10, -1);
end

if (EVENT == 2401)then
	SelectMsg(UID, 4, 715, 22184, NPC, 3000,2402,4005,-1);
end

if (EVENT == 2402)then
	SaveEvent(UID, 13171);
end

if (EVENT == 2406)then
	SaveEvent(UID, 13173);
end

if (EVENT == 2405) then
	MonsterCount = CountMonsterQuestSub(UID, 715, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 715, 22184, NPC, 18, 2407);
	else
		SelectMsg(UID, 4, 715, 22184, NPC, 3000, 2408, 27, -1);
	end
end

if (EVENT == 2407) then
	ShowMap(UID, 831);
end

if (EVENT == 2408) then
    SaveEvent(UID, 13172);
	SaveEvent(UID, 13183);
    RunQuestExchange(UID, 3183)
end

if (EVENT == 2501)then
	SelectMsg(UID, 4, 717, 22186, NPC, 3000,2502,4005,-1);
end

if (EVENT == 2502)then
	SaveEvent(UID, 13183);
end

if (EVENT == 2506)then
	SaveEvent(UID, 13185);
end

if (EVENT == 2505) then
	MonsterCount01 = CountMonsterQuestSub(UID, 717, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 717, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 5, 717, 22186, NPC,3000,2509,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 717, 22186, NPC, 18, 2507);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 717, 22186, NPC, 18, 2508);
		end
	end
end

if (EVENT == 2507) then
	ShowMap(UID, 871);
end

if (EVENT == 2508) then
	ShowMap(UID, 873);
end

if (EVENT == 2509) then
    SaveEvent(UID, 13184);
	SaveEvent(UID, 13195);
    RunQuestExchange(UID, 3184,STEP,1);
	ShowEffect(UID, 300391)
end

if (EVENT == 2601)then
	SelectMsg(UID, 4, 719, 22188, NPC, 3000,2602,4005,-1);
end

if (EVENT == 2602)then
	SaveEvent(UID, 13195);
end

if (EVENT == 2606)then
	SaveEvent(UID, 13197);
end

if (EVENT == 2605)then
	ITEM1_COUNT = HowmuchItem(UID, 900237000);  
	ITEM2_COUNT = HowmuchItem(UID, 900238000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 719, 22188, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 719, 22188, NPC, 22, 2607, 27, -1);
end
end

if (EVENT == 2607) then
    SaveEvent(UID, 13196);
	SaveEvent(UID, 13207);
    RunQuestExchange(UID, 3185)
	SelectMsg(UID, 2, -1, 22370, NPC, 10, 2708,4005,-1);
end

if (EVENT == 2701)then
	SelectMsg(UID, 4, 720, 22190, NPC, 3000,2702,4005,-1);
end

if (EVENT == 2702)then
	SaveEvent(UID, 13207);
end

if (EVENT == 2706)then
	SaveEvent(UID, 13209);
end

if (EVENT == 2708)then
MonsterStoneQuestJoin(UID,720);
end

if (EVENT == 2705)then
	QuestStatusCheck = GetQuestStatus(UID, 720)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22370, NPC, 10, 2708,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900239000);  
    ITEM2_COUNT = HowmuchItem(UID, 900240000); 
    ITEM3_COUNT = HowmuchItem(UID, 900241000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22370, NPC, 10, 2708,4005,-1);
	else
	SelectMsg(UID, 5, 720, 22190, NPC,3000,2707,3005,-1);
end
end
end

if (EVENT == 2707) then
    SaveEvent(UID, 13208);
	SaveEvent(UID, 13219);
    RunQuestExchange(UID, 3186,STEP,1);
	RobItem(UID, 900239000, 1);
	RobItem(UID, 900240000, 1);
	RobItem(UID, 900241000, 1);
	SelectMsg(UID, 2, -1, 22507, NPC, 10, -1);
end

if (EVENT == 2801)then
	SelectMsg(UID, 4, 721, 22192, NPC, 3000,2802,4005,-1);
end

if (EVENT == 2802)then
	SaveEvent(UID, 13219);
end

if (EVENT == 2806)then
	SaveEvent(UID, 13221);
end

if (EVENT == 2805) then
	MonsterCount = CountMonsterQuestSub(UID, 721, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 721, 22192, NPC, 18, 2807);
	else
		SelectMsg(UID, 4, 721, 22192, NPC, 3000, 2808, 27, -1);
	end
end

if (EVENT == 2807) then
	ShowMap(UID, 832);
end

if (EVENT == 2808) then
    SaveEvent(UID, 13220);
	SaveEvent(UID, 13231);
    RunQuestExchange(UID, 3187)
	SelectMsg(UID, 2, -1, 22530, NPC, 10, -1);
end

if (EVENT == 2901)then
	SelectMsg(UID, 4, 723, 22194, NPC, 3000,2902,4005,-1);
end

if (EVENT == 2902)then
	SaveEvent(UID, 13231);
end

if (EVENT == 2906)then
	SaveEvent(UID, 13233);
end

if (EVENT == 2905) then
	MonsterCount01 = CountMonsterQuestSub(UID, 723, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 723, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 5, 723, 22186, NPC,3000,2909,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 723, 22186, NPC, 18, 2907);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 723, 22186, NPC, 18, 2908);
		end
	end
end

if (EVENT == 2907) then
	ShowMap(UID, 882);
end

if (EVENT == 2908) then
	ShowMap(UID, 884);
end

if (EVENT == 2909) then
    SaveEvent(UID, 13232);
	SaveEvent(UID, 13243);
    RunQuestExchange(UID, 3188,STEP,1);
	ShowEffect(UID, 300391)
	SelectMsg(UID, 2, -1, 22777, NPC, 10, -1);
end

if (EVENT == 3001)then
	SelectMsg(UID, 4, 725, 22196, NPC, 3000,3002,4005,-1);
end

if (EVENT == 3002)then
	SaveEvent(UID, 13243);
end

if (EVENT == 3006)then
	SaveEvent(UID, 13245);
end

if (EVENT == 3005)then
	ITEM1_COUNT = HowmuchItem(UID, 900243000);  
	ITEM2_COUNT = HowmuchItem(UID, 900244000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 725, 22196, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 725, 22196, NPC, 22, 3007, 27, -1);
end
end

if (EVENT == 3007) then
    SaveEvent(UID, 13244);
	SaveEvent(UID, 13255);
    RunQuestExchange(UID, 3189)
	SelectMsg(UID, 2, -1, 22554, NPC, 10, 3008,4005,-1);
end

if (EVENT == 3101)then
	SelectMsg(UID, 4, 726, 22198, NPC, 3000,3102,4005,-1);
end

if (EVENT == 3102)then
	SaveEvent(UID, 13255);
end

if (EVENT == 3106)then
	SaveEvent(UID, 13257);
end

if (EVENT == 3008)then
	MonsterStoneQuestJoin(UID,726);
end

if (EVENT == 3105)then
	QuestStatusCheck = GetQuestStatus(UID, 726)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22554, NPC, 10, 3008,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900245000);  
    ITEM2_COUNT = HowmuchItem(UID, 900246000); 
    ITEM3_COUNT = HowmuchItem(UID, 900247000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22554, NPC, 10, 3008,4005,-1);
	else
	SelectMsg(UID, 5, 726, 22190, NPC,3000,3107,3005,-1);
end
end
end

if (EVENT == 3107) then
    SaveEvent(UID, 13256);
	SaveEvent(UID, 13267);
    RunQuestExchange(UID, 3190,STEP,1);
	RobItem(UID, 900245000, 1);
	RobItem(UID, 900246000, 1);
	RobItem(UID, 900247000, 1);
	SelectMsg(UID, 2, -1, 22566, NPC, 10, -1);
end

if (EVENT == 3201)then
	SelectMsg(UID, 4, 727, 22200, NPC, 3000,3202,4005,-1);
end

if (EVENT == 3202)then
	SaveEvent(UID, 13267);
end

if (EVENT == 3206)then
	SaveEvent(UID, 13269);
end

if (EVENT == 3205) then
	MonsterCount = CountMonsterQuestSub(UID, 727, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 727, 22200, NPC, 18, 3207);
	else
		SelectMsg(UID, 4, 727, 22200, NPC, 3000, 3208, 27, -1);
	end
end

if (EVENT == 3207) then
	ShowMap(UID, 833);
end

if (EVENT == 3208) then
    SaveEvent(UID, 13268);
	SaveEvent(UID, 13279);
    RunQuestExchange(UID, 3191)
	SelectMsg(UID, 2, -1, 22578, NPC, 10, -1);
end

if (EVENT == 3301)then
	SelectMsg(UID, 4, 729, 22202, NPC, 3000,3302,4005,-1);
end

if (EVENT == 3302)then
	SaveEvent(UID, 13279);
end

if (EVENT == 3306)then
	SaveEvent(UID, 13281);
end

if (EVENT == 3305) then
	MonsterCount01 = CountMonsterQuestSub(UID, 729, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 729, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 5, 729, 22202, NPC,3000,3309,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 729, 22202, NPC, 18, 3307);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 729, 22202, NPC, 18, 3308);
		end
	end
end

if (EVENT == 3307) then
	ShowMap(UID, 883);
end

if (EVENT == 3308) then
	ShowMap(UID, 885);
end

if (EVENT == 3309) then
    SaveEvent(UID, 13280);
	SaveEvent(UID, 13291);
    RunQuestExchange(UID, 3192,STEP,1);
	SelectMsg(UID, 2, -1, 22777, NPC, 10, -1);
end

if (EVENT == 3401)then
	SelectMsg(UID, 4, 731, 22204, NPC, 3000,3402,4005,-1);
end

if (EVENT == 3402)then
	SaveEvent(UID, 13291);
end

if (EVENT == 3406)then
	SaveEvent(UID, 13293);
end

if (EVENT == 3405)then
	ITEM1_COUNT = HowmuchItem(UID, 900249000);  
	ITEM2_COUNT = HowmuchItem(UID, 900250000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 731, 22204, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 731, 22204, NPC, 22, 3407, 27, -1);
end
end

if (EVENT == 3407) then
    SaveEvent(UID, 13292);
	SaveEvent(UID, 13303);
    RunQuestExchange(UID, 3193)
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 3508,4005,-1);
end

if (EVENT == 3501)then
	SelectMsg(UID, 4, 732, 22206, NPC, 3000,3502,4005,-1);
end

if (EVENT == 3502)then
	SaveEvent(UID, 13303);
end

if (EVENT == 3506)then
	SaveEvent(UID, 13305);
end

if (EVENT == 3508)then
	MonsterStoneQuestJoin(UID,732);
end

if (EVENT == 3505)then
	QuestStatusCheck = GetQuestStatus(UID, 732)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 3508,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900251000);  
    ITEM2_COUNT = HowmuchItem(UID, 900252000); 
    ITEM3_COUNT = HowmuchItem(UID, 900253000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 3508,4005,-1);
	else
	SelectMsg(UID, 4, 732, 22206, NPC,3000,3507,3005,-1);
end
end
end

if (EVENT == 3507) then
    SaveEvent(UID, 13304);
	SaveEvent(UID, 13315);
    RunQuestExchange(UID, 3194)
	SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 3601)then
	SelectMsg(UID, 4, 733, 22208, NPC, 3000,3602,4005,-1);
end

if (EVENT == 3602)then
	SaveEvent(UID, 13315);
end

if (EVENT == 3606)then
	SaveEvent(UID, 13317);
end

if (EVENT == 3605) then
	MonsterCount = CountMonsterQuestSub(UID, 733, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 733, 22208, NPC, 18, 3607);
	else
		SelectMsg(UID, 4, 733, 22208, NPC, 3000, 3608, 27, -1);
	end
end

if (EVENT == 3607) then
	ShowMap(UID, 834);
end

if (EVENT == 3608) then
    SaveEvent(UID, 13316);
	SaveEvent(UID, 13327);
    RunQuestExchange(UID, 3195)
	SelectMsg(UID, 2, -1, 22628, NPC, 10,-1);
end

if (EVENT == 3701)then
	SelectMsg(UID, 4, 735, 22210, NPC, 3000,3702,4005,-1);
end

if (EVENT == 3702)then
	SaveEvent(UID, 13327);
end

if (EVENT == 3706)then
	SaveEvent(UID, 13329);
end

if (EVENT == 3705) then
	MonsterCount01 = CountMonsterQuestSub(UID, 735, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 735, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 4, 735, 22210, NPC,3000,3709,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 735, 22210, NPC, 18, 3707);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 735, 22210, NPC, 18, 3708);
		end
	end
end

if (EVENT == 3707) then
	ShowMap(UID, 894);
end

if (EVENT == 3708) then
	ShowMap(UID, 896);
end

if (EVENT == 3709) then
    SaveEvent(UID, 13328);
	SaveEvent(UID, 13339);
    RunQuestExchange(UID, 3196)
	SelectMsg(UID, 2, -1, 22628, NPC, 10,-1);
end

if (EVENT == 3801)then
	SelectMsg(UID, 4, 737, 22212, NPC, 3000,3802,4005,-1);
end

if (EVENT == 3802)then
	SaveEvent(UID, 13339);
end

if (EVENT == 3806)then
	SaveEvent(UID, 13341);
end

if (EVENT == 3805)then
	ITEM1_COUNT = HowmuchItem(UID, 900255000);  
	ITEM2_COUNT = HowmuchItem(UID, 900256000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 737, 22212, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 737, 22212, NPC, 22, 3807, 27, -1);
end
end

if (EVENT == 3807) then
    SaveEvent(UID, 13340);
	SaveEvent(UID, 13351);
    RunQuestExchange(UID, 3197)
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 3908,4005,-1);
end

if (EVENT == 3901)then
	SelectMsg(UID, 4, 738, 22214, NPC, 3000,3902,4005,-1);
end

if (EVENT == 3902)then
	SaveEvent(UID, 13351);
end

if (EVENT == 3906)then
	SaveEvent(UID, 13353);
end

if (EVENT == 3908)then
	MonsterStoneQuestJoin(UID,738);
end

if (EVENT == 3905)then
	QuestStatusCheck = GetQuestStatus(UID, 738)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 3908,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900257000);  
    ITEM2_COUNT = HowmuchItem(UID, 900258000); 
    ITEM3_COUNT = HowmuchItem(UID, 900259000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 3908,4005,-1);
	else
	SelectMsg(UID, 4, 738, 22206, NPC,3000,3907,3005,-1);
end
end
end

if (EVENT == 3907) then
    SaveEvent(UID, 13352);
	SaveEvent(UID, 13363);
    RunQuestExchange(UID, 3198)
	--SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 4001)then
	SelectMsg(UID, 4, 739, 22216, NPC, 3000,4002,4005,-1);
end

if (EVENT == 4002)then
	SaveEvent(UID, 13363);
end

if (EVENT == 4006)then
	SaveEvent(UID, 13365);
end

if (EVENT == 4005) then
	MonsterCount = CountMonsterQuestSub(UID, 739, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 739, 22216, NPC, 18, 4007);
	else
		SelectMsg(UID, 4, 739, 22216, NPC, 3000, 4008, 27, -1);
	end
end

if (EVENT == 4008) then
    SaveEvent(UID, 13364);
	SaveEvent(UID, 13375);
    RunQuestExchange(UID, 3199)
	--SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 4101)then
	SelectMsg(UID, 4, 741, 22218, NPC, 3000,4102,4005,-1);
end

if (EVENT == 4102)then
	SaveEvent(UID, 13375);
end

if (EVENT == 4106)then
	SaveEvent(UID, 13377);
end

if (EVENT == 4105) then
	MonsterCount01 = CountMonsterQuestSub(UID, 741, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 741, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 4, 741, 22218, NPC,3000,4109,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 741, 22218, NPC, 18, 4107);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 741, 22218, NPC, 18, 4108);
		end
	end
end

if (EVENT == 4107) then
	ShowMap(UID, 895);
end

if (EVENT == 4108) then
	ShowMap(UID, 897);
end

if (EVENT == 4109) then
    SaveEvent(UID, 13376);
	SaveEvent(UID, 13387);
    RunQuestExchange(UID, 3200)
	--SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 4201)then
	SelectMsg(UID, 4, 743, 22220, NPC, 3000,4202,4005,-1);
end

if (EVENT == 4202)then
	SaveEvent(UID, 13387);
end

if (EVENT == 4206)then
	SaveEvent(UID, 13389);
end

if (EVENT == 4205)then
	ITEM1_COUNT = HowmuchItem(UID, 900261000);  
	ITEM2_COUNT = HowmuchItem(UID, 900262000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 743, 22220, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 743, 22220, NPC, 22, 4208, 27, -1);
end
end

if (EVENT == 4208) then
    SaveEvent(UID, 13388);
	SaveEvent(UID, 13399);
    RunQuestExchange(UID, 3201)
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 4309,4005,-1);
end

if (EVENT == 4301)then
	SelectMsg(UID, 4, 744, 22222, NPC, 3000,4302,4005,-1);
end

if (EVENT == 4302)then
	SaveEvent(UID, 13399);
end

if (EVENT == 4306)then
	SaveEvent(UID, 13401);
end

if (EVENT == 4309)then
	MonsterStoneQuestJoin(UID,744);
end

if (EVENT == 4305)then
	QuestStatusCheck = GetQuestStatus(UID, 744)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 4309,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900263000);  
    ITEM2_COUNT = HowmuchItem(UID, 900264000); 
    ITEM3_COUNT = HowmuchItem(UID, 900265000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 4309,4005,-1);
	else
	SelectMsg(UID, 4, 744, 22222, NPC,3000,4307,3005,-1);
end
end
end

if (EVENT == 4307) then
    SaveEvent(UID, 13400);
	SaveEvent(UID, 13411);
    RunQuestExchange(UID, 3202)
	--SelectMsg(UID, 2, -1, 22602, NPC, 10, 4209,4005,-1);
end

if (EVENT == 4401)then
	SelectMsg(UID, 4, 745, 22224, NPC, 3000,4402,4005,-1);
end

if (EVENT == 4402)then
	SaveEvent(UID, 13411);
end

if (EVENT == 4406)then
	SaveEvent(UID, 13413);
end

if (EVENT == 4405) then
	MonsterCount = CountMonsterQuestSub(UID, 745, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 745, 22224, NPC, 18, 4407);
	else
		SelectMsg(UID, 4, 745, 22224, NPC, 3000, 4408, 27, -1);
	end
end

if (EVENT == 4407) then
	ShowMap(UID, 836);
end

if (EVENT == 4408) then
    SaveEvent(UID, 13412);
	SaveEvent(UID, 13423);
    RunQuestExchange(UID, 3203)
	--SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 4501)then
	SelectMsg(UID, 4, 747, 22226, NPC, 3000,4502,4005,-1);
end

if (EVENT == 4502)then
	SaveEvent(UID, 13423);
end

if (EVENT == 4506)then
	SaveEvent(UID, 13425);
end

if (EVENT == 4505) then
	MonsterCount01 = CountMonsterQuestSub(UID, 747, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 747, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 4, 747, 22226, NPC,3000,4509,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 747, 22226, NPC, 18, 4507);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 747, 22226, NPC, 18, 4508);
		end
	end
end

if (EVENT == 4507) then
	ShowMap(UID, 906);
end

if (EVENT == 4508) then
	ShowMap(UID, 908);
end

if (EVENT == 4509) then
    SaveEvent(UID, 13424);
	SaveEvent(UID, 13435);
    RunQuestExchange(UID, 3204)
	--SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 4601)then
	SelectMsg(UID, 4, 749, 22228, NPC, 3000,4602,4005,-1);
end

if (EVENT == 4602)then
	SaveEvent(UID, 13435);
end

if (EVENT == 4606)then
	SaveEvent(UID, 13437);
end

if (EVENT == 4605)then
	ITEM1_COUNT = HowmuchItem(UID, 900267000);  
	ITEM2_COUNT = HowmuchItem(UID, 900268000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 749, 22228, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 749, 22228, NPC, 22, 4608, 27, -1);
end
end

if (EVENT == 4608) then
    SaveEvent(UID, 13436);
	SaveEvent(UID, 13447);
    RunQuestExchange(UID, 3205)
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 4709,4005,-1);
end

if (EVENT == 4701)then
	SelectMsg(UID, 4, 750, 22230, NPC, 3000,4702,4005,-1);
end

if (EVENT == 4702)then
	SaveEvent(UID, 13447);
end

if (EVENT == 4706)then
	SaveEvent(UID, 13449);
end

if (EVENT == 4709)then
	MonsterStoneQuestJoin(UID,750);
end

if (EVENT == 4705)then
	QuestStatusCheck = GetQuestStatus(UID, 750)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 4709,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900269000);  
    ITEM2_COUNT = HowmuchItem(UID, 900270000); 
    ITEM3_COUNT = HowmuchItem(UID, 900271000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 4709,4005,-1);
	else
	SelectMsg(UID, 4, 750, 22230, NPC,3000,4707,3005,-1);
end
end
end

if (EVENT == 4707) then
    SaveEvent(UID, 13448);
	SaveEvent(UID, 13459);
    RunQuestExchange(UID, 3206)
	--SelectMsg(UID, 2, -1, 22602, NPC, 10, 4209,4005,-1);
end

if (EVENT == 4801)then
	SelectMsg(UID, 4, 751, 22232, NPC, 3000,4802,4005,-1);
end

if (EVENT == 4802)then
	SaveEvent(UID, 13459);
end

if (EVENT == 4806)then
	SaveEvent(UID, 13461);
end

if (EVENT == 4805) then
	MonsterCount = CountMonsterQuestSub(UID, 751, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 751, 22232, NPC, 18, 4807);
	else
		SelectMsg(UID, 4, 751, 22232, NPC, 3000, 4808, 27, -1);
	end
end

if (EVENT == 4807) then
	ShowMap(UID, 904);
end

if (EVENT == 4808) then
    SaveEvent(UID, 13460);
	SaveEvent(UID, 13471);
    RunQuestExchange(UID, 3207)
	SelectMsg(UID, 2, -1, 22774, NPC, 10,-1);
end

if (EVENT == 4901)then
	SelectMsg(UID, 4, 753, 22234, NPC, 3000,4902,4005,-1);
end

if (EVENT == 4902)then
	SaveEvent(UID, 13471);
end

if (EVENT == 4906)then
	SaveEvent(UID, 13473);
end

if (EVENT == 4905) then
	MonsterCount01 = CountMonsterQuestSub(UID, 753, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 753, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 4, 753, 22234, NPC,3000,4909,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 753, 22234, NPC, 18, 4907);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 753, 22234, NPC, 18, 4908);
		end
	end
end

if (EVENT == 4907) then
	ShowMap(UID, 907);
end
if (EVENT == 4908) then
	ShowMap(UID, 909);
end

if (EVENT == 4909) then
    SaveEvent(UID, 13472);
	SaveEvent(UID, 13483);
    RunQuestExchange(UID, 3208)
	SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 5001)then
	SelectMsg(UID, 4, 755, 22236, NPC, 3000,5002,4005,-1);
end

if (EVENT == 5002)then
	SaveEvent(UID, 13483);
end

if (EVENT == 5006)then
	SaveEvent(UID, 13485);
end

if (EVENT == 5005)then
	ITEM1_COUNT = HowmuchItem(UID, 900273000);  
	ITEM2_COUNT = HowmuchItem(UID, 900274000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 755, 22236, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 755, 22236, NPC, 22, 5008, 27, -1);
end
end

if (EVENT == 5008) then
    SaveEvent(UID, 13484);
	SaveEvent(UID, 13495);
    RunQuestExchange(UID, 3209)
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5109,4005,-1);
end

if (EVENT == 5101)then
	SelectMsg(UID, 4, 756, 22238, NPC, 3000,5102,4005,-1);
end

if (EVENT == 5102)then
	SaveEvent(UID, 13495);
end

if (EVENT == 5106)then
	SaveEvent(UID, 13497);
end

if (EVENT == 5109)then
	MonsterStoneQuestJoin(UID,756);
end

if (EVENT == 5105)then
	QuestStatusCheck = GetQuestStatus(UID, 756)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5109,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900275000);  
    ITEM2_COUNT = HowmuchItem(UID, 900276000); 
    ITEM3_COUNT = HowmuchItem(UID, 900277000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5109,4005,-1);
	else
	SelectMsg(UID, 4, 756, 22238, NPC,3000,5107,3005,-1);
end
end
end

if (EVENT == 5107) then
    SaveEvent(UID, 13496);
	SaveEvent(UID, 13507);
    RunQuestExchange(UID, 3210)
	--SelectMsg(UID, 2, -1, 22602, NPC, 10, 4209,4005,-1);
end

if (EVENT == 5201)then
	SelectMsg(UID, 4, 757, 22240, NPC, 3000,5202,4005,-1);
end

if (EVENT == 5202)then
	SaveEvent(UID, 13507);
end

if (EVENT == 5206)then
	SaveEvent(UID, 13509);
end

if (EVENT == 5205) then
	MonsterCount = CountMonsterQuestSub(UID, 757, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 757, 22240, NPC, 18, 5207);
	else
		SelectMsg(UID, 4, 757, 22240, NPC, 3000, 5208, 27, -1);
	end
end

if (EVENT == 5207) then
	ShowMap(UID, 905);
end

if (EVENT == 5208) then
    SaveEvent(UID, 13508);
	SaveEvent(UID, 13519);
    RunQuestExchange(UID, 3211)
	SelectMsg(UID, 2, -1, 22774, NPC, 10,-1);
end

if (EVENT == 5301)then
	SelectMsg(UID, 4, 759, 22242, NPC, 3000,5302,4005,-1);
end

if (EVENT == 5302)then
	SaveEvent(UID, 13519);
end

if (EVENT == 5306)then
	SaveEvent(UID, 13521);
end

if (EVENT == 5305) then
	MonsterCount01 = CountMonsterQuestSub(UID, 759, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 759, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 4, 759, 22242, NPC,3000,5309,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 759, 22242, NPC, 18, 5307);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 759, 22242, NPC, 18, 5308);
		end
	end
end

if (EVENT == 5307) then
	ShowMap(UID, 907);
end
if (EVENT == 5308) then
	ShowMap(UID, 909);
end

if (EVENT == 5309) then
    SaveEvent(UID, 13520);
	SaveEvent(UID, 13531);
    RunQuestExchange(UID, 3212)
	SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 5401)then
	SelectMsg(UID, 4, 761, 22244, NPC, 3000,5402,4005,-1);
end

if (EVENT == 5402)then
	SaveEvent(UID, 13531);
end

if (EVENT == 5406)then
	SaveEvent(UID, 13533);
end

if (EVENT == 5405)then
	ITEM1_COUNT = HowmuchItem(UID, 900279000);  
	ITEM2_COUNT = HowmuchItem(UID, 900280000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 761, 22244, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 761, 22244, NPC, 22, 5408, 27, -1);
end
end

if (EVENT == 5408) then
    SaveEvent(UID, 13532);
	SaveEvent(UID, 13543);
    RunQuestExchange(UID, 3213)
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5509,4005,-1);
end

if (EVENT == 5501)then
	SelectMsg(UID, 4, 762, 22246, NPC, 3000,5502,4005,-1);
end

if (EVENT == 5502)then
	SaveEvent(UID, 13543);
end

if (EVENT == 5506)then
	SaveEvent(UID, 13545);
end

if (EVENT == 5509)then
	MonsterStoneQuestJoin(UID,762);
end

if (EVENT == 5505)then
	QuestStatusCheck = GetQuestStatus(UID, 762)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5509,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900281000);  
    ITEM2_COUNT = HowmuchItem(UID, 900282000); 
    ITEM3_COUNT = HowmuchItem(UID, 900283000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5509,4005,-1);
	else
	SelectMsg(UID, 4, 762, 22246, NPC,3000,5507,3005,-1);
end
end
end

if (EVENT == 5507) then
    SaveEvent(UID, 13544);
	SaveEvent(UID, 13555);
    RunQuestExchange(UID, 3214)
	--SelectMsg(UID, 2, -1, 22602, NPC, 10, 4209,4005,-1);
end

if (EVENT == 5601)then
	SelectMsg(UID, 4, 763, 22248, NPC, 3000,5602,4005,-1);
end

if (EVENT == 5602)then
	SaveEvent(UID, 13555);
end

if (EVENT == 5606)then
	SaveEvent(UID, 13557);
end

if (EVENT == 5605) then
	MonsterCount = CountMonsterQuestSub(UID, 763, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 763, 22248, NPC, 18, 5607);
	else
		SelectMsg(UID, 4, 763, 22248, NPC, 3000, 5608, 27, -1);
	end
end

if (EVENT == 5607) then
	ShowMap(UID, 905);
end

if (EVENT == 5608) then
    SaveEvent(UID, 13556);
	SaveEvent(UID, 13567);
    RunQuestExchange(UID, 3215)
	SelectMsg(UID, 2, -1, 22774, NPC, 10,-1);
end

if (EVENT == 5701)then
	SelectMsg(UID, 4, 765, 22250, NPC, 3000,5702,4005,-1);
end

if (EVENT == 5702)then
	SaveEvent(UID, 13567);
end

if (EVENT == 5706)then
	SaveEvent(UID, 13569);
end

if (EVENT == 5705) then
	MonsterCount01 = CountMonsterQuestSub(UID, 765, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 765, 2); 
	if (MonsterCount01 > 0 and MonsterCount02 > 49) then 
		SelectMsg(UID, 4, 765, 22250, NPC,3000,5709,3005,-1);
	else
		if (MonsterCount01 < 1) then
			SelectMsg(UID, 2, 765, 22250, NPC, 18, 5707);
		elseif ( MonsterCount02 < 50) then
			SelectMsg(UID, 2, 765, 22250, NPC, 18, 5708);
		end
	end
end

if (EVENT == 5707) then
	ShowMap(UID, 919);
end
if (EVENT == 5708) then
	ShowMap(UID, 921);
end

if (EVENT == 5709) then
    SaveEvent(UID, 13568);
	SaveEvent(UID, 13579);
    RunQuestExchange(UID, 3216)
	SelectMsg(UID, 2, -1, 22615, NPC, 10,-1);
end

if (EVENT == 5801)then
	SelectMsg(UID, 4, 767, 22252, NPC, 3000,5802,4005,-1);
end

if (EVENT == 5802)then
	SaveEvent(UID, 13579);
end

if (EVENT == 5806)then
	SaveEvent(UID, 13581);
end

if (EVENT == 5805)then
	ITEM1_COUNT = HowmuchItem(UID, 900285000);  
	ITEM2_COUNT = HowmuchItem(UID, 900286000); 
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1) then
	SelectMsg(UID, 2, 767, 22252, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 767, 22252, NPC, 22, 5808, 27, -1);
end
end

if (EVENT == 5808) then
    SaveEvent(UID, 13580);
	SaveEvent(UID, 13591);
    RunQuestExchange(UID, 3217)
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5909,4005,-1);
end

if (EVENT == 5901)then
	SelectMsg(UID, 4, 768, 22254, NPC, 3000,5902,4005,-1);
end

if (EVENT == 5902)then
	SaveEvent(UID, 13591);
end

if (EVENT == 5906)then
	SaveEvent(UID, 13593);
end

if (EVENT == 5909)then
	MonsterStoneQuestJoin(UID,768);
end

if (EVENT == 5905)then
	QuestStatusCheck = GetQuestStatus(UID, 768)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5909,4005,-1);
	else
	ITEM1_COUNT = HowmuchItem(UID, 900287000);  
    ITEM2_COUNT = HowmuchItem(UID, 900288000); 
    ITEM3_COUNT = HowmuchItem(UID, 900289000); 	
	if (ITEM1_COUNT < 1 and ITEM2_COUNT < 1 and ITEM3_COUNT < 1) then
	SelectMsg(UID, 2, -1, 22602, NPC, 10, 5909,4005,-1);
	else
	SelectMsg(UID, 4, 768, 22254, NPC,3000,5907,3005,-1);
end
end
end

if (EVENT == 5907) then
    SaveEvent(UID, 13592);
	SaveEvent(UID, 13603);
    RunQuestExchange(UID, 3218)
	--SelectMsg(UID, 2, -1, 22602, NPC, 10, 4209,4005,-1);
end

if (EVENT == 6001)then
	SelectMsg(UID, 4, 769, 22256, NPC, 3000,6002,4005,-1);
end

if (EVENT == 6002)then
	SaveEvent(UID, 13603);
end

if (EVENT == 6006)then
	SaveEvent(UID, 13605);
end

if (EVENT == 6005) then
	MonsterCount = CountMonsterQuestSub(UID, 769, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 769, 22256, NPC, 18, 6007);
	else
		SelectMsg(UID, 4, 769, 22256, NPC, 3000, 6008, 27, -1);
	end
end

if (EVENT == 6007) then
	ShowMap(UID, 905);
end

if (EVENT == 6008) then
    SaveEvent(UID, 13604);
	SaveEvent(UID, 13615);
    RunQuestExchange(UID, 3219)
	SelectMsg(UID, 2, -1, 22258, NPC, 10,-1);
end

if (EVENT == 6301)then
	SelectMsg(UID, 4, 773, 22262, NPC, 3000,6302,4005,-1);
end

if (EVENT == 6302)then
	SaveEvent(UID, 13639);
end

if (EVENT == 6306)then
	SaveEvent(UID, 13641);
end

if (EVENT == 6305)then
	ITEM1_COUNT = HowmuchItem(UID, 900294000);  
	if (ITEM1_COUNT < 1) then
	SelectMsg(UID, 2, 773, 22262, NPC, 10, -1);
	else
	SelectMsg(UID, 4, 773, 22262, NPC, 22, 6307, 27, -1);
end
end

if (EVENT == 6307) then
    SaveEvent(UID, 13640);
	SaveEvent(UID, 13651);
    RunQuestExchange(UID, 3222)
end

if (EVENT == 6401)then
	SelectMsg(UID, 2, 774, 22264, NPC, 3000,6402,4005,-1);
end

if (EVENT == 6402)then
	SaveEvent(UID, 13651);
end

if (EVENT == 6403)then
    SelectMsg(UID, 4, 774, 22264, NPC, 3000,6404,4005,-1);
	SaveEvent(UID, 13653);
end

if (EVENT == 6404)then
    SaveEvent(UID, 13663);
	SaveEvent(UID, 13652);
	RunQuestExchange(UID, 3223)
end

if (EVENT == 6501)then
	SelectMsg(UID, 4, 775, 22266, NPC, 3000,6502,4005,-1);
end

if (EVENT == 6502)then
	SaveEvent(UID, 13663);
end

if (EVENT == 6506)then
	SaveEvent(UID, 13665);
end

if (EVENT == 6505) then
	MonsterCount = CountMonsterQuestSub(UID, 775, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 775, 22266, NPC, 18, 6507);
	else
		SelectMsg(UID, 5, 775, 22266, NPC, 3000, 6508, 27, -1);
	end
end

if (EVENT == 6507) then
	ShowMap(UID, 855);
end

if (EVENT == 6508)then
	ShowMap(UID, 792);
    SaveEvent(UID, 13664);
	SaveEvent(UID, 13675);
	RunQuestExchange(UID, 3224,STEP,1);
end