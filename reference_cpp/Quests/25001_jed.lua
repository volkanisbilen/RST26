-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 25001;

if (EVENT == 100) then
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then
		SelectMsg(UID, 2, -1, 43651, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 43651, NPC)
	else
		EVENT = QuestNum
	end
end

if(EVENT == 110) then
	SelectMsg(UID, 2, 1210, 43650, NPC, 40151, 111,3019,-1);
end

if (EVENT == 111) then
	REQUEST = HowmuchItem(UID, 900599000);
	if (REQUEST < 1) then
		SelectMsg(UID, 2, 1210, 43650, NPC, 19, 113);
	else
		SelectMsg(UID, 4, 1210, 43650, NPC, 65, 116, 27, -1);
	end
end

if(EVENT == 114) then
	SaveEvent(UID, 7370);
end

if(EVENT == 113) then
	ShowMap(UID, 1187);
end

if(EVENT == 116) then
	RunQuestExchange(UID, 6006);
	SaveEvent(UID, 7369)
end

if(EVENT == 122) then
	SelectMsg(UID, 2, 1211, 43654, NPC, 40152, 123);
end

if(EVENT == 123) then
	SelectMsg(UID, 4, 1211, 43668, NPC, 22, 124,23,-1);
end

if(EVENT == 124) then
	SaveEvent(UID, 7374)
end

if(EVENT == 127) then
	SaveEvent(UID, 7376)
end

if(EVENT == 125) then
	MonsterCount  = CountMonsterQuestSub(UID, 1211, 1);
	if (MonsterCount < 2) then
		SelectMsg(UID, 2, 1211, 8267, NPC, 18, 129);
	else
		SelectMsg(UID, 4, 1211, 8268, NPC, 41, 130, 27, 158);
	end
end

if(EVENT == 129) then
	ShowMap(UID, 1198)
end

if(EVENT == 130) then
	RunQuestExchange(UID, 6007);
	SaveEvent(UID, 7375)
end

if(EVENT == 132) then
	SelectMsg(UID, 2, 1214, 43676, NPC, 40157, 133);
end

if(EVENT == 133) then
	SaveEvent(UID, 7392)
end

if(EVENT == 137) then
	SaveEvent(UID, 7394)
end

if (EVENT == 135) then
	CONTTA = HowmuchItem(UID, 900632000);
	CONTTB = HowmuchItem(UID, 900631000);
	if (CONTTA < 1 and CONTTB < 1) then
		SelectMsg(UID, 2, 1214, 43676, NPC, 19, 136);
	else
		SelectMsg(UID, 2, 1214, 43677, NPC, 10, 138);
	end
end

if(EVENT == 136) then
	ShowMap(UID, 1198)
end

if(EVENT == 138) then
	RunQuestExchange(UID, 6008);
	SaveEvent(UID, 7393)
end

if(EVENT == 142) then
	SelectMsg(UID, 2, 1215, 43686, NPC, 8259, 143);
end

if(EVENT == 143) then
	SelectMsg(UID, 2, 1215, 43687, NPC, 40265, 144);
end

if(EVENT == 144) then
	SaveEvent(UID, 7398)
end

if(EVENT == 147) then
	SaveEvent(UID, 7400)
end

if (EVENT == 145) then
	CONTTA = HowmuchItem(UID, 810418000);
	if (CONTTA < 4) then
		SelectMsg(UID, 2, 1215, 43691, NPC, 19, 146);
	else
		SelectMsg(UID, 4, 1215, 43691, NPC, 10, 148,20,-1);
	end
end

if(EVENT == 146) then
	ShowMap(UID, 567)
end

if(EVENT == 148) then
	RunQuestExchange(UID, 6009);
	SaveEvent(UID, 7399)
end

if(EVENT == 152) then
	SelectMsg(UID, 2, 1216, 43693, NPC, 40157, 153);
end

if(EVENT == 153) then
	SaveEvent(UID, 7404)
end

if(EVENT == 157) then
	SaveEvent(UID, 7406)
end

if(EVENT == 155) then
	MonsterCount  = CountMonsterQuestSub(UID, 1216, 1);
	if (MonsterCount < 10) then
		SelectMsg(UID, 2, 1216, 43695, NPC, 18, 156);
	else
		SelectMsg(UID, 4, 1216, 43695, NPC, 41, 158, 27, 158);
	end
end

if(EVENT == 156) then
	ShowMap(UID, 1253)
end

if(EVENT == 158) then
	RunQuestExchange(UID, 6010);
	SaveEvent(UID, 7405)
end


if(EVENT == 162) then
	SelectMsg(UID, 2, 1217, 43696, NPC, 40157, 163);
end

if(EVENT == 163) then
	SaveEvent(UID, 7410)
end

if(EVENT == 167) then
	SaveEvent(UID, 7412)
end

if (EVENT == 165) then
	CONTTA = HowmuchItem(UID, 810418000);
	if (CONTTA < 2) then
		SelectMsg(UID, 2, 1217, 43698, NPC, 19, 166);
	else
		SelectMsg(UID, 4, 1217, 43698, NPC, 10, 168,20,-1);
	end
end

if(EVENT == 166) then
	ShowMap(UID, 1253)
end

if(EVENT == 168) then
	RunQuestExchange(UID, 6011);
	SaveEvent(UID, 7411)
end

if(EVENT == 172) then
	SelectMsg(UID, 2, 1218, 43699, NPC, 40157, 173);
end

if(EVENT == 173) then
	SaveEvent(UID, 7416)
end

if(EVENT == 177) then
	SaveEvent(UID, 7418)
end

if (EVENT == 175) then
	CONTTA = HowmuchItem(UID, 900615000);
	if (CONTTA < 1) then
		SelectMsg(UID, 2, 1218, 43701, NPC, 19, 176);
	else
		SelectMsg(UID, 4, 1218, 43701, NPC, 10, 178,20,-1);
	end
end

if(EVENT == 176) then
	ShowMap(UID, 1256)
end

if(EVENT == 178) then
	RunQuestExchange(UID, 6012);
	SaveEvent(UID, 7417)
end

if(EVENT == 192) then
	SelectMsg(UID, 2, 1220, 43705, NPC, 40174, 193);
end

if(EVENT == 193) then
	SelectMsg(UID, 2, 1220, 43706, NPC, 40157, 194);
end

if(EVENT == 194) then
	SaveEvent(UID, 7428)
end

if(EVENT == 197) then
	SaveEvent(UID, 7430)
end

if (EVENT == 195) then
	MonsterCount01 = CountMonsterQuestSub(UID, 1220, 1);
	MonsterCount02 = CountMonsterQuestSub(UID, 1220, 2);
	MonsterCount03 = CountMonsterQuestSub(UID, 1220, 3);
	MonsterCount04 = CountMonsterQuestSub(UID, 1220, 4);
	if (MonsterCount01 > 2 and MonsterCount02 > 2 and MonsterCount03 > 2 and MonsterCount04 > 2) then 
	SelectMsg(UID, 4, 1220, 43708, NPC, 10, 196, 27, -1);
	else
		if (MonsterCount01 < 3) then
			SelectMsg(UID, 2, 1220, 43706, NPC, 18, 198);
		elseif ( MonsterCount02 < 3) then
			SelectMsg(UID, 2, 1220, 43706, NPC, 18, 199);
		elseif ( MonsterCount03 < 3) then
			SelectMsg(UID, 2, 1220, 43706, NPC, 18, 200);
		elseif ( MonsterCount04 < 3) then
			SelectMsg(UID, 2, 1220, 43706, NPC, 18, 201);
		end
	end
end
	
if(EVENT == 198) then
	ShowMap(UID, 580);
end

if(EVENT == 199) then
	ShowMap(UID, 576);
end

if(EVENT == 200) then
	ShowMap(UID, 572);
end

if(EVENT == 201) then
	ShowMap(UID, 523);
end

if(EVENT == 196) then
	RunQuestExchange(UID, 6014);
	SaveEvent(UID, 7429);
end

if(EVENT == 202) then
	SelectMsg(UID, 2, 1221, 43710, NPC, 40175, 203);
end

if(EVENT == 203) then
	SelectMsg(UID, 2, 1221, 43711, NPC, 40176, 204);
end

if(EVENT == 204) then
	SelectMsg(UID, 2, 1221, 43712, NPC, 40177, 208);
end

if(EVENT == 208) then
	SelectMsg(UID, 2, 1221, 43713, NPC, 40178, 206,40179,-1);
end

if(EVENT == 206) then
	SaveEvent(UID, 7434);
end

if(EVENT == 207) then
	SaveEvent(UID, 7436);
end

if(EVENT == 205) then
	CountA = HowmuchItem(UID, 810418000)
	if( CountA < 2) then
		SelectMsg(UID, 2, 1221, 43715, NPC, 18, -1);
	else
		SelectMsg(UID, 4, 1221, 43715, NPC, 41, 209, 27, -1);
	end
end

if(EVENT == 209) then
	RunQuestExchange(UID, 6015);
	SaveEvent(UID, 7435);
end

if(EVENT == 212) then
	SelectMsg(UID, 2, 1222, 43716, NPC, 40181, 213,40182,-1);
end

if(EVENT == 213) then
	SelectMsg(UID, 2, 1222, 43717, NPC, 40183, -1);
	SaveEvent(UID, 7440);
end

if(EVENT == 214) then
	SaveEvent(UID, 7443);
end

if(EVENT == 217) then
	SaveEvent(UID, 7442);
end

if(EVENT == 215) then
	CountA = HowmuchItem(UID, 900608000)
	if( CountA < 1) then
		SelectMsg(UID, 2, 1222, 43718, NPC, 18,-1);
	else
		SelectMsg(UID, 4, 1222, 43718, NPC, 41, 216, 27, -1);
	end
end

if(EVENT == 216) then
	RunQuestExchange(UID, 6016);
	SaveEvent(UID, 7441);
end

if(EVENT == 222) then
	SelectMsg(UID, 2, 1223, 43719, NPC, 40320, 223);
end

if(EVENT == 223) then
	SelectMsg(UID, 2, 1223, 43720, NPC, 40192, 224,40193,-1);
end

if(EVENT == 224) then
	SelectMsg(UID, 2, 1223, 43721, NPC, 40142,-1);
	SaveEvent(UID, 7446);
end

if(EVENT == 225) then
	QuestStatusCheck = GetQuestStatus(UID, 1223)	
	if(QuestStatusCheck == 1) then
	SelectMsg(UID, 2, 1223, 43721, NPC, 40142,-1);
		else
    SelectMsg(UID, 4, 1223, 43722, NPC, 10, 226,27,-1);
	end
end

if(EVENT == 226) then
	RunQuestExchange(UID, 6017);
	SaveEvent(UID, 7447);
end

if(EVENT == 232) then
	SelectMsg(UID, 4, 1226, 43723, NPC, 22, 233,23,-1);
end

if(EVENT == 233) then
	SaveEvent(UID, 7464);
end

if(EVENT == 237) then
	SaveEvent(UID, 7466);
end

if(EVENT == 235) then
	CountA = HowmuchItem(UID, 900609000)
	if( CountA < 1) then
		SelectMsg(UID, 2, 1226, 43725, NPC, 18,236);
	else
		SelectMsg(UID, 4, 1226, 43725, NPC, 40185, 238, 23, -1);
	end
end

if (EVENT == 236) then
	ShowMap(UID, 1197);
end

if(EVENT == 238) then
SelectMsg(UID, 2, 1226, 44076, NPC, 10,-1);
	RunQuestExchange(UID, 6020);
	SaveEvent(UID, 7465);
end

if(EVENT == 242) then
	SelectMsg(UID, 4, 1227, 43726, NPC, 8280, 243,23,-1);
end

if(EVENT == 243) then
	SaveEvent(UID, 7470);
end

if(EVENT == 247) then
	SaveEvent(UID, 7472);
end

if (EVENT == 245) then
	ITEMA  = HowmuchItem(UID, 810418000);
	ITEMB  = HowmuchItem(UID, 900678000);
	if(ITEMA > 2 and ITEMB > 0) then
		SelectMsg(UID, 4, 1227, 43728, NPC, 4452, 248, 27, -1);
	else
		SelectMsg(UID, 2, 1227, 43726, NPC, 10, 246);
	end
end

if (EVENT == 246) then
	ShowMap(UID, 22);
end

if(EVENT == 248) then
	RunQuestExchange(UID, 6021);
	SaveEvent(UID, 7471);
end

if(EVENT == 252) then
		SelectMsg(UID, 2, 1228, 43729, NPC, 8280, 253);
end

if(EVENT == 253) then
	SelectMsg(UID, 2, 1228, 43730, NPC, 40186, -1);
	SaveEvent(UID, 7476);
	--SaveEvent(UID, 7478)
end

if(EVENT == 255) then
	QuestStatusCheck = GetQuestStatus(UID, 1228)	
	if(QuestStatusCheck == 1) then
		SelectMsg(UID, 2, 1228, 43730, NPC, 40186, -1);
	else
		SelectMsg(UID, 2, 1228, 43731, NPC, 40187, 256,27,-1);	
	end
end

if(EVENT == 256) then
	RunQuestExchange(UID, 6022);
	SaveEvent(UID, 7477);
end

if(EVENT == 262) then
	SelectMsg(UID, 4, 1229, 43732, NPC, 22, 263,23,-1);
end

if(EVENT == 263) then
	SaveEvent(UID, 7482);
end

if(EVENT == 267) then
	SaveEvent(UID, 7484);
end

if(EVENT == 265) then
	MonsterCount  = CountMonsterQuestSub(UID, 1229, 1);
	if (MonsterCount < 1) then
		SelectMsg(UID, 2, 1229, 43734, NPC, 18, 266);
	else
		SelectMsg(UID, 4, 1229, 43734, NPC, 40189, 267, 22, -1);
	end
end

if (EVENT == 266) then
	ShowMap(UID, 1257);
end

if(EVENT == 267) then
	RunQuestExchange(UID, 6023);
	SaveEvent(UID, 7483);
end

if(EVENT == 272) then
	SelectMsg(UID, 4, 1230, 43735, NPC, 65, 273,23,-1);
end

if(EVENT == 273) then
	RunQuestExchange(UID, 6024);
	SaveEvent(UID, 7489);
end

if(EVENT == 182) then
	SelectMsg(UID, 2, 1219, 43702, NPC, 40157, 183);
end

if(EVENT == 183) then
	SaveEvent(UID, 7422);
end

if(EVENT == 187) then
	SaveEvent(UID, 7424);
end

if(EVENT == 185) then
	CountA = HowmuchItem(UID, 810418000)	
	if( CountA < 4) then
		SelectMsg(UID, 2, 1219, 43704, NPC, 18, 186);
	else
		SelectMsg(UID, 4, 1219, 43704, NPC, 4319, 188);
	end
end

if (EVENT == 186 ) then
	ShowMap(UID, 1);
end

if(EVENT == 188) then
	RunQuestExchange(UID, 6013);
	SaveEvent(UID, 7423);
end

if(EVENT == 282) then
	SelectMsg(UID, 2, 1231, 43737, NPC, 40194, 283);
end

if(EVENT == 283) then
	SelectMsg(UID, 2, 1231, 43738, NPC, 40195, 284);
end

if(EVENT == 284) then
	SelectMsg(UID, 2, 1231, 43739, NPC, 8280, 286);
end

if(EVENT == 286) then
	SaveEvent(UID, 7494);

end

if(EVENT == 287) then
	SaveEvent(UID, 7496);
end

if(EVENT == 285) then
	CountA = HowmuchItem(UID, 900614000)	
	if( CountA < 1) then
		SelectMsg(UID, 2, 1231, 43739, NPC, 18, 288);
	else
		SelectMsg(UID, 2, 1231, 43741, NPC, 40196, 289);
	end
end

if (EVENT == 288 ) then
	ShowMap(UID, 1257);
end

if(EVENT == 289) then
	SaveEvent(UID, 7495);
	SaveEvent(UID, 7500);
end