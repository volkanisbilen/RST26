-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24439;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 973, NPC, 10, -1);
	elseif (QuestNum > 1 and QuestNum < 100) then
		NpcMsg(UID, 973, NPC);
	else
		EVENT = QuestNum
	end
end

if (EVENT == 111) then
	SelectMsg(UID, 4, 274, 120, NPC, 22, 112, 23, -1);
end

if (EVENT == 112) then
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 19);
	end
end

if (EVENT == 115) then
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 274, 120, NPC, 21, 117);
		else
			SaveEvent(UID, 24);
		end
	end
end

if (EVENT == 116) then
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 274, 120, NPC, 21, 117);
		else
			SelectMsg(UID, 4, 274, 120, NPC, 41, 118, 23, -1);
		end
	end
end

if (EVENT == 117) then
	ShowMap(UID, 78);
end

if (EVENT == 118) then
	QuestStatus = GetQuestStatus(UID, 274)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 274, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 274, 120, NPC, 21, 117);
		else
			RunQuestExchange(UID, 3);
			SaveEvent(UID, 23);
		end
	end
end

if (EVENT == 120) then
	SelectMsg(UID, 4, 275, 142, NPC, 22, 121, 23, -1);
end

if (EVENT == 121) then
	QuestStatus = GetQuestStatus(UID, 275)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 33);
	end
end

if (EVENT == 124) then
	QuestStatus = GetQuestStatus(UID, 275)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 275, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 275, 142, NPC, 21, 126);
		else
			SaveEvent(UID, 35);
		end
	end
end

if (EVENT == 125) then
	QuestStatus = GetQuestStatus(UID, 275)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 275, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 275, 142, NPC, 21, 126);
		else
			SelectMsg(UID, 4, 275, 142, NPC, 41, 127, 23, -1);
		end
	end
end

if (EVENT == 126) then
	ShowMap(UID, 607);
end

if (EVENT == 127) then
	QuestStatus = GetQuestStatus(UID, 275)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 275, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 275, 142, NPC, 21, 126);
		else
			RunQuestExchange(UID, 4);
			SaveEvent(UID, 34);
		end
	end
end

if (EVENT == 130) then
	SelectMsg(UID, 4, 276, 152, NPC, 22, 131, 23, -1);
end

if (EVENT == 131) then
	QuestStatus = GetQuestStatus(UID, 276)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 43);
	end
end

if (EVENT == 134) then
	QuestStatus = GetQuestStatus(UID, 276)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 276, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 276, 152, NPC, 21, 136);
		else
			SaveEvent(UID, 69);
		end
	end
end

if (EVENT == 135) then
	QuestStatus = GetQuestStatus(UID, 276)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 276, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 276, 152, NPC, 21, 136);
		else
			SelectMsg(UID, 4, 276, 152, NPC, 41, 137, 23, -1);
		end
	end
end

if (EVENT == 136) then
	ShowMap(UID, 84);
end

if (EVENT == 137) then
	QuestStatus = GetQuestStatus(UID, 276)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 276, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 276, 152, NPC, 21, 136);
		else
			RunQuestExchange(UID, 9);
			SaveEvent(UID, 44);
		end
	end
end

if (EVENT == 140) then
	SelectMsg(UID, 4, 277, 171, NPC, 22, 141, 23, -1);
end

if (EVENT == 141) then
	QuestStatus = GetQuestStatus(UID, 277)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 84);
	end
end

if (EVENT == 144) then
	QuestStatus = GetQuestStatus(UID, 277)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 277, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 277, 171, NPC, 21, 146);
		else
			SaveEvent(UID, 86);
		end
	end
end

if (EVENT == 145) then
	QuestStatus = GetQuestStatus(UID, 277)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 277, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 277, 171, NPC, 21, 146);
		else
			SelectMsg(UID, 4, 277, 171, NPC, 41, 147, 23, -1);
		end
	end
end

if (EVENT == 146) then
	ShowMap(UID, 616);
end

if (EVENT == 147) then
	QuestStatus = GetQuestStatus(UID, 277)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 277, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 277, 171, NPC, 21, 146);
		else
			RunQuestExchange(UID, 10);
			SaveEvent(UID, 85);
		end
	end
end

if (EVENT == 150) then
	SelectMsg(UID, 4, 985, 194, NPC, 22, 151, 23, -1);
end

if (EVENT == 151) then
	QuestStatus = GetQuestStatus(UID, 985)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 108);
	end
end

if (EVENT == 154) then
	QuestStatus = GetQuestStatus(UID, 985)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 985, 1);
		if (MonsterCount < 120) then
			SelectMsg(UID, 2, 985, 142, NPC, 21, 156);
		else
			SaveEvent(UID, 110);
		end
	end
end

if (EVENT == 155) then
	QuestStatus = GetQuestStatus(UID, 985)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 985, 1);
		if (MonsterCount < 120) then
			SelectMsg(UID, 2, 985, 142, NPC, 21, 156);
		else
			SelectMsg(UID, 4, 985, 142, NPC, 41, 157, 23, -1);
		end
	end
end

if (EVENT == 156) then
	ShowMap(UID, 618);
end

if (EVENT == 157) then
	QuestStatus = GetQuestStatus(UID, 985)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 985, 1);
		if (MonsterCount < 120) then
			SelectMsg(UID, 2, 985, 142, NPC, 21, 156);
		else
			ExpChange(UID, 1400000);
			SaveEvent(UID, 109);
		end
	end
end

if (EVENT == 170) then
	SelectMsg(UID, 4, 1015, 971, NPC, 22, 171, 23, -1);
end

if (EVENT == 171) then
	QuestStatus = GetQuestStatus(UID, 1015)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 707);
	end
end

if (EVENT == 174) then
	QuestStatus = GetQuestStatus(UID, 1015)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1015, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 1015, 142, NPC, 21, 176);
		else
			SaveEvent(UID, 709);
		end
	end
end

if (EVENT == 175) then
	QuestStatus = GetQuestStatus(UID, 1015)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1015, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 1015, 142, NPC, 21, 176);
		else
			SelectMsg(UID, 4, 1015, 142, NPC, 41, 177, 23, -1);
		end
	end
end

if (EVENT == 176) then
	ShowMap(UID, 130);
end

if (EVENT == 177) then
	QuestStatus = GetQuestStatus(UID, 1015)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1015, 1);
		if (MonsterCount < 100) then
			SelectMsg(UID, 2, 1015, 142, NPC, 21, 176);
		else
			ExpChange(UID, 1400000);
			SaveEvent(UID, 708);
		end
	end
end

if (EVENT == 160) then
	SelectMsg(UID, 4, 278, 964, NPC, 22, 161, 23, -1);
end

if (EVENT == 161) then
	QuestStatus = GetQuestStatus(UID, 278)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9992);
	end
end

if (EVENT == 164) then
	QuestStatus = GetQuestStatus(UID, 278)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 278, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 278, 964, NPC, 21, 166);
		else
			SaveEvent(UID, 7738);
		end
	end
end

if (EVENT == 165) then
	QuestStatus = GetQuestStatus(UID, 278)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 278, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 278, 964, NPC, 21, 166);
		else
			SelectMsg(UID, 4, 278, 964, NPC, 41, 167, 23, -1);
		end
	end
end

if (EVENT == 166) then
	ShowMap(UID, 128);
end

if (EVENT == 167) then
	QuestStatus = GetQuestStatus(UID, 278)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 278, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 278, 964, NPC, 21, 166);
		else
			RunQuestExchange(UID, 109);
			SaveEvent(UID, 7737);
		end
	end
end

if (EVENT == 1102) then
	SelectMsg(UID, 4, 523, 20015, NPC, 22, 1103, 27, -1);
end

if (EVENT == 1103) then
	SaveEvent(UID, 11116);
end

if (EVENT == 1104) then
	SelectMsg(UID, 4, 523, 20177, NPC, 22, 1105, 27, -1);
	SaveEvent(UID, 11118);
end

if (EVENT == 1105) then
	SaveEvent(UID, 11117);
	SaveEvent(UID, 11128);
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 524, 20017, NPC, 22, 1203, 27, -1);
end

if (EVENT == 1203) then
	QuestStatus = GetQuestStatus(UID, 524)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11128);
	end
end

if (EVENT == 1206) then
	QuestStatus = GetQuestStatus(UID, 524)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 524, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 524, 20017, NPC, 18, 1208);
		else
			SaveEvent(UID, 11130);
		end
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 524)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 524, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 524, 20017, NPC, 18, 1208);
		else
			SelectMsg(UID, 5, 524, 20017, NPC, 41, 1207,23, -1);
		end
	end
end

if (EVENT == 1208) then
	ShowMap(UID, 1181);
end

if (EVENT == 1207)then
	QuestStatus = GetQuestStatus(UID, 524)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 524, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 524, 20017, NPC, 18, 1208);
		else
			RunQuestExchange(UID,3011,STEP,1);
			SaveEvent(UID, 11129);
			SaveEvent(UID, 11140);
		end
	end
end

if (EVENT == 1302) then
	SelectMsg(UID, 4, 529, 20027, NPC, 22, 1303, 27, -1);
end

if (EVENT == 1303) then
	QuestStatus = GetQuestStatus(UID, 529)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11188);
	end
end

if (EVENT == 1308) then
	QuestStatus = GetQuestStatus(UID, 529)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910215000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 529, 20027, NPC, 18,-1);
		else
			SaveEvent(UID, 11190);
		end
	end
end

if (EVENT == 1305) then
	QuestStatus = GetQuestStatus(UID, 529)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910215000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 529, 20027, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 529, 20027, NPC, 22, 1307,27, -1);
		end
	end
end

if (EVENT == 1307)then
	RunQuestExchange(UID,3016);
	SaveEvent(UID,11189);
	SaveEvent(UID,11200);
end

if (EVENT == 1402) then
	SelectMsg(UID, 4, 530, 20029, NPC, 22, 1403, 27, -1);
end

if (EVENT == 1403) then
	QuestStatus = GetQuestStatus(UID, 530)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11200);
	end
end

if (EVENT == 1408) then
	QuestStatus = GetQuestStatus(UID, 530)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 508105000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 530, 20029, NPC, 18,1406);
		else
			SaveEvent(UID, 11202);
		end
	end
end

if (EVENT == 1405) then
	QuestStatus = GetQuestStatus(UID, 530)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 508105000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 530, 20029, NPC, 18,1406);
		else
			SelectMsg(UID, 4, 530, 20029, NPC, 22, 1407,27, -1);
		end
	end
end

if (EVENT == 1406) then
	ShowMap(UID, 1177);
end

if (EVENT == 1407)then
	QuestStatus = GetQuestStatus(UID, 530)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 508105000);   
		if (ITEM_COUNT < 5) then
			SelectMsg(UID, 2, 530, 20029, NPC, 18,1406);
		else
			SelectMsg(UID, 2, 530, 20241, NPC, 10,-1);
			RunQuestExchange(UID,3017);
			SaveEvent(UID,11201);
			SaveEvent(UID,11218);
		end
	end
end

if (EVENT == 1502) then
	SelectMsg(UID, 4, 533, 20032, NPC, 22, 1503, 27, -1);
end

if (EVENT == 1503) then
	QuestStatus = GetQuestStatus(UID, 533)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11236);
	end
end

if (EVENT == 1508) then
	QuestStatus = GetQuestStatus(UID, 533)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910216000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 533, 20032, NPC, 18,-1);
		else
			SaveEvent(UID, 11238);
		end
	end
end

if (EVENT == 1505) then
	QuestStatus = GetQuestStatus(UID, 533)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910216000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 533, 20032, NPC, 18,-1);
		else
			SelectMsg(UID, 4, 533, 20032, NPC, 22, 1507,27, -1);
		end
	end
end

if (EVENT == 1507)then
	QuestStatus = GetQuestStatus(UID, 533)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ITEM_COUNT = HowmuchItem(UID, 910216000);   
		if (ITEM_COUNT < 1) then
			SelectMsg(UID, 2, 533, 20032, NPC, 18,-1);
		else
			RunQuestExchange(UID,3020);
			SaveEvent(UID,11237);
			SaveEvent(UID,11248);
		end
	end
end

if (EVENT == 1602) then
	SelectMsg(UID, 4, 534, 20037, NPC, 22, 1603, 27, -1);
end

if (EVENT == 1603) then
	QuestStatus = GetQuestStatus(UID, 534)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 11248);
	end
end

if (EVENT == 1606) then
	QuestStatus = GetQuestStatus(UID, 534)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 534, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 534, 20037, NPC, 18, 1607);
		else
			SaveEvent(UID, 11250);
		end
	end
end

if (EVENT == 1605) then
	QuestStatus = GetQuestStatus(UID, 534)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 534, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 534, 20037, NPC, 18, 1607);
		else
			SelectMsg(UID, 5, 534, 20037, NPC, 41, 1608,23, -1);
		end
	end
end

if (EVENT == 1608)then
	QuestStatus = GetQuestStatus(UID, 534)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 534, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 534, 20037, NPC, 18, 1607);
		else
			RunQuestExchange(UID,3021,STEP,1);
			SaveEvent(UID, 11249);
			SaveEvent(UID, 11260);
		end
	end
end

if (EVENT == 1705) then
	CHERICHERO = HowmuchItem(UID, 910229000);
	if (CHERICHERO < 1) then
		SelectMsg(UID, 2, 551, 21624, NPC, 10, -1);
	else
		SelectMsg(UID, 4, 551, 20066, NPC, 10, 1709, 27, -1);
	end
end

if (EVENT == 1709) then
	RELICHERO = HowmuchItem(UID, 910229000);
	if (RELICHERO < 1) then
		SelectMsg(UID, 2, 551, 21624, NPC, 10, -1);
	else
	RunQuestExchange(UID,3041);
	SaveEvent(UID, 11483);
	SaveEvent(UID, 11494);
    end
end