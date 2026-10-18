-- =============================================
-- RİMA GUARD  //  www.RimaGUARD.com 
-- Knight Online Pvp v24xx Server Files & AntiCheat System
-- =============================================
local NPC = 24406;

if (EVENT == 190) then
	--RunQuestExchange(UID,941,0,1);
	QuestNum = SearchQuest(UID, NPC);
	if (QuestNum == 0) then 
		SelectMsg(UID, 2, -1, 1312, NPC, 10, -1);
	elseif (QuestNum > 1 and  QuestNum < 100) then
		NpcMsg(UID, 1313, NPC)
	else
		EVENT = QuestNum
	end
end

if (EVENT == 1002) then
	SelectMsg(UID, 2, 427, 8157, NPC, 10, 1010);
end

if (EVENT == 1010) then
	SelectMsg(UID, 4, 427, 8236, NPC, 22, 1003, 23, -1);
end

if (EVENT == 1003) then
	QuestStatus = GetQuestStatus(UID, 427)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2158);
	end
end

if (EVENT == 1005) then
	QuestStatus = GetQuestStatus(UID, 427)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 427, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 427, 8417, NPC, 18, 1008);
		else
			SelectMsg(UID, 2, 427, 8416, NPC, 3007, -1);
			SaveEvent(UID, 2160);
		end
	end
end

if (EVENT == 1007) then
	QuestStatus = GetQuestStatus(UID, 427)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 427, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 427, 8417, NPC, 18, 1008);
		else
			SelectMsg(UID, 4, 427, 8237, NPC, 10, 1009, 27, -1);
		end
	end
end

if (EVENT == 1008) then
	ShowMap(UID, 545);
end

if (EVENT == 1009) then
	QuestStatus = GetQuestStatus(UID, 427)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 427, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 427, 8417, NPC, 18, 1008);
		else
			RunQuestExchange(UID,1202);
			SaveEvent(UID, 2159);
		end
	end
end

if (EVENT == 8052) then
	SelectMsg(UID, 2, 203, 8235, NPC, 10, 8060);
end

if (EVENT == 8060) then
	SelectMsg(UID, 4, 203, 8236, NPC, 22, 8053, 23, -1);
end

if (EVENT == 8053) then
	QuestStatus = GetQuestStatus(UID, 203)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 8974);
	end
end

if (EVENT == 8055) then
	QuestStatus = GetQuestStatus(UID, 203)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 203, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 203, 8417, NPC, 18, 8058);
		else
			SelectMsg(UID, 2, 203, 8416, NPC, 3007, -1);
			SaveEvent(UID, 8976);
		end
	end
end

if (EVENT == 8057) then
	QuestStatus = GetQuestStatus(UID, 203)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 203, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 203, 8417, NPC, 18, 8058);
		else
			SelectMsg(UID, 4, 203, 8237, NPC, 41, 8059, 27, -1);
		end
	end
end

if (EVENT == 8058) then
	ShowMap(UID, 545);
end

if (EVENT == 8059) then
	QuestStatus = GetQuestStatus(UID, 203)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 203, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 203, 8417, NPC, 18, 8058);
		else
			RunQuestExchange(UID,951);
			SaveEvent(UID, 8975);
		end
	end
end

if (EVENT == 9512) then
	SelectMsg(UID, 2, 210, 8768, NPC, 10, 9515);
end

if (EVENT == 9515) then
	SelectMsg(UID, 4, 210, 8768, NPC, 22, 9513, 23, -1);
end

if (EVENT == 9513) then
	QuestStatus = GetQuestStatus(UID, 210)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then --Warrior
			SaveEvent(UID, 9655);
		elseif (Class == 2 or Class == 7 or Class == 8) then --Rogue
			SaveEvent(UID, 9640);
		elseif (Class == 3 or Class == 9 or Class == 10) then --Mage
			SaveEvent(UID, 9645);
		elseif (Class == 4 or Class == 11 or Class == 12) then --Priest
			SaveEvent(UID, 9650);
		end
	end
end

if (EVENT == 9520) then
	QuestStatus = GetQuestStatus(UID, 210)	
	if(QuestStatus == 2) then
		SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
	else
		MonsterCount = CountMonsterQuestSub(UID, 210, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 210, 8417, NPC, 18, 9517);
		else
			Class = CheckClass(UID);
			if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then --Warrior
				SaveEvent(UID, 9657);
				SelectMsg(UID, 2, -1, 8418, NPC, 3007, -1);
			elseif (Class == 2 or Class == 7 or Class == 8) then --Rogue
				SaveEvent(UID, 9642);
				SelectMsg(UID, 2, -1, 8418, NPC, 3007, -1);
			elseif (Class == 3 or Class == 9 or Class == 10) then --Mage
				SaveEvent(UID, 9647);
				SelectMsg(UID, 2, -1, 8418, NPC, 3007, -1);
			elseif (Class == 4 or Class == 11 or Class == 12) then --Priest
				SaveEvent(UID, 9652);
				SelectMsg(UID, 2, -1, 8418, NPC, 3007, -1);
			end
		end
	end
end

if (EVENT == 9516) then
	QuestStatus = GetQuestStatus(UID, 210)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 210, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 210, 8417, NPC, 18, 9517);
		else
			SelectMsg(UID, 5, 210, 8237, NPC, 41, 9518, 27, -1);
		end
	end
end

if (EVENT == 9517) then
	ShowMap(UID, 508);
end

if (EVENT == 9518) then
	QuestStatus = GetQuestStatus(UID, 210)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 210, 1);
		if (MonsterCount < 40) then
			SelectMsg(UID, 2, 210, 8417, NPC, 18, 9517);
		else
	Class = CheckClass(UID);
		if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then --Warrior
			RunQuestExchange(UID,1138,STEP,1); 
			SaveEvent(UID, 9641);
		elseif (Class == 2 or Class == 7 or Class == 8) then --Rogue
			RunQuestExchange(UID,1139,STEP,1);
			SaveEvent(UID, 9646);
		elseif (Class == 3 or Class == 9 or Class == 10) then --Mage
			RunQuestExchange(UID,1140,STEP,1);
			SaveEvent(UID, 9651);
		elseif (Class == 4 or Class == 11 or Class == 12) then --Priest
			RunQuestExchange(UID,1141,STEP,1);
			SaveEvent(UID, 9656);
			end
		end
	end
end

if (EVENT == 202) then
	SelectMsg(UID, 2, 461, 8158, NPC, 10, 210);
end

if (EVENT == 210) then
	SelectMsg(UID, 4, 461, 8236, NPC, 22, 203, 23, -1);
end

if (EVENT == 203) then
	QuestStatus = GetQuestStatus(UID, 461)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2218);
	end
end

if (EVENT == 205) then
	QuestStatus = GetQuestStatus(UID, 461)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount  = CountMonsterQuestSub(UID, 461, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 461, 8417, NPC, 18, 208);
		else
			SelectMsg(UID, 2, 461, 8416, NPC, 3007, -1);
			SaveEvent(UID, 2220);
		end
	end
end

if (EVENT == 207) then
	QuestStatus = GetQuestStatus(UID, 461)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount  = CountMonsterQuestSub(UID, 461, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 461, 8417, NPC, 18, 208);
		else
			SelectMsg(UID, 4, 461, 8237, NPC, 41, 209, 27, -1);
		end
	end
end

if (EVENT == 208) then
	ShowMap(UID, 58);
end

if (EVENT == 209) then
	QuestStatus = GetQuestStatus(UID, 461)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount  = CountMonsterQuestSub(UID, 461, 1);
		if (MonsterCount < 10) then
			SelectMsg(UID, 2, 461, 8417, NPC, 18, 208);
		else
			RunQuestExchange(UID,1940);
			SaveEvent(UID, 2219);
		end
	end
end


if (EVENT == 8452) then
	SelectMsg(UID, 2, 225, 8239, NPC, 10, 8460);
end

if (EVENT == 8460) then
	SelectMsg(UID, 4, 225, 8240, NPC, 22, 8453, 23, -1);
end

if (EVENT == 8453) then
	QuestStatus = GetQuestStatus(UID, 225)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9034);
	end
end

if (EVENT == 8455) then
	QuestStatus = GetQuestStatus(UID, 225)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 225, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 225, 8417, NPC, 18, 8458);
		else
			SelectMsg(UID, 2, 225, 8416, NPC, 3014, -1);
			SaveEvent(UID, 9036);
		end
	end
end

if (EVENT == 8457) then
	QuestStatus = GetQuestStatus(UID, 225)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 225, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 225, 8417, NPC, 18, 8458);
		else
			SelectMsg(UID, 5, 225, 8241, NPC, 41, 8459, 27, -1);
		end
	end
end

if (EVENT == 8458) then
	ShowMap(UID, 58);
end

if (EVENT == 8459) then
	QuestStatus = GetQuestStatus(UID, 225)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 225, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 225, 8417, NPC, 18, 8458);
		else
			Class = CheckClass(UID);
			if (Class == 1 or Class == 5 or Class == 6 or Class == 13 or Class == 14 or Class == 15) then --Warrior
				GiveItem(UID,925002595)
			elseif (Class == 2 or Class == 7 or Class == 8) then --Rogue
				GiveItem(UID,925007596)
			elseif (Class == 3 or Class == 9 or Class == 10) then --Mage
				GiveItem(UID,926002597)
			elseif (Class == 4 or Class == 11 or Class == 12) then --Priest
				GiveItem(UID,926007598)
			elseif (Class == 13 or Class == 14 or Class == 15) then --KurainPorto
				GiveItem(UID,925002595)
			end
			--RunQuestExchange(UID,941,STEP,1);
			SaveEvent(UID, 9035);
		end
	end
end

if (EVENT == 302) then
	SelectMsg(UID, 2, 477, 8157, NPC, 10, 310);
end

if (EVENT == 310) then
	SelectMsg(UID, 4, 477, 8236, NPC, 22, 303, 23, -1);
end

if (EVENT == 303) then
	QuestStatus = GetQuestStatus(UID, 477)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2314);
	end
end

if (EVENT == 305) then
	QuestStatus = GetQuestStatus(UID, 477)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 477, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 477, 8417, NPC, 18, 308);
		else
			SelectMsg(UID, 2, 477, 8416, NPC, 3007, -1);
			SaveEvent(UID, 2316);
		end
	end
end

if (EVENT == 307) then
	QuestStatus = GetQuestStatus(UID, 477)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 477, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 477, 8417, NPC, 18, 308);
		else
			SelectMsg(UID, 4, 477, 8237, NPC, 41, 309, 27, -1);
		end
	end
end

if (EVENT == 308) then
	ShowMap(UID, 703);
end

if (EVENT == 309) then
	QuestStatus = GetQuestStatus(UID, 477)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 477, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 477, 8417, NPC, 18, 308);
		else
			RunQuestExchange(UID,11090);
			SaveEvent(UID, 2315);
		end
	end
end

if (EVENT == 402) then
	SelectMsg(UID, 2, 480, 8157, NPC, 10, 410);
end

if (EVENT == 410) then
	SelectMsg(UID, 4, 480, 8236, NPC, 22, 403, 23, -1);
end

if (EVENT == 403) then
	QuestStatus = GetQuestStatus(UID, 480)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 2338);
	end
end

if (EVENT == 405) then
	QuestStatus = GetQuestStatus(UID, 480)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 480, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 480, 8417, NPC, 18, 408);
		else
			SelectMsg(UID, 2, 480, 8416, NPC, 3007, -1);
			SaveEvent(UID, 2340);
		end
	end
end

if (EVENT == 407) then
	QuestStatus = GetQuestStatus(UID, 480)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 480, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 480, 8417, NPC, 18, 408);
		else
			SelectMsg(UID, 4, 480, 8237, NPC, 41, 409, 27, -1);
		end
	end
end

if (EVENT == 408) then
	ShowMap(UID, 601);
end

if (EVENT == 409) then
	QuestStatus = GetQuestStatus(UID, 480)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 480, 1);
		if (MonsterCount < 30) then
			SelectMsg(UID, 2, 480, 8417, NPC, 18, 408);
		else
			RunQuestExchange(UID,11093);
			SaveEvent(UID, 2339);
		end
	end
end

if (EVENT == 9332) then
	SelectMsg(UID, 2, 447, 8678, NPC, 10, 9340);
end

if (EVENT == 9340) then
	SelectMsg(UID, 4, 447, 8678, NPC, 22, 9333, 23, -1);
end

if (EVENT == 9333) then
	QuestStatus = GetQuestStatus(UID, 447)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9352);
	end
end

if (EVENT == 9335) then
	QuestStatus = GetQuestStatus(UID, 447)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 447, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 447, 8575, NPC, 10, 9338);
		else
			SaveEvent(UID, 9354);
			SelectMsg(UID, 2, 447, 8416, NPC, 3014, -1);
		end
	end
end

if (EVENT == 9337) then
	QuestStatus = GetQuestStatus(UID, 447)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 447, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 447, 8575, NPC, 10, 9338);
		else
			SelectMsg(UID, 4, 447, 8581, NPC, 41, 9339, 27, -1);
		end
	end
end

if (EVENT == 9338) then
	ShowMap(UID, 702);
end

if (EVENT == 9339) then
	QuestStatus = GetQuestStatus(UID, 447)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 447, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 447, 8575, NPC, 10, 9338);
		else
			RunQuestExchange(UID,1090);
			SaveEvent(UID, 9353);
		end
	end
end

if (EVENT == 9352) then
	SelectMsg(UID, 2, 272, 8682, NPC, 10, 9360);
end

if (EVENT == 9360) then
	SelectMsg(UID, 4, 272, 8236, NPC, 22, 9353, 23, -1);
end

if (EVENT == 9353) then
	QuestStatus = GetQuestStatus(UID, 272)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
			SaveEvent(UID, 9376);
	end
end

if (EVENT == 9355) then
	QuestStatus = GetQuestStatus(UID, 272)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 272, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 272, 8557, NPC, 18, 9358);
		else
			SaveEvent(UID, 9378);
			SelectMsg(UID, 2, 272, 8416, NPC, 3014, -1);
		end
	end
end

if (EVENT == 9357) then
	QuestStatus = GetQuestStatus(UID, 272)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 272, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 272, 8557, NPC, 18, 9358);
		else
			SelectMsg(UID, 4, 272, 8569, NPC, 41, 9359, 27, -1);
		end
	end
end

if (EVENT == 9358) then
	ShowMap(UID, 601);
end

if (EVENT == 9359) then
	QuestStatus = GetQuestStatus(UID, 272)	
		if(QuestStatus == 2) then
			SelectMsg(UID, 2, -1, 44614, NPC, 10, -1);
		else
	MonsterCount = CountMonsterQuestSub(UID, 272, 1);
		if (MonsterCount < 20) then
			SelectMsg(UID, 2, 272, 8557, NPC, 18, 9358);
		else
			RunQuestExchange(UID,1093);
			SaveEvent(UID, 9377);
		end
	end
end