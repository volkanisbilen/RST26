-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25163;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43829, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43829, NPC)
	else
		EVENT = QuestNum
	end
end

if(EVENT == 1112) then 
	SelectMsg(UID, 4, 1312, 43829, NPC, 22, 1113, 23, -1);
end

if(EVENT == 1113) then
	QuestStatus = GetQuestStatus(UID, 1312)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3582);
	end
end

if(EVENT == 1117) then
	QuestStatus = GetQuestStatus(UID, 1312)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1312, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1312, 43829, NPC, 10, 1116);
		else
			SaveEvent(UID, 3584);
		end
	end
end

if (EVENT == 1115) then
	QuestStatus = GetQuestStatus(UID, 1312)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1312, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1312, 43829, NPC, 10, 1116);
		else
			SelectMsg(UID, 4, 1312, 43829, NPC, 10, 1118, 27, -1);
		end
	end
end

if(EVENT == 1116) then
	ShowMap(UID, 1337);
end

if(EVENT == 1118) then
	QuestStatus = GetQuestStatus(UID, 1312)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1312, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1312, 43829, NPC, 10, 1116);
		else
			SaveEvent(UID, 3583);
			RunQuestExchange(UID,6105);
		end
	end
end

if(EVENT == 1122) then 
	SelectMsg(UID, 4, 1313, 43826, NPC, 10, 1123, 23, -1);
end

if(EVENT == 1123) then
	QuestStatus = GetQuestStatus(UID, 1313)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3588);
	end
end

if(EVENT == 1127) then
	QuestStatus = GetQuestStatus(UID, 1313)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1313, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1313, 43826, NPC, 10, 1126);
		else
			SaveEvent(UID, 3590);
		end
	end
end
	
if (EVENT == 1125) then
	QuestStatus = GetQuestStatus(UID, 1313)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1313, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1313, 43826, NPC, 10, 1126);
		else
			SelectMsg(UID, 4, 1313, 43826, NPC, 10, 1128, 27, -1);
		end
	end
end

if(EVENT == 1126) then
	ShowMap(UID, 113);
end

if(EVENT == 1128) then
	QuestStatus = GetQuestStatus(UID, 1313)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1313, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1313, 43826, NPC, 10, 1126);
		else
			SaveEvent(UID, 3589);
			RunQuestExchange(UID,6106);
		end
	end
end

if (EVENT == 1132) then
	SelectMsg(UID, 4, 1314, 43833, NPC, 10, 1133, 23, -1);
end

if(EVENT == 1133) then
	QuestStatus = GetQuestStatus(UID, 1314)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3594);
	end
end

if(EVENT == 1137) then
	QuestStatus = GetQuestStatus(UID, 1314)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	KINGAIF = HowmuchItem(UID, 900653000)
		if( KINGAIF < 1) then
			SelectMsg(UID, 2, 1314, 43833, NPC, 18, 1136);
		else
			SaveEvent(UID, 3596);
		end
	end
end
	
if(EVENT == 1135) then
	QuestStatus = GetQuestStatus(UID, 1314)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	KINGAIF = HowmuchItem(UID, 900653000)
		if( KINGAIF < 1) then
			SelectMsg(UID, 2, 1314, 43833, NPC, 18, 1136);
		else
			SelectMsg(UID, 4, 1314, 43833, NPC, 10, 1138, 27, -1);
		end
	end
end

if(EVENT == 1136) then
	ShowMap(UID, 1337);
end

if(EVENT == 1138) then
	QuestStatus = GetQuestStatus(UID, 1314)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	KINGAIF = HowmuchItem(UID, 900653000)
		if( KINGAIF < 1) then
			SelectMsg(UID, 2, 1314, 43833, NPC, 18, 1136);
		else
			SaveEvent(UID, 3595);
			RunQuestExchange(UID,6107);
		end
	end
end

if (EVENT == 1152) then
	SelectMsg(UID, 4, 1315, 43835, NPC, 10, 1153, 23, -1);
end

if(EVENT == 1153) then
	QuestStatus = GetQuestStatus(UID, 1315)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3600);
	end
end

if(EVENT == 1157) then
	QuestStatus = GetQuestStatus(UID, 1315)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BRACE = HowmuchItem(UID, 900654000)
	EARRIN = HowmuchItem(UID, 900655000)
		if (BRACE < 1) then
			SelectMsg(UID, 2, 1315, 43833, NPC, 18, 1156);
		elseif (EARRIN < 1) then
			SelectMsg(UID, 2, 1315, 43833, NPC, 18, 1156);
		else
			SaveEvent(UID, 3602);
		end
	end
end

if(EVENT == 1155) then
	QuestStatus = GetQuestStatus(UID, 1315)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BRACE = HowmuchItem(UID, 900654000)
	EARRIN = HowmuchItem(UID, 900655000)
		if (BRACE < 1) then
			SelectMsg(UID, 2, 1315, 43833, NPC, 18, 1156);
		elseif (EARRIN < 1) then
			SelectMsg(UID, 2, 1315, 43833, NPC, 18, 1156);
		else
			SelectMsg(UID, 4, 1315, 43833, NPC, 10, 1158, 27, -1);
		end
	end
end

if(EVENT == 1156) then
	ShowMap(UID, 1336);
end		

if(EVENT == 1158) then
	QuestStatus = GetQuestStatus(UID, 1315)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	BRACE = HowmuchItem(UID, 900654000)
	EARRIN = HowmuchItem(UID, 900655000)
		if (BRACE < 1) then
			SelectMsg(UID, 2, 1315, 43833, NPC, 18, 1156);
		elseif (EARRIN < 1) then
			SelectMsg(UID, 2, 1315, 43833, NPC, 18, 1156);
		else
			SaveEvent(UID, 3601);
			RunQuestExchange(UID,6109);
		end
	end
end

if (EVENT == 1162) then
	SelectMsg(UID, 4, 1316, 43839, NPC, 10, 1163, 23, -1);
end

if(EVENT == 1163) then
	QuestStatus = GetQuestStatus(UID, 1316)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3606);
	end
end

if(EVENT == 1167) then
	QuestStatus = GetQuestStatus(UID, 1316)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1316, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1316, 43839, NPC, 10, 1166);
		else
			SaveEvent(UID, 3608);
		end
	end
end

if (EVENT == 1165) then
	QuestStatus = GetQuestStatus(UID, 1316)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1316, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1316, 43839, NPC, 10, 1166);
		else
			SelectMsg(UID, 4, 1316, 43839, NPC, 10, 1168, 27, -1);
		end
	end
end

if(EVENT == 1166) then
	ShowMap(UID, 1336);
end

if(EVENT == 1168) then
	QuestStatus = GetQuestStatus(UID, 1316)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1316, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1316, 43839, NPC, 10, 1166);
		else
			SaveEvent(UID, 3607);
			RunQuestExchange(UID,6110);
		end
	end
end

if (EVENT == 1172) then
	SelectMsg(UID, 4, 1317, 43842, NPC, 10, 1173, 23, -1);
end

if(EVENT == 1173) then
	QuestStatus = GetQuestStatus(UID, 1317)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3612);
	end
end

if(EVENT == 1177) then
	QuestStatus = GetQuestStatus(UID, 1317)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1317, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1317, 43842, NPC, 10, 1176);
		else
			SaveEvent(UID, 3614);
		end
	end
end

if (EVENT == 1175) then
	QuestStatus = GetQuestStatus(UID, 1317)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1317, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1317, 43842, NPC, 10, 1176);
		else
			SelectMsg(UID, 4, 1317, 43842, NPC, 10, 1178, 27, -1);
		end
	end
end

if(EVENT == 1176) then
ShowMap(UID, 1319);
end

if(EVENT == 1178) then
	QuestStatus = GetQuestStatus(UID, 1317)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1317, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1317, 43842, NPC, 10, 1176);
		else
			SaveEvent(UID, 3613);
			RunQuestExchange(UID,6111);
		end
	end
end

if (EVENT == 1182) then
	SelectMsg(UID, 4, 1318, 43845, NPC, 10, 1183, 23, -1);
end

if(EVENT == 1183) then
	QuestStatus = GetQuestStatus(UID, 1318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3618);
	end
end

if(EVENT == 1187) then
	QuestStatus = GetQuestStatus(UID, 1318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1318, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1318, 43845, NPC, 10, 1186);
		else
			SaveEvent(UID, 3620);
		end
	end
end

if (EVENT == 1185) then
	QuestStatus = GetQuestStatus(UID, 1318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1318, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1318, 43845, NPC, 10, 1186);
		else
			SelectMsg(UID, 4, 1318, 43845, NPC, 10, 1188, 27, -1);
		end
	end
end

if(EVENT == 1186) then
ShowMap(UID, 1338);
end

if(EVENT == 1188) then
	QuestStatus = GetQuestStatus(UID, 1318)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1318, 1);
		if (MonsterCount < 1) then
			SelectMsg(UID, 2, 1318, 43845, NPC, 10, 1186);
		else
			SaveEvent(UID, 3619);
			RunQuestExchange(UID,6112);
		end
	end
end

if (EVENT == 1192) then
	SelectMsg(UID, 4, 1319, 43848, NPC, 10, 1193, 23, -1);
end

if(EVENT == 1193) then
	QuestStatus = GetQuestStatus(UID, 1319)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 3624);
	end
end

if(EVENT == 1197) then
	QuestStatus = GetQuestStatus(UID, 1319)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1319, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1319, 43848, NPC, 10, 1196);
		else
			SaveEvent(UID, 3626);
		end
	end
end

if (EVENT == 1195) then
	QuestStatus = GetQuestStatus(UID, 1319)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1319, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1319, 43848, NPC, 10, 1196);
		else
			SelectMsg(UID, 4, 1319, 43848, NPC, 10, 1198, 27, -1);
		end
	end
end

if(EVENT == 1196) then
ShowMap(UID, 547);
end

if(EVENT == 1198) then
	QuestStatus = GetQuestStatus(UID, 1319)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 1319, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 1319, 43848, NPC, 10, 1196);
		else
			SaveEvent(UID, 3625);
			RunQuestExchange(UID,6113);
		end
	end
end

if (EVENT == 1202) then
	SelectMsg(UID, 4, 1320, 43851, NPC, 10, 1203, 23, -1);
end

if(EVENT == 1203) then
	SaveEvent(UID, 3630);
	SaveEvent(UID, 3632);
end

if(EVENT == 1205) then
	SaveEvent(UID, 3631);
    RunQuestExchange(UID,6114);
end