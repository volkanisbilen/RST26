-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25019;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then 
		SelectMsg(UID, 2, -1, 43943, NPC, 10, 0);
	elseif (QuestNum > 1 and  QuestNum < 100) then 
		NpcMsg(UID, 43943, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1112) then 
	SelectMsg(UID, 2, 1336, 43944, NPC, 40307, 1113);
end

if (EVENT == 1113) then 
	SelectMsg(UID, 2, 1336, 43945, NPC, 40308, 1114);
end

if (EVENT == 1114) then 
	SelectMsg(UID, 2, 1336, 43946, NPC, 40309, 1115);
end

if (EVENT == 1115) then 
	SelectMsg(UID, 4, 1336, 43947, NPC, 65, 1116, 27,-1);
end

if (EVENT == 1116) then
    SaveEvent(UID, 3726);
	SaveEvent(UID, 3728);
	SaveEvent(UID, 3727);
	RunQuestExchange(UID,6130) ;
end

if (EVENT == 1122) then 
	SelectMsg(UID, 4, 1337, 43948, NPC, 22, 1123, 23, -1);
end

if (EVENT == 1123) then
	QuestStatus = GetQuestStatus(UID, 1337)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3732);
	end
end

if (EVENT == 1127) then
	QuestStatus = GetQuestStatus(UID, 1337)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MARCILTOKEN = HowmuchItem(UID, 900683000);
		if (MARCILTOKEN < 10) then
			SelectMsg(UID, 2, 1337, 43948, NPC, 19, 1126);
		else
			SaveEvent(UID, 3734);
		end
	end
end

if (EVENT == 1125) then
	QuestStatus = GetQuestStatus(UID, 1337)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MARCILTOKEN = HowmuchItem(UID, 900683000);
		if (MARCILTOKEN < 10) then
			SelectMsg(UID, 2, 1337, 43948, NPC, 19, 1126);
		else
			SelectMsg(UID, 4, 1337, 43948, NPC, 22, 1128, 23, -1);
		end
	end
end

if (EVENT == 1128) then
	QuestStatus = GetQuestStatus(UID, 1337)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MARCILTOKEN = HowmuchItem(UID, 900683000);
		if (MARCILTOKEN < 10) then
			SelectMsg(UID, 2, 1337, 43948, NPC, 19, 1126);
		else
			RunQuestExchange(UID,6131);
			SaveEvent(UID, 3733);
		end
	end
end

if (EVENT == 1132) then 
	SelectMsg(UID, 4, 1338, 44126, NPC, 22, 1133, 23, -1);
end

if (EVENT == 1133) then
	QuestStatus = GetQuestStatus(UID, 1338)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3738);
	end
end

if (EVENT == 1137) then
	QuestStatus = GetQuestStatus(UID, 1338)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1338, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1338, 44126, NPC, 18, 1134);
		else
			SaveEvent(UID, 3740);
		end
	end
end

if (EVENT == 1135) then
	QuestStatus = GetQuestStatus(UID, 1338)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1338, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1338, 44126, NPC, 18, 1134);
		else
			SelectMsg(UID, 4, 1338, 44126, NPC, 22, 1136, 23, -1);
		end
	end
end

if (EVENT == 1134) then
	ShowMap(UID, 1325);
end

if (EVENT == 1136) then
	QuestStatus = GetQuestStatus(UID, 1338)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1338, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1338, 44126, NPC, 18, 1134);
		else
			RunQuestExchange(UID,6132);
			SaveEvent(UID, 3739);
		end
	end
end

if (EVENT == 1142) then 
	SelectMsg(UID, 4, 1339, 44127, NPC, 22, 1143, 23, 0);
end

if (EVENT == 1143) then
	QuestStatus = GetQuestStatus(UID, 1339)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3744);
	end
end

if (EVENT == 1147) then
	QuestStatus = GetQuestStatus(UID, 1339)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BADINTOKEN = HowmuchItem(UID, 900684000);
		if (BADINTOKEN < 10) then
			SelectMsg(UID, 2, 1339, 44127, NPC, 19, 1146);
		else
			SaveEvent(UID, 3746);
		end
	end
end

if (EVENT == 1145) then
	QuestStatus = GetQuestStatus(UID, 1339)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BADINTOKEN = HowmuchItem(UID, 900684000);
		if (BADINTOKEN < 10) then
			SelectMsg(UID, 2, 1339, 44127, NPC, 19, 1146);
		else
			SelectMsg(UID, 4, 1339, 44127, NPC, 22, 1148, 23, -1);
		end
	end
end

if (EVENT == 1148) then
	QuestStatus = GetQuestStatus(UID, 1339)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BADINTOKEN = HowmuchItem(UID, 900684000);
		if (BADINTOKEN < 10) then
			SelectMsg(UID, 2, 1339, 44127, NPC, 19, 1146);
		else
			RunQuestExchange(UID,6133) ;
			SaveEvent(UID, 3745);
		end
	end
end

if (EVENT == 1152) then 
	SelectMsg(UID, 4, 1340, 44128, NPC, 22, 1153, 23, -1);
end

if (EVENT == 1153) then
	QuestStatus = GetQuestStatus(UID, 1340)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3750);
	end
end

if (EVENT == 1157) then
	QuestStatus = GetQuestStatus(UID, 1340)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1340, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1340, 44128, NPC, 18, 1156);
		else
			SaveEvent(UID, 3752);
		end
	end
end

if (EVENT == 1155) then
	QuestStatus = GetQuestStatus(UID, 1340)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1340, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1340, 44128, NPC, 18, 1156);
		else
			SelectMsg(UID, 4, 1340, 44128, NPC, 22, 1158, 23, -1);
		end
	end
end

if (EVENT == 1158) then
	QuestStatus = GetQuestStatus(UID, 1340)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1340, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1340, 44128, NPC, 18, 1156);
		else
			RunQuestExchange(UID,6134);
			SaveEvent(UID, 3751);
		end
	end
end

if (EVENT == 1162) then 
	SelectMsg(UID, 4, 1341, 44129, NPC, 22, 1163, 23, -1);
end

if (EVENT == 1163) then
	QuestStatus = GetQuestStatus(UID, 1341)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3756);
	end
end

if (EVENT == 1167) then
	QuestStatus = GetQuestStatus(UID, 1341)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	GARLONGTOKEN = HowmuchItem(UID, 900685000);
		if (GARLONGTOKEN < 10) then
			SelectMsg(UID, 2, 1341, 44129, NPC, 19, 1166);
		else
			SaveEvent(UID, 3758);
		end
	end
end

if (EVENT == 1165) then
	QuestStatus = GetQuestStatus(UID, 1341)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	GARLONGTOKEN = HowmuchItem(UID, 900685000);
		if (GARLONGTOKEN < 10) then
			SelectMsg(UID, 2, 1341, 44129, NPC, 19, 1166);
		else
			SelectMsg(UID, 4, 1341, 44129, NPC, 22, 1168, 23, -1);
		end
	end
end

if (EVENT == 1168) then
	QuestStatus = GetQuestStatus(UID, 1341)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	GARLONGTOKEN = HowmuchItem(UID, 900685000);
		if (GARLONGTOKEN < 10) then
			SelectMsg(UID, 2, 1341, 44129, NPC, 19, 1166);
		else
			RunQuestExchange(UID,6135);
			SaveEvent(UID, 3757);
		end
	end
end

if (EVENT == 1172) then 
	SelectMsg(UID, 4, 1342, 44130, NPC, 22, 1173, 23, -1);
end

if (EVENT == 1173) then
	QuestStatus = GetQuestStatus(UID, 1342)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3762);
	end
end

if (EVENT == 1177) then
	QuestStatus = GetQuestStatus(UID, 1342)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1342, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1342, 44130, NPC, 18, 1176);
		else
			SaveEvent(UID, 3764);
		end
	end
end

if (EVENT == 1175) then
	QuestStatus = GetQuestStatus(UID, 1342)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1342, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1342, 44130, NPC, 18, 1176);
		else
			SelectMsg(UID, 4, 1342, 44130, NPC, 22, 1178, 23, -1);
		end
	end
end

if (EVENT == 1178) then
	QuestStatus = GetQuestStatus(UID, 1342)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1342, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1342, 44130, NPC, 18, 1176);
		else
			RunQuestExchange(UID,6136);
			SaveEvent(UID, 3763);
		end
	end
end

if (EVENT == 1182) then 
	SelectMsg(UID, 4, 1343, 44131, NPC, 22, 1183, 23, -1);
end

if (EVENT == 1183) then
	QuestStatus = GetQuestStatus(UID, 1343)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3768);
	end
end

if (EVENT == 1187) then
	QuestStatus = GetQuestStatus(UID, 1343)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Monster01 = CountMonsterQuestSub(UID, 1343, 1);
	Monster02 = CountMonsterQuestSub(UID, 1343, 2);
	Monster03 = CountMonsterQuestSub(UID, 1343, 3);
	    if (Monster01 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1188);
		elseif ( Monster02 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1189);
		elseif ( Monster03 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1190);
		else
			SaveEvent(UID, 3770);
		end
	end
end

if (EVENT == 1185) then
	QuestStatus = GetQuestStatus(UID, 1343)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Monster01 = CountMonsterQuestSub(UID, 1343, 1);
	Monster02 = CountMonsterQuestSub(UID, 1343, 2);
	Monster03 = CountMonsterQuestSub(UID, 1343, 3);
	    if (Monster01 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1188);
		elseif ( Monster02 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1189);
		elseif ( Monster03 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1190);
		else
			SelectMsg(UID, 4, 1343, 44131, NPC, 22,1186,27, -1);
		end
	end
end

if (EVENT == 1188) then
	ShowMap(UID, 736);
end

if (EVENT == 1189) then
	ShowMap(UID, 894);
end

if (EVENT == 1190) then
	ShowMap(UID, 738);
end

if (EVENT == 1186) then
	QuestStatus = GetQuestStatus(UID, 1343)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Monster01 = CountMonsterQuestSub(UID, 1343, 1);
	Monster02 = CountMonsterQuestSub(UID, 1343, 2);
	Monster03 = CountMonsterQuestSub(UID, 1343, 3);
	    if (Monster01 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1188);
		elseif ( Monster02 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1189);
		elseif ( Monster03 < 1) then
			SelectMsg(UID, 2, 1343, 44131, NPC, 18, 1190);
		else
			RunQuestExchange(UID,6137);
			SaveEvent(UID, 3769);
		end
	end
end

if (EVENT == 1192) then 
	SelectMsg(UID, 4, 1344, 44132, NPC, 22, 1193, 23, -1);
end

if (EVENT == 1193) then
	QuestStatus = GetQuestStatus(UID, 1344)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3774);
	end
end

if (EVENT == 1197) then
	QuestStatus = GetQuestStatus(UID, 1344)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ESC = HowmuchItem(UID, 900634000);
	JER = HowmuchItem(UID, 900635000);
	ESP = HowmuchItem(UID, 900633000);
		if (ESC < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1199);
		elseif ( JER < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1200);
		elseif ( ESP < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1201);
		else
			SaveEvent(UID, 3776);
		end
	end
end

if (EVENT == 1195) then
	QuestStatus = GetQuestStatus(UID, 1344)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ESC = HowmuchItem(UID, 900634000);
	JER = HowmuchItem(UID, 900635000);
	ESP = HowmuchItem(UID, 900633000);
		if (ESC < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1199);
		elseif ( JER < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1200);
		elseif ( ESP < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1201);
		else
			SelectMsg(UID, 4, 1344, 44132, NPC, 22, 1198, 23, -1);
		end
	end
end

if (EVENT == 1198) then
	QuestStatus = GetQuestStatus(UID, 1344)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	ESC = HowmuchItem(UID, 900634000);
	JER = HowmuchItem(UID, 900635000);
	ESP = HowmuchItem(UID, 900633000);
		if (ESC < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1199);
		elseif ( JER < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1200);
		elseif ( ESP < 1) then
			SelectMsg(UID, 2, 1344, 44132, NPC, 18, 1201);
		else
			RunQuestExchange(UID,6138);
			SaveEvent(UID, 3775);
		end
	end
end

if (EVENT == 1202) then 
	SelectMsg(UID, 4, 1345, 44133, NPC, 22, 1203, 23, -1);
end

if (EVENT == 1203) then
	QuestStatus = GetQuestStatus(UID, 1345)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3780);
	end
end

if (EVENT == 1207) then
	QuestStatus = GetQuestStatus(UID, 1345)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1345, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1345, 44133, NPC, 18, 1206);
		else
			SaveEvent(UID, 3782);
		end
	end
end

if (EVENT == 1205) then
	QuestStatus = GetQuestStatus(UID, 1345)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1345, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1345, 44133, NPC, 18, 1206);
		else
			SelectMsg(UID, 4, 1345, 44133, NPC, 22, 1208, 23, -1);
		end
	end
end

if (EVENT == 1206) then
	ShowMap(UID, 1328);
end

if (EVENT == 1208) then
	QuestStatus = GetQuestStatus(UID, 1345)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1345, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1345, 44133, NPC, 18, 1206);
		else
			RunQuestExchange(UID,6139);
			SaveEvent(UID, 3781);
		end
	end
end

if (EVENT == 1212) then 
	SelectMsg(UID, 4, 1346, 44134, NPC, 22, 1213, 23, -1);
end

if (EVENT == 1213) then
SaveEvent(UID, 3786);
SaveEvent(UID, 3788);
SaveEvent(UID, 3787);
RunQuestExchange(UID,6140);
end